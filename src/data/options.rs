pub const CLEANING_OPTIONS: [(&str, &str); 3] = [
    ("Recycle Bin", "Empty the Recycle Bin to free up disk space"),
    ("Windows Temp", "Delete system temp files from C:\\Windows\\Temp"),
    (
        "User Temp",
        "Delete per-user temp files from %LOCALAPPDATA%\\Temp",
    ),
];

pub const TWEAKS_OPTIONS: [(&str, &str); 3] = [
    (
        "Toggle Dark Mode",
        "Switch system apps between light and dark theme",
    ),
    (
        "Show File Extensions",
        "Show file extensions in File Explorer (.exe, .png, etc.)",
    ),
    ("Windows Activation", "Activate Windows with massgrave"),
];
