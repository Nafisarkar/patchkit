use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Cell, Row, Table, TableState},
};

use crate::data::options::TWEAKS_OPTIONS;

pub fn draw(frame: &mut Frame, area: Rect, state: &mut TableState) {
    let rows = TWEAKS_OPTIONS
        .iter()
        .map(|(name, description)| Row::new(vec![Cell::from(*name), Cell::from(*description)]));

    let table = Table::new(
        rows,
        [Constraint::Percentage(35), Constraint::Percentage(65)],
    )
    .header(
        Row::new(vec!["Action", "Description"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(Block::bordered())
    .row_highlight_style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_stateful_widget(table, area, state);
}
