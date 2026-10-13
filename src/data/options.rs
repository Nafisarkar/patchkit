//! The user-facing option lists for each tab. Every entry is a
//! `(name, description)` pair, so the pages can build their rows from here
//! without hardcoding the text.

/// Rows of the Cleaning tab, in display order.
pub const CLEANING_OPTIONS: [(&str, &str); 4] = [
    ("Recycle Bin", "Empty the Recycle Bin to free up disk space"),
    (
        "Windows Temp",
        "Delete system temp files from C:\\Windows\\Temp",
    ),
    (
        "User Temp",
        "Delete per-user temp files from %LOCALAPPDATA%\\Temp",
    ),
    ("Disk Clean", "Will open windows disk cleaning utility"),
];

/// Rows of the Tweaks tab, in display order.
pub const TWEAKS_OPTIONS: [(&str, &str); 12] = [
    (
        "Toggle Dark Mode",
        "Switch system apps between light and dark theme",
    ),
    (
        "Show File Extensions",
        "Show file extensions in File Explorer (.exe, .png, etc.)",
    ),
    (
        "Show Hidden Files",
        "Show or hide hidden files and folders in File Explorer",
    ),
    ("Windows Activation", "Activate Windows with massgrave"),
    (
        "Wallpaper compression",
        "Disable the compression of wallpapers in Windows 11/10 to improve quality",
    ),
    (
        "Remove Preinstalled Apps",
        "Uninstall preinstalled store apps (Solitaire, Copilot, Xbox overlays, etc.) for all users",
    ),
    (
        "Edge Debloat",
        "Disables telemetry, popups and other annoyances in Microsoft Edge",
    ),
    (
        "Start Menu Bing Search",
        "Toggles Bing web search results in Windows Search",
    ),
    (
        "Classic Right-Click Menu",
        "Restores the Windows 10 full context menu in File Explorer instead of the Windows 11 one",
    ),
    (
        "Windows AI",
        "Removes and disables all AI features/packages (Recall, Copilot, Notepad AI)",
    ),
    (
        "Reserved Storage",
        "Toggles the ~7 GB Windows reserves for updates; freeing it helps small drives",
    ),
    (
        "Remove OneDrive",
        "Uninstalls OneDrive with its own uninstaller and cleans the leftover files",
    ),
];
