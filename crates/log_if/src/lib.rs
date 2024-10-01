pub mod log;
pub mod plotable;
pub mod util;

pub mod prelude {
    pub use crate::log::{GitMetadata, LogEntry, SkytemLog};
    pub use crate::plotable::{ExpectedPlotRange, PlotLabels, Plotable, RawPlot};
    pub use crate::util::*;
}
