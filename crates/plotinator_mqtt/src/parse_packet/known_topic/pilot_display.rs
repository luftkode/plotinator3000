use std::num::ParseFloatError;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct PilotDisplaySpeedPacket {
    #[serde(rename(deserialize = "Speed"))]
    speed: String,
}

impl PilotDisplaySpeedPacket {
    pub fn speed(self) -> Result<f64, ParseFloatError> {
        self.speed.parse()
    }
}

#[derive(Deserialize)]
pub struct PilotDisplayAltitudePacket {
    #[serde(rename(deserialize = "Height"))]
    height: String,
}

impl PilotDisplayAltitudePacket {
    pub fn height(self) -> Result<f64, ParseFloatError> {
        self.height.parse()
    }
}

#[derive(Deserialize)]
pub struct PilotDisplayHeadingPacket {
    #[serde(rename(deserialize = "Heading"))]
    heading: String,
}

impl PilotDisplayHeadingPacket {
    pub fn heading(self) -> Result<f64, ParseFloatError> {
        self.heading.parse()
    }
}

#[derive(Deserialize)]
pub struct PilotDisplayClosestLinePacket {
    distance: f64,
}

impl PilotDisplayClosestLinePacket {
    pub fn distance(&self) -> f64 {
        self.distance
    }
}
