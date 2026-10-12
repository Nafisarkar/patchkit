//! The Tweaks tab: toggles the theme and File Explorer options, and can
//! activate Windows.

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Cell, Row, TableState},
};

use crate::data::options::TWEAKS_OPTIONS;
use crate::tools::tweaks::{self, TweaksStatus};

/// Render the Tweaks table from the cached `status`.
pub fn draw(frame: &mut Frame, area: Rect, state: &mut TableState, status: &TweaksStatus) {
    let rows = TWEAKS_OPTIONS
        .iter()
        .map(|(name, description)| {
            // Pick the status text that belongs to this row.
            let value = match *name {
                "Toggle Dark Mode" => status.dark_mode.as_str(),
                "Show File Extensions" => status.file_extensions.as_str(),
                "Show Hidden Files" => status.hidden_files.as_str(),
                "Windows Activation" => status.activation.as_str(),
                _ => " ",
            };

            Row::new(vec![
                Cell::from(*name),
                Cell::from(*description),
                Cell::from(value),
            ])
        })
        .collect::<Vec<_>>();

    super::action_table(frame, area, state, rows);
}

/// Run the tweak for the selected row, then refresh the status. Activation is
/// the only action that changes the (slow to read) activation column.
pub fn activate(index: usize, status: &mut TweaksStatus) -> Result<String, String> {
    let result = match index {
        0 => tweaks::toggle_dark_mode(),
        1 => tweaks::show_file_extensions(),
        2 => tweaks::toggle_hidden_files(),
        3 => {
            let result = tweaks::activate_windows();
            status.refresh_activation();
            result
        }
        _ => return Ok(String::new()),
    };

    status.refresh();
    result
}
