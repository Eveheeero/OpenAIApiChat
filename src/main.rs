#![cfg_attr(not(test), windows_subsystem = "windows")]

mod api;
mod fonts;
mod ui;
use eframe::egui;
use eframe::App;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::sync::Mutex;

// https://platform.openai.com/account/limits
// https://platform.openai.com/docs/guides/rate-limits/usage-tiers?context=tier-one
fn main() -> eframe::Result<()> {
    let mut native_options = eframe::NativeOptions::default();
    native_options.centered = true;
    native_options.viewport.decorations = Some(true);
    native_options.vsync = false;
    eframe::run_native(
        "OpenAI API Chat",
        native_options,
        Box::new(|cc| Ok(Box::new(Chat::new(cc)))),
    )
}

struct Chat {
    db: sled::Db,
    model: api::Machine,
    api_key: String,
    api_key_input: String,
    last_result: Arc<Mutex<Vec<String>>>,
    user_input: Vec<UserInput>,
    template: Vec<UserInput>,

    temperature: f64,

    runtime: tokio::runtime::Runtime,
}

impl Chat {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 폰트 설정
        cc.egui_ctx.set_fonts(fonts::get_fonts());

        let db = sled::open(".chat").expect("데이터베이스를 생성하는데 실패하였습니다.");

        let temperature = if let Some(temperature) = db.get("temperature").unwrap() {
            let temperature = std::str::from_utf8(&temperature)
                .expect("temperature에 올바른 문자열이 저장되지 않았습니다.");
            temperature
                .parse()
                .expect("temperature에 올바른 숫자가 저장되지 않았습니다.")
        } else {
            1.0
        };

        let model = if let Some(model) = db.get("model").unwrap() {
            let model =
                std::str::from_utf8(&model).expect("model에 올바른 문자열이 저장되지 않았습니다.");
            serde_json::from_str(model).expect("model에 올바른 JSON이 저장되지 않았습니다.")
        } else {
            Default::default()
        };

        let api_key = if let Some(api_key) = db.get("api_key").unwrap() {
            std::str::from_utf8(&api_key)
                .expect("openai key에 올바른 문자열이 저장되지 않았습니다.")
                .to_string()
        } else {
            Default::default()
        };

        let last_result: Vec<String> = if let Some(last_result) = db.get("last_result").unwrap() {
            let last_result = std::str::from_utf8(&last_result)
                .expect("last_result에 올바른 문자열이 저장되지 않았습니다.");
            serde_json::from_str(last_result)
                .expect("last_result에 올바른 JSON이 저장되지 않았습니다.")
        } else {
            Vec::new()
        };
        let last_result = Arc::new(Mutex::new(last_result));

        let user_input = if let Some(user_input) = db.get("user_input").unwrap() {
            let user_input = std::str::from_utf8(&user_input)
                .expect("user_input에 올바른 문자열이 저장되지 않았습니다.");
            serde_json::from_str(user_input)
                .expect("user_input에 올바른 JSON이 저장되지 않았습니다.")
        } else {
            let mut user_input = Vec::new();
            user_input.push(UserInput::default());
            user_input
        };

        let template = if let Some(template) = db.get("template").unwrap() {
            let template = std::str::from_utf8(&template)
                .expect("template에 올바른 문자열이 저장되지 않았습니다.");
            serde_json::from_str(template).expect("template에 올바른 JSON이 저장되지 않았습니다.")
        } else {
            Vec::new()
        };

        let result = Self {
            db,
            model,
            api_key,
            api_key_input: String::new(),
            last_result,
            temperature,
            user_input,
            template,
            runtime: tokio::runtime::Runtime::new()
                .expect("tokio runtime을 생성하는데 실패하였습니다."),
        };

        result
    }
}

impl Drop for Chat {
    fn drop(&mut self) {
        self.db
            .insert("temperature", self.temperature.to_string().as_bytes())
            .expect("창의성을 저장하는데 실패하였습니다.");

        self.db
            .insert(
                "model",
                serde_json::to_string(&self.model)
                    .expect("model을 저장하는데 실패하였습니다.")
                    .as_bytes(),
            )
            .expect("model을 저장하는데 실패하였습니다.");

        self.db
            .insert(
                "last_result",
                serde_json::to_string(&*self.last_result.lock().unwrap())
                    .expect("last_result를 저장하는데 실패하였습니다.")
                    .as_bytes(),
            )
            .expect("last_result를 저장하는데 실패하였습니다.");

        self.db
            .insert(
                "user_input",
                serde_json::to_string(&self.user_input)
                    .expect("user_input를 저장하는데 실패하였습니다.")
                    .as_bytes(),
            )
            .expect("user_input를 저장하는데 실패하였습니다.");

        self.db
            .insert(
                "template",
                serde_json::to_string(&self.template)
                    .expect("template를 저장하는데 실패하였습니다.")
                    .as_bytes(),
            )
            .expect("template를 저장하는데 실패하였습니다.");

        self.db
            .flush()
            .expect("데이터베이스를 저장하는데 실패하였습니다.");
    }
}

