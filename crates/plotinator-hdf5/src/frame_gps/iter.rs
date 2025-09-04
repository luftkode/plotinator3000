//! Contains an iterator abstraction over all the datasets in the frame gps hdf5 file
use hdf5::types::FixedAscii;
use ndarray::{Array2, ArrayView1, Ix1, Ix2, iter::Iter};

/// A struct representing a single frame-gps sample.
#[derive(Debug, PartialEq)]
pub struct GpsEntry<'a> {
    pub gps_time: &'a FixedAscii<30>,
    pub hdop: &'a f64,
    pub pdop: &'a f64,
    pub vdop: &'a f64,
    pub mode: &'a u8,
    pub position: ArrayView1<'a, f64>,
    pub satellites: &'a u8,
    pub speed: &'a f32,
    pub timestamp: &'a i64,
}

/// A container that owns the GPS data arrays read from the HDF5 file.
pub struct GpsData {
    pub(super) gps_time: Array2<FixedAscii<30>>,
    pub(super) hdop: Array2<f64>,
    pub(super) pdop: Array2<f64>,
    pub(super) vdop: Array2<f64>,
    pub(super) mode: Array2<u8>,
    pub(super) position: Array2<f64>,
    pub(super) satellites: Array2<u8>,
    pub(super) speed: Array2<f32>,
    pub(super) timestamp: Array2<i64>,
}

/// An iterator that yields `GpsEntry` items by borrowing from a `GpsData` struct.
pub struct GpsDataIterator<'a> {
    gps_time: Iter<'a, FixedAscii<30>, Ix2>,
    hdop: Iter<'a, f64, Ix2>,
    pdop: Iter<'a, f64, Ix2>,
    vdop: Iter<'a, f64, Ix2>,
    mode: Iter<'a, u8, Ix2>,
    position: ndarray::iter::LanesIter<'a, f64, Ix1>, // Use Lanes iterator for rows
    satellites: Iter<'a, u8, Ix2>,
    speed: Iter<'a, f32, Ix2>,
    timestamp: Iter<'a, i64, Ix2>,
}

impl<'a> Iterator for GpsDataIterator<'a> {
    type Item = GpsEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = GpsEntry {
            gps_time: self.gps_time.next()?,
            hdop: self.hdop.next()?,
            pdop: self.pdop.next()?,
            vdop: self.vdop.next()?,
            mode: self.mode.next()?,
            position: self.position.next()?,
            satellites: self.satellites.next()?,
            speed: self.speed.next()?,
            timestamp: self.timestamp.next()?,
        };
        Some(entry)
    }
}

impl GpsData {
    /// Returns a custom iterator over the GPS data.
    pub fn iter<'a>(&'a self) -> GpsDataIterator<'a> {
        GpsDataIterator {
            gps_time: self.gps_time.iter(),
            hdop: self.hdop.iter(),
            pdop: self.pdop.iter(),
            vdop: self.vdop.iter(),
            mode: self.mode.iter(),
            position: self.position.lanes(ndarray::Axis(1)).into_iter(),
            satellites: self.satellites.iter(),
            speed: self.speed.iter(),
            timestamp: self.timestamp.iter(),
        }
    }
}
