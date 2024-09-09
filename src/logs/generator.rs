use std::{
    fmt, fs,
    io::{self, BufRead, BufReader},
    path::Path,
    str::FromStr,
};

use chrono::NaiveDateTime;

use super::{Log, LogEntry};

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct GeneratorLog {
    entries: Vec<GeneratorLogEntry>,
}

impl GeneratorLog {
    pub fn file_is_generator_log(fpath: &Path) -> io::Result<bool> {
        let file = fs::File::open(fpath)?;
        let mut buf_reader = BufReader::new(file);
        let mut first_line = String::new();
        buf_reader.read_line(&mut first_line)?;
        let is_first_line_gen_log_entry =
            GeneratorLogEntry::is_line_valid_generator_log_entry(&first_line);

        Ok(is_first_line_gen_log_entry)
    }
}

impl Log for GeneratorLog {
    type Entry = GeneratorLogEntry;

    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let buf_reader = io::BufReader::new(reader);
        let mut entries = Vec::new();

        for line in buf_reader.lines() {
            let line = line?;
            match GeneratorLogEntry::from_str(&line) {
                Ok(entry) => entries.push(entry),
                Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e)),
            }
        }

        Ok(GeneratorLog { entries })
    }

    fn entries(&self) -> &[Self::Entry] {
        &self.entries
    }
}

impl fmt::Display for GeneratorLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Generator Log:")?;
        for (index, entry) in self.entries.iter().enumerate() {
            writeln!(f, "Entry {}: {}", index + 1, entry)?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct GeneratorLogEntry {
    timestamp: NaiveDateTime,
    vout: f32,
    vbat: f32,
    iout: f32,
    rpm: u32,
    load: f32,
    pwm: f32,
    temp1: f32,
    temp2: f32,
    iin: f32,
    irotor: f32,
    rrotor: f32,
}

impl GeneratorLogEntry {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, io::Error> {
        let mut bufreader = BufReader::new(bytes);
        let mut line = String::new();
        bufreader.read_line(&mut line)?;
        Self::from_str(&line)
    }

    pub fn is_bytes_valid_generator_log_entry(bytes: &[u8]) -> bool {
        let mut bufreader = BufReader::new(bytes);
        let mut line = String::new();
        if bufreader.read_line(&mut line).is_err() {
            return false;
        }
        Self::is_line_valid_generator_log_entry(&line)
    }

    pub fn is_line_valid_generator_log_entry(line: &str) -> bool {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if *parts.get(1).unwrap_or(&"") != "Vout:" {
            return false;
        }
        if *parts.get(3).unwrap_or(&"") != "Vbat:" {
            return false;
        }
        if *parts.get(5).unwrap_or(&"") != "Iout:" {
            return false;
        }
        if *parts.get(7).unwrap_or(&"") != "RPM:" {
            return false;
        }
        if *parts.get(9).unwrap_or(&"") != "Load:" {
            return false;
        }
        if *parts.get(11).unwrap_or(&"") != "PWM:" {
            return false;
        }
        if *parts.get(13).unwrap_or(&"") != "Temp1" {
            return false;
        }
        if *parts.get(15).unwrap_or(&"") != "Temp2" {
            return false;
        }
        if *parts.get(17).unwrap_or(&"") != "IIn:" {
            return false;
        }
        if *parts.get(19).unwrap_or(&"") != "Irotor:" {
            return false;
        }
        if *parts.get(21).unwrap_or(&"") != "Rrotor:" {
            return false;
        }
        true
    }
}

impl LogEntry for GeneratorLogEntry {
    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let mut line = String::new();
        let mut bufreader = BufReader::new(reader);
        bufreader.read_line(&mut line)?;

        Self::from_str(&line)
    }

    fn timestamp_ms(&self) -> u32 {
        todo!()
    }
}

impl fmt::Display for GeneratorLogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{timestamp}: {vout} {vbat} {iout} {rpm} {load} {pwm} {temp1} {temp2} {iin} {irotor} {rrotor}",
            timestamp = self.timestamp,
            vout = self.vout,
            vbat =  self.vbat,
            iout = self.iout,
            rpm = self.rpm,
            load = self.load,
            pwm = self.pwm,
            temp1 = self.temp1,
            temp2 = self.temp2,
            iin = self.iin,
            irotor = self.irotor,
            rrotor = self.rrotor
        )
    }
}

impl FromStr for GeneratorLogEntry {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();

