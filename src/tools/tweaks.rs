//! Backend for the Tweaks tab: theme and File Explorer toggles, wallpaper
//! compression, Bing search, preinstalled app removal, Edge debloat, and
//! Windows activation.
//!
//! Registry values are read and written with `reg.exe`, so no extra
//! dependency is needed. The `*_enabled` / `*_visible` functions are
//! read-only (they fill the "Status" column); the `toggle_*` / action
//! functions change the system.

use crate::sys;
use std::path::{Path, PathBuf};

use super::label;

// ------------------------------------------------------------------ registry

/// `HKCU\...\Themes\Personalize` — holds the app/system theme values.
const PERSONALIZE: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
/// `HKCU\...\Explorer\Advanced` — holds the File Explorer view options.
const ADVANCED: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";

/// `HKCU\Control Panel\Desktop` — holds the wallpaper import quality.
const DESKTOP: &str = r"HKCU\Control Panel\Desktop";
/// `HKLM\SYSTEM\CurrentControlSet\Control\PriorityControl` — scheduler
/// priority separation.
const PRIORITY_CONTROL: &str = r"HKLM\SYSTEM\CurrentControlSet\Control\PriorityControl";

/// Read a REG_DWORD value from the registry, or `None` if it is missing or the
/// command fails.
fn reg_dword(path: &str, name: &str) -> Option<u32> {
    let out = sys::run("reg", &["query", path, "/v", name]).ok()?;
    // The value line looks like "    AppsUseLightTheme    REG_DWORD    0x1".
    let line = out.lines().find(|l| l.contains("REG_DWORD"))?;
    let raw = line.split_whitespace().last()?;
    u32::from_str_radix(raw.trim_start_matches("0x"), 16).ok()
}

/// Read a REG_SZ or REG_EXPAND_SZ (string) value, or `None` if it is missing
/// or the command fails.
fn reg_sz(path: &str, name: &str) -> Option<String> {
    let out = sys::run("reg", &["query", path, "/v", name]).ok()?;
    // The value line looks like "    Value    REG_SZ    Allow". Values are
    // also written as REG_EXPAND_SZ (variable-prefixed strings), which
    // contains "REG_SZ" when matched loosely — so only read the line whose
    // *type token* is a string type.
    let line = out
        .lines()
        .find(|l| l.split_whitespace().any(|t| t == "REG_SZ" || t == "REG_EXPAND_SZ"))?;
    // Everything after the type token is the data (it may contain spaces).
    let mut parts = line.split_whitespace().skip_while(|t| !t.starts_with("REG"));
    parts.next()?; // the type token
    Some(parts.collect::<Vec<_>>().join(" "))
}

/// Write a registry value of any supported type, creating it if necessary.
fn set_reg(path: &str, name: &str, reg_type: &str, data: &str) -> Result<(), String> {
    sys::run(
        "reg",
        &["add", path, "/v", name, "/t", reg_type, "/d", data, "/f"],
    )
    .map(|_| ())
}

/// Write a REG_DWORD value, creating it if necessary.
fn set_dword(path: &str, name: &str, value: u32) -> Result<(), String> {
    set_reg(path, name, "REG_DWORD", &value.to_string())
}

/// Error out unless the process runs with administrator rights.
fn require_admin() -> Result<(), String> {
    if sys::is_admin() {
        Ok(())
    } else {
        Err("administrator rights required — restart PatchKit as administrator".to_string())
    }
}

