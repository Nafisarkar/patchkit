//! Backend for the Tweaks tab: light/dark theme, File Explorer options (file
//! extensions and hidden files), and Windows activation.
//!
//! Registry values are read and written with `reg.exe`, so no extra dependency
//! is needed. The `*_enabled` / `*_visible` helpers are read-only (they fill
//! the "Status" column); the `toggle_*` / `show_*` / `activate_*` functions
//! change the system.

use crate::sys;

// ------------------------------------------------------------------ registry

/// `HKCU\...\Themes\Personalize` — holds the app/system theme values.
const PERSONALIZE: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
/// `HKCU\...\Explorer\Advanced` — holds the File Explorer view options.
const ADVANCED: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";

/// Read a REG_DWORD value from the registry, or `None` if it is missing or the
/// command fails.
fn reg_dword(path: &str, name: &str) -> Option<u32> {
    let out = sys::run("reg", &["query", path, "/v", name]).ok()?;
    // The value line looks like "    AppsUseLightTheme    REG_DWORD    0x1".
    let line = out.lines().find(|l| l.contains("REG_DWORD"))?;
    let raw = line.split_whitespace().last()?;
    u32::from_str_radix(raw.trim_start_matches("0x"), 16).ok()
}

/// Write a REG_DWORD value, creating it if necessary.
fn set_dword(path: &str, name: &str, value: u32) -> Result<(), String> {
    sys::run(
        "reg",
        &[
            "add",
            path,
            "/v",
            name,
            "/t",
            "REG_DWORD",
            "/d",
            &value.to_string(),
            "/f",
        ],
    )
    .map(|_| ())
}

// ----------------------------------------------------------------- dark mode

/// True when apps use the dark theme.
pub fn dark_mode_enabled() -> Option<bool> {
    // AppsUseLightTheme: 0 = dark, 1 = light.
    Some(reg_dword(PERSONALIZE, "AppsUseLightTheme")? == 0)
}

/// Switch apps and the system between light and dark theme.
pub fn toggle_dark_mode() -> Result<String, String> {
    let now_dark = dark_mode_enabled().unwrap_or(false);
    let light = u32::from(now_dark);
    set_dword(PERSONALIZE, "AppsUseLightTheme", light)?;
    set_dword(PERSONALIZE, "SystemUsesLightTheme", light)?;
    let _ = sys::broadcast_setting_change();
    Ok(format!(
        "switched to {} theme",
        if now_dark { "light" } else { "dark" }
    ))
}

// ----------------------------------------------------------- file extensions

/// True when File Explorer shows file extensions.
pub fn file_extensions_visible() -> Option<bool> {
    // HideFileExt: 0 = extensions shown, 1 = hidden.
    Some(reg_dword(ADVANCED, "HideFileExt")? == 0)
}

/// Make File Explorer show file extensions (restarts Explorer to apply it).
pub fn show_file_extensions() -> Result<String, String> {
    if file_extensions_visible() == Some(true) {
        return Ok("file extensions are already shown".to_string());
    }
    set_dword(ADVANCED, "HideFileExt", 0)?;
    sys::restart_explorer()?;
    Ok("file extensions shown (Explorer restarted)".to_string())
}

// ------------------------------------------------------------- hidden files

/// True when File Explorer shows hidden files and folders.
pub fn hidden_files_visible() -> Option<bool> {
    // Hidden: 1 = shown, 2 = hidden.
    Some(reg_dword(ADVANCED, "Hidden")? == 1)
}

/// Show or hide hidden files and folders (restarts Explorer to apply it).
pub fn toggle_hidden_files() -> Result<String, String> {
    let now_visible = hidden_files_visible().unwrap_or(false);
    set_dword(ADVANCED, "Hidden", if now_visible { 2 } else { 1 })?;
    sys::restart_explorer()?;
    Ok(format!(
        "hidden files {}",
        if now_visible { "hidden" } else { "shown" }
    ))
}

// -------------------------------------------------------- windows activation

/// True when Windows is activated.
pub fn activation_status() -> Option<bool> {
    // LicenseStatus 1 == Licensed.
    let script = "(Get-CimInstance -ClassName SoftwareLicensingProduct \
        -Filter \"PartialProductKey is not null and \
        ApplicationID='55c92734-d682-4d71-983e-d6ec3f16059f'\").LicenseStatus";
    let out = sys::powershell(script).ok()?;
    // A machine can report more than one product line; it is licensed when any
    // of them reports status 1.
    Some(out.lines().any(|line| line.trim() == "1"))
}

/// Activate Windows with the Microsoft Activation Scripts (HWID method).
/// Requires administrator rights, and may take a while.
pub fn activate_windows() -> Result<String, String> {
    if !sys::is_admin() {
        return Err(
            "administrator rights required — restart PatchKit as administrator".to_string(),
        );
    }
    // Runs the official MAS launcher in the same PowerShell process.
    sys::powershell("& ([ScriptBlock]::Create((irm https://get.activated.win))) /HWID")?;
    Ok("activation completed".to_string())
}

// -------------------------------------------------------------------- status

/// Snapshot of the Tweaks tab's "Status" column.
#[derive(Default)]
pub struct TweaksStatus {
    /// Text for the "Toggle Dark Mode" row.
    pub dark_mode: String,
    /// Text for the "Show File Extensions" row.
    pub file_extensions: String,
    /// Text for the "Show Hidden Files" row.
    pub hidden_files: String,
    /// Text for the "Windows Activation" row.
    pub activation: String,
}

impl TweaksStatus {
    /// Full snapshot, including the slow activation check.
    pub fn new() -> Self {
        let mut status = Self::default();
        status.refresh();
        status.refresh_activation();
        status
    }

    /// Refresh the registry-backed values. Cheap; leaves `activation` untouched.
    pub fn refresh(&mut self) {
        self.dark_mode = label(dark_mode_enabled(), "Dark", "Light");
        self.file_extensions = label(file_extensions_visible(), "Shown", "Hidden");
        self.hidden_files = label(hidden_files_visible(), "Shown", "Hidden");
    }

    /// Refresh the activation status. Slow (spawns PowerShell/WMI), so it is
    /// only run at startup and after the activation tweak.
    pub fn refresh_activation(&mut self) {
        self.activation = label(activation_status(), "Activated", "Not activated");
    }
}

/// Turn an `Option<bool>` into a display string: `yes` for `Some(true)`, `no`
/// for `Some(false)`, and "unknown" for `None`.
fn label(value: Option<bool>, yes: &str, no: &str) -> String {
    match value {
        Some(true) => yes.to_string(),
        Some(false) => no.to_string(),
        None => "unknown".to_string(),
    }
}
