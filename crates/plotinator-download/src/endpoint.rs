#[derive(Debug, Clone, Copy, strum::Display)]
pub enum Endpoint {
    #[strum(serialize = "/api/download/latest")]
    DownloadLatestData,
    #[strum(serialize = "/api/download/today")]
    DownloadTodaysData,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_str_eq;

    #[test]
    fn test_serialize_endpoint() {
        let endpoint = Endpoint::DownloadLatestData;

        assert_str_eq!(endpoint.to_string(), "/api/download/latest");
    }
}
