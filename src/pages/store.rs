//! The Store tab: download apps that aren't on the Microsoft Store into the
//! user's Downloads folder.

use ratatui::{Frame, layout::Rect, widgets::TableState};

use crate::data::options::{Action, STORE_OPTIONS};
use crate::tools::downloader::StoreStatus;

/// Render the Store table from the cached `status`.
pub fn draw(frame: &mut Frame, area: Rect, state: &mut TableState, status: &StoreStatus) {
    let rows = STORE_OPTIONS
        .iter()
        .map(|(name, description, needs_admin, _)| {
            let value = match *name {
                "mpv" => status.mpv.as_str(),
                "Dorion" => status.dorion.as_str(),
                _ => " ",
            };

            super::option_row(name, description, *needs_admin, value)
        })
        .collect::<Vec<_>>();

    super::action_table(frame, area, state, rows);
}

/// The action, label and admin requirement behind the selected row; `main.rs`
/// runs the action on a worker thread so the UI keeps animating.
pub fn action(index: usize) -> Option<(Action, &'static str, bool)> {
    let (name, _, needs_admin, action) = STORE_OPTIONS.get(index)?;
    let action = (*action)?;
    Some((action, name, *needs_admin))
}
