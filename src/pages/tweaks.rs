//! The Tweaks tab: theme, File Explorer and privacy toggles, preinstalled app
//! removal, Edge debloat, and Windows activation.

use ratatui::{Frame, layout::Rect, widgets::TableState};

use crate::data::options::{Job, TWEAKS_OPTIONS};
use crate::tools::tweaks::TweaksStatus;

/// Render the Tweaks table from the cached `status`.
pub fn draw(frame: &mut Frame, area: Rect, state: &mut TableState, status: &TweaksStatus) {
    let rows = TWEAKS_OPTIONS
        .iter()
        .map(|(name, description, needs_admin, _)| {
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
                "Store Suggestions" => status.store_search.as_str(),
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

            super::option_row(name, description, *needs_admin, value)
        })
        .collect::<Vec<_>>();

    super::action_table(frame, area, state, rows);
}

/// The action, label and admin requirement behind the selected row; `main.rs`
/// runs the action on a worker thread so the UI keeps animating.
pub fn action(index: usize) -> Option<(Job, &'static str, bool)> {
    let (name, _, needs_admin, action) = TWEAKS_OPTIONS.get(index)?;
    let action = (*action)?;
    Some((Box::new(action), name, *needs_admin))
}
