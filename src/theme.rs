//! Colour palette and shared widget styles, so every part of the UI stays
//! consistent. The colours are tuned for dark terminals; if you use a light
//! theme, this is the only file you need to change.

use ratatui::style::{Color, Modifier, Style};

/// Accent colour: the title, the active tab, and data values.
pub const ACCENT: Color = Color::Rgb(122, 162, 247);
/// Muted colour for secondary text (inactive tabs, headers, hints, rules).
pub const MUTED: Color = Color::Rgb(110, 118, 150);
/// Error colour for failed actions in the status bar.
pub const ERROR: Color = Color::Rgb(231, 130, 132);

const SELECTED_BG: Color = Color::Rgb(43, 49, 70);
const SELECTED_FG: Color = Color::Rgb(205, 214, 244);
const ADMIN_FG: Color = Color::Rgb(240, 160, 210);

pub fn title() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn tab() -> Style {
    Style::default().fg(MUTED)
}

pub fn tab_active() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn heading() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn dim() -> Style {
    Style::default().fg(MUTED)
}

pub fn accent() -> Style {
    Style::default().fg(ACCENT)
}

pub fn error() -> Style {
    Style::default().fg(ERROR)
}

pub fn selected() -> Style {
    Style::default().bg(SELECTED_BG).fg(SELECTED_FG)
}

/// Style for a row name: bold magenta text when the row needs administrator
/// rights (no background fill, so the selection highlight stays clean),
/// default otherwise.
pub fn admin_row(needs_admin: bool) -> Style {
    if needs_admin {
        Style::default().fg(ADMIN_FG).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}
