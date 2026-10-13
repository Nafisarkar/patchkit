//! The per-tab pages. Each page has a `draw` function that renders its table
//! and an `activate` function that runs the selected row.

pub mod cleaning;
pub mod tweaks;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Cell, Paragraph, Row, Table, TableState, Widget},
};

use crate::data::options::{CLEANING_OPTIONS, TWEAKS_OPTIONS};
use crate::theme;

/// Width of the status column. Also used to right-align its text, so its right
/// edge lines up with the rules, the tagline and the key hints.
const STATUS_WIDTH: usize = 16;

/// Space between columns.
const SPACING: u16 = 2;

/// Render the "Action / Description / Status" table shared by both pages.
/// Pages only build the `rows`; the layout, colours and selection style live
/// here so the pages stay small and look identical.
///
/// The table is borderless: a thin rule above the rows and a subtle
/// background on the selected row.
pub fn action_table(frame: &mut Frame, area: Rect, state: &mut TableState, rows: Vec<Row<'_>>) {
    // Split into: a separator line, then the selectable rows.
    let [rule_area, list_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);

    // Thin rule separating the tab bar from the rows.
    Paragraph::new("─".repeat(rule_area.width as usize))
        .style(theme::dim())
        .render(rule_area, frame.buffer_mut());

    // The Action column is as wide as the longest row label, so no name is
    // ever truncated when new rows are added.
    let action_width = CLEANING_OPTIONS
        .iter()
        .chain(TWEAKS_OPTIONS.iter())
        .map(|(name, _)| name.chars().count())
        .max()
        .unwrap_or(22) as u16;
    let widths = [
        Constraint::Length(action_width),
        Constraint::Min(20),
        Constraint::Length(STATUS_WIDTH as u16),
    ];

    // The rows themselves.
    let list = Table::new(rows, widths)
        .column_spacing(SPACING)
        .row_highlight_style(theme::selected());
    frame.render_stateful_widget(list, list_area, state);
}

/// Build the status cell shared by both pages: right-aligned so the values
/// line up, and drawn in the accent colour.
pub fn status_cell(value: &str) -> Cell<'static> {
    Cell::from(format!("{value:>width$}", width = STATUS_WIDTH)).style(theme::accent())
}
