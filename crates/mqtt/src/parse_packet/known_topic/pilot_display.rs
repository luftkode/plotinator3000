use serde::Deserialize;

#[derive(Deserialize)]
pub struct PilotDisplaySpeedPacket {
    #[serde(rename(deserialize = "Speed"))]
    speed: String,
}

impl PilotDisplaySpeedPacket {
    pub fn speed(self) -> f64 {
        self.speed
            .parse()
            .expect("Failed to parse PilotDisplaySpeedPacket")
    }
}

#[derive(Deserialize)]
pub struct PilotDisplayAltitudePacket {
    #[serde(rename(deserialize = "Height"))]
    height: String,
}

impl PilotDisplayAltitudePacket {
    pub fn height(self) -> f64 {
        self.height
            .parse()
            .expect("Failed to parse PilotDisplayAltitudePacket")
    }
}

#[derive(Deserialize)]
pub struct PilotDisplayHeadingPacket {
    #[serde(rename(deserialize = "Heading"))]
    heading: String,
}

impl PilotDisplayHeadingPacket {
    pub fn heading(self) -> f64 {
        self.heading
            .parse()
            .expect("Failed to parse PilotDisplayHeadingPacket")
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
