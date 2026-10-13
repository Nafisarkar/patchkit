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
                "Activity History" => status.activity_history.as_str(),
                "Disable BitLocker" => status.bitlocker.as_str(),
                "Consumer Features" => status.consumer_features.as_str(),
                "Delivery Optimization" => status.delivery_optimization.as_str(),
                "Location Tracking" => status.location.as_str(),
                "Store Recommendations" => status.store_search.as_str(),
                "Device Companion Apps" => status.device_metadata.as_str(),
                "Services to Manual" => status.services.as_str(),
                "Explorer Home & Gallery" => status.explorer_home.as_str(),
                "Visual Effects" => status.visual_effects.as_str(),
                "Background Apps" => status.background_apps.as_str(),
                "Hyper-V" => status.hyperv.as_str(),
                "WSL" => status.wsl.as_str(),
                "O&O ShutUp10++" => status.ooshutup.as_str(),
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

/// The action function behind the selected row; the caller runs it on a
/// worker thread so the UI keeps animating.
pub fn action(index: usize) -> Option<fn() -> Result<String, String>> {
    Some(match index {
        0 => tweaks::toggle_dark_mode,
        1 => tweaks::show_file_extensions,
        2 => tweaks::toggle_hidden_files,
        3 => tweaks::activate_windows,
        4 => tweaks::wallpaper_compression_set_max,
        5 => tweaks::remove_preinstalled_apps,
        6 => tweaks::edge_debloat,
        7 => tweaks::toggle_bing_search,
        8 => tweaks::toggle_classic_context_menu,
        9 => tweaks::disable_windows_ai,
        10 => tweaks::toggle_reserved_storage,
        11 => tweaks::remove_onedrive,
        12 => tweaks::disable_activity_history,
        13 => tweaks::disable_bitlocker,
        14 => tweaks::disable_consumer_features,
        15 => tweaks::disable_delivery_optimization,
        16 => tweaks::disable_location_tracking,
        17 => tweaks::toggle_store_search,
        18 => tweaks::prevent_device_metadata,
        19 => tweaks::set_services_manual,
        20 => tweaks::toggle_explorer_home,
        21 => tweaks::set_visual_effects_performance,
        22 => tweaks::toggle_background_apps,
        23 => tweaks::enable_hyperv,
        24 => tweaks::enable_wsl,
        25 => tweaks::run_ooshutup,
        _ => return None,
    })
}
