use std::{
    io,
    sync::mpsc::{Receiver, Sender, channel},
};

use crate::app::LoadedFiles;

fn execute<F: std::future::Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

struct WebFileContents {
    name: String,
    contents: Vec<u8>,
}

#[derive(Debug)]
pub struct WebFileDialog {
    file_sender: Sender<WebFileContents>,
    file_receiver: Receiver<WebFileContents>,
}

impl Default for WebFileDialog {
    fn default() -> Self {
        let (file_sender, file_receiver) = channel();
        Self {
            file_sender,
            file_receiver,
        }
    }
}

impl WebFileDialog {
    pub(crate) fn open(&self, ctx: egui::Context) {
        Self::open_dialog(ctx, self.file_sender.clone());
    }

    pub(crate) fn poll_received_files(&self, loaded_files: &mut LoadedFiles) -> io::Result<()> {
        if let Ok(file_web_content) = self.file_receiver.try_recv() {
            log::debug!("Received file: {}", file_web_content.name);
            loaded_files.parse_raw_buffer(&file_web_content.contents)?;
        }
        Ok(())
    }

    fn open_dialog(ctx: egui::Context, sender: Sender<WebFileContents>) {
        let task = rfd::AsyncFileDialog::new().pick_files();

        execute(async move {
            let files = task.await;
            if let Some(files) = files {
                for f in files {
                    let name = f.file_name();
                    let fwebcontents = WebFileContents {
                        name,
                        contents: f.read().await,
                    };
                    let _ = sender.send(fwebcontents);
                    ctx.request_repaint();
                }
            }
        });
    }
}
