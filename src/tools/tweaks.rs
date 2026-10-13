//! Backend for the Tweaks tab: theme and File Explorer toggles, wallpaper
//! compression, Bing search, preinstalled app removal, Edge debloat, and
//! Windows activation.
//!
//! Registry values are read and written with `reg.exe`, so no extra
//! dependency is needed. The `*_enabled` / `*_visible` functions are
//! read-only (they fill the "Status" column); the `toggle_*` / action
//! functions change the system.

use crate::sys;
use std::path::Path;

// ------------------------------------------------------------------ registry

/// `HKCU\...\Themes\Personalize` — holds the app/system theme values.
const PERSONALIZE: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";
/// `HKCU\...\Explorer\Advanced` — holds the File Explorer view options.
const ADVANCED: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";

/// `HKCU\Control Panel\Desktop` — holds the wallpaper import quality.
const DESKTOP: &str = r"HKCU\Control Panel\Desktop";

/// Read a REG_DWORD value from the registry, or `None` if it is missing or the
/// command fails.
fn reg_dword(path: &str, name: &str) -> Option<u32> {
    let out = sys::run("reg", &["query", path, "/v", name]).ok()?;
    // The value line looks like "    AppsUseLightTheme    REG_DWORD    0x1".
    let line = out.lines().find(|l| l.contains("REG_DWORD"))?;
    let raw = line.split_whitespace().last()?;
    u32::from_str_radix(raw.trim_start_matches("0x"), 16).ok()
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

// ----------------------------------------------------------------- wallpaper compression

/// True when Windows still compresses wallpapers. The value defaults to 85
/// (compression on) when it has never been set; 100 disables compression.
pub fn wallpaper_compression_enabled() -> Option<bool> {
    Some(reg_dword(DESKTOP, "JPEGImportQuality").unwrap_or(85) != 100)
}

/// Disable the automatic JPEG compression of wallpapers by setting
/// `JPEGImportQuality` to 100. Windows applies it when the next wallpaper is
/// set, not at boot.
pub fn wallpaper_compression_set_max() -> Result<String, String> {
    if wallpaper_compression_enabled() == Some(false) {
        return Ok("wallpaper compression is already disabled".to_string());
    }
    set_dword(DESKTOP, "JPEGImportQuality", 100)?;
    Ok("wallpaper compression disabled (applies on the next wallpaper)".to_string())
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
        "removed preinstalled apps: {}",
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
    let failures: Vec<String> = EDGE_POLICY_VALUES
        .iter()
        .filter_map(|(path, name, reg_type, data)| set_reg(path, name, reg_type, data).err())
        .collect();
    if failures.is_empty() {
        Ok("edge debloat policies applied".to_string())
    } else {
        Err(format!(
            "some Edge policies failed: {}",
            failures.join("; ")
        ))
    }
}

// ----------------------------------------------------------------- bing search

/// `HKCU\...\CurrentVersion\Search` — Start Menu search settings.
const SEARCH: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Search";

/// True when Start Menu search shows Bing web results. A missing value means
/// the Windows default, which is enabled.
pub fn bing_search_enabled() -> Option<bool> {
    Some(reg_dword(SEARCH, "BingSearchEnabled").unwrap_or(0) == 0)
}

/// Toggle Bing web search results in the Start Menu (restarts Explorer so
/// the change applies immediately).
pub fn toggle_bing_search() -> Result<String, String> {
    let now_enabled = bing_search_enabled().unwrap_or(true);
    set_dword(SEARCH, "BingSearchEnabled", u32::from(!now_enabled))?;
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
        sys::run(
            "reg",
            &[
                "add",
                &format!("{CLASSIC_CONTEXT_MENU}\\InprocServer32"),
                "/ve",
                "/d",
                "",
                "/f",
            ],
        )?;
        sys::restart_explorer()?;
        Ok("restored the classic right-click menu".to_string())
    }
}

// ----------------------------------------------------------------- windows ai

/// `HKLM\...\Policies\Explorer` — hides settings pages such as the AI ones.
const EXPLORER_POLICIES: &str =
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer";
/// `HKLM\SOFTWARE\Policies\WindowsNotepad` — Notepad AI features policy.
const WINDOWS_NOTEPAD: &str = r"HKLM\SOFTWARE\Policies\WindowsNotepad";

/// True when the Windows AI features were disabled and removed.
pub fn windows_ai_disabled() -> Option<bool> {
    Some(reg_dword(WINDOWS_NOTEPAD, "DisableAIFeatures").unwrap_or(0) == 1)
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
    if reserved_storage_enabled().unwrap_or(true) {
        sys::run("DISM", &["/Online", "/Set-ReservedStorageState", "/State:Disabled"])?;
        Ok("reserved storage disabled".to_string())
    } else {
        sys::run("DISM", &["/Online", "/Set-ReservedStorageState", "/State:Enabled"])?;
        Ok("reserved storage enabled".to_string())
    }
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
}

Get-Service -Name 'OneSyncSvc*' -ErrorAction SilentlyContinue |
    Set-Service -StartupType Disabled -ErrorAction SilentlyContinue";
    sys::powershell(script)?;
    sys::restart_explorer()?;
    Ok("OneDrive removed (Explorer restarted)".to_string())
}

// ----------------------------------------------------------------- dark mode

/// True when apps use the dark theme.
pub fn dark_mode_enabled() -> Option<bool> {
    // AppsUseLightTheme: 0 = dark, 1 = light.
    Some(reg_dword(PERSONALIZE, "AppsUseLightTheme")? == 0)
}

/// Switch apps and the system between light and dark theme. Each of the two
/// theme values flips independently, so a split setup stays split instead of
/// being forced to match.
pub fn toggle_dark_mode() -> Result<String, String> {
    let now_dark = dark_mode_enabled().unwrap_or(false);
    let now_dark_system = reg_dword(PERSONALIZE, "SystemUsesLightTheme")
        .map(|value| value == 0)
        .unwrap_or(now_dark);
    set_dword(PERSONALIZE, "AppsUseLightTheme", u32::from(!now_dark))?;
    set_dword(
        PERSONALIZE,
        "SystemUsesLightTheme",
        u32::from(!now_dark_system),
    )?;
    sys::broadcast_setting_change();
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
