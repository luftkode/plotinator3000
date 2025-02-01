#[cfg(feature = "hdf")]
fn main() -> eframe::Result {
    plotinator3000::run_app()
}

#[cfg(not(feature = "hdf"))]
fn main() {
    panic!("This binary requires the HDF feature to be enabled");
}
