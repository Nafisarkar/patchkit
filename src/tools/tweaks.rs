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

/// Read a REG_DWORD value from the registry, or `None` if it is missing or the
/// command fails.
fn reg_dword(path: &str, name: &str) -> Option<u32> {
    let out = sys::run("reg", &["query", path, "/v", name]).ok()?;
    // The value line looks like "    AppsUseLightTheme    REG_DWORD    0x1".
    let line = out.lines().find(|l| l.contains("REG_DWORD"))?;
    let raw = line.split_whitespace().last()?;
    u32::from_str_radix(raw.trim_start_matches("0x"), 16).ok()
}

/// Read a REG_SZ (string) value, or `None` if it is missing or the command
/// fails.
fn reg_sz(path: &str, name: &str) -> Option<String> {
    let out = sys::run("reg", &["query", path, "/v", name]).ok()?;
    // The value line looks like "    Value    REG_SZ    Allow".
    let line = out.lines().find(|l| l.contains("REG_SZ"))?;
    // Everything after the type token is the data (it may contain spaces).
    Some(line.split_once("REG_SZ")?.1.trim().to_string())
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
    Some(reg_dword(CLOUD_CONTENT, "DisableWindowsConsumerFeatures").unwrap_or(0) == 1)
}

/// Stop promoted app installs and reduce Microsoft Store app suggestions.
/// Requires administrator rights.
pub fn disable_consumer_features() -> Result<String, String> {
    require_admin()?;
    set_dword(CLOUD_CONTENT, "DisableWindowsConsumerFeatures", 1)?;
    Ok("consumer features disabled".to_string())
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

/// Set unneeded services to manual/disabled and align the svchost splitting
/// threshold with installed memory, which reduces the number of svchost.exe
/// processes. Requires administrator rights.
pub fn set_services_manual() -> Result<String, String> {
    require_admin()?;
    // Some of these services do not exist on every install; skip failures.
    set_service_start("CscService", "disabled");
    set_service_start("DiagTrack", "disabled");
    set_service_start("SharedAccess", "disabled");
    set_service_start("MapsBroker", "manual");
    set_service_start("StorSvc", "manual");
    // Splitting every service into its own svchost only pays off with plenty
    // of RAM; the threshold in KB should match the installed memory.
    sys::powershell(
        "\
$memory = (Get-CimInstance Win32_PhysicalMemory | Measure-Object Capacity -Sum).Sum / 1KB
Set-ItemProperty -Path 'HKLM:\\SYSTEM\\CurrentControlSet\\Control' \
    -Name SvcHostSplitThresholdInKB -Value $memory -Type DWord",
    )?;
    Ok("services updated and svchost splitting tuned".to_string())
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
        for key in [EXPLORER_HOME, EXPLORER_GALLERY] {
            sys::run(
                "reg",
                &["delete", key, "/v", "System.IsPinnedToNameSpaceTree", "/f"],
            )?;
        }
        sys::run("reg", &["delete", ADVANCED, "/v", "LaunchTo", "/f"])?;
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
const VISUAL_FX_VALUES: [(&str, &str, &str, &str); 13] = [
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
