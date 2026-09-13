//! Linger's Osaka Jade vocabulary. Palette adapted from Omarchy's quattro
//! theme; source and licence are documented in docs/VISUAL-DESIGN.md.
use rataflow::{Palette, Theme};
use ratatui::{
    style::{Color, Modifier, Style},
    widgets::BorderType,
};

pub const BORDER: BorderType = BorderType::Plain;
pub const SELECTION_RAIL: &str = "▌ ";
pub const GRID: Color = Color::Rgb(35, 55, 43); // #23372B
pub const SELECTION: Color = Color::Rgb(50, 71, 59); // #32473B
pub const BRIGHT_TEXT: Color = Color::Rgb(247, 232, 178); // #F7E8B2

pub fn theme() -> Theme {
    Theme::Custom(Palette {
        canvas_bg: Color::Rgb(12, 21, 18), // dark_background
        surface: Color::Rgb(17, 28, 24),   // background
        muted: Color::Rgb(83, 104, 91),    // muted
        subtle: Color::Rgb(129, 184, 168), // dark_foreground
        accent: Color::Rgb(80, 148, 117),  // accent
        text: Color::Rgb(193, 196, 151),   // foreground
        success: Color::Rgb(99, 176, 122), // bright_green
        error: Color::Rgb(255, 83, 69),    // red
    })
}

pub fn selected() -> Style {
    Style::default()
        .bg(SELECTION)
        .fg(BRIGHT_TEXT)
        .add_modifier(Modifier::BOLD)
}
