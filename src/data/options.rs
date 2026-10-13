//! The user-facing option lists for each tab. Every entry is a
//! `(name, description, needs_admin)` triple, so the pages can build their
//! rows from here without hardcoding the text, and admin-only rows can be
//! badged in the UI.

/// Rows of the Cleaning tab, in display order.
pub const CLEANING_OPTIONS: [(&str, &str, bool); 4] = [
    ("Recycle Bin", "Empty the Recycle Bin to free up disk space", false),
    (
        "Windows Temp",
        "Delete system temp files from C:\\Windows\\Temp",
        false,
    ),
    (
        "User Temp",
        "Delete per-user temp files from %LOCALAPPDATA%\\Temp",
        false,
    ),
    ("Disk Clean", "Will open windows disk cleaning utility", false),
];

/// Rows of the Tweaks tab, in display order.
pub const TWEAKS_OPTIONS: [(&str, &str, bool); 26] = [
    (
        "Toggle Dark Mode",
        "Switch system apps between light and dark theme",
        false,
    ),
    (
        "Show File Extensions",
        "Show file extensions in File Explorer (.exe, .png, etc.)",
        false,
    ),
    (
        "Show Hidden Files",
        "Show or hide hidden files and folders in File Explorer",
        false,
    ),
    ("Windows Activation", "Activate Windows with massgrave", true),
    (
        "Wallpaper compression",
        "Disable the compression of wallpapers in Windows 11/10 to improve quality",
        false,
    ),
    (
        "Remove Preinstalled Apps",
        "Uninstall preinstalled store apps (Solitaire, Copilot, Xbox overlays, etc.) for all users",
        true,
    ),
    (
        "Edge Debloat",
        "Disables telemetry, popups and other annoyances in Microsoft Edge",
        true,
    ),
    (
        "Start Menu Bing Search",
        "Toggles Bing web search results in Windows Search",
        false,
    ),
    (
        "Classic Right-Click Menu",
        "Restores the Windows 10 full context menu in File Explorer instead of the Windows 11 one",
        false,
    ),
    (
        "Windows AI",
        "Removes and disables all AI features/packages (Recall, Copilot, Notepad AI)",
        true,
    ),
    (
        "Reserved Storage",
        "Toggles the ~7 GB Windows reserves for updates; freeing it helps small drives",
        true,
    ),
    (
        "Remove OneDrive",
        "Uninstalls OneDrive with its own uninstaller and cleans the leftover files",
        true,
    ),
    (
        "Activity History",
        "Stops Windows from publishing or uploading user activities; keeps clipboard history",
        true,
    ),
    (
        "Disable BitLocker",
        "Disables BitLocker encryption on the system drive (decrypts in the background)",
        true,
    ),
    (
        "Consumer Features",
        "Stops promoted app installs and reduces Microsoft Store app suggestions",
        true,
    ),
    (
        "Delivery Optimization",
        "Stops Windows from using your bandwidth to upload updates to other PCs",
        true,
    ),
    (
        "Location Tracking",
        "Denies app location access and switches off the location sensor",
        true,
    ),
    (
        "Store Suggestions",
        "Toggles suggested Microsoft Store apps in Start menu search",
        false,
    ),
    (
        "Device Companion Apps",
        "Prevents extra software from installing when plugging in devices",
        true,
    ),
    (
        "Services to Manual",
        "Sets unneeded services to manual/disabled and reduces svchost.exe count",
        true,
    ),
    (
        "Explorer Home & Gallery",
        "Hides Home and Gallery in File Explorer and opens This PC by default",
        false,
    ),
    (
        "Visual Effects",
        "Sets visual effects to best performance (no animations, shadows or transparency)",
        false,
    ),
    (
        "Background Apps",
        "Toggles whether Microsoft Store apps may run in the background",
        false,
    ),
    (
        "Hyper-V",
        "Enables the Hyper-V virtualization feature (restart required)",
        true,
    ),
    (
        "WSL",
        "Enables Windows Subsystem for Linux (restart required)",
        true,
    ),
    (
        "O&O ShutUp10++",
        "Downloads and launches the O&O ShutUp10++ privacy tool",
        false,
    ),
];
