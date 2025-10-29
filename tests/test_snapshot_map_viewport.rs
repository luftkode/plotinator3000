#![cfg(not(target_arch = "wasm32"))]
mod util;
use plotinator_hdf5::Tsc;
use plotinator_log_if::prelude::*;
use plotinator_map_ui::{MapViewPort, commander::MapCommand};
use plotinator_test_util::test_file_defs;
use util::*;

#[test]
fn test_snapshot_render_map_default() {
    let mut map_viewport = MapViewPort::default();
    let (_cmd_sender, _msg_receiver) = map_viewport.open();
    let mut harness = Harness::new(|ctx| map_viewport.update_direct(ctx));
    harness.run();
    harness.snapshot("default_map_window");
}

#[test]
fn test_snapshot_render_map_with_tsc_geo_data() {
    let mut map_viewport = MapViewPort::default();
    let (cmd_sender, _msg_receiver) = map_viewport.open();
    let cmd_sender = cmd_sender.unwrap();

    let mut harness = Harness::new(|ctx| map_viewport.update_direct(ctx));

    let tsc = Tsc::from_path(test_file_defs::tsc::tsc()).unwrap();
    for p in tsc.raw_plots() {
        match p {
            RawPlot::Generic { .. } => (),
            RawPlot::GeoSpatialDataset(geo_spatial_dataset) => cmd_sender
                .send(MapCommand::AddGeoData(Box::new(
                    geo_spatial_dataset.clone(),
                )))
                .unwrap(),
        };
    }

    harness.run();
    harness.snapshot("map_with_tsc_geo_data");
}
