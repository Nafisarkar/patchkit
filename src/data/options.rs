//! The user-facing option lists for each tab. Every entry is a
//! `(name, description, needs_admin, action)` row: the pages build the table
//! from the first three fields, and `main.rs` runs the action on a worker
//! thread. `None` actions are display-only rows.

/// A tweak or cleaning action: run on a worker thread, returns the text for
/// the status bar.
pub type Action = fn() -> Result<String, String>;

/// A unit of work run on a worker thread. Unlike [`Action`] it may capture
/// data (for example a Store entry), so it can't live in a `const` table.
pub type Job = Box<dyn FnOnce() -> Result<String, String> + Send>;

use crate::tools::{cleaning, tweaks};

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

/// A downloadable app on the Store tab: a name, a description and a link.
///
/// The link is either a direct file URL, or a GitHub
/// `releases/latest/download/<name>` URL whose file name contains a `*` glob,
/// in which case the newest matching release asset is fetched instead. The
/// install method is chosen from the downloaded file's extension: archives are
/// unpacked, installers are run.
pub struct StoreApp {
    /// Row label, also used as the install folder name.
    pub name: &'static str,
    /// One-line description shown in the table.
    pub description: &'static str,
    /// Download link (see the type docs for the `*` glob form).
    pub link: &'static str,
}

/// Rows of the Store tab, in display order. Add a `StoreApp` here to add an
/// app; nothing else needs wiring up.
pub static STORE_OPTIONS: [StoreApp; 10] = [
    StoreApp {
        name: "VSCode",
        description: "Visual Studio Code — Microsoft's code editor",
        link: "https://code.visualstudio.com/sha/download?build=stable&os=win32-x64-user",
    },
    StoreApp {
        name: "Zed",
        description: "Fast, multiplayer code editor",
        link: "https://github.com/zed-industries/zed/releases/latest/download/Zed-x86_64.exe",
    },
    StoreApp {
        name: "Git",
        description: "Git for Windows",
        link: "https://github.com/git-for-windows/git/releases/latest/download/Git-*-64-bit.exe",
    },
    StoreApp {
        name: "Node",
        description: "Node.js JavaScript runtime",
        link: "https://nodejs.org/dist/latest/node-v*-x64.msi",
    },
    StoreApp {
        name: "Bun",
        description: "Bun JavaScript runtime and toolkit",
        link: "https://github.com/oven-sh/bun/releases/latest/download/bun-windows-x64.zip",
    },
    StoreApp {
        name: "uv",
        description: "uv — a fast Python package manager",
        link: "https://github.com/astral-sh/uv/releases/latest/download/uv-x86_64-pc-windows-msvc.zip",
    },
    StoreApp {
        name: "Chrome",
        description: "Google Chrome browser",
        link: "https://dl.google.com/chrome/install/latest/chrome_installer.exe",
    },
    StoreApp {
        name: "Helium",
        description: "Helium — private, fast web browser",
        link: "https://github.com/imputnet/helium-windows/releases/latest/download/helium_*_x64-installer.exe",
    },
    StoreApp {
        name: "mpv",
        description: "Media player",
        link: "https://github.com/shinchiro/mpv-winbuild-cmake/releases/latest/download/mpv-x86_64-*.7z",
    },
    StoreApp {
        name: "Dorion",
        description: "Lightweight Discord client",
        link: "https://github.com/SpikeHD/Dorion/releases/latest/download/Dorion_*_x64-setup.exe",
    },
];
