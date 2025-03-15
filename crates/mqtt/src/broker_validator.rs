use std::time::{Duration, Instant};

use crate::util;

#[derive(Default)]
pub(crate) struct BrokerValidator {
    previous_broker_input: String,
    broker_status: Option<Result<(), String>>,
    validation_in_progress: bool,
    last_input_change: Option<Instant>,
    broker_validation_receiver: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
}

impl BrokerValidator {
    pub fn broker_status(&self) -> Option<&Result<(), String>> {
        self.broker_status.as_ref()
    }

    pub fn validation_in_progress(&self) -> bool {
        self.validation_in_progress
    }

    pub(crate) fn poll_broker_status(&mut self, ip: &str, port: &str) {
        let current_broker_input = format!("{ip}{port}");

        // Detect input changes
        if current_broker_input != self.previous_broker_input {
            self.previous_broker_input = current_broker_input.clone();
            self.last_input_change = Some(Instant::now());
            self.broker_status = None;
        }

        // Debounce and validate after 500ms
        if let Some(last_change) = self.last_input_change {
            if last_change.elapsed() >= Duration::from_millis(500) && !self.validation_in_progress {
                let (tx, rx) = std::sync::mpsc::channel();
                self.broker_validation_receiver = Some(rx);
                self.validation_in_progress = true;
                self.last_input_change = None;

                // Spawn validation thread
                let (cp_host, cp_port) = (ip.to_owned(), port.to_owned());
                std::thread::spawn(move || {
                    let result = util::validate_broker(&cp_host, &cp_port);
                    if let Err(e) = tx.send(result) {
                        log::error!("{e}");
                    }
                });
            }
        }

        // Check for validation results, if we got a result we store the result and reset the check status
        if let Some(receiver) = &mut self.broker_validation_receiver {
            if let Ok(result) = receiver.try_recv() {
                self.broker_status = Some(result);
                self.validation_in_progress = false;
                self.broker_validation_receiver = None;
            }
        }
    }
}
