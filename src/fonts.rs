use eframe::{
    egui::{FontData, FontDefinitions},
    epaint::FontFamily,
};

pub(super) fn get_fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "pretendard".into(),
        FontData::from_static(include_bytes!("../PretendardJP-Medium.ttf")),
    );
    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, "pretendard".into());
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "pretendard".into());
    fonts
}
