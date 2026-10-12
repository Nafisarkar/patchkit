//! The Cleaning tab: shows what the Recycle Bin and temp folders contain, and
//! runs the selected cleanup action.

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Cell, Row, TableState},
};

use crate::data::options::CLEANING_OPTIONS;
use crate::theme;
use crate::tools::cleaning::{self, CleaningStatus};

/// Render the Cleaning table from the cached `status`.
pub fn draw(frame: &mut Frame, area: Rect, state: &mut TableState, status: &CleaningStatus) {
    let rows = CLEANING_OPTIONS
        .iter()
        .map(|(name, description)| {
            // Pick the status text that belongs to this row.
            let value = match *name {
                "Recycle Bin" => status.recycle_bin.as_str(),
                "Windows Temp" => status.windows_temp.as_str(),
                "User Temp" => status.user_temp.as_str(),
                _ => " ",
            };

            Row::new(vec![
                Cell::from(*name),
                Cell::from(*description).style(theme::dim()),
                super::status_cell(value),
            ])
        })
        .collect::<Vec<_>>();

    super::action_table(frame, area, state, rows);
}

/// Run the cleaning action for the selected row, then refresh the status.
pub fn activate(index: usize, status: &mut CleaningStatus) -> Result<String, String> {
    let result = match index {
        0 => cleaning::clear_recycle_bin(),
        1 => cleaning::clean_windows_temp(),
        2 => cleaning::clean_user_temp(),
        3 => cleaning::run_disk_cleanup(),
        _ => return Ok(String::new()),
    };

    status.refresh();
    result
}
