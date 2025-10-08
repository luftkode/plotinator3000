use crate::util;

#[derive(Debug)]
pub(crate) struct RootMetadata {
    linear_time: f64,
    kickin_usec: f64,
    hm_ontime_usec: f64,
    gates_pr_decade_on: f64,
    metadata: Vec<(String, String)>,
}

impl RootMetadata {
    pub fn parse_from_tsc(h5: &hdf5::File) -> anyhow::Result<Self> {
        let mut metadata = vec![];
        for attr_name in h5.attr_names()? {
            let attr = h5.attr(&attr_name)?;
            let attr_str = util::read_any_attribute_to_string(&attr)?;
            metadata.push((attr_name, attr_str));
        }

        let linear_time = h5.attr("t_lin")?.read_scalar::<f64>()?;
        let kickin_usec = h5.attr("kickin_usec")?.read_scalar::<f64>()?;
        let hm_ontime_usec = h5.attr("hm_ontime_usec")?.read_scalar::<f64>()?;
        let gates_pr_decade_on = h5.attr("gates_pr_decade_on")?.read_scalar()?;

        Ok(Self {
            linear_time,
            kickin_usec,
            hm_ontime_usec,
            gates_pr_decade_on,
            metadata,
        })
    }
    /// 0-based
    pub(crate) fn last_gate_on_index(&self) -> usize {
        self.last_gate_on_count() - 1
    }
    /// 1-based (count)
    pub(crate) fn last_gate_on_count(&self) -> usize {
        let k_ramp_up = gates_pr_decade_to_k(self.gates_pr_decade_on);
        let k_ontime = gates_pr_decade_to_k(self.gates_pr_decade_on);
        let gate_kick_in = sinh_gates(self.kickin_usec, self.linear_time, k_ramp_up);

        let gate_from_kick_in_to_ramp_down: Vec<f64> = sinh_gates(
            self.hm_ontime_usec - self.kickin_usec,
            self.linear_time,
            k_ontime,
        )
        .into_iter()
        .map(|x| x + self.kickin_usec)
        .collect();

        gate_kick_in.len() + gate_from_kick_in_to_ramp_down.len()
    }

    pub(crate) fn metadata_strings(&self) -> Vec<(String, String)> {
        self.metadata.clone()
    }
}

fn gates_pr_decade_to_k(gpd: f64) -> f64 {
    1.0 / (gpd * std::f64::consts::E.log10())
}

fn sinh_gates(t: f64, t_lin: f64, k: f64) -> Vec<f64> {
    // Calculate n_max = ceil((1/k) * arcsinh(t / t_lin))
    let n_max = ((1.0 / k) * (t / t_lin).asinh()).ceil() as usize;

    // Build tgates = [t_lin * sinh(k * n) for n in 1..=n_max]
    let mut tgates: Vec<f64> = (1..=n_max)
        .map(|n| t_lin * (k * (n as f64)).sinh())
        .collect();

    // Replace last element with t
    if let Some(last) = tgates.last_mut() {
        *last = t;
    }

    tgates
}

#[cfg(test)]
mod tests {
    use plotinator_test_util::test_file_defs::tsc::tsc;
    use testresult::TestResult;

    use super::*;

    #[test]
    fn test_read_tsc_attributes() -> TestResult {
        let h5file = hdf5::File::open(tsc())?;

        let root_metadata = RootMetadata::parse_from_tsc(&h5file)?;

        let last_on = root_metadata.last_gate_on_count();
        assert_eq!(last_on, 15);

        Ok(())
    }
}