impl App for Chat {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // 접혀있는 System (설정 및 출력 갯수)
                self.settings(ui);

                // 템플릿들
                egui::Window::new("Template")
                    .default_open(false)
                    .default_pos([ui.available_width(), 0.0])
                    .show(ui.ctx(), |ui| {
                        // 각 템플릿을 로드하거나 삭제하는 내용
                        for user_input in self.template.iter_mut() {
                            ui.horizontal(|ui| {
                                if ui.button("-").clicked() {
                                    user_input.delete = true;
                                    return;
                                }
                                ui.add_enabled(
                                    false,
                                    egui::RadioButton::new(
                                        user_input.role == api::Role::System,
                                        "System",
                                    ),
                                );
                                ui.add_enabled(
                                    false,
                                    egui::RadioButton::new(
                                        user_input.role == api::Role::User,
                                        "User",
                                    ),
                                );
                                ui.add_enabled(
                                    false,
                                    egui::TextEdit::singleline(&mut user_input.save_name),
                                );
                                if ui.button("Load").clicked() {
                                    self.user_input.push(user_input.clone());
                                }
                            });

                            ui.add_enabled_ui(false, |ui| {
                                ui.add_sized(
                                    [ui.available_width(), 0.0],
                                    egui::TextEdit::multiline(&mut user_input.text),
                                );
                            });
                        }
                    });

                /* 선처리 */
                // 템플릿 삭제 처리
                self.template.retain(|user_input| !user_input.delete);
                // 유저 인풋 삭제 처리
                self.user_input.retain(|user_input| !user_input.delete);
                // 위치 수정
                let mut swap_list = Vec::new();
                let length = self.user_input.len();
                for (i, user_input) in self.user_input.iter_mut().enumerate() {
                    if user_input.to_up {
                        if i != 0 {
                            swap_list.push(i);
                        }
                        user_input.to_up = false;
                    }
                    if user_input.to_down {
                        if i != length - 1 {
                            swap_list.push(i + 1);
                        }
                        user_input.to_down = false;
                    }
                }
                for i in swap_list {
                    self.user_input.swap(i - 1, i);
                }

                // 메세지
                for user_input in &mut self.user_input.iter_mut() {
                    ui.horizontal(|ui| {
                        if ui.button("-").clicked() {
                            user_input.delete = true;
                            return;
                        }
                        ui.radio_value(&mut user_input.role, api::Role::System, "System");
                        ui.radio_value(&mut user_input.role, api::Role::User, "User");
                        ui.text_edit_singleline(&mut user_input.save_name);
                        if ui.button("Save").clicked() {
                            self.template.push(user_input.clone());
                        }
                        if ui.button("▲").clicked() {
                            user_input.to_up = true;
                        }
                        if ui.button("▼").clicked() {
                            user_input.to_down = true;
                        }
                    });

                    ui.add_sized(
                        [ui.available_width(), 0.0],
                        egui::TextEdit::multiline(&mut user_input.text),
                    );
                }

                ui.separator();

                // 버튼
                ui.horizontal(|ui| {
                    if ui.button("+").clicked() {
                        self.user_input.push(UserInput::default());
                    }
                    if ui.button("Send").clicked() {
                        self.send(ctx);
                    }
                });

                ui.separator();

                // 결과
                let last_result = self.last_result.lock().unwrap();
                for (i, result) in last_result.iter().enumerate() {
                    if i != 0 {
                        ui.separator();
                    }
                    let mut result = result.clone();
                    ui.add_sized(
                        [ui.available_width(), 0.0],
                        egui::TextEdit::multiline(&mut result),
                    );
                }
            });
        });
    }
}

impl Chat {
    fn send(&mut self, ctx: &egui::Context) {
        let user_input = self.user_input.clone();
        let last_result = self.last_result.clone();
        let api_key = self.api_key.clone();
        let temperature = self.temperature;
        let model = self.model;

        let mut input = Vec::new();
        for user_input in user_input.iter() {
            match user_input.role {
                api::Role::System => {
                    input.push(api::Message::system(user_input.text.clone()));
                }
                api::Role::User => {
                    input.push(api::Message::user(user_input.text.clone()));
                }
            }
        }

        let ctx = ctx.clone();
        self.runtime.spawn(async move {
            let result = api::chat(api_key, model, &input, temperature).await;
            let mut last_result = last_result.lock().unwrap();
            match result {
                Ok(result) => {
                    last_result.clear();
                    last_result.extend(result);
                }
                Err(err) => {
                    last_result.clear();
                    last_result.push(err);
                }
            }
            ctx.request_repaint();
        });
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct UserInput {
    role: api::Role,
    save_name: String,
    text: String,

    #[serde(skip)]
    delete: bool,
    #[serde(skip)]
    to_down: bool,
    #[serde(skip)]
    to_up: bool,
}
