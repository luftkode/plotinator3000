use egui::TextEdit;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Deserialize, Serialize)]
pub struct NumberEditor {
    tmp_value: String,
    value: Option<f64>,
    err_msg: String,
}

impl Default for NumberEditor {
    fn default() -> Self {
        Self {
            tmp_value: "-".to_owned(),
            value: Default::default(),
            err_msg: Default::default(),
        }
    }
}

impl NumberEditor {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let resp = ui.add(TextEdit::singleline(&mut self.tmp_value));
        if resp.changed() && self.tmp_value != "-" {
            match self.tmp_value.parse::<f64>() {
                Ok(new_val) => {
                    self.err_msg.clear();
                    self.value = Some(new_val);
                }
                Err(e) => {
                    self.err_msg = format!("⚠ {e} ⚠");
                }
            };
        }

        if !self.err_msg.is_empty() {
            ui.label(self.err_msg.clone());
        }
    }

    pub fn current(&self) -> Option<f64> {
        self.value
    }
}
