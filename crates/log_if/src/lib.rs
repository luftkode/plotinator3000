use std::{fmt::Display, io};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use util::ExpectedPlotRange;

pub mod util;

pub trait Plotable {
    /// Returns a slice of all the plottable data.
    fn raw_plots(&self) -> &[RawPlot];
    /// Return the first timestamp, meaning the timestamp of the first entry
    fn first_timestamp(&self) -> DateTime<Utc>;
    /// A name that uniquely identifies the type of log
    fn unique_name(&self) -> &str;
}

/// A given log should implement this trait
pub trait Log: Plotable + Clone + Display + Send + Sync + Sized {
    type Entry: LogEntry;
    /// Create a [Log] instance from a reader
    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self>;
    /// Return a borrowed slice (list) of log entries
    fn entries(&self) -> &[Self::Entry];
}

/// A given log entry should implement this trait
pub trait LogEntry: Sized + Display + Send + Sync {
    /// Create a [`LogEntry`] instance from a reader
    fn from_reader<R: io::Read>(reader: &mut R) -> io::Result<Self>;
    /// Timestamp in nanoseconds
    fn timestamp_ns(&self) -> f64;
}

/// [`RawPlot`] represents some plottable data from a log, e.g. RPM measurements
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RawPlot {
    name: String,
    points: Vec<[f64; 2]>,
    expected_range: ExpectedPlotRange,
}

impl RawPlot {
    pub fn new(name: String, points: Vec<[f64; 2]>, expected_range: ExpectedPlotRange) -> Self {
        Self {
            name,
            points,
            expected_range,
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn points(&self) -> &[[f64; 2]] {
        &self.points
    }
    pub fn expected_range(&self) -> ExpectedPlotRange {
        self.expected_range
    }
}

/// A given log header should implement this
pub trait GitMetadata {
    fn project_version(&self) -> String;
    fn git_short_sha(&self) -> String;
    fn git_branch(&self) -> String;
    fn git_repo_status(&self) -> String;
}
