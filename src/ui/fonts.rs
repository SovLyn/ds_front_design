//! Bundled, subsetted Noto CJK faces.
//!
//! Only the glyphs used by the atlas are shipped. The subset files live in
//! `assets/fonts` and are licensed under the SIL Open Font License; the full
//! license text is next to them.

use egui::{FontData, FontDefinitions, FontFamily, FontId};

pub const DISPLAY_LIGHT: &str = "atlas-display-light";
pub const DISPLAY: &str = "atlas-display";

/// Register the two families used by the interface. The sans face is also
/// prepended to egui's proportional family so that any Chinese string that
/// escapes the subset text still has a chance of rendering.
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "atlas-serif-light".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/NotoSerifSC-Light.ttf")).into(),
    );
    fonts.font_data.insert(
        "atlas-serif".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/NotoSerifSC-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "atlas-sans".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/NotoSansSC-Regular.ttf")).into(),
    );

    let display_light = FontFamily::Name(DISPLAY_LIGHT.into());
    let display = FontFamily::Name(DISPLAY.into());

    fonts.families.insert(
        display_light,
        vec!["atlas-serif-light".to_owned(), "atlas-sans".to_owned()],
    );
    fonts.families.insert(
        display,
        vec!["atlas-serif".to_owned(), "atlas-sans".to_owned()],
    );

    if let Some(proportional) = fonts.families.get_mut(&FontFamily::Proportional) {
        proportional.insert(0, "atlas-sans".to_owned());
    }
    if let Some(monospace) = fonts.families.get_mut(&FontFamily::Monospace) {
        monospace.push("atlas-sans".to_owned());
    }

    ctx.set_fonts(fonts);
}

pub fn display_light(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(DISPLAY_LIGHT.into()))
}

pub fn display(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(DISPLAY.into()))
}

pub fn body(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}
