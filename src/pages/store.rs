//! The Store tab: download and install the apps listed in `STORE_OPTIONS`.

use ratatui::{Frame, layout::Rect, widgets::TableState};

use crate::data::options::{Job, STORE_OPTIONS};
use crate::tools::downloader::{self, StoreStatus};

/// Render the Store table.
pub fn draw(frame: &mut Frame, area: Rect, state: &mut TableState, status: &StoreStatus) {
    let rows = STORE_OPTIONS
        .iter()
        .enumerate()
        .map(|(index, app)| {
            super::option_row(app.name, app.description, false, status.value(index))
        })
        .collect();

    super::action_table(frame, area, state, rows);
}

/// The action behind the selected row; `main.rs` runs it on a worker thread.
pub fn action(index: usize) -> Option<(Job, &'static str, bool)> {
    let app = STORE_OPTIONS.get(index)?;
    Some((Box::new(move || downloader::install(app)), app.name, false))
}
