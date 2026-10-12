//! The per-tab pages. Each page has a `draw` function that renders its table
//! and an `activate` function that runs the selected row.

pub mod cleaning;
pub mod tweaks;

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Cell, Paragraph, Row, Table, TableState, Widget},
};

use crate::theme;

/// Width of the status column. Also used to right-align its text, so its right
/// edge lines up with the rules, the tagline and the key hints.
const STATUS_WIDTH: usize = 14;

/// Column widths shared by the header and the rows, so they line up.
const WIDTHS: [Constraint; 3] = [
    // Action: fixed, just wide enough for the longest label.
    Constraint::Length(22),
    // Description: takes whatever space is left.
    Constraint::Min(20),
    // Status: fixed.
    Constraint::Length(STATUS_WIDTH as u16),
];

/// Space between columns.
const SPACING: u16 = 2;

/// Render the "Action / Description / Status" table shared by both pages.
/// Pages only build the `rows`; the layout, colours and selection style live
/// here so the pages stay small and look identical.
///
/// The table is borderless: an uppercase header row, a thin rule under it, and
/// a subtle background on the selected row.
pub fn action_table(frame: &mut Frame, area: Rect, state: &mut TableState, rows: Vec<Row<'_>>) {
    // Split into: column titles, a separator line, then the selectable rows.
    let [header_area, rule_area, list_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .areas(area);

    // A header-only table, so the titles line up with the rows below. It uses
    // an empty row iterator and therefore draws nothing but the header.
    let header = Table::new(std::iter::empty::<Row>(), WIDTHS)
        .column_spacing(SPACING)
        .header(
            Row::new(vec![
                "ACTION".to_string(),
                "DESCRIPTION".to_string(),
                format!("{:>width$}", "STATUS", width = STATUS_WIDTH),
            ])
            .style(theme::heading()),
        );
    frame.render_widget(header, header_area);

    // Thin rule separating the header from the rows.
    Paragraph::new("─".repeat(rule_area.width as usize))
        .style(theme::dim())
        .render(rule_area, frame.buffer_mut());

    // The rows themselves.
    let list = Table::new(rows, WIDTHS)
        .column_spacing(SPACING)
        .row_highlight_style(theme::selected());
    frame.render_stateful_widget(list, list_area, state);
}

/// Build the status cell shared by both pages: right-aligned so the values
/// line up, and drawn in the accent colour.
pub fn status_cell(value: &str) -> Cell<'static> {
    Cell::from(format!("{value:>width$}", width = STATUS_WIDTH)).style(theme::accent())
}
