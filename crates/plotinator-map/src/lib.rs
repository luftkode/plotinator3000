use egui::Color32;
use walkers::Position;

/// A single point in a path with optional heading
#[derive(Clone, Copy, Debug)]
pub struct PathPoint {
    pub position: Position,
    pub heading: Option<f64>, // Heading in degrees (0 = North, 90 = East, etc.)
    pub altitude: Option<f32>,
}

impl PathPoint {
    pub fn new(lon: f64, lat: f64) -> Self {
        Self {
            position: Position::new(lon, lat),
            heading: None,
            altitude: None,
        }
    }

    pub fn with_heading(lon: f64, lat: f64, heading: f64) -> Self {
        Self {
            position: Position::new(lon, lat),
            heading: Some(heading),
            altitude: None,
        }
    }

    pub fn from_position(position: Position) -> Self {
        Self {
            position,
            heading: None,
            altitude: None,
        }
    }

    pub fn from_position_with_heading(position: Position, heading: f64) -> Self {
        Self {
            position,
            heading: Some(heading),
            altitude: None,
        }
    }
}

/// A path with points and optional custom color
#[derive(Clone, Debug)]
pub struct PathData {
    pub points: Vec<PathPoint>,
    pub color: Option<Color32>,
}

impl PathData {
    pub fn new(points: Vec<PathPoint>) -> Self {
        Self {
            points,
            color: None,
        }
    }

    pub fn with_color(points: Vec<PathPoint>, color: Color32) -> Self {
        Self {
            points,
            color: Some(color),
        }
    }

    /// Helper to create from simple (lat, lon) pairs
    pub fn from_coords(coords: Vec<(f64, f64)>) -> Self {
        let points = coords
            .iter()
            .map(|(lat, lon)| PathPoint::new(*lon, *lat))
            .collect();
        Self::new(points)
    }

    /// Helper to create from (lat, lon, heading) tuples
    pub fn from_coords_with_heading(coords: Vec<(f64, f64, f64)>) -> Self {
        let points = coords
            .iter()
            .map(|(lat, lon, heading)| PathPoint::with_heading(*lon, *lat, *heading))
            .collect();
        Self::new(points)
    }
}
