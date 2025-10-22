#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/favicon.ico");

    // Set version information
    res.set("ProductName", "Plotinator3000");
    res.set("FileDescription", "Log viewer app for viewing plots of data from projects such as motor and generator control");
    res.set("CompanyName", "SkyTEM Surveys");
    res.set("OriginalFilename", "plotinator3000.exe");
    res.set("InternalName", "plotinator3000");

    res.set("Comments", "Supports: Mbed Motor Control, Generator logs, Navsys, CSV formats (Njord INS PPP, GrafNav PPP), HDF5 formats (Bifrost TX, Njord Altimeter/INS, Frame sensors, TSC)");
    res.compile()
        .expect("Failed setting custom windows metadata");
}

#[cfg(unix)]
fn main() {}