        if parts.len() != 23 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid log entry format",
            ));
        }

        Ok(GeneratorLogEntry {
            timestamp: NaiveDateTime::parse_from_str(parts[0], "%Y%m%d_%H%M%S")
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            vout: parts[2]
                .parse::<f32>()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            vbat: parts[4]
                .parse::<f32>()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            iout: parts[6]
                .parse::<f32>()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            rpm: parts[8]
                .parse::<u32>()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            load: parts[10]
                .parse::<f32>()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            pwm: parts[12]
                .parse::<f32>()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            temp1: parts[14]
                .parse::<f32>()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            temp2: parts[16]
                .parse::<f32>()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            iin: parts[18]
                .parse::<f32>()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            irotor: parts[20]
                .parse::<f32>()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            rrotor: parts[22]
                .parse::<f32>()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use chrono::NaiveDate;
    use testresult::TestResult;

    const TEST_DATA: &str = "test_data/generator/20230124_134738_Gen.log";

    #[test]
    fn test_deserialize() -> TestResult {
        let data = fs::read(TEST_DATA)?;
        let log = GeneratorLog::from_reader(&mut data.as_slice())?;

        let first_entry = log.entries().first().unwrap();
        assert_eq!(
            first_entry.timestamp,
            NaiveDate::from_ymd_opt(2023, 1, 24)
                .unwrap()
                .and_hms_opt(13, 47, 45)
                .unwrap()
        );
        assert_eq!(first_entry.vout, 74.3);
        assert_eq!(first_entry.vbat, 0.1);
        assert_eq!(first_entry.iout, 0.0);
        assert_eq!(first_entry.rpm, 6075);
        assert_eq!(first_entry.load, 10.2);
        assert_eq!(first_entry.pwm, 10.2);
        assert_eq!(first_entry.temp1, 6.9);
        assert_eq!(first_entry.temp2, 8.4);
        assert_eq!(first_entry.iin, 8.8);
        assert_eq!(first_entry.irotor, 0.7);
        assert_eq!(first_entry.rrotor, 11.2);

        let last_entry = log.entries().last().unwrap();
        assert_eq!(
            last_entry.timestamp,
            NaiveDate::from_ymd_opt(2023, 1, 24)
                .unwrap()
                .and_hms_opt(15, 3, 30)
                .unwrap()
        );
        assert_eq!(last_entry.vout, 78.3);
        assert_eq!(last_entry.vbat, 0.1);
        assert_eq!(last_entry.iout, 0.0);
        assert_eq!(last_entry.rpm, 5932);
        assert_eq!(last_entry.load, 9.7);
        assert_eq!(last_entry.pwm, 9.7);
        assert_eq!(last_entry.temp1, 6.9);
        assert_eq!(last_entry.temp2, 8.5);
        assert_eq!(last_entry.iin, 8.3);
        assert_eq!(last_entry.irotor, 0.7);
        assert_eq!(last_entry.rrotor, 11.0);

        Ok(())
    }

    #[test]
    fn test_is_valid_line_valid() -> TestResult {
        let valid_line = "20230124_134852 Vout: 77.8 Vbat: 0.1 Iout: 0.0 RPM: 5925 Load: 17.6 PWM: 17.5 Temp1 7.2 Temp2 9.9 IIn: 61.6 Irotor: 1.4 Rrotor: 9.7";

        assert!(GeneratorLogEntry::is_line_valid_generator_log_entry(
            valid_line
        ));

        Ok(())
    }

    #[test]
    fn test_is_valid_line_invalid() -> TestResult {
        let invalid_line =
            "20230124_134852 Vout: 77.8 Vbat: 0.1 Iout: 0.0 RPM: 5925 something_else: 9.7";

        assert!(!GeneratorLogEntry::is_line_valid_generator_log_entry(
            invalid_line
        ));

        Ok(())
    }

    #[test]
    fn test_is_bytes_valid_line_valid() -> TestResult {
        let valid_line_as_bytes = b"20230124_134745 Vout: 74.3 Vbat: 0.1 Iout: 0.0 RPM: 6075 Load: 10.2 PWM: 10.2 Temp1 6.9 Temp2 8.4 IIn: 8.8 Irotor: 0.7 Rrotor: 11.2
20230124_134746 Vout: 59.3 Vbat: 0.1 Iout: 0.0 RPM: 5438 Load: 81.2 PWM: 18.0 Temp1 6.9 Temp2 8.6 IIn: 35.5 Irotor: 0.9 Rrotor: 11.5";

        assert!(GeneratorLogEntry::is_bytes_valid_generator_log_entry(
            valid_line_as_bytes
        ));
        Ok(())
    }

    #[test]
    fn test_is_bytes_valid_line_invalid() -> TestResult {
        let invalid_bytes = b"20230124_134745 Vo 74. 0.1 Iout: 0.0 RPM: 6075 Load: 10.2 PWM:  8.4 IIn: 8.8 Irotor: 0.7 Rrotor: 11.2
20230124_134746 Vout: 59.3 Vbat: 0.1 Iout: 0.0 RPM: 5438 Load: 81.2 PWM: 18.0 Temp1 6.9 Temp2 8.6 IIn: 35.5 Irotor: 0.9 Rrotor: 11.5";

        assert!(!GeneratorLogEntry::is_bytes_valid_generator_log_entry(
            invalid_bytes
        ));
        Ok(())
    }
}
