use chrono::{DateTime, Utc};
use derive_more::derive::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Display, PartialEq, Deserialize, Serialize)]
#[display("MA{id} {timestamp}: {field_nanotesla}")]
pub struct MagSensor {
    pub id: u8,
    timestamp: DateTime<Utc>,
    field_nanotesla: f64,
}

impl MagSensor {
    pub(crate) fn timestamp_ns(&self) -> f64 {
        self.timestamp
            .timestamp_nanos_opt()
            .expect("timestamp as nanoseconds out of range") as f64
    }
}
