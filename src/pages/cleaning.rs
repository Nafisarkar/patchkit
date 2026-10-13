//! The Cleaning tab: shows what the Recycle Bin and temp folders contain, and
//! runs the selected cleanup action.

use ratatui::{
    Frame,
    layout::Rect,
    widgets::TableState,
};

use crate::data::options::{Action, CLEANING_OPTIONS};
use crate::tools::cleaning::CleaningStatus;

/// Render the Cleaning table from the cached `status`.
pub fn draw(frame: &mut Frame, area: Rect, state: &mut TableState, status: &CleaningStatus) {
    let rows = CLEANING_OPTIONS
        .iter()
        .map(|(name, description, needs_admin, _)| {
            // Pick the status text that belongs to this row.
            let value = match *name {
                "Recycle Bin" => status.recycle_bin.as_str(),
                "Windows Temp" => status.windows_temp.as_str(),
                "User Temp" => status.user_temp.as_str(),
                "Disk Clean" => "—",
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
    let (name, _, needs_admin, action) = CLEANING_OPTIONS.get(index)?;
    let action = (*action)?;
    Some((action, name, *needs_admin))
}
