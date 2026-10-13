//! The user-facing option lists for each tab. Every entry is a
//! `(name, description, needs_admin, action)` row: the pages build the table
//! from the first three fields, and `main.rs` runs the action on a worker
//! thread. `None` actions are display-only rows.

/// A tweak or cleaning action: run on a worker thread, returns the text for
/// the status bar.
pub type Action = fn() -> Result<String, String>;

use crate::tools::{cleaning, downloader, tweaks};

/// Rows of the Cleaning tab, in display order.
pub const CLEANING_OPTIONS: [(&str, &str, bool, Option<Action>); 4] = [
    (
        "Recycle Bin",
        "Empty the Recycle Bin to free up disk space",
        false,
        Some(cleaning::clear_recycle_bin),
    ),
    (
        "Windows Temp",
        "Delete system temp files from C:\\Windows\\Temp",
        false,
        Some(cleaning::clean_windows_temp),
    ),
    (
        "User Temp",
        "Delete per-user temp files from %LOCALAPPDATA%\\Temp",
        false,
        Some(cleaning::clean_user_temp),
    ),
    (
        "Disk Clean",
        "Will open windows disk cleaning utility",
        false,
        Some(cleaning::run_disk_cleanup),
    ),
];

/// Rows of the Tweaks tab, in display order.
pub const TWEAKS_OPTIONS: [(&str, &str, bool, Option<Action>); 26] = [
    (
        "Toggle Dark Mode",
        "Switch system apps between light and dark theme",
        false,
        Some(tweaks::toggle_dark_mode),
    ),
    (
        "Show File Extensions",
        "Show file extensions in File Explorer (.exe, .png, etc.)",
        false,
        Some(tweaks::show_file_extensions),
    ),
    (
        "Show Hidden Files",
        "Show or hide hidden files and folders in File Explorer",
        false,
        Some(tweaks::toggle_hidden_files),
    ),
    (
        "Windows Activation",
        "Activate Windows with massgrave",
        true,
        Some(tweaks::activate_windows),
    ),
    (
        "Wallpaper compression",
        "Disable the compression of wallpapers in Windows 11/10 to improve quality",
        false,
        Some(tweaks::disable_wallpaper_compression),
    ),
    (
        "Remove Preinstalled Apps",
        "Uninstall preinstalled store apps (Solitaire, Copilot, Xbox overlays, etc.) for all users",
        true,
        Some(tweaks::remove_preinstalled_apps),
    ),
    (
        "Edge Debloat",
        "Disables telemetry, popups and other annoyances in Microsoft Edge",
        true,
        Some(tweaks::edge_debloat),
    ),
    (
        "Start Menu Bing Search",
        "Toggles Bing web search results in Windows Search",
        false,
        Some(tweaks::toggle_bing_search),
    ),
    (
        "Classic Right-Click Menu",
        "Restores the Windows 10 full context menu in File Explorer instead of the Windows 11 one",
        false,
        Some(tweaks::toggle_classic_context_menu),
    ),
    (
        "Windows AI",
        "Removes and disables all AI features/packages (Recall, Copilot, Notepad AI)",
        true,
        Some(tweaks::disable_windows_ai),
    ),
    (
        "Reserved Storage",
        "Toggles the ~7 GB Windows reserves for updates; freeing it helps small drives",
        true,
        Some(tweaks::toggle_reserved_storage),
    ),
    (
        "Remove OneDrive",
        "Uninstalls OneDrive with its own uninstaller and cleans the leftover files",
        true,
        Some(tweaks::remove_onedrive),
    ),
    (
        "Activity History",
        "Stops Windows from publishing or uploading user activities; keeps clipboard history",
        true,
        Some(tweaks::disable_activity_history),
    ),
    (
        "Disable BitLocker",
        "Disables BitLocker encryption on the system drive (decrypts in the background)",
        true,
        Some(tweaks::disable_bitlocker),
    ),
    (
        "Consumer Features",
        "Stops promoted app installs and reduces Microsoft Store app suggestions",
        true,
        Some(tweaks::disable_consumer_features),
    ),
    (
        "Delivery Optimization",
        "Stops Windows from using your bandwidth to upload updates to other PCs",
        true,
        Some(tweaks::disable_delivery_optimization),
    ),
    (
        "Location Tracking",
        "Denies app location access and switches off the location sensor",
        true,
        Some(tweaks::disable_location_tracking),
    ),
    (
        "Store Suggestions",
        "Toggles suggested Microsoft Store apps in Start menu search",
        false,
        Some(tweaks::toggle_store_search),
    ),
    (
        "Device Companion Apps",
        "Prevents extra software from installing when plugging in devices",
        true,
        Some(tweaks::prevent_device_metadata),
    ),
    (
        "Services to Manual",
        "Sets unneeded services to manual/disabled and reduces svchost.exe count",
        true,
        Some(tweaks::set_services_manual),
    ),
    (
        "Explorer Home & Gallery",
        "Hides Home and Gallery in File Explorer and opens This PC by default",
        false,
        Some(tweaks::toggle_explorer_home),
    ),
    (
        "Visual Effects",
        "Sets visual effects to best performance (no animations, shadows or transparency)",
        false,
        Some(tweaks::set_visual_effects_performance),
    ),
    (
        "Background Apps",
        "Toggles whether Microsoft Store apps may run in the background",
        false,
        Some(tweaks::toggle_background_apps),
    ),
    (
        "Hyper-V",
        "Enables the Hyper-V virtualization feature (restart required)",
        true,
        Some(tweaks::enable_hyperv),
    ),
    (
        "WSL",
        "Enables Windows Subsystem for Linux (restart required)",
        true,
        Some(tweaks::enable_wsl),
    ),
    (
        "O&O ShutUp10++",
        "Downloads and launches the O&O ShutUp10++ privacy tool",
        false,
        Some(tweaks::run_ooshutup),
    ),
];

/// Rows of the Store tab, in display order.
pub const STORE_OPTIONS: [(&str, &str, bool, Option<Action>); 2] = [
    (
        "mpv",
        "Media player — downloads the latest Windows build to your Downloads folder",
        false,
        Some(downloader::download_mpv),
    ),
    (
        "Dorion",
        "Lightweight Discord client — downloads the Windows installer to your Downloads folder",
        false,
        Some(downloader::download_dorion),
    ),
];
