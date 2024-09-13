use egui_plot::{Line, PlotPoints};

use crate::logs::LogEntry;

pub fn line_from_log_entry<XF, YF, L: LogEntry>(
    pid_logs: &[L],
    x_extractor: XF,
    y_extractor: YF,
) -> Line
where
    XF: Fn(&L) -> f64,
    YF: Fn(&L) -> f64,
{
    let points: PlotPoints = pid_logs
        .iter()
        .map(|e| [x_extractor(e), y_extractor(e)])
        .collect();
    Line::new(points)
}
