use chrono::{DateTime, Local, NaiveDateTime, Utc};
use egui::{Response, TextEdit};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct DateEditor {
    tmp_date: String,
    current_date: Option<DateTime<Utc>>,
    err_msg: String,
}

impl Default for DateEditor {
    fn default() -> Self {
        Self::new(Local::now().to_utc())
    }
}

impl DateEditor {
    pub fn new(current_date: DateTime<Utc>) -> Self {
        Self {
            tmp_date: current_date.format("%Y-%m-%d %H:%M:%S%.f").to_string(),
            current_date: Some(current_date),
            err_msg: Default::default(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Response {
        let resp = ui.add(TextEdit::singleline(&mut self.tmp_date));

        if resp.changed() {
            match NaiveDateTime::parse_from_str(&self.tmp_date, "%Y-%m-%d %H:%M:%S%.f") {
                Ok(new_dt) => {
                    self.err_msg.clear();
                    self.current_date = Some(new_dt.and_utc());
                }
                Err(e) => {
                    self.current_date = None;
                    self.err_msg = format!("⚠ {e} ⚠");
                }
            };
        }

        if !self.err_msg.is_empty() {
            ui.label(self.err_msg.clone());
        }
        resp
    }

    pub fn set(&mut self, date: DateTime<Utc>) {
        self.tmp_date = date.format("%Y-%m-%d %H:%M:%S%.f").to_string();
        self.current_date = Some(date);
    }

    pub fn current(&self) -> Option<DateTime<Utc>> {
        self.current_date
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construct_date_editor() {
        let de = DateEditor::default();
        assert!(de.current().is_some());
        assert_eq!(de.err_msg, String::new());
    }
}
