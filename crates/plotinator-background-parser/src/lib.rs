use plotinator_supported_formats::SupportedFormat;
use plotinator_ui_file_io::{ParseUpdate, UpdateChannel};
use smallvec::{SmallVec, smallvec};
use std::{path::PathBuf, sync::mpsc::Sender, thread};

use crate::loaded_format::LoadedSupportedFormat;

pub mod loaded_format;

pub struct ParserThreads {
    update_tx: UpdateChannel,
    threads: Vec<ParserThread>,
}

impl ParserThreads {
    pub fn new(tx: Sender<ParseUpdate>) -> Self {
        Self {
            update_tx: UpdateChannel::new(tx),
            threads: vec![],
        }
    }

    pub fn parse_path(&mut self, path: PathBuf) {
        let new_thread = ParserThread::new(path, self.update_tx.clone());
        self.threads.push(new_thread);
    }

    pub fn poll(&mut self) -> SmallVec<[LoadedSupportedFormat; 1]> {
        let mut loaded_formats = smallvec![];
        let running_threads: Vec<_> = self.threads.drain(..).collect();
        for t in running_threads {
            if t.is_finished() {
                if let Some(lf) = t.finish() {
                    loaded_formats.push(lf);
                }
            } else {
                self.threads.push(t);
            }
        }
        loaded_formats
    }
}

pub struct ParserThread {
    path: PathBuf,
    update_tx: UpdateChannel,
    handle: Option<thread::JoinHandle<anyhow::Result<LoadedSupportedFormat>>>,
}

impl ParserThread {
    pub fn new(path: PathBuf, update_tx: UpdateChannel) -> Self {
        let handle = thread::Builder::new()
            .name(path.to_string_lossy().into_owned())
            .spawn({
                let p = path.clone();
                let update_tx = update_tx.clone();
                move || {
                    let parsed_format = SupportedFormat::parse_from_path(&p, update_tx.clone())?;
                    update_tx.send(ParseUpdate::Progress {
                        path: p.clone(),
                        progress: 50.,
                    });
                    let loaded_format = LoadedSupportedFormat::new(parsed_format);
                    update_tx.send(ParseUpdate::Progress {
                        path: p.clone(),
                        progress: 99.,
                    });
                    Ok(loaded_format)
                }
            })
            .expect("Failed spawning parser thread");
        Self {
            path,
            update_tx,
            handle: Some(handle),
        }
    }

    pub fn is_finished(&self) -> bool {
        debug_assert!(
            self.handle.is_some(),
            "called is_finished on a parser thread that should've been finished/consumed"
        );
        self.handle.as_ref().is_some_and(|h| h.is_finished())
    }

    pub fn finish(mut self) -> Option<LoadedSupportedFormat> {
        let h = self
            .handle
            .take()
            .expect("tried finishing a thread that was already finished");

        match h.join() {
            Ok(parse_res) => match parse_res {
                Ok(s) => return Some(s),
                Err(e) => self.update_tx.send(ParseUpdate::Failed {
                    path: self.path.clone(),
                    error_msg: format!("Not valid: {e}"),
                }),
            },
            Err(e) => {
                self.update_tx.send(ParseUpdate::Failed {
                    path: self.path.clone(),
                    error_msg: format!(
                        "Unexpected crash: {e:?}, please file an issue on the github page"
                    ),
                });
            }
        }
        None
    }
}
