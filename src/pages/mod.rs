//! The per-tab pages. Each page has a `draw` function that renders its table
//! and an `activate` function that runs the selected row.

pub mod cleaning;
pub mod tweaks;

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Row, Table, TableState},
};

/// Render the three-column "Action / Description / Status" table shared by
/// both pages. Pages only build the `rows`; the layout and colours live here
/// so the two pages stay small and look identical.
///
/// The table is borderless: a dimmed, bold header row and a reversed highlight
/// on the selected row keep it readable without drawing a box.
pub fn action_table(frame: &mut Frame, area: Rect, state: &mut TableState, rows: Vec<Row<'_>>) {
    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(50),
            Constraint::Percentage(20),
        ],
    )
    .header(
        Row::new(vec!["Action", "Description", "Status"]).style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .column_spacing(2)
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    frame.render_stateful_widget(table, area, state);
}