/// Apply a table of `(path, name, type, data)` registry values, continuing
/// past individual failures and reporting every value that failed.
fn apply_registry_values(values: &[(&str, &str, &str, &str)]) -> Result<(), String> {
    let failures: Vec<String> = values
        .iter()
        .filter_map(|(path, name, reg_type, data)| set_reg(path, name, reg_type, data).err())
        .collect();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

/// Set a Windows service's startup type ("disabled", "manual", ...).
/// Services that do not exist on this install are ignored.
fn set_service_start(name: &str, mode: &str) {
    let _ = sys::run("sc", &["config", name, "start=", mode]);
}

// ----------------------------------------------------------------- wallpaper compression

/// True when Windows still compresses wallpapers. The value defaults to 85
/// (compression on) when it has never been set; 100 disables compression.
pub fn wallpaper_compression_enabled() -> Option<bool> {
    Some(reg_dword(DESKTOP, "JPEGImportQuality").unwrap_or(85) != 100)
}

/// Disable the automatic JPEG compression of wallpapers by setting
/// `JPEGImportQuality` to 100. Windows applies it when the next wallpaper is
/// set, not at boot.
pub fn disable_wallpaper_compression() -> Result<String, String> {
    if wallpaper_compression_enabled() == Some(false) {
        return Ok("wallpaper compression is already disabled".to_string());
    }
    set_dword(DESKTOP, "JPEGImportQuality", 100)?;
    Ok("wallpaper compression disabled (applies on the next wallpaper)".to_string())
}

// ----------------------------------------------------------------- process priority

/// The value to write for a short, variable, high foreground boost: bits 1-0
/// 01 (variable), bits 3-2 10 (high boost), bit 5 1 (short quantum). See
/// https://forums.blurbusters.com/viewtopic.php?t=8535 — Windows defaults to
/// 2 (long, variable, no boost).
const WIN32_PRIORITY_HIGH_BOOST: u32 = 38;
/// The Windows default `Win32PrioritySeparation` value (long, variable, no
/// foreground boost) restored by the toggle.
const WIN32_PRIORITY_DEFAULT: u32 = 2;

/// True when the scheduler uses a high short variable foreground boost.
pub fn foreground_boost_enabled() -> Option<bool> {
    Some(
        reg_dword(PRIORITY_CONTROL, "Win32PrioritySeparation").unwrap_or(WIN32_PRIORITY_DEFAULT)
            == WIN32_PRIORITY_HIGH_BOOST,
    )
}

/// Toggle the scheduler's foreground process priority: on, foreground apps
/// get short, variable, high boost quanta (0x26); off, Windows' default
/// balance (long quanta, no boost). Applies system-wide from the next
/// scheduled task; no restart needed.
pub fn toggle_foreground_boost() -> Result<String, String> {
    require_admin()?;
    let current = reg_dword(PRIORITY_CONTROL, "Win32PrioritySeparation")
        .unwrap_or(WIN32_PRIORITY_DEFAULT);
    if current == WIN32_PRIORITY_HIGH_BOOST {
        set_dword(PRIORITY_CONTROL, "Win32PrioritySeparation", WIN32_PRIORITY_DEFAULT)?;
        Ok("process priority reset to Windows default (long, no boost)".to_string())
    } else {
        set_dword(PRIORITY_CONTROL, "Win32PrioritySeparation", WIN32_PRIORITY_HIGH_BOOST)?;
        Ok("process priority set to high foreground boost (short, 0x26)".to_string())
    }
}

// ----------------------------------------------------------------- webdav limit

/// `HKLM\SYSTEM\CurrentControlSet\Services\WebClient\Parameters` — the
/// WebDAV client transfer size limit.
const WEBDAV_PARAMETERS: &str = r"HKLM\SYSTEM\CurrentControlSet\Services\WebClient\Parameters";
/// The lifted limit: `0xffffffff` bytes ≈ 4 GB (the maximum the client
/// accepts); Windows ships at about 0x19000000 (~400 MB).
const WEBDAV_LIFTED_LIMIT: u32 = 0xffffffff;

/// True when the WebDAV client's file size limit is lifted to its maximum.
pub fn webdav_limit_lifted() -> Option<bool> {
    Some(
        reg_dword(WEBDAV_PARAMETERS, "FileSizeLimitInBytes").unwrap_or(0x19000000)
            == WEBDAV_LIFTED_LIMIT,
    )
}

/// Lift the WebDAV client (`WebClient` service) file size limit to about 4 GB
/// (`0xffffffff` bytes), so large files may be transferred over WebDAV drives.
/// Restart the WebClient service (or reboot) for the new limit to apply.
pub fn lift_webdav_limit() -> Result<String, String> {
    require_admin()?;
    if webdav_limit_lifted() == Some(true) {
        return Ok("the WebDAV file size limit is already lifted".to_string());
    }
    set_dword(WEBDAV_PARAMETERS, "FileSizeLimitInBytes", WEBDAV_LIFTED_LIMIT)?;
    Ok("WebDAV file size limit lifted to 4 GB (restart the WebClient service to apply)".to_string())
}

// ----------------------------------------------------------------- gpu scheduling

/// `HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers` — display
/// scheduling settings.
const GRAPHICS_DRIVERS: &str = r"HKLM\SYSTEM\CurrentControlSet\Control\GraphicsDrivers";

/// True when Hardware-Accelerated GPU Scheduling is on (`HwSchMode` 2);
/// Windows ships the feature off (`1`).
pub fn gpu_scheduling_enabled() -> Option<bool> {
    Some(reg_dword(GRAPHICS_DRIVERS, "HwSchMode").unwrap_or(1) == 2)
}

/// Toggle Hardware-Accelerated GPU Scheduling: on, the GPU manages its own
/// VRAM scheduling (lower latency, better 1% lows); off, back to the Windows
/// driver-driven default (`HwSchMode` 1). Trustworthy only on drivers that
/// support it; a restart is required for the change to take effect.
pub fn toggle_gpu_scheduling() -> Result<String, String> {
    require_admin()?;
    let on = if gpu_scheduling_enabled().unwrap_or(false) { 1 } else { 2 };
    set_dword(GRAPHICS_DRIVERS, "HwSchMode", on)?;
    Ok(format!(
        "GPU scheduling {} — restart to apply",
        if on == 2 { "enabled" } else { "disabled (default)" }
    ))
}

// ----------------------------------------------------------------- multimedia scheduler

/// `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile`
/// — the Multimedia Class Scheduler Service (MMCSS) tuning values.
const MMCSS_PROFILE: &str = r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile";

/// Optimize the Multimedia Class Scheduler Service (MMCSS) for low latency:
/// disables lazy startup, keeps the service always on, removes network
/// throttling, raises system responsiveness to its minimum, and gives game
/// tasks high CPU, I/O and GPU priority. Registry only; applies without a
/// restart. Requires administrator rights.
pub fn apply_multimedia_scheduler() -> Result<String, String> {
    require_admin()?;
    apply_registry_values(&[
        (MMCSS_PROFILE, "NoLazyMode", "REG_DWORD", "1"),
        (MMCSS_PROFILE, "AlwaysOn", "REG_DWORD", "1"),
        // 0xffffffff removes network throttling entirely.
        (MMCSS_PROFILE, "NetworkThrottlingIndex", "REG_DWORD", "0xffffffff"),
        // The minimum meaningful boost; clamped values below 10 count as 20.
        (MMCSS_PROFILE, "SystemResponsiveness", "REG_DWORD", "10"),
        (r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games", "Priority", "REG_DWORD", "2"),
        (r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games", "Scheduling Category", "REG_SZ", "High"),
        (r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games", "SFIO Priority", "REG_SZ", "High"),
        (r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games", "GPU Priority", "REG_DWORD", "8"),
    ])?;
    Ok("multimedia scheduler optimized for low latency".to_string())
}

/// True when the MMCSS optimizations from [`apply_multimedia_scheduler`] are
/// in place (checking the two signature values the apply writes).
pub fn multimedia_scheduler_applied() -> Option<bool> {
    let profile = reg_dword(MMCSS_PROFILE, "NetworkThrottlingIndex") == Some(0xffffffff)
        && reg_dword(MMCSS_PROFILE, "SystemResponsiveness") == Some(10);
    let games = reg_dword(
        r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile\Tasks\Games",
        "GPU Priority",
    );
    Some(profile && games == Some(8))
}

// ----------------------------------------------------------------- remove preinstalled apps

const PREINSTALLED_APPS: [&str; 43] = [
    "Microsoft.3DBuilder",
    "Microsoft.BingWeather",
    "Microsoft.Microsoft3DViewer",
    "Microsoft.Copilot",
    "Microsoft.549981C3F5F10",
    "Microsoft.Windows.DevHome",
    "Microsoft.WindowsFeedbackHub",
    "Microsoft.GetHelp",
    "Microsoft.ZuneMusic",
    "microsoft.windowscommunicationsapps",
    "Microsoft.WindowsMaps",
    "Microsoft.MicrosoftOfficeHub",
    "Clipchamp.Clipchamp",
    "Microsoft.Wallet",
    "Microsoft.MixedReality.Portal",
    "MicrosoftTeams",
    "MSTeams",
    "Microsoft.Todos",
    "Microsoft.Whiteboard",
    "Microsoft.ZuneVideo",
    "Microsoft.BingNews",
    "Microsoft.Office.OneNote",
    "Microsoft.MSPaint",
    "Microsoft.People",
    "Microsoft.YourPhone",
    "Microsoft.PowerAutomateDesktop",
    "Microsoft.MicrosoftPowerBIDesktop",
    "MicrosoftCorporationII.QuickAssist",
    "Microsoft.SkypeApp",
    "Microsoft.MicrosoftSolitaireCollection",
    "Microsoft.StorePurchaseApp",
    "Microsoft.Getstarted",
    "Microsoft.WindowsAlarms",
    "Microsoft.VP9VideoExtensions",
    "Microsoft.WebMediaExtensions",
    "Microsoft.WebpImageExtension",
    "Microsoft.GamingApp",
    "Microsoft.XboxSpeechToTextOverlay",
    "Microsoft.XboxApp",
    "Microsoft.XboxGamingOverlay",
    "Microsoft.XboxGameOverlay",
    "Microsoft.Xbox.TCUI",
    "Microsoft.XboxIdentityProvider",
];

/// Remove the preinstalled store apps for the current user, for all users,
/// and from the provisioned list (so they do not come back for new users).
/// Requires administrator rights, and may take a while.
pub fn remove_preinstalled_apps() -> Result<String, String> {
    require_admin()?;

    let names = PREINSTALLED_APPS
        .iter()
        .map(|app| format!("'{app}'"))
        .collect::<Vec<_>>()
        .join(", ");

    // Ask which of the apps are installed first, so the result only lists
    // apps that were actually present. `-Name` does not take an array on
    // Windows PowerShell 5.1, so query one app at a time.
    let out = sys::powershell(&format!(
        "ForEach($app in @({names})) {{ \
            (Get-AppxPackage -Name $app -ErrorAction SilentlyContinue).Name \
        }}"
    ))?;
    // Get-AppxPackage reports one row per architecture, so deduplicate.
    let mut installed: Vec<&str> = out
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    installed.sort_unstable();
    installed.dedup();
    if installed.is_empty() {
        return Ok("no preinstalled apps were found".to_string());
    }

    // One PowerShell run does all three passes. Errors are silenced so one
    // locked or absent app does not abort the rest; some apps only fully
    // disappear after a restart.
    let script = format!(
        "$appname = @({names})
ForEach($app in $appname) {{
    Get-AppxPackage -Name $app | Remove-AppxPackage -ErrorAction SilentlyContinue
}}
ForEach($app in $appname) {{
    Get-AppxPackage -AllUsers -Name $app | Remove-AppxPackage -AllUsers -ErrorAction SilentlyContinue
}}
ForEach($app in $appname) {{
    Get-AppxProvisionedPackage -Online |
        Where-Object {{ $_.PackageName -like ('*' + $app + '*') }} |
        Remove-AppxProvisionedPackage -Online -ErrorAction SilentlyContinue
}}"
    );
    sys::powershell(&script)?;

    Ok(format!(
        "preinstalled apps removed where present: {}",
        installed.join(", ")
    ))
}

// ----------------------------------------------------------------- edge debloat

/// `HKLM\...\Policies\Microsoft\Edge` — Edge policy values.
const EDGE: &str = r"HKLM\SOFTWARE\Policies\Microsoft\Edge";
/// `HKLM\...\Policies\Microsoft\Edge\ExtensionInstallBlocklist`.
const EDGE_BLOCKLIST: &str = r"HKLM\SOFTWARE\Policies\Microsoft\Edge\ExtensionInstallBlocklist";
/// `HKLM\...\Policies\Microsoft\EdgeUpdate`.
const EDGE_UPDATE: &str = r"HKLM\SOFTWARE\Policies\Microsoft\EdgeUpdate";

/// The policy values applied by the Edge debloat: `(path, name, type, data)`.
const EDGE_POLICY_VALUES: [(&str, &str, &str, &str); 17] = [
    (EDGE_UPDATE, "CreateDesktopShortcutDefault", "REG_DWORD", "0"),
    (EDGE, "PersonalizationReportingEnabled", "REG_DWORD", "0"),
    (
        EDGE_BLOCKLIST,
        "1",
        "REG_SZ",
        "ofefcgjbeghpigppfmkologfjadafddi",
    ),
    (EDGE, "ShowRecommendationsEnabled", "REG_DWORD", "0"),
    (EDGE, "HideFirstRunExperience", "REG_DWORD", "1"),
    (EDGE, "UserFeedbackAllowed", "REG_DWORD", "0"),
    (EDGE, "ConfigureDoNotTrack", "REG_DWORD", "1"),
    (EDGE, "AlternateErrorPagesEnabled", "REG_DWORD", "0"),
    (EDGE, "EdgeCollectionsEnabled", "REG_DWORD", "0"),
    (EDGE, "EdgeShoppingAssistantEnabled", "REG_DWORD", "0"),
    (EDGE, "MicrosoftEdgeInsiderPromotionEnabled", "REG_DWORD", "0"),
    (EDGE, "ShowMicrosoftRewards", "REG_DWORD", "0"),
    (EDGE, "WebWidgetAllowed", "REG_DWORD", "0"),
    (EDGE, "DiagnosticData", "REG_DWORD", "0"),
    (EDGE, "EdgeAssetDeliveryServiceEnabled", "REG_DWORD", "0"),
    (EDGE, "WalletDonationEnabled", "REG_DWORD", "0"),
    (EDGE, "DefaultBrowserSettingsCampaignEnabled", "REG_DWORD", "0"),
];

/// True when the Edge debloat policies are applied. `HideFirstRunExperience`
/// doubles as the sentinel: it only exists once the policy set was written,
/// so it is a proxy — a value set by another tool also reads as "Applied".
pub fn edge_debloat_enabled() -> Option<bool> {
    Some(reg_dword(EDGE, "HideFirstRunExperience").unwrap_or(0) == 1)
}

/// Apply the Edge debloat policies: telemetry, popups and promotions off,
/// Do Not Track on. Requires administrator rights. One failed write does not
/// stop the rest; every failure is reported at the end.
pub fn edge_debloat() -> Result<String, String> {
    require_admin()?;
    apply_registry_values(&EDGE_POLICY_VALUES)
        .map_err(|failures| format!("some Edge policies failed: {failures}"))?;
    Ok("edge debloat policies applied".to_string())
}

// ----------------------------------------------------------------- bing search

/// `HKCU\...\CurrentVersion\Search` — Start Menu search settings.
const SEARCH: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Search";

/// `HKCU\Software\Policies\Microsoft\Windows\Explorer` — per-user Start
/// Menu policies.
const EXPLORER_POLICIES_USER: &str = r"HKCU\Software\Policies\Microsoft\Windows\Explorer";

/// True when Start Menu search shows Bing web results. A missing value means
/// the Windows default, which is enabled.
pub fn bing_search_enabled() -> Option<bool> {
    // The value is inverted: BingSearchEnabled = 1 turns Bing *off*.
    Some(reg_dword(SEARCH, "BingSearchEnabled").unwrap_or(0) == 0)
}

/// Toggle Bing web search results in the Start Menu (restarts Explorer so
/// the change applies immediately).
pub fn toggle_bing_search() -> Result<String, String> {
    let now_enabled = bing_search_enabled().unwrap_or(true);
    // Writing 1 disables Bing, 0 enables it — hence the non-obvious value.
    set_dword(SEARCH, "BingSearchEnabled", u32::from(now_enabled))?;
    // DisableSearchBoxSuggestions is the policy that stops the search box
    // from reaching the web at all: set it while disabling, remove it when
    // re-enabling. The delete is ignored — the value may not exist yet.
    if now_enabled {
        set_dword(EXPLORER_POLICIES_USER, "DisableSearchBoxSuggestions", 1)?;
    } else {
        let _ = sys::run(
            "reg",
            &["delete", EXPLORER_POLICIES_USER, "/v", "DisableSearchBoxSuggestions", "/f"],
        );
    }
    sys::restart_explorer()?;
    Ok(format!(
        "Bing search {} in the Start Menu",
        if now_enabled { "disabled" } else { "enabled" }
    ))
}

// ------------------------------------------------------- classic right-click menu

/// `HKCU\...\CLSID\{86ca1aa0-…}` — creating this key with an empty
/// `InprocServer32` default value makes File Explorer show the full
/// Windows 10-style right-click menu instead of the simplified Windows 11 one.
const CLASSIC_CONTEXT_MENU: &str =
    r"HKCU\Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}";

/// True when File Explorer uses the classic full right-click menu. The
/// override key only exists once the classic menu was enabled.
pub fn classic_context_menu_enabled() -> Option<bool> {
    Some(
        sys::run(
            "reg",
            &["query", &format!("{CLASSIC_CONTEXT_MENU}\\InprocServer32")],
        )
        .is_ok(),
    )
}

/// Toggle between the classic (Windows 10) full right-click menu and the
/// simplified Windows 11 menu (restarts Explorer to apply it).
pub fn toggle_classic_context_menu() -> Result<String, String> {
    if classic_context_menu_enabled().unwrap_or(false) {
        sys::run("reg", &["delete", CLASSIC_CONTEXT_MENU, "/f"])?;
        sys::restart_explorer()?;
        Ok("restored the Windows 11 right-click menu".to_string())
    } else {
        // The empty default value on InprocServer32 is the entire trick.
        // (`/ve` writes the key's unnamed default value; no /d needed.)
        sys::run(
            "reg",
            &["add", &format!("{CLASSIC_CONTEXT_MENU}\\InprocServer32"), "/ve", "/f"],
        )?;
        sys::restart_explorer()?;
        Ok("restored the classic right-click menu".to_string())
    }
}

// --------------------------------------------------------------- desktop icons

/// `HKCU\...\Explorer\HideDesktopIcons\NewStartPanel` — a value per
/// namespace CLSID: `1` hides the desktop icon, `0` (or the key's default)
/// shows it. Windows writes only the icons it hides, so each icon's absent
/// state differs — This PC, User folder and others are hidden by default.
const DESKTOP_ICONS: &str =
    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\HideDesktopIcons\NewStartPanel";
/// This PC ("{20D04FE0-…}") — hidden by default on the desktop.
const THIS_PC_ICON: &str = "{20D04FE0-3AEA-1069-A2D8-08002B30309D}";
/// Recycle Bin ("{645FF040-…}") — shown by default.
const RECYCLE_BIN_ICON: &str = "{645FF040-5081-101B-9F08-00AA002F954E}";
/// The user's profile folder ("{59031a47-…}") — hidden by default.
const USER_FOLDER_ICON: &str = "{59031a47-3f72-44a7-89c5-5595fe6b30ee}";
/// Network ("{F02C1A0D-…}") — hidden by default.
const NETWORK_ICON: &str = "{F02C1A0D-BE21-4350-88B0-7367FC96EF3C}";
/// Control Panel ("{5399E694-…}") — hidden by default.
const CONTROL_PANEL_ICON: &str = "{5399E694-6CE5-4D6C-8FCE-1D8870FDCBA0}";

/// True when a desktop icon is shown; an absent value counts as
/// `default_shown` (the icon's stock state on a fresh install).
fn desktop_icon_shown(name: &str, default_shown: bool) -> Option<bool> {
    Some(reg_dword(DESKTOP_ICONS, name).unwrap_or(u32::from(!default_shown)) == 0)
}

/// Show or hide one desktop icon, then refresh the desktop via Explorer.
fn set_desktop_icon(name: &str, show: bool) -> Result<(), String> {
    set_dword(DESKTOP_ICONS, name, u32::from(!show))?;
    sys::restart_explorer()
}

/// Flip one desktop icon between shown and hidden.
fn toggle_desktop_icon(name: &str, default_shown: bool, what: &str) -> Result<String, String> {
    let showing = desktop_icon_shown(name, default_shown).unwrap_or(default_shown);
    set_desktop_icon(name, !showing)?;
    Ok(format!(
        "{what} icon {}",
        if showing { "hidden" } else { "shown" }
    ))
}

/// Toggle the This PC desktop icon.
pub fn toggle_this_pc_icon() -> Result<String, String> {
    toggle_desktop_icon(THIS_PC_ICON, false, "This PC")
}

/// True when This PC is shown on the desktop.
pub fn this_pc_icon_shown() -> Option<bool> {
    desktop_icon_shown(THIS_PC_ICON, false)
}

/// Toggle the Recycle Bin desktop icon.
pub fn toggle_recycle_bin_icon() -> Result<String, String> {
    toggle_desktop_icon(RECYCLE_BIN_ICON, true, "Recycle Bin")
}

/// True when the Recycle Bin is shown on the desktop.
pub fn recycle_bin_icon_shown() -> Option<bool> {
    desktop_icon_shown(RECYCLE_BIN_ICON, true)
}

/// Toggle the user profile folder desktop icon.
pub fn toggle_user_folder_icon() -> Result<String, String> {
    toggle_desktop_icon(USER_FOLDER_ICON, false, "user folder")
}

/// True when the user profile folder is shown on the desktop.
pub fn user_folder_icon_shown() -> Option<bool> {
    desktop_icon_shown(USER_FOLDER_ICON, false)
}

/// Toggle the Network desktop icon.
pub fn toggle_network_icon() -> Result<String, String> {
    toggle_desktop_icon(NETWORK_ICON, false, "Network")
}

/// True when Network is shown on the desktop.
pub fn network_icon_shown() -> Option<bool> {
    desktop_icon_shown(NETWORK_ICON, false)
}

/// Toggle the Control Panel desktop icon.
pub fn toggle_control_panel_icon() -> Result<String, String> {
    toggle_desktop_icon(CONTROL_PANEL_ICON, false, "Control Panel")
}

/// True when Control Panel is shown on the desktop.
pub fn control_panel_icon_shown() -> Option<bool> {
    desktop_icon_shown(CONTROL_PANEL_ICON, false)
}

/// True when the desktop shows icons at all (`HideIcons` 0 or absent).
pub fn desktop_icons_visible() -> Option<bool> {
    Some(reg_dword(ADVANCED, "HideIcons").unwrap_or(0) == 0)
}

/// Toggle the whole desktop icon visibility (right-click Desktop > view).
pub fn toggle_desktop_icons() -> Result<String, String> {
    let show = !desktop_icons_visible().unwrap_or(true);
    set_dword(ADVANCED, "HideIcons", u32::from(!show))?;
    sys::restart_explorer()?;
    Ok(format!(
        "desktop icons {}",
        if show { "shown" } else { "hidden" }
    ))
}

// --------------------------------------------------------------- shortcut arrow

/// `HKLM\...\Explorer\Shell Icons` — the value `29` overrides the shortcut
/// arrow overlay icon.
const SHELL_ICONS: &str = r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\Shell Icons";
/// A transparent icon resource inside shell32 (the classic no-arrow trick).
const BLANK_ARROW_ICON: &str = r"%windir%\System32\shell32.dll,-50";

/// True when shortcuts still show their arrow overlay: no override value, or
/// one that is not a known blank/transparent icon.
pub fn shortcut_arrow_visible() -> Option<bool> {
    let value = reg_sz(SHELL_ICONS, "29");
    Some(match value {
        None => true,
        Some(value) => !{
            let lower = value.to_ascii_lowercase();
            lower.ends_with(".dll,-50") || lower.ends_with(".dll,50")
                || lower.ends_with("blank.ico") || lower.ends_with("transparent.ico")
                || lower.ends_with("blankicon.ico")
        },
    })
}

/// Toggle the shortcut arrow overlay: hidden points the `Shell Icons`
/// `29` value at a transparent icon; shown deletes the value back to the
/// Windows default. HKLM, so administrator rights are needed.
pub fn toggle_shortcut_arrow() -> Result<String, String> {
    require_admin()?;
    if shortcut_arrow_visible().unwrap_or(true) {
        set_reg(SHELL_ICONS, "29", "REG_EXPAND_SZ", BLANK_ARROW_ICON)?;
        sys::restart_explorer()?;
        Ok("shortcut arrow removed".to_string())
    } else {
        sys::run("reg", &["delete", SHELL_ICONS, "/v", "29", "/f"])?;
        sys::restart_explorer()?;
        Ok("shortcut arrow restored".to_string())
    }
}

// ----------------------------------------------------------- user experience

/// `HKCU\...\Explorer\Serialize` — startup delay tuning.
const EXPLORER_SERIALIZE: &str =
    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Serialize";

/// True when the Explorer startup delay and menu delay are both zeroed.
/// MenuShowDelay is the Control Panel "MenuShowDelay" string metric.
pub fn explorer_menus_sped_up() -> Option<bool> {
    Some(
        reg_dword(EXPLORER_SERIALIZE, "StartupDelayInMSec").unwrap_or(400) == 0
            && reg_sz(DESKTOP, "MenuShowDelay").unwrap_or_default().trim() == "0",
    )
}

/// Toggle the Explorer startup delay and menu animation delay: on,
/// `StartupDelayInMSec` is 0 and `MenuShowDelay` 0 (menus open instantly);
/// off, both go back to the Windows defaults (no Serialize value, menu delay
/// 400).
pub fn toggle_explorer_menu_speed() -> Result<String, String> {
    if explorer_menus_sped_up().unwrap_or(false) {
        sys::run(
            "reg",
            &["delete", EXPLORER_SERIALIZE, "/v", "StartupDelayInMSec", "/f"],
        )?;
        set_reg(DESKTOP, "MenuShowDelay", "REG_SZ", "400")?;
        Ok("Explorer startup and menu delays restored".to_string())
    } else {
        set_dword(EXPLORER_SERIALIZE, "StartupDelayInMSec", 0)?;
        set_reg(DESKTOP, "MenuShowDelay", "REG_SZ", "0")?;
        Ok("Explorer startup and menu delays removed".to_string())
    }
}

/// True when the Start menu's Recommended section is hidden by policy.
pub fn start_recommended_hidden() -> Option<bool> {
    Some(reg_dword(EXPLORER_POLICIES_USER, "HideRecommendedSection").unwrap_or(0) == 1)
}

/// Toggle the Start menu Recommended section (Windows 11): hidden shows only
/// pinned apps and keeps its suggestions private; shown restores the default.
/// Applies after Explorer restarts (done for you).
pub fn toggle_start_recommended() -> Result<String, String> {
    if start_recommended_hidden().unwrap_or(false) {
        sys::run(
            "reg",
            &["delete", EXPLORER_POLICIES_USER, "/v", "HideRecommendedSection", "/f"],
        )?;
        sys::restart_explorer()?;
        Ok("Start menu Recommended section shown".to_string())
    } else {
        set_dword(EXPLORER_POLICIES_USER, "HideRecommendedSection", 1)?;
        sys::restart_explorer()?;
        Ok("Start menu Recommended section hidden".to_string())
    }
}

/// `HKLM\SOFTWARE\Microsoft\WindowsUpdate\UX\Settings` — pause settings.
const UPDATE_UX_SETTINGS: &str = r"HKLM\SOFTWARE\Microsoft\WindowsUpdate\UX\Settings";

/// True when the Windows Update pause slider can go to the maximum
/// (`FlightSettingsMaxPauseDays` at the DWORD maximum).
pub fn update_pause_limit_lifted() -> Option<bool> {
    Some(reg_dword(UPDATE_UX_SETTINGS, "FlightSettingsMaxPauseDays").unwrap_or(0) == 0xffffffff)
}

/// Let the Settings pause slider pause for as long as Windows will honor:
/// `FlightSettingsMaxPauseDays` at the DWORD maximum. Versions that dropped
/// the setting ignore it; a safe no-op either way. Requires admin rights.
pub fn lift_update_pause_limit() -> Result<String, String> {
    require_admin()?;
    if update_pause_limit_lifted() == Some(true) {
        return Ok("the update pause limit is already maximized".to_string());
    }
    set_dword(UPDATE_UX_SETTINGS, "FlightSettingsMaxPauseDays", 0xffffffff)?;
    Ok("Windows Update pause slider maximized".to_string())
}

// ----------------------------------------------------------------- windows ai

/// `HKLM\...\Policies\Explorer` — hides settings pages such as the AI ones.
const EXPLORER_POLICIES: &str =
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer";
/// `HKLM\SOFTWARE\Policies\WindowsNotepad` — Notepad AI features policy.
const WINDOWS_NOTEPAD: &str = r"HKLM\SOFTWARE\Policies\WindowsNotepad";
/// `HKLM\...\Policies\Microsoft\Windows\WindowsAI` — Recall and Click To Do
/// AI policies. Mirror values written in HKCU for per-user coverage.
const WINDOWS_AI_POLICIES: &str = r"HKLM\SOFTWARE\Policies\Microsoft\Windows\WindowsAI";
const WINDOWS_AI_POLICIES_USER: &str = r"HKCU\SOFTWARE\Policies\Microsoft\Windows\WindowsAI";

/// True when the Windows AI features were disabled and removed.
pub fn windows_ai_disabled() -> Option<bool> {
    Some(
        reg_dword(WINDOWS_NOTEPAD, "DisableAIFeatures").unwrap_or(0) == 1
            && reg_dword(WINDOWS_AI_POLICIES, "DisableAIDataAnalysis").unwrap_or(0) == 1,
    )
}

/// Disable and remove Windows AI: Recall, Copilot, Notepad AI features and the
/// CoreAI package. Requires administrator rights, and may take a while.
pub fn disable_windows_ai() -> Result<String, String> {
    require_admin()?;
    set_reg(
        EXPLORER_POLICIES,
        "SettingsPageVisibility",
        "REG_SZ",
        "hide:aicomponents",
    )?;
    set_dword(WINDOWS_NOTEPAD, "DisableAIFeatures", 1)?;

    // Kill-switch policies: DisableAIDataAnalysis stops Recall snapshots (and
    // deletes existing ones); TurnOffSavingSnapshots mirrors the documented
    // "Turn off saving snapshots" policy; AllowRecallEnablement=0 is device
    // scope only and removes the Recall bits on 24H2 (26100.3915+);
    // DisableClickToDo turns off the Click To Do AI overlay.
    apply_registry_values(&[
        (WINDOWS_AI_POLICIES, "DisableAIDataAnalysis", "REG_DWORD", "1"),
        (WINDOWS_AI_POLICIES, "AllowRecallEnablement", "REG_DWORD", "0"),
        (WINDOWS_AI_POLICIES, "TurnOffSavingSnapshots", "REG_DWORD", "1"),
        (WINDOWS_AI_POLICIES, "DisableClickToDo", "REG_DWORD", "1"),
        (WINDOWS_AI_POLICIES_USER, "DisableAIDataAnalysis", "REG_DWORD", "1"),
        (WINDOWS_AI_POLICIES_USER, "TurnOffSavingSnapshots", "REG_DWORD", "1"),
        (WINDOWS_AI_POLICIES_USER, "DisableClickToDo", "REG_DWORD", "1"),
    ])?;

    // Errors inside the script are silenced so one absent component (no
    // Recall, no winget, …) does not abort the rest.
    let script = "\
$Appx = (Get-AppxPackage MicrosoftWindows.Client.CoreAI).PackageFullName
$Sid = (Get-LocalUser $Env:UserName).Sid.Value

if ($Appx -and $Sid) {
    New-Item \"HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Appx\\AppxAllUserStore\\EndOfLife\\$Sid\\$Appx\" -Force -ErrorAction SilentlyContinue | Out-Null
}

Get-AppxPackage -AllUsers '*Copilot*' | Remove-AppxPackage -AllUsers -ErrorAction SilentlyContinue
winget uninstall -e --name 'Copilot' --silent --force --accept-source-agreements 2>$null
Get-AppxPackage -AllUsers Microsoft.MicrosoftOfficeHub | Remove-AppxPackage -AllUsers -ErrorAction SilentlyContinue

if ($Appx) {
    Remove-AppxPackage $Appx -ErrorAction SilentlyContinue
}

Set-Service -Name WSAIFabricSvc -StartupType Disabled -ErrorAction SilentlyContinue
Disable-WindowsOptionalFeature -FeatureName Recall -Online -NoRestart -ErrorAction SilentlyContinue | Out-Null";
    sys::powershell(script)?;
    Ok("Windows AI disabled and removed (restart to finish)".to_string())
}

// ----------------------------------------------------------- reserved storage

/// True when Windows keeps its ~7 GB of reserved storage for updates. Windows
/// enables it by default. Detection parses `DISM /Get-ReservedStorageState`
/// output, so non-English systems report "unknown".
pub fn reserved_storage_enabled() -> Option<bool> {
    let out = sys::run("DISM", &["/Online", "/Get-ReservedStorageState"]).ok()?;
    // The state line reads "Reserved Storage State : Disabled" (English DISM).
    if out.contains("Disabled") {
        Some(false)
    } else if out.contains("Enabled") {
        Some(true)
    } else {
        None
    }
}

/// Toggle Windows Reserved Storage (requires administrator rights). Freeing
/// it helps small drives; re-enable it before major feature updates.
pub fn toggle_reserved_storage() -> Result<String, String> {
    require_admin()?;
    let state = if reserved_storage_enabled().unwrap_or(true) {
        "/State:Disabled"
    } else {
        "/State:Enabled"
    };
    sys::run("DISM", &["/Online", "/Set-ReservedStorageState", state])?;
    Ok(format!(
        "reserved storage {}",
        if state == "/State:Disabled" {
            "disabled"
        } else {
            "enabled"
        }
    ))
}

// ---------------------------------------------------------- activity history

/// `HKLM\...\Policies\Microsoft\Windows\System` — activity feed policies.
const WINDOWS_SYSTEM_POLICIES: &str = r"HKLM\SOFTWARE\Policies\Microsoft\Windows\System";

/// True when activity history publishing/uploading is disabled. The local
/// activity feed (and clipboard history) stays on.
pub fn activity_history_disabled() -> Option<bool> {
    Some(
        reg_dword(WINDOWS_SYSTEM_POLICIES, "PublishUserActivities").unwrap_or(1) == 0
            && reg_dword(WINDOWS_SYSTEM_POLICIES, "UploadUserActivities").unwrap_or(1) == 0,
    )
}

/// Stop Windows from publishing or uploading user activities while preserving
/// clipboard history. Requires administrator rights.
pub fn disable_activity_history() -> Result<String, String> {
    require_admin()?;
    set_dword(WINDOWS_SYSTEM_POLICIES, "EnableActivityFeed", 1)?;
    set_dword(WINDOWS_SYSTEM_POLICIES, "PublishUserActivities", 0)?;
    set_dword(WINDOWS_SYSTEM_POLICIES, "UploadUserActivities", 0)?;
    Ok("activity history publishing disabled".to_string())
}

// ----------------------------------------------------------------- bitlocker

/// True when BitLocker protection is on for the system drive. Detection
/// parses `manage-bde -status` output, so non-English systems report
/// "unknown".
pub fn bitlocker_enabled() -> Option<bool> {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
    let out = sys::run("manage-bde", &["-status", &drive]).ok()?;
    // The line reads "Protection Status: Protection On" (English manage-bde).
    if out.contains("Protection On") {
        Some(true)
    } else if out.contains("Protection Off") {
        Some(false)
    } else {
        None
    }
}

/// Disable BitLocker on the system drive; decryption runs in the background
/// afterwards. Requires administrator rights. Re-enabling is not offered here
/// because it needs the interactive recovery-key setup flow.
pub fn disable_bitlocker() -> Result<String, String> {
    require_admin()?;
    sys::powershell("Disable-BitLocker -MountPoint $Env:SystemDrive")?;
    Ok("BitLocker disabled (drive is decrypting in the background)".to_string())
}

// ---------------------------------------------------------- consumer features

/// `HKLM\...\Policies\Microsoft\Windows\CloudContent` — Store content policies.
const CLOUD_CONTENT: &str = r"HKLM\SOFTWARE\Policies\Microsoft\Windows\CloudContent";

/// True when promoted app installs and Store suggestions are disabled.
pub fn consumer_features_disabled() -> Option<bool> {
    Some(
        reg_dword(CLOUD_CONTENT, "DisableWindowsConsumerFeatures").unwrap_or(0) == 1
            && reg_dword(CLOUD_CONTENT, "DisableConsumerAccountStateContent").unwrap_or(0) == 1,
    )
}

/// Stop promoted app installs and reduce Microsoft Store app suggestions,
/// plus the Microsoft 365 account ads in Settings. Requires admin rights.
pub fn disable_consumer_features() -> Result<String, String> {
    require_admin()?;
    set_dword(CLOUD_CONTENT, "DisableWindowsConsumerFeatures", 1)?;
    set_dword(CLOUD_CONTENT, "DisableConsumerAccountStateContent", 1)?;
    Ok("consumer features and Microsoft 365 ads disabled".to_string())
}

// ----------------------------------------------------- delivery optimization

/// `HKLM\...\Policies\Microsoft\Windows\DeliveryOptimization`.
const DELIVERY_OPTIMIZATION: &str =
    r"HKLM\SOFTWARE\Policies\Microsoft\Windows\DeliveryOptimization";

/// True when Windows no longer shares updates with other PCs (download mode 0).
pub fn delivery_optimization_disabled() -> Option<bool> {
    Some(reg_dword(DELIVERY_OPTIMIZATION, "DODownloadMode").unwrap_or(1) == 0)
}

/// Stop Windows from using your bandwidth to upload updates to other PCs on
/// the internet or local network. Requires administrator rights.
pub fn disable_delivery_optimization() -> Result<String, String> {
    require_admin()?;
    set_dword(DELIVERY_OPTIMIZATION, "DODownloadMode", 0)?;
    Ok("delivery optimization disabled".to_string())
}

// --------------------------------------------------------- location tracking

/// `HKLM\...\CapabilityAccessManager\ConsentStore\location` — global location
/// consent for apps.
const LOCATION_CONSENT: &str =
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location";
/// Sensor override GUID for the location sensor.
const LOCATION_SENSOR_OVERRIDE: &str =
    r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Sensor\Overrides\{BFA794E4-F964-4FDB-90F6-51056BFE4B44}";
/// `HKLM\SYSTEM\Maps` — automatic maps updates.
const SYSTEM_MAPS: &str = r"HKLM\SYSTEM\Maps";

/// True when location tracking is on for apps. Windows allows it by default.
pub fn location_tracking_enabled() -> Option<bool> {
    Some(reg_sz(LOCATION_CONSENT, "Value").unwrap_or_else(|| "Allow".to_string()) == "Allow")
}

/// Disable location tracking: deny app access, switch off the location sensor
/// and set the Geolocation Service to disabled. Requires administrator rights.
pub fn disable_location_tracking() -> Result<String, String> {
    require_admin()?;
    set_service_start("lfsvc", "disabled");
    set_reg(LOCATION_CONSENT, "Value", "REG_SZ", "Deny")?;
    set_dword(LOCATION_SENSOR_OVERRIDE, "SensorPermissionState", 0)?;
    set_dword(SYSTEM_MAPS, "AutoUpdateEnabled", 0)?;
    Ok("location tracking disabled".to_string())
}

// ------------------------------------------------------------- store search

/// The Microsoft Store's local database, which feeds the recommended apps in
/// Start menu search.
fn store_db_path() -> Option<PathBuf> {
    Some(
        Path::new(&std::env::var_os("LocalAppData")?)
            .join("Packages\\Microsoft.WindowsStore_8wekyb3d8bbwe\\LocalState\\store.db"),
    )
}

/// True when icacls output shows a deny ACE. Older Windows builds render it
/// as `Everyone:(DENY)(F)`; newer builds render the same deny-special
/// "no access" mask without the DENY keyword, as `Everyone:(N)`.
pub(crate) fn icacls_shows_deny(out: &str) -> bool {
    out.lines()
        .any(|line| line.contains("(DENY)") || line.contains(":(N)"))
}

/// True when the Store database is locked for Everyone, hiding the suggested
/// Store apps in Start menu search.
pub fn store_search_disabled() -> Option<bool> {
    let db = store_db_path()?;
    // Newer Windows builds no longer ship store.db; nothing to lock.
    if !db.exists() {
        return Some(false);
    }
    let out = sys::run("icacls", &[&db.to_string_lossy()]).ok()?;
    Some(icacls_shows_deny(&out))
}

/// Toggle the suggested Microsoft Store apps in Start menu search results by
/// granting or denying Everyone access to the Store's local database.
pub fn toggle_store_search() -> Result<String, String> {
    let db = store_db_path()
        .ok_or("could not resolve the Microsoft Store database path")?
        .to_string_lossy()
        .into_owned();
    if !Path::new(&db).exists() {
        return Err(
            "store.db does not exist on this Windows build — nothing to toggle"
                .to_string(),
        );
    }
    if store_search_disabled().unwrap_or(false) {
        sys::run("icacls", &[&db, "/grant", "*S-1-1-0:F"])?;
        Ok("store suggestions shown again".to_string())
    } else {
        sys::run("icacls", &[&db, "/deny", "*S-1-1-0:F"])?;
        Ok("store suggestions hidden".to_string())
    }
}

// --------------------------------------------------------- device metadata

/// `HKLM\...\Policies\Microsoft\Windows\Device Metadata`.
const DEVICE_METADATA: &str = r"HKLM\SOFTWARE\Policies\Microsoft\Windows\Device Metadata";

/// True when Windows is blocked from downloading device metadata (companion
/// apps) from the internet.
pub fn device_metadata_blocked() -> Option<bool> {
    Some(
        reg_dword(DEVICE_METADATA, "PreventDeviceMetadataFromNetwork").unwrap_or(0) == 1,
    )
}

/// Prevent additional software from being installed when plugging in devices
/// (e.g. ads when plugging in a monitor). Requires administrator rights.
pub fn prevent_device_metadata() -> Result<String, String> {
    require_admin()?;
    set_dword(DEVICE_METADATA, "PreventDeviceMetadataFromNetwork", 1)?;
    Ok("device metadata downloads blocked".to_string())
}

// ------------------------------------------------------------------- services

/// True when the services tweak was applied, judged by the DiagTrack
/// (Connected User Experiences and Telemetry) service being disabled.
pub fn services_applied() -> Option<bool> {
    Some(
        reg_dword(r"HKLM\SYSTEM\CurrentControlSet\Services\DiagTrack", "Start").unwrap_or(2) == 4,
    )
}

/// The service startup types to enforce, `(name, sc config start= mode)`.
/// Services that do not exist on an install are skipped. The list mirrors the
/// classic "ConfigureServices" debloat: telemetry/remote/RPC service trees
/// disabled, rarely-used services manual, core services kept on automatic
/// (delayed where Windows uses it).
const SERVICE_STARTUP: [(&str, &str); 239] = [
    ("AJRouter", "disabled"),
    ("ALG", "manual"),
    ("AppIDSvc", "manual"),
    ("AppMgmt", "manual"),
    ("AppReadiness", "manual"),
    ("AppVClient", "disabled"),
    ("AppXSvc", "manual"),
    ("Appinfo", "manual"),
    ("AssignedAccessManagerSvc", "disabled"),
    ("AudioEndpointBuilder", "auto"),
    ("Audiosrv", "auto"),
    ("AxInstSV", "manual"),
    ("BDESVC", "manual"),
    ("BFE", "auto"),
    ("BITS", "delayed-auto"),
    ("BTAGService", "manual"),
    ("BrokerInfrastructure", "auto"),
    ("Browser", "manual"),
    ("BthAvctpSvc", "auto"),
    ("BthHFSrv", "auto"),
    ("CDPSvc", "manual"),
    ("COMSysApp", "manual"),
    ("CertPropSvc", "manual"),
    ("ClipSVC", "manual"),
    ("CoreMessagingRegistrar", "auto"),
    ("CryptSvc", "auto"),
    ("CscService", "manual"),
    ("DPS", "auto"),
    ("DcomLaunch", "auto"),
    ("DcpSvc", "manual"),
    ("DevQueryBroker", "manual"),
    ("DeviceAssociationService", "manual"),
    ("DeviceInstall", "manual"),
    ("Dhcp", "auto"),
    ("DiagTrack", "disabled"),
    ("DialogBlockingService", "disabled"),
    ("DispBrokerDesktopSvc", "auto"),
    ("DisplayEnhancementService", "manual"),
    ("DmEnrollmentSvc", "manual"),
    ("Dnscache", "auto"),
    ("EFS", "manual"),
    ("EapHost", "manual"),
    ("EntAppSvc", "manual"),
    ("EventLog", "auto"),
    ("EventSystem", "auto"),
    ("FDResPub", "manual"),
    ("Fax", "disabled"),
    ("FontCache", "auto"),
    ("FrameServer", "manual"),
    ("FrameServerMonitor", "manual"),
    ("GraphicsPerfSvc", "manual"),
    ("HomeGroupListener", "disabled"),
    ("HomeGroupProvider", "disabled"),
    ("HvHost", "manual"),
    ("IEEtwCollectorService", "manual"),
    ("IKEEXT", "manual"),
    ("InstallService", "manual"),
    ("InventorySvc", "manual"),
    ("IpxlatCfgSvc", "manual"),
    ("KeyIso", "auto"),
    ("KtmRm", "manual"),
    ("LSM", "auto"),
    ("LanmanServer", "manual"),
    ("LanmanWorkstation", "auto"),
    ("LicenseManager", "manual"),
    ("LxpSvc", "manual"),
    ("MSDTC", "manual"),
    ("MSiSCSI", "manual"),
    ("MapsBroker", "delayed-auto"),
    ("McpManagementService", "manual"),
    ("MicrosoftEdgeElevationService", "manual"),
    ("MixedRealityOpenXRSvc", "manual"),
    ("MpsSvc", "auto"),
    ("MsKeyboardFilter", "manual"),
    ("NaturalAuthentication", "manual"),
    ("NcaSvc", "manual"),
    ("NcbService", "manual"),
    ("NcdAutoSetup", "manual"),
    ("NetSetupSvc", "manual"),
    ("NetTcpPortSharing", "disabled"),
    ("Netman", "auto"),
    ("NgcCtnrSvc", "manual"),
    ("NgcSvc", "manual"),
    ("NlaSvc", "auto"),
    ("PNRPAutoReg", "manual"),
    ("PNRPsvc", "manual"),
    ("PcaSvc", "manual"),
    ("PeerDistSvc", "manual"),
    ("PerfHost", "manual"),
    ("PhoneSvc", "manual"),
    ("PlugPlay", "auto"),
    ("PolicyAgent", "manual"),
    ("Power", "auto"),
    ("PrintNotify", "manual"),
    ("ProfSvc", "auto"),
    ("PushToInstall", "manual"),
    ("QWAVE", "manual"),
    ("RasAuto", "manual"),
    ("RasMan", "manual"),
    ("RemoteAccess", "disabled"),
    ("RemoteRegistry", "disabled"),
    ("RetailDemo", "disabled"),
    ("RmSvc", "manual"),
    ("RpcEptMapper", "auto"),
    ("RpcLocator", "manual"),
    ("RpcSs", "auto"),
    ("SCPolicySvc", "manual"),
    ("SCardSvr", "manual"),
    ("SDRSVC", "manual"),
    ("SEMgrSvc", "manual"),
    ("SENS", "auto"),
    ("SNMPTRAP", "manual"),
    ("SSDPSRV", "manual"),
    ("SamSs", "auto"),
    ("ScDeviceEnum", "manual"),
    ("Schedule", "auto"),
    ("SensorDataService", "manual"),
    ("SensorService", "manual"),
    ("SensrSvc", "manual"),
    ("SessionEnv", "manual"),
    ("SharedAccess", "manual"),
    ("SharedRealitySvc", "manual"),
    ("ShellHWDetection", "auto"),
    ("SmsRouter", "manual"),
    ("Spooler", "manual"),
    ("SstpSvc", "manual"),
    ("StiSvc", "manual"),
    ("StorSvc", "manual"),
    ("SystemEventsBroker", "auto"),
    ("TabletInputService", "manual"),
    ("TapiSrv", "manual"),
    ("TermService", "manual"),
    ("Themes", "auto"),
    ("TieringEngineService", "manual"),
    ("TimeBroker", "manual"),
    ("TimeBrokerSvc", "manual"),
    ("TokenBroker", "manual"),
    ("TrkWks", "auto"),
    ("TroubleshootingSvc", "manual"),
    ("UI0Detect", "manual"),
    ("UevAgentService", "disabled"),
    ("UmRdpService", "manual"),
    ("UserManager", "auto"),
    ("UsoSvc", "manual"),
    ("VGAuthService", "manual"),
    ("VMTools", "manual"),
    ("VSS", "manual"),
    ("VacSvc", "manual"),
    ("VaultSvc", "auto"),
    ("W32Time", "manual"),
    ("WEPHOSTSVC", "manual"),
    ("WFDSConMgrSvc", "manual"),
    ("WMPNetworkSvc", "manual"),
    ("WManSvc", "manual"),
    ("WPDBusEnum", "manual"),
    ("WSearch", "manual"),
    ("WaaSMedicSvc", "manual"),
    ("WalletService", "manual"),
    ("WarpJITSvc", "manual"),
    ("WbioSrvc", "manual"),
    ("Wcmsvc", "auto"),
    ("WcsPlugInService", "manual"),
    ("WdiServiceHost", "manual"),
    ("WdiSystemHost", "manual"),
    ("WebClient", "manual"),
    ("Wecsvc", "manual"),
    ("WerSvc", "manual"),
    ("WiaRpc", "manual"),
    ("WinHttpAutoProxySvc", "manual"),
    ("WinRM", "manual"),
    ("Winmgmt", "auto"),
    ("WlanSvc", "auto"),
    ("WpcMonSvc", "manual"),
    ("WpnService", "manual"),
    ("XblAuthManager", "manual"),
    ("XblGameSave", "manual"),
    ("XboxGipSvc", "manual"),
    ("XboxNetApiSvc", "manual"),
    ("autotimesvc", "manual"),
    ("bthserv", "manual"),
    ("camsvc", "manual"),
    ("cloudidsvc", "manual"),
    ("dcsvc", "manual"),
    ("defragsvc", "manual"),
    ("diagnosticshub.standardcollector.service", "manual"),
    ("diagsvc", "manual"),
    ("dmwappushservice", "manual"),
    ("dot3svc", "manual"),
    ("edgeupdate", "manual"),
    ("edgeupdatem", "manual"),
    ("embeddedmode", "manual"),
    ("fdPHost", "manual"),
    ("fhsvc", "manual"),
    ("hidserv", "manual"),
    ("icssvc", "manual"),
    ("iphlpsvc", "auto"),
    ("lfsvc", "manual"),
    ("lltdsvc", "manual"),
    ("lmhosts", "manual"),
    ("msiserver", "manual"),
    ("netprofm", "manual"),
    ("nsi", "auto"),
    ("p2pimsvc", "manual"),
    ("p2psvc", "manual"),
    ("perceptionsimulation", "manual"),
    ("pla", "manual"),
    ("seclogon", "manual"),
    ("shpamsvc", "disabled"),
    ("smphost", "manual"),
    ("spectrum", "manual"),
    ("sppsvc", "delayed-auto"),
    ("ssh-agent", "disabled"),
    ("svsvc", "manual"),
    ("swprv", "manual"),
    ("tiledatamodelsvc", "auto"),
    ("tzautoupdate", "disabled"),
    ("upnphost", "manual"),
    ("vds", "manual"),
    ("vm3dservice", "manual"),
    ("vmicguestinterface", "manual"),
    ("vmicheartbeat", "manual"),
    ("vmickvpexchange", "manual"),
    ("vmicrdv", "manual"),
    ("vmicshutdown", "manual"),
    ("vmictimesync", "manual"),
    ("vmicvmsession", "manual"),
    ("vmicvss", "manual"),
    ("vmvss", "manual"),
    ("wbengine", "manual"),
    ("wcncsvc", "manual"),
    ("webthreatdefsvc", "manual"),
    ("wercplsupport", "manual"),
    ("wisvc", "manual"),
    ("wlidsvc", "manual"),
    ("wlpasvc", "manual"),
    ("wmiApSrv", "manual"),
    ("workfolderssvc", "manual"),
    ("wscsvc", "delayed-auto"),
    ("wuauserv", "manual"),

];

/// Set unneeded services to manual/disabled, restore the intended automatic
/// services to their default startup, and align the svchost splitting
/// threshold with installed memory, which reduces the number of svchost.exe
/// processes. Requires administrator rights.
pub fn set_services_manual() -> Result<String, String> {
    require_admin()?;
    // Some of these services do not exist on every install; skip failures.
    for (name, mode) in &SERVICE_STARTUP {
        set_service_start(name, mode);
    }
    // Splitting every service into its own svchost only pays off with plenty
    // of RAM; the threshold in KB should match the installed memory.
    sys::powershell(
        "\
$memory = (Get-CimInstance Win32_PhysicalMemory | Measure-Object Capacity -Sum).Sum / 1KB
Set-ItemProperty -Path 'HKLM:\\SYSTEM\\CurrentControlSet\\Control' \
    -Name SvcHostSplitThresholdInKB -Value $memory -Type DWord",
    )?;
    Ok("service startup types updated and svchost splitting tuned".to_string())
}

// ------------------------------------------------- explorer home and gallery

/// `HKCU\...\CLSID\{f874310e-…}` — the File Explorer Home namespace entry.
const EXPLORER_HOME: &str =
    r"HKCU\Software\Classes\CLSID\{f874310e-b6b7-47dc-bc84-b9e6b38f5903}";
/// `HKCU\...\CLSID\{e88865ea-…}` — the Gallery namespace entry.
const EXPLORER_GALLERY: &str =
    r"HKCU\Software\Classes\CLSID\{e88865ea-0e1c-4e20-9aa6-edcd0212c87c}";

/// True when File Explorer's Home and Gallery are hidden and it opens This
/// PC. The un-pin flags only exist once this tweak was applied.
pub fn explorer_home_hidden() -> Option<bool> {
    Some(
        reg_dword(EXPLORER_HOME, "System.IsPinnedToNameSpaceTree").unwrap_or(1) == 0,
    )
}

/// Toggle File Explorer's Home and Gallery pages and make This PC the default
/// location (restarts Explorer to apply it).
pub fn toggle_explorer_home() -> Result<String, String> {
    if explorer_home_hidden().unwrap_or(false) {
        // Restock the defaults: the values may not exist on every install,
        // so ignore the deletes instead of failing the whole restore.
        for key in [EXPLORER_HOME, EXPLORER_GALLERY] {
            let _ = sys::run(
                "reg",
                &["delete", key, "/v", "System.IsPinnedToNameSpaceTree", "/f"],
            );
        }
        let _ = sys::run("reg", &["delete", ADVANCED, "/v", "LaunchTo", "/f"]);
        sys::restart_explorer()?;
        Ok("Home and Gallery shown again".to_string())
    } else {
        set_dword(EXPLORER_HOME, "System.IsPinnedToNameSpaceTree", 0)?;
        set_dword(EXPLORER_GALLERY, "System.IsPinnedToNameSpaceTree", 0)?;
        set_dword(ADVANCED, "LaunchTo", 1)?;
        sys::restart_explorer()?;
        Ok("Home and Gallery hidden; This PC is the default".to_string())
    }
}

// ------------------------------------------------------------ visual effects

/// `HKCU\Control Panel\Desktop\WindowMetrics`.
const WINDOW_METRICS: &str = r"HKCU\Control Panel\Desktop\WindowMetrics";
/// `HKCU\Control Panel\Keyboard`.
const KEYBOARD: &str = r"HKCU\Control Panel\Keyboard";
/// `HKCU\...\Explorer\VisualEffects` — the visual effects preset selector.
const VISUAL_EFFECTS: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects";
/// `HKCU\Software\Microsoft\Windows\DWM`.
const DWM: &str = r"HKCU\Software\Microsoft\Windows\DWM";

/// The values that make up the "best performance" visual effects preset:
/// `(path, name, type, data)`. String values are REG_SZ because that is what
/// the Control Panel applets write and read back.
const VISUAL_FX_VALUES: [(&str, &str, &str, &str); 14] = [
    (DESKTOP, "DragFullWindows", "REG_SZ", "0"),
    (DESKTOP, "MenuShowDelay", "REG_SZ", "200"),
    (WINDOW_METRICS, "MinAnimate", "REG_SZ", "0"),
    (DESKTOP, "UserPreferencesMask", "REG_BINARY", "9012038010000000"),
    (KEYBOARD, "KeyboardDelay", "REG_SZ", "0"),
    (ADVANCED, "ListviewAlphaSelect", "REG_DWORD", "0"),
    (ADVANCED, "ListviewShadow", "REG_DWORD", "0"),
    (ADVANCED, "TaskbarAnimations", "REG_DWORD", "0"),
    (VISUAL_EFFECTS, "VisualFXSetting", "REG_DWORD", "3"),
    (DWM, "EnableAeroPeek", "REG_DWORD", "0"),
    (PERSONALIZE, "EnableTransparency", "REG_DWORD", "0"),
    (ADVANCED, "TaskbarMn", "REG_DWORD", "0"),
    (ADVANCED, "ShowTaskViewButton", "REG_DWORD", "0"),
    (SEARCH, "SearchboxTaskbarMode", "REG_DWORD", "0"),
];

/// True when the visual effects were set to best performance. VisualFXSetting
/// doubles as the sentinel: 3 is the "custom" preset the manual performance
/// setting produces.
pub fn visual_effects_performance() -> Option<bool> {
    Some(reg_dword(VISUAL_EFFECTS, "VisualFXSetting").unwrap_or(0) == 3)
}

/// Set the visual effects to best performance: no animations, shadows or
/// transparency, a shorter menu delay, and a minimal taskbar (restarts
/// Explorer to apply it).
pub fn set_visual_effects_performance() -> Result<String, String> {
    apply_registry_values(&VISUAL_FX_VALUES)
        .map_err(|failures| format!("some visual effect values failed: {failures}"))?;
    sys::restart_explorer()?;
    Ok("visual effects set to best performance".to_string())
}

// ----------------------------------------------------------- background apps

/// `HKCU\...\BackgroundAccessApplications` — global background app switch.
const BACKGROUND_ACCESS: &str =
    r"HKCU\Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications";

/// True when Store apps are blocked from running in the background. Windows
/// allows them by default.
pub fn background_apps_disabled() -> Option<bool> {
    Some(reg_dword(BACKGROUND_ACCESS, "GlobalUserDisabled").unwrap_or(0) == 1)
}

/// Toggle whether Microsoft Store apps may run in the background.
pub fn toggle_background_apps() -> Result<String, String> {
    let now_disabled = background_apps_disabled().unwrap_or(false);
    set_dword(
        BACKGROUND_ACCESS,
        "GlobalUserDisabled",
        u32::from(!now_disabled),
    )?;
    Ok(format!(
        "background apps {}",
        if now_disabled { "enabled" } else { "disabled" }
    ))
}

// -------------------------------------------------------------------- hyper-v

/// The Hyper-V optional feature's DISM name.
const HYPERV_FEATURE: &str = "Microsoft-Hyper-V-All";

/// True when the named optional feature is enabled. Detection parses
/// `DISM /Get-FeatureInfo` output, so non-English systems report "unknown".
fn dism_feature_enabled(feature: &str) -> Option<bool> {
    let out = sys::run(
        "DISM",
        &[
            "/Online",
            "/Get-FeatureInfo",
            &format!("/FeatureName:{feature}"),
        ],
    )
    .ok()?;
    if out.contains("Enabled") {
        Some(true)
    } else if out.contains("Disabled") {
        Some(false)
    } else {
        None
    }
}

/// Enable an optional Windows feature via DISM without restarting.
fn dism_enable_feature(feature: &str) -> Result<(), String> {
    sys::run(
        "DISM",
        &[
            "/Online",
            "/Enable-Feature",
            &format!("/FeatureName:{feature}"),
            "/All",
            "/NoRestart",
        ],
    )
    .map(|_| ())
}

pub fn hyperv_enabled() -> Option<bool> {
    dism_feature_enabled(HYPERV_FEATURE)
}

/// Enable the Hyper-V optional feature. Requires administrator rights and a
/// restart to finish.
pub fn enable_hyperv() -> Result<String, String> {
    require_admin()?;
    dism_enable_feature(HYPERV_FEATURE)?;
    Ok("Hyper-V enabled (restart to finish)".to_string())
}

// ------------------------------------------------------------------------ wsl

/// The Windows Subsystem for Linux optional features' DISM names.
const WSL_FEATURE: &str = "Microsoft-Windows-Subsystem-Linux";
const WSL_VM_PLATFORM: &str = "VirtualMachinePlatform";

pub fn wsl_enabled() -> Option<bool> {
    dism_feature_enabled(WSL_FEATURE)
}

/// Enable the Windows Subsystem for Linux features. Requires administrator
/// rights and a restart to finish.
pub fn enable_wsl() -> Result<String, String> {
    require_admin()?;
    let failures: Vec<String> = [WSL_VM_PLATFORM, WSL_FEATURE]
        .iter()
        .filter_map(|feature| dism_enable_feature(feature).err())
        .collect();
    if failures.is_empty() {
        Ok("WSL enabled (restart to finish)".to_string())
    } else {
        Err(format!(
            "some WSL features failed: {}",
            failures.join("; ")
        ))
    }
}

// ------------------------------------------------------------- o&o shutup10++

/// Official download URL of O&O ShutUp10++, a free privacy tool.
const OO_SHUTUP_URL: &str = "https://dl5.oo-software.com/files/ooshutup10/OOSU10.exe";

/// Where O&O ShutUp10++ is cached after the first download.
fn ooshutup_path() -> Option<PathBuf> {
    Some(Path::new(&std::env::var_os("LocalAppData")?).join("PatchKit\\OOSU10.exe"))
}

/// True when the O&O ShutUp10++ tool has been downloaded already.
pub fn ooshutup_downloaded() -> Option<bool> {
    Some(ooshutup_path()?.exists())
}

/// Download O&O ShutUp10++ (unless it is cached) and launch the tool.
pub fn run_ooshutup() -> Result<String, String> {
    let path = ooshutup_path().ok_or("could not resolve the download path")?;
    if !path.exists() {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("could not create {dir:?}: {e}"))?;
        }
        sys::run(
            "curl.exe",
            &[
                "-L",
                "--silent",
                "--show-error",
                "--fail",
                "-o",
                &path.to_string_lossy(),
                OO_SHUTUP_URL,
            ],
        )?;
    }
    sys::spawn(&path.to_string_lossy(), &[])?;
    Ok("O&O ShutUp10++ launched".to_string())
}

// -------------------------------------------------------------- remove onedrive

/// True when OneDrive is still installed: either the system-wide or the
/// per-user setup files are present.
pub fn onedrive_installed() -> Option<bool> {
    let system_root = std::env::var_os("SystemRoot")?;
    let local_app_data = std::env::var_os("LocalAppData")?;
    Some(
        Path::new(&system_root)
            .join("System32\\OneDriveSetup.exe")
            .exists()
            || Path::new(&local_app_data)
                .join("Microsoft\\OneDrive")
                .exists(),
    )
}

/// Remove Microsoft OneDrive with its own uninstaller, clean the leftovers,
/// and disable the OneSyncSvc sync service. Restarting Explorer afterwards
/// releases its handles on OneDrive files. Requires administrator rights.
pub fn remove_onedrive() -> Result<String, String> {
    require_admin()?;
    let script = "\
$OneDriveDir = $Env:OneDrive
if ($OneDriveDir -and (Test-Path $OneDriveDir)) {
    # Deny the Administrators group delete rights so the uninstaller cannot
    # take the user's files with it; grant them back afterwards.
    icacls $OneDriveDir /deny '*S-1-5-32-544:(D,DC)' | Out-Null
}

$setup = Join-Path $Env:SystemRoot 'System32\\OneDriveSetup.exe'
if (-not (Test-Path $setup)) {
    $setup = Join-Path $Env:LOCALAPPDATA 'Microsoft\\OneDrive\\OneDriveSetup.exe'
}
if (Test-Path $setup) {
    Start-Process -FilePath $setup -ArgumentList '/uninstall' -Wait
}

Stop-Process -Name FileCoAuth -ErrorAction SilentlyContinue
Remove-Item \"$Env:LOCALAPPDATA\\Microsoft\\OneDrive\" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item \"$Env:ProgramData\\Microsoft OneDrive\" -Recurse -Force -ErrorAction SilentlyContinue

if ($OneDriveDir -and (Test-Path $OneDriveDir)) {
    icacls $OneDriveDir /grant '*S-1-5-32-544:(D,DC)' | Out-Null
    # Only delete the folder when the user's files are all gone.
    if (-not (Get-ChildItem -Path $OneDriveDir)) {
        Remove-Item -Path $OneDriveDir -Recurse -Force
        [Environment]::SetEnvironmentVariable('OneDrive', $null, 'User')
    }
}";
    sys::powershell(script)?;
    // Disable the OneSyncSvc service *template*: the per-user instances
    // (OneSyncSvc_<SID>) are ACL-protected and cannot be set directly.
    set_dword(r"HKLM\SYSTEM\CurrentControlSet\Services\OneSyncSvc", "Start", 4)?;
    sys::restart_explorer()?;
    Ok("OneDrive removed (Explorer restarted)".to_string())
}

// ----------------------------------------------------------------- dark mode

/// True when apps use the dark theme. A missing value means the Windows
/// default, which is light.
pub fn dark_mode_enabled() -> Option<bool> {
    // AppsUseLightTheme: 0 = dark, 1 = light.
    Some(reg_dword(PERSONALIZE, "AppsUseLightTheme").unwrap_or(1) == 0)
}

/// The `*UsesLightTheme` data for the *next* theme state: the flags are
/// inverted (0 = dark, 1 = light), so toggling away from dark writes 1 and
/// toggling away from light writes 0. Kept pure so a unit test can pin the
/// polarity — writing back the value that was just read is a silent no-op.
fn flipped_uses_light(now_dark: bool) -> u32 {
    u32::from(now_dark)
}

/// Switch apps and the system between light and dark theme. Each of the two
/// theme values flips independently, so a split setup stays split instead of
/// being forced to match.
pub fn toggle_dark_mode() -> Result<String, String> {
    let now_dark = dark_mode_enabled().unwrap_or(false);
    let now_dark_system = reg_dword(PERSONALIZE, "SystemUsesLightTheme")
        .map(|value| value == 0)
        .unwrap_or(now_dark);
    set_dword(PERSONALIZE, "AppsUseLightTheme", flipped_uses_light(now_dark))?;
    set_dword(
        PERSONALIZE,
        "SystemUsesLightTheme",
        flipped_uses_light(now_dark_system),
    )?;
    sys::broadcast_setting_change();
    Ok(format!(
        "switched to {} theme",
        if now_dark { "light" } else { "dark" }
    ))
}

// ----------------------------------------------------------- file extensions

/// True when File Explorer shows file extensions. A missing value means the
/// Windows default, which is hidden.
pub fn file_extensions_visible() -> Option<bool> {
    // HideFileExt: 0 = extensions shown, 1 = hidden.
    Some(reg_dword(ADVANCED, "HideFileExt").unwrap_or(1) == 0)
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

/// True when File Explorer shows hidden files and folders. A missing value
/// means the Windows default, which is hidden.
pub fn hidden_files_visible() -> Option<bool> {
    // Hidden: 1 = shown, 2 = hidden.
    Some(reg_dword(ADVANCED, "Hidden").unwrap_or(2) == 1)
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
    require_admin()?;
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
    /// Text for the "Wallpaper compression" row.
    pub wallpaper: String,
    /// Text for the "Edge Debloat" row.
    pub edge: String,
    /// Text for the "Start Menu Bing Search" row.
    pub bing_search: String,
    /// Text for the "Classic Right-Click Menu" row.
    pub right_click_menu: String,
    /// Text for the "Windows AI" row.
    pub windows_ai: String,
    /// Text for the "Reserved Storage" row.
    pub reserved_storage: String,
    /// Text for the "Remove OneDrive" row.
    pub one_drive: String,
    /// Text for the "Activity History" row.
    pub activity_history: String,
    /// Text for the "Disable BitLocker" row.
    pub bitlocker: String,
    /// Text for the "Consumer Features" row.
    pub consumer_features: String,
    /// Text for the "Delivery Optimization" row.
    pub delivery_optimization: String,
    /// Text for the "Location Tracking" row.
    pub location: String,
    /// Text for the "Store Suggestions" row.
    pub store_search: String,
    /// Text for the "Device Companion Apps" row.
    pub device_metadata: String,
    /// Text for the "Services to Manual" row.
    pub services: String,
    /// Text for the "Explorer Home & Gallery" row.
    pub explorer_home: String,
    /// Text for the "Visual Effects" row.
    pub visual_effects: String,
    /// Text for the "Background Apps" row.
    pub background_apps: String,
    /// Text for the "Hyper-V" row.
    pub hyperv: String,
    /// Text for the "WSL" row.
    pub wsl: String,
    /// Text for the "O&O ShutUp10++" row.
    pub ooshutup: String,
    /// Text for the "Process Priority" row.
    pub process_priority: String,
    /// Text for the "GPU Scheduling" row.
    pub gpu_scheduling: String,
    /// Text for the "Multimedia Scheduler" row.
    pub multimedia_scheduler: String,
    /// Text for the "WebDAV Limit" row.
    pub webdav_limit: String,
    /// Text for the "Explorer & Menu Speed" row.
    pub explorer_speed: String,
    /// Text for the "Start Recommended" row.
    pub start_recommended: String,
    /// Text for the "Update Pause Limit" row.
    pub update_pause_limit: String,
    /// Text for the "Desktop Icons" row.
    pub desktop_icons: String,
    /// Text for the "This PC Icon" row.
    pub this_pc_icon: String,
    /// Text for the "Recycle Bin Icon" row.
    pub recycle_bin_icon: String,
    /// Text for the "User Folder Icon" row.
    pub user_folder_icon: String,
    /// Text for the "Network Icon" row.
    pub network_icon: String,
    /// Text for the "Control Panel Icon" row.
    pub control_panel_icon: String,
    /// Text for the "Shortcut Arrow" row.
    pub shortcut_arrow: String,
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
        self.wallpaper = label(wallpaper_compression_enabled(), "Compressed", "Full quality");
        self.edge = label(edge_debloat_enabled(), "Applied", "Default");
        self.bing_search = label(bing_search_enabled(), "On", "Off");
        self.right_click_menu = label(classic_context_menu_enabled(), "Classic", "Windows 11");
        self.windows_ai = label(windows_ai_disabled(), "Disabled", "Enabled");
        self.reserved_storage = label(reserved_storage_enabled(), "On", "Off");
        self.one_drive = label(onedrive_installed(), "Installed", "Removed");
        self.activity_history = label(activity_history_disabled(), "Disabled", "Enabled");
        self.bitlocker = label(bitlocker_enabled(), "On", "Off");
        self.consumer_features = label(consumer_features_disabled(), "Disabled", "Enabled");
        self.delivery_optimization =
            label(delivery_optimization_disabled(), "Disabled", "Enabled");
        self.location = label(location_tracking_enabled(), "On", "Off");
        self.store_search = label(store_search_disabled(), "Off", "On");
        self.device_metadata = label(device_metadata_blocked(), "Blocked", "Allowed");
        self.services = label(services_applied(), "Applied", "Default");
        self.explorer_home = label(explorer_home_hidden(), "Hidden", "Shown");
        self.visual_effects = label(visual_effects_performance(), "Performance", "Default");
        self.background_apps = label(background_apps_disabled(), "Disabled", "Enabled");
        self.hyperv = label(hyperv_enabled(), "On", "Off");
        self.wsl = label(wsl_enabled(), "On", "Off");
        self.ooshutup = label(ooshutup_downloaded(), "Downloaded", "Not downloaded");
        self.process_priority = label(foreground_boost_enabled(), "Boost", "Default");
        self.gpu_scheduling = label(gpu_scheduling_enabled(), "On", "Off");
        self.multimedia_scheduler =
            label(multimedia_scheduler_applied(), "Applied", "Default");
        self.webdav_limit = label(webdav_limit_lifted(), "4 GB", "400 MB");
        self.explorer_speed = label(explorer_menus_sped_up(), "Fast", "Default");
        self.start_recommended = label(start_recommended_hidden(), "Hidden", "Shown");
        self.update_pause_limit = label(update_pause_limit_lifted(), "Max", "Default");
        self.desktop_icons = label(desktop_icons_visible(), "Shown", "Hidden");
        self.this_pc_icon = label(this_pc_icon_shown(), "Shown", "Hidden");
        self.recycle_bin_icon = label(recycle_bin_icon_shown(), "Shown", "Hidden");
        self.user_folder_icon = label(user_folder_icon_shown(), "Shown", "Hidden");
        self.network_icon = label(network_icon_shown(), "Shown", "Hidden");
        self.control_panel_icon = label(control_panel_icon_shown(), "Shown", "Hidden");
        self.shortcut_arrow = label(shortcut_arrow_visible(), "Shown", "Removed");
    }

    /// Refresh the activation status. Slow (spawns PowerShell/WMI), so it is
    /// only run at startup and after the activation tweak.
    pub fn refresh_activation(&mut self) {
        self.activation = label(activation_status(), "Activated", "Not activated");
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_mode_toggle_writes_the_opposite_theme() {
        // 0 = dark, 1 = light: leaving the dark theme must write 1, and
        // leaving the light theme must write 0.
        assert_eq!(flipped_uses_light(true), 1);
        assert_eq!(flipped_uses_light(false), 0);
    }

    #[test]
    fn status_labels_map_option_bool() {
        assert_eq!(label(Some(true), "Dark", "Light"), "Dark");
        assert_eq!(label(Some(false), "Dark", "Light"), "Light");
        assert_eq!(label(None, "Dark", "Light"), "unknown");
    }

    #[test]
    fn icacls_deny_detection_covers_both_spellings() {
        // Older builds spell the deny ACE out; newer builds show the same
        // deny-special mask as (N) without the DENY keyword.
        assert!(icacls_shows_deny(
            "C:\\db store.db\n    Everyone:(DENY)(F)\n    BUILTIN\\Administrators:(I)(F)"
        ));
        assert!(icacls_shows_deny(
            "C:\\db store.db\n    Everyone:(N)\n    BUILTIN\\Administrators:(I)(F)"
        ));
        // A plain allow ACE must not read as denied.
        assert!(!icacls_shows_deny(
            "C:\\db store.db\n    Everyone:(F)\n    NT AUTHORITY\\SYSTEM:(I)(F)"
        ));
        assert!(!icacls_shows_deny(""));
    }
}
