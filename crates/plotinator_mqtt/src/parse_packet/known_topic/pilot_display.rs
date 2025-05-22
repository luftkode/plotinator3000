use serde::Deserialize;

#[derive(Deserialize)]
pub struct PilotDisplayCoordinates {
    lon: f64,
    lat: f64,
}

impl PilotDisplayCoordinates {
    pub fn lat(&self) -> f64 {
        self.lat
    }
    pub fn lon(&self) -> f64 {
        self.lon
    }
}

#[derive(Deserialize)]
pub struct PilotDisplayRemainingDistance {
    distance: f64,
}

impl PilotDisplayRemainingDistance {
    pub fn distance(&self) -> f64 {
        self.distance
    }
}
