//! The Tweaks tab: theme, File Explorer and privacy toggles, preinstalled app
//! removal, Edge debloat, and Windows activation.

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Cell, Row, TableState},
};

use crate::data::options::TWEAKS_OPTIONS;
use crate::theme;
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
                "Wallpaper compression" => status.wallpaper.as_str(),
                "Edge Debloat" => status.edge.as_str(),
                "Start Menu Bing Search" => status.bing_search.as_str(),
                "Classic Right-Click Menu" => status.right_click_menu.as_str(),
                "Windows AI" => status.windows_ai.as_str(),
                "Reserved Storage" => status.reserved_storage.as_str(),
                "Remove OneDrive" => status.one_drive.as_str(),
                "Remove Preinstalled Apps" => "—",
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
        4 => tweaks::wallpaper_compression_set_max(),
        5 => tweaks::remove_preinstalled_apps(),
        6 => tweaks::edge_debloat(),
        7 => tweaks::toggle_bing_search(),
        8 => tweaks::toggle_classic_context_menu(),
        9 => tweaks::disable_windows_ai(),
        10 => tweaks::toggle_reserved_storage(),
        11 => tweaks::remove_onedrive(),
        _ => return Ok(String::new()),
    };

    status.refresh();
    result
}
