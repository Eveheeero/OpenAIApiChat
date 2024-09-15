use crate::{api::Machine, Chat};
use eframe::egui;

impl Chat {
    pub(super) fn settings(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Settings")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("모델");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.model, Machine::Gpt35Turbo, "GPT-3.5-TURBO");
                    ui.radio_value(&mut self.model, Machine::Gpt4, "GPT-4");
                    ui.radio_value(&mut self.model, Machine::Gpt4Turbo, "GPT-4-TURBO");
                    ui.radio_value(&mut self.model, Machine::Gpt4O, "GPT-4O");
                    ui.radio_value(&mut self.model, Machine::Gpt4OMini, "GPT-4O-Mini");
                    // ui.radio_value(&mut self.model, Machine::GptO1, "O1");
                    // ui.radio_value(&mut self.model, Machine::GptO1Mini, "O1-Mini");
                });

                ui.separator();

                ui.label("창의성");
                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut self.temperature, 0.0..=2.0));
                    if ui.button("리셋").clicked() {
                        self.temperature = 1.0;
                    }
                });

                ui.separator();

                ui.label("API Key");
                ui.horizontal(|ui| {
                    let key = egui::TextEdit::singleline(&mut self.api_key_input);
                    ui.add(key);
                    if ui.button("저장").clicked() {
                        self.api_key = self.api_key_input.trim().to_string();
                        self.db
                            .insert("api_key", self.api_key_input.as_bytes())
                            .expect("api key를 저장하는데 실패하였습니다.");
                        self.api_key_input.clear();
                    }
                });
            });
    }
}
