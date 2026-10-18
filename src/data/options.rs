//! The user-facing option lists for each tab. The Cleaning and Tweaks tabs
//! are tables of `(name, description, needs_admin, action)` rows; the Store
//! tab is a list of `StoreApp` downloads grouped by `StoreCategory`. The
//! pages build the table rows, and `main.rs` runs the selected action on a
//! worker thread.

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
pub const TWEAKS_OPTIONS: [(&str, &str, bool, Option<Action>); 39] = [
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
        "Explorer & Menu Speed",
        "Removes the Explorer startup delay and makes menus open instantly",
        false,
        Some(tweaks::toggle_explorer_menu_speed),
    ),
    (
        "Start Recommended",
        "Hides the Recommended section in the Start menu (Windows 11)",
        false,
        Some(tweaks::toggle_start_recommended),
    ),
    (
        "Update Pause Limit",
        "Lets the update pause slider go to the maximum Windows honors",
        true,
        Some(tweaks::lift_update_pause_limit),
    ),
    (
        "WebDAV Limit",
        "Raises the WebDAV file transfer limit from ~400 MB to 4 GB",
        true,
        Some(tweaks::lift_webdav_limit),
    ),
    (
        "Multimedia Scheduler",
        "Optimizes MMCSS for low latency: no network throttling, min responsiveness, high game task priority",
        true,
        Some(tweaks::apply_multimedia_scheduler),
    ),
    (
        "Desktop Icons",
        "Shows or hides all desktop icons",
        false,
        Some(tweaks::toggle_desktop_icons),
    ),
    (
        "This PC Icon",
        "Shows This PC on the desktop",
        false,
        Some(tweaks::toggle_this_pc_icon),
    ),
    (
        "Recycle Bin Icon",
        "Shows the Recycle Bin on the desktop",
        false,
        Some(tweaks::toggle_recycle_bin_icon),
    ),
    (
        "User Folder Icon",
        "Shows the user profile folder on the desktop",
        false,
        Some(tweaks::toggle_user_folder_icon),
    ),
    (
        "Network Icon",
        "Shows the Network shortcut on the desktop",
        false,
        Some(tweaks::toggle_network_icon),
    ),
    (
        "Control Panel Icon",
        "Shows the Control Panel on the desktop",
        false,
        Some(tweaks::toggle_control_panel_icon),
    ),
    (
        "Shortcut Arrow",
        "Removes the arrow overlay from desktop shortcuts",
        true,
        Some(tweaks::toggle_shortcut_arrow),
    ),
    (
        "GPU Scheduling",
        "Toggles hardware-accelerated GPU scheduling (HAGS; restart required)",
        true,
        Some(tweaks::toggle_gpu_scheduling),
    ),
    (
        "Process Priority",
        "Give foreground apps short, high-boost CPU quanta (Win32PrioritySeparation 0x26)",
        true,
        Some(tweaks::toggle_foreground_boost),
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

/// An entry on the Store tab: what to install, and how.
///
/// `link` is either a direct file URL, or a GitHub/HTTP link whose file name
/// contains a `*` glob (resolved before download). Script-installed apps
/// instead run an install command/([`StoreScript`]) and skip downloading
/// altogether. Build one with [`StoreApp::new`] or [`StoreApp::with_args`]
/// for downloads, [`StoreApp::from_script`] / [`StoreApp::from_package`] for
/// scripted installs.
pub enum StoreScript {
    /// Launch `program` with `args`.
    Command(&'static str, &'static [&'static str]),
    /// Install `package` globally: with bun when bun is on PATH, npm
    /// otherwise; an error surfaces when neither is installed.
    GlobalPackage(&'static str),
}

/// An entry on the Store tab: what to install, and how.
///
/// `link` is either a direct file URL, or a GitHub/HTTP link whose file name
/// contains a `*` glob (resolved before download; the extension decides
/// whether the file is unpacked or run as an installer). Script-installed
/// apps instead run a [`StoreScript`] command and skip downloading.
/// Build one with [`StoreApp::new`] or [`StoreApp::with_args`] for
/// downloads, [`StoreApp::from_script`] / [`StoreApp::from_package`] for
/// scripted installs.
pub struct StoreApp {
    /// Row label, also used as the install folder name.
    pub name: &'static str,
    /// One-line description shown in the table.
    pub description: &'static str,
    /// Download link (see the type docs for the `*` glob form).
    pub link: &'static str,
    /// Arguments for the installer (empty uses the default silent flag).
    pub args: &'static [&'static str],
    /// An install script to run instead of downloading a file.
    pub script: Option<StoreScript>,
    /// Install marker used by the status column: an executable or install
    /// folder (relative to Program Files, the user profile, the local app
    /// data, or the app's own download folder) that only exists once the app
    /// is installed. Script apps are detected via PATH instead.
    pub exe: &'static str,
}

impl StoreApp {
    /// An app whose installer runs with the default silent arguments.
    pub const fn new(
        name: &'static str,
        description: &'static str,
        exe: &'static str,
        link: &'static str,
    ) -> Self {
        Self {
            name,
            description,
            link,
            args: &[],
            script: None,
            exe,
        }
    }

    /// An app whose installer needs specific arguments (e.g. `&["-y"]`).
    pub const fn with_args(
        name: &'static str,
        description: &'static str,
        exe: &'static str,
        link: &'static str,
        args: &'static [&'static str],
    ) -> Self {
        Self {
            name,
            description,
            link,
            args,
            script: None,
            exe,
        }
    }

    /// An app installed by running a script instead of downloading a file:
    /// `program` is launched with `args` and nothing is downloaded. Its
    /// installed status is whether `exe` responds to `--version`.
    pub const fn from_script(
        name: &'static str,
        description: &'static str,
        exe: &'static str,
        program: &'static str,
        args: &'static [&'static str],
    ) -> Self {
        Self {
            name,
            description,
            link: "",
            args: &[],
            script: Some(StoreScript::Command(program, args)),
            exe,
        }
    }

    /// An npm-style global package, installed with bun when bun is found on
    /// PATH, npm otherwise.
    pub const fn from_package(
        name: &'static str,
        description: &'static str,
        exe: &'static str,
        package: &'static str,
    ) -> Self {
        Self {
            name,
            description,
            link: "",
            args: &[],
            script: Some(StoreScript::GlobalPackage(package)),
            exe,
        }
    }
}

/// A group of Store apps, shown under a header on the Store tab.
pub struct StoreCategory {
    /// Header text, e.g. "Multimedia".
    pub title: &'static str,
    /// The apps in this category, in display order.
    pub apps: &'static [StoreApp],
}

/// The Store apps, grouped by category in display order. Add a `StoreApp` to a
/// category to add an app; nothing else needs wiring up.
pub static STORE_CATEGORIES: [StoreCategory; 8] = [
    StoreCategory {
        title: "IDEs & Dev Tools",
        apps: &[
            StoreApp::new(
                "VSCode",
                "Visual Studio Code — Microsoft's code editor",
                "Microsoft VS Code/Code.exe",
                "https://code.visualstudio.com/sha/download?build=stable&os=win32-x64-user",
            ),
            StoreApp::new(
                "Zed",
                "Fast, multiplayer code editor",
                "Zed/Zed.exe",
                "https://github.com/zed-industries/zed/releases/latest/download/Zed-x86_64.exe",
            ),
            StoreApp::new(
                "Git",
                "Git for Windows",
                "Git/cmd/git.exe",
                "https://github.com/git-for-windows/git/releases/latest/download/Git-*-64-bit.exe",
            ),
            StoreApp::from_script(
                "GitHub CLI",
                "GitHub's official command-line tool (gh)",
                "gh.exe",
                "winget",
                &["install", "--id", "GitHub.cli"],
            ),
            StoreApp::new(
                "Node",
                "Node.js JavaScript runtime",
                "nodejs/node.exe",
                "https://nodejs.org/dist/latest/node-v*-x64.msi",
            ),
            StoreApp::from_script(
                "Bun",
                "Bun JavaScript runtime and toolkit",
                "bun.exe",
                "powershell",
                &["-c", "irm bun.sh/install.ps1|iex"],
            ),
            StoreApp::from_script(
                "Uv",
                "Fast Python package manager",
                "uv.exe",
                "powershell",
                &["-ExecutionPolicy", "ByPass", "-c", "irm https://astral.sh/uv/install.ps1 | iex"],
            ),
            StoreApp::from_script(
                "Chocolatey",
                "Windows package manager",
                "chocolatey/bin/choco.exe",
                "powershell",
                &[
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Scope",
                    "Process",
                    "-Force",
                    "-c",
                    concat!(
                        "$ProgressPreference = 'SilentlyContinue'; ",
                        "[System.Net.ServicePointManager]::SecurityProtocol = ",
                        "[System.Net.ServicePointManager]::SecurityProtocol -bor 3072; ",
                        "iex ((New-Object System.Net.WebClient).DownloadString(",
                        "'https://community.chocolatey.org/install.ps1'))"
                    ),
                ],
            ),
            StoreApp::with_args(
                "Rust",
                "Rust toolchain installer (rustup)",
                ".cargo/bin/cargo.exe",
                "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe",
                &["-y"],
            ),
        ],
    },
    StoreCategory {
        title: "AI",
        apps: &[
            StoreApp::from_script(
                "OpenCode",
                "AI coding agent (CLI)",
                "opencode.exe",
                "bash",
                &["-c", "curl -fsSL https://opencode.ai/v2/install | bash"],
            ),
            StoreApp::from_script(
                "Pi",
                "Terminal coding-agent harness",
                "pi.cmd",
                "powershell",
                &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", "irm https://pi.dev/install.ps1 | iex"],
            ),
            StoreApp::from_package(
                "Command Code Harness",
                "command-code CLI, installed on bun or npm",
                "command-code",
                "command-code@latest",
            ),
        ],
    },
    StoreCategory {
        title: "Browsers",
        apps: &[
            StoreApp::new(
                "Chrome",
                "Google Chrome browser",
                "Google/Chrome/Application/chrome.exe",
                "https://dl.google.com/chrome/install/latest/chrome_installer.exe",
            ),
            StoreApp::new(
                "Brave",
                "Privacy-focused web browser",
                "BraveSoftware/Brave-Browser/Application/brave.exe",
                "https://github.com/brave/brave-browser/releases/latest/download/BraveBrowserStandaloneSilentSetup.exe",
            ),
            StoreApp::new(
                "LibreWolf",
                "Privacy-hardened Firefox fork",
                "LibreWolf/librewolf.exe",
                "https://dl.librewolf.net/librewolf/157.0.1-1/librewolf-157.0.1-1-windows-x86_64-setup.exe",
            ),
            StoreApp::new(
                "Zen",
                "Calm, Firefox-based browser",
                "Zen Browser/zen.exe",
                "https://github.com/zen-browser/desktop/releases/latest/download/zen.installer.exe",
            ),
            StoreApp::new(
                "Helium",
                "Helium — private, fast web browser",
                "Helium/helium.exe",
                "https://github.com/imputnet/helium-windows/releases/latest/download/helium_*_x64-installer.exe",
            ),
        ],
    },
    StoreCategory {
        title: "Multimedia",
        apps: &[
            StoreApp::from_script(
                "Mpv",
                "Media player",
                "mpv.exe",
                "choco",
                &["install", "mpv", "-y"],
            ),
            StoreApp::new(
                "VLC",
                "Open-source media player",
                "VideoLAN/VLC/vlc.exe",
                "https://get.videolan.org/vlc/last/win64/vlc-*-win64.exe",
            ),
            StoreApp::new(
                "OBS Studio",
                "Screen recording and live streaming",
                "obs-studio/bin/64bit/obs64.exe",
                "https://github.com/obsproject/obs-studio/releases/latest/download/OBS-Studio-*-Windows-x64-Installer.exe",
            ),
        ],
    },
    StoreCategory {
        title: "Communication",
        apps: &[
            StoreApp::new(
                "Discord",
                "Official Discord client",
                "Discord",
                "https://discord.com/api/downloads/distributions/app/installers/latest?channel=stable&platform=win&arch=x64",
            ),
            StoreApp::new(
                "Dorion",
                "Lightweight Discord client",
                "Dorion/Dorion.exe",
                "https://github.com/SpikeHD/Dorion/releases/latest/download/Dorion_*_x64-setup.exe",
            ),
        ],
    },
    StoreCategory {
        title: "Utilities",
        apps: &[
            StoreApp::new(
                "LocalSend",
                "Share files with nearby devices",
                "LocalSend/localsend_app.exe",
                "https://github.com/localsend/localsend/releases/latest/download/LocalSend-*-windows-x86-64.exe",
            ),
            StoreApp::new(
                "f.lux",
                "Screen color temperature adjuster",
                "FluxSoftware\\Flux\\flux.exe",
                "https://justgetflux.com/flux-setup.exe",
            ),
            StoreApp::new(
                "Oxdm",
                "Download manager",
                "oxdm.exe",
                "https://github.com/jd1378/oxdm/releases/latest/download/oxdm-*-x86_64-pc-windows-msvc.tar.gz",
            ),
            StoreApp::new(
                "qBittorrent",
                "Open-source BitTorrent client",
                "qBittorrent/qbittorrent.exe",
                "https://github.com/qbittorrent/qBittorrent/releases/latest/download/qbittorrent_*_x64_setup.exe",
            ),
            StoreApp::from_script(
                "WinRAR",
                "Archive manager",
                "WinRAR/WinRAR.exe",
                "choco",
                &["install", "winrar", "-y"],
            ),
            StoreApp::with_args(
                "Bulk Crap Uninstaller",
                "Bulk uninstaller for Windows",
                "Bulk Crap Uninstaller/BCUninstaller.exe",
                "https://github.com/BCUninstaller/Bulk-Crap-Uninstaller/releases/latest/download/BCUninstaller_*_setup.exe",
                &["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"],
            ),
            StoreApp::new(
                "LightBulb",
                "Reduces eye strain by adjusting screen gamma",
                "LightBulb/LightBulb.exe",
                "https://github.com/Tyrrrz/LightBulb/releases/latest/download/LightBulb-Installer.win-x64.exe",
            ),
            StoreApp::from_script(
                "Bitwarden",
                "Open-source password manager",
                "Bitwarden/Bitwarden.exe",
                "choco",
                &["install", "bitwarden", "-y"],
            ),
            StoreApp::from_script(
                "Windows Terminal",
                "Modern terminal emulator for Windows",
                "chocolatey/bin/wt.exe",
                "choco",
                &["install", "microsoft-windows-terminal", "-y"],
            ),
            StoreApp::from_script(
                "7-Zip",
                "File archiver",
                "7-Zip/7z.exe",
                "choco",
                &["install", "7zip.install", "-y"],
            ),
            StoreApp::from_script(
                "Notepad++",
                "Source code editor and Notepad replacement",
                "Notepad++/notepad++.exe",
                "choco",
                &["install", "notepadplusplus.install", "-y"],
            ),
            StoreApp::new(
                "Obsidian",
                "Markdown notes and knowledge base",
                "Obsidian/Obsidian.exe",
                "https://github.com/obsidianmd/obsidian-releases/releases/latest/download/Obsidian-*.exe",
            ),
        ],
    },
    StoreCategory {
        title: "Games",
        apps: &[StoreApp::new(
            "Steam",
            "Game store and launcher",
                "Steam/steam.exe",
            "https://cdn.akamai.steamstatic.com/client/installer/SteamSetup.exe",
        )],
    },
    StoreCategory {
        title: "Creative",
        apps: &[
            StoreApp::new(
                "PhotoCraft",
                "Open-source image editor",
                "PhotoCraft",
                "https://github.com/storytold/photocraft/releases/latest/download/photocraft-*-windows-x64.msi",
            ),
            StoreApp::new(
                "VectorCraft",
                "Open-source vector illustration",
                "VectorCraft",
                "https://github.com/storytold/vectorcraft/releases/latest/download/vectorcraft-*-windows-x64.msi",
            ),
            StoreApp::new(
                "FilmCraft",
                "Open-source video editor",
                "FilmCraft",
                "https://github.com/storytold/filmcraft/releases/latest/download/filmcraft-*-windows-x64.msi",
            ),
            StoreApp::new(
                "LightCraft",
                "Open-source photo library and raw developer",
                "LightCraft",
                "https://github.com/storytold/lightcraft/releases/latest/download/lightcraft-*-windows-x64.msi",
            ),
            StoreApp::new(
                "PdfCraft",
                "Open-source PDF workbench",
                "PdfCraft",
                "https://github.com/storytold/pdfcraft/releases/latest/download/pdfcraft-*-windows-x64.msi",
            ),
            StoreApp::new(
                "EffectCraft",
                "Open-source motion graphics and VFX",
                "EffectCraft",
                "https://github.com/storytold/effectcraft/releases/latest/download/effectcraft-*-windows-x64.msi",
            ),
            StoreApp::new(
                "DesignCraft",
                "Open-source page layout and publishing",
                "DesignCraft",
                "https://github.com/storytold/designcraft/releases/latest/download/designcraft-*-windows-x64.msi",
            ),
            StoreApp::new(
                "FigCraft",
                "Open-source interface design tool",
                "FigCraft",
                "https://github.com/storytold/figcraft/releases/latest/download/figcraft-*-windows-x64.msi",
            ),
            StoreApp::new(
                "WordCraft",
                "Open-source word processor",
                "WordCraft",
                "https://github.com/storytold/wordcraft/releases/latest/download/wordcraft-*-windows-x64.msi",
            ),
            StoreApp::new(
                "SoundCraft",
                "Open-source digital audio workstation",
                "SoundCraft",
                "https://github.com/storytold/soundcraft/releases/latest/download/soundcraft-*-windows-x64.msi",
            ),
            StoreApp::new(
                "DeckCraft",
                "Open-source presentation app",
                "DeckCraft",
                "https://github.com/storytold/deckcraft/releases/latest/download/deckcraft-*-windows-x64.msi",
            ),
            StoreApp::new(
                "CADCraft",
                "Open-source CAD and drafting",
                "CADCraft",
                "https://github.com/storytold/cadcraft/releases/latest/download/cadcraft-*-windows-x64.msi",
            ),
            StoreApp::new(
                "GridCraft",
                "Open-source spreadsheets and data analysis",
                "GridCraft",
                "https://github.com/storytold/gridcraft/releases/latest/download/gridcraft-*-windows-x64.msi",
            ),
        ],
    },
];
