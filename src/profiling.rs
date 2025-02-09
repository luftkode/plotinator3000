pub fn start_puffin_server() {
    puffin::set_scopes_on(true); // tell puffin to collect data

    match puffin_http::Server::new("127.0.0.1:8585") {
        Ok(puffin_server) => {
            log::info!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");

            std::process::Command::new("puffin_viewer")
                .arg("--url")
                .arg("127.0.0.1:8585")
                .spawn()
                .ok();

            // We can store the server if we want, but in this case we just want
            // it to keep running. Dropping it closes the server, so let's not drop it!
            #[allow(clippy::mem_forget)]
            std::mem::forget(puffin_server);
        }
        Err(err) => {
            log::error!("Failed to start puffin server: {err}");
        }
    };
}

pub fn ui_add_keep_repainting_checkbox(ui: &mut egui::Ui, keep_repainting: &mut bool) {
    ui.checkbox(keep_repainting, "Keep repainting");
    if *keep_repainting {
        ui.spinner();
        ui.ctx().request_repaint();
    } else {
        ui.label("Repainting on events (e.g. mouse movement)");
    }
}
