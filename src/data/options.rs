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
/// The link is either a direct file URL, or a GitHub/HTTP link whose file name
/// contains a `*` glob (resolved to the newest match). The install method comes
/// from the downloaded file's extension. Build one with [`StoreApp::new`], or
/// [`StoreApp::with_args`] when the installer needs arguments.
pub struct StoreApp {
    /// Row label, also used as the install folder name.
    pub name: &'static str,
    /// One-line description shown in the table.
    pub description: &'static str,
    /// Download link (see the type docs for the `*` glob form).
    pub link: &'static str,
    /// Arguments for the installer (empty uses the default silent flag).
    pub args: &'static [&'static str],
}

impl StoreApp {
    /// An app whose installer runs with the default silent arguments.
    pub const fn new(name: &'static str, description: &'static str, link: &'static str) -> Self {
        Self {
            name,
            description,
            link,
            args: &[],
        }
    }

    /// An app whose installer needs specific arguments (e.g. `&["-y"]`).
    pub const fn with_args(
        name: &'static str,
        description: &'static str,
        link: &'static str,
        args: &'static [&'static str],
    ) -> Self {
        Self {
            name,
            description,
            link,
            args,
        }
    }
}

/// Rows of the Store tab, in display order. Add a `StoreApp` here to add an
/// app; nothing else needs wiring up.
pub static STORE_OPTIONS: [StoreApp; 29] = [
    StoreApp::new(
        "VSCode",
        "Visual Studio Code — Microsoft's code editor",
        "https://code.visualstudio.com/sha/download?build=stable&os=win32-x64-user",
    ),
    StoreApp::new(
        "Zed",
        "Fast, multiplayer code editor",
        "https://github.com/zed-industries/zed/releases/latest/download/Zed-x86_64.exe",
    ),
    StoreApp::new(
        "Git",
        "Git for Windows",
        "https://github.com/git-for-windows/git/releases/latest/download/Git-*-64-bit.exe",
    ),
    StoreApp::new(
        "GitHub CLI",
        "GitHub's official command-line tool (gh)",
        "https://github.com/cli/cli/releases/latest/download/gh_*_windows_amd64.msi",
    ),
    StoreApp::new(
        "Node",
        "Node.js JavaScript runtime",
        "https://nodejs.org/dist/latest/node-v*-x64.msi",
    ),
    StoreApp::new(
        "Bun",
        "Bun JavaScript runtime and toolkit",
        "https://github.com/oven-sh/bun/releases/latest/download/bun-windows-x64.zip",
    ),
    StoreApp::new(
        "uv",
        "uv — a fast Python package manager",
        "https://github.com/astral-sh/uv/releases/latest/download/uv-x86_64-pc-windows-msvc.zip",
    ),
    StoreApp::with_args(
        "Rust",
        "Rust toolchain installer (rustup)",
        "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe",
        &["-y"],
    ),
    StoreApp::new(
        "Chrome",
        "Google Chrome browser",
        "https://dl.google.com/chrome/install/latest/chrome_installer.exe",
    ),
    StoreApp::new(
        "Helium",
        "Helium — private, fast web browser",
        "https://github.com/imputnet/helium-windows/releases/latest/download/helium_*_x64-installer.exe",
    ),
    StoreApp::new(
        "mpv",
        "Media player",
        "https://github.com/shinchiro/mpv-winbuild-cmake/releases/latest/download/mpv-x86_64-*.7z",
    ),
    StoreApp::new(
        "VLC",
        "Open-source media player",
        "https://get.videolan.org/vlc/last/win64/vlc-*-win64.exe",
    ),
    StoreApp::new(
        "oxdm",
        "Download manager",
        "https://github.com/jd1378/oxdm/releases/latest/download/oxdm-*-x86_64-pc-windows-msvc.tar.gz",
    ),
    StoreApp::new(
        "Steam",
        "Game store and launcher",
        "https://cdn.akamai.steamstatic.com/client/installer/SteamSetup.exe",
    ),
    StoreApp::new(
        "Dorion",
        "Lightweight Discord client",
        "https://github.com/SpikeHD/Dorion/releases/latest/download/Dorion_*_x64-setup.exe",
    ),
    StoreApp::new(
        "LightBulb",
        "Reduces eye strain by adjusting screen gamma",
        "https://github.com/Tyrrrz/LightBulb/releases/latest/download/LightBulb-Installer.win-x64.exe",
    ),
    StoreApp::new(
        "PhotoCraft",
        "Open-source image editor",
        "https://github.com/storytold/photocraft/releases/latest/download/photocraft-*-windows-x64.msi",
    ),
    StoreApp::new(
        "VectorCraft",
        "Open-source vector illustration",
        "https://github.com/storytold/vectorcraft/releases/latest/download/vectorcraft-*-windows-x64.msi",
    ),
    StoreApp::new(
        "FilmCraft",
        "Open-source video editor",
        "https://github.com/storytold/filmcraft/releases/latest/download/filmcraft-*-windows-x64.msi",
    ),
    StoreApp::new(
        "LightCraft",
        "Open-source photo library and raw developer",
        "https://github.com/storytold/lightcraft/releases/latest/download/lightcraft-*-windows-x64.msi",
    ),
    StoreApp::new(
        "PdfCraft",
        "Open-source PDF workbench",
        "https://github.com/storytold/pdfcraft/releases/latest/download/pdfcraft-*-windows-x64.msi",
    ),
    StoreApp::new(
        "EffectCraft",
        "Open-source motion graphics and VFX",
        "https://github.com/storytold/effectcraft/releases/latest/download/effectcraft-*-windows-x64.msi",
    ),
    StoreApp::new(
        "DesignCraft",
        "Open-source page layout and publishing",
        "https://github.com/storytold/designcraft/releases/latest/download/designcraft-*-windows-x64.msi",
    ),
    StoreApp::new(
        "FigCraft",
        "Open-source interface design tool",
        "https://github.com/storytold/figcraft/releases/latest/download/figcraft-*-windows-x64.msi",
    ),
    StoreApp::new(
        "WordCraft",
        "Open-source word processor",
        "https://github.com/storytold/wordcraft/releases/latest/download/wordcraft-*-windows-x64.msi",
    ),
    StoreApp::new(
        "SoundCraft",
        "Open-source digital audio workstation",
        "https://github.com/storytold/soundcraft/releases/latest/download/soundcraft-*-windows-x64.msi",
    ),
    StoreApp::new(
        "DeckCraft",
        "Open-source presentation app",
        "https://github.com/storytold/deckcraft/releases/latest/download/deckcraft-*-windows-x64.msi",
    ),
    StoreApp::new(
        "CADCraft",
        "Open-source CAD and drafting",
        "https://github.com/storytold/cadcraft/releases/latest/download/cadcraft-*-windows-x64.msi",
    ),
    StoreApp::new(
        "GridCraft",
        "Open-source spreadsheets and data analysis",
        "https://github.com/storytold/gridcraft/releases/latest/download/gridcraft-*-windows-x64.msi",
    ),
];
