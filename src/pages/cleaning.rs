use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Cell, Row, Table, TableState},
};

use crate::tools::recycle::{preview_recycle_bin, preview_user_temp};
use crate::{data::options::CLEANING_OPTIONS, tools::recycle::preview_windows_temp};

pub fn draw(frame: &mut Frame, area: Rect, state: &mut TableState) {
    let rows = CLEANING_OPTIONS.iter().map(|(name, description)| {
        let status = match *name {
            "Recycle Bin" => preview_recycle_bin()
                .to_owned()
                .unwrap_or_else(|| "unknown".to_string()),

            "Windows Temp" => preview_windows_temp()
                .to_owned()
                .unwrap_or_else(|| "unknown".to_string()),
            "User Temp" => preview_user_temp()
                .to_owned()
                .unwrap_or_else(|| "unknown".to_string()),
            _ => " ".to_string(),
        };

        Row::new(vec![
            Cell::from(*name),
            Cell::from(*description),
            Cell::from(status),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(35),
            Constraint::Percentage(55),
            Constraint::Percentage(20),
        ],
    )
    .header(
        Row::new(vec!["Action", "Description", "Status"])
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
