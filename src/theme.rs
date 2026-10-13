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

/// Background of the selected table row.
const SELECTED_BG: Color = Color::Rgb(43, 49, 70);
/// Foreground of the selected table row.
const SELECTED_FG: Color = Color::Rgb(205, 214, 244);

/// App title.
pub fn title() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Inactive tab.
pub fn tab() -> Style {
    Style::default().fg(MUTED)
}

/// Active tab.
pub fn tab_active() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Dimmed secondary text and separator lines.
pub fn dim() -> Style {
    Style::default().fg(MUTED)
}

/// Accent-coloured data values (the status column).
pub fn accent() -> Style {
    Style::default().fg(ACCENT)
}

/// Failed action messages.
pub fn error() -> Style {
    Style::default().fg(ERROR)
}

/// The selected table row.
pub fn selected() -> Style {
    Style::default().bg(SELECTED_BG).fg(SELECTED_FG)
}
