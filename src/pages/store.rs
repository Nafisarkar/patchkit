//! The Store tab: download and install apps listed in `STORE_OPTIONS`.

use ratatui::{Frame, layout::Rect, widgets::TableState};

use crate::data::options::{Job, STORE_OPTIONS};
use crate::tools::downloader::{self, StoreStatus};

/// Render the Store table from the cached `status`.
pub fn draw(frame: &mut Frame, area: Rect, state: &mut TableState, status: &StoreStatus) {
    let rows = STORE_OPTIONS
        .iter()
        .enumerate()
        .map(|(index, app)| {
            super::option_row(app.name, app.description, false, status.value(index))
        })
        .collect::<Vec<_>>();

    super::action_table(frame, area, state, rows);
}

/// The action, label and admin requirement behind the selected row; `main.rs`
/// runs the action on a worker thread so the UI keeps animating.
pub fn action(index: usize) -> Option<(Job, &'static str, bool)> {
    let app = STORE_OPTIONS.get(index)?;
    Some((Box::new(move || downloader::install(app)), app.name, false))
}
