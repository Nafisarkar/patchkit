//! Low-level Windows helpers shared by the rest of the app: checking admin
//! rights, running external commands, formatting byte sizes, and safely
//! deleting the contents of a directory.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

// ---------------------------------------------------------------- elevation

/// True when the current process has administrator rights.
pub fn is_admin() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut TOKEN_ELEVATION as *mut _),
            u32::try_from(size_of::<TOKEN_ELEVATION>()).unwrap_or(0),
            &mut size,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

// ------------------------------------------------------------------ commands

/// Run a command, returning trimmed stdout on success or a readable error.
pub fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let out = run_capture(program, args)?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(failure_message(program, &out))
    }
}

/// Run a command and return the raw output regardless of exit status.
pub fn run_capture(program: &str, args: &[&str]) -> Result<Output, String> {
    Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("failed to start {program}: {e}"))
}

/// Start a process without waiting for it (GUI tools like cleanmgr/wsreset).
pub fn spawn(program: &str, args: &[&str]) -> Result<(), String> {
    Command::new(program)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to start {program}: {e}"))
}

/// Build a readable error from a command's stderr (preferred) or stdout.
fn failure_message(program: &str, out: &Output) -> String {
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("exit code {:?}", out.status.code())
    };
    format!("{program}: {detail}")
}

/// Run a PowerShell script (used for appx/service operations with no CLI).
pub fn powershell(script: &str) -> Result<String, String> {
    run(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", script],
    )
}

/// Tell running applications that a user setting changed, so tweaks such as
/// the light/dark theme apply without a re-login.
pub fn broadcast_setting_change() -> Result<(), String> {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
    };

    // "ImmersiveColorSet" prompts the shell and apps to re-read theme colours.
    let param: Vec<u16> = "ImmersiveColorSet\0".encode_utf16().collect();
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(param.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            1000,
            None,
        );
    }
    Ok(())
}

/// Restart Windows Explorer so shell registry changes take effect.
pub fn restart_explorer() -> Result<(), String> {
    let _ = run("taskkill", &["/f", "/im", "explorer.exe"]);
    std::process::Command::new("explorer.exe")
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to restart explorer: {e}"))
}

// -------------------------------------------------------------------- format

/// Format a byte count as a short string such as "12.34 MB".
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

// ----------------------------------------------------------------- deletion

/// What a delete operation managed to remove, and what it had to skip.
#[derive(Default)]
pub struct ClearReport {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub skipped: u64,
}

impl ClearReport {
    /// One-line, human-readable summary of the report.
    pub fn summary(&self) -> String {
        let mut s = format!(
            "{} file(s), {} dir(s) removed — {} freed",
            self.files,
            self.dirs,
            format_bytes(self.bytes)
        );
        if self.skipped > 0 {
            s.push_str(&format!(", {} skipped (in use or no access)", self.skipped));
        }
        s
    }
}

/// Delete everything inside `path` (keeps the directory itself).
pub fn clear_dir(path: &Path) -> Result<ClearReport, String> {
    if !path.is_dir() {
        return Err(format!("{} does not exist", path.display()));
    }
    let mut report = ClearReport::default();
    clear_into(path, &mut report);
    Ok(report)
}

/// Recursively delete the entries inside `path`, accumulating counts into
/// `report`. The directory itself is left in place.
fn clear_into(path: &Path, report: &mut ClearReport) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => {
            report.skipped += 1;
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else {
            report.skipped += 1;
            continue;
        };
        if meta.is_dir() {
            // A junction or symlink is only a link, not a real folder: remove
            // the link itself and never recurse through it.
            if is_reparse_point(&path) {
                if fs::remove_dir(&path).is_ok() {
                    report.dirs += 1;
                } else {
                    report.skipped += 1;
                }
                continue;
            }
            let (files, dirs, bytes) = subtree_stats(&path);
            match fs::remove_dir_all(&path) {
                Ok(()) => {
                    report.files += files;
                    report.dirs += dirs + 1;
                    report.bytes += bytes;
                }
                // Partially locked: fall back to recursing manually.
                Err(_) => {
                    clear_into(&path, report);
                    if fs::remove_dir(&path).is_ok() {
                        report.dirs += 1;
                    }
                }
            }
        } else {
            match fs::remove_file(&path) {
                Ok(()) => {
                    report.files += 1;
                    report.bytes += meta.len();
                }
                Err(_) => report.skipped += 1,
            }
        }
    }
}

/// Windows marks symlinks and directory junctions with this attribute. We
/// treat them as links, never as real folders, so we don't loop forever or
/// delete another location's files.
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

/// True when `path` is a symlink or a junction (a reparse point).
fn is_reparse_point(path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    match fs::symlink_metadata(path) {
        Ok(meta) => meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0,
        Err(_) => false,
    }
}

/// File count, directory count and total byte size of a directory tree (best
/// effort; junction and symlink targets are not followed).
pub fn dir_stats(path: &Path) -> (u64, u64, u64) {
    fn walk(path: &Path, files: &mut u64, dirs: &mut u64, bytes: &mut u64) {
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            // Never walk through a junction or symlink: it can point at an
            // ancestor (endless loop) or somewhere outside this tree.
            if is_reparse_point(&path) {
                continue;
            }
            if meta.is_dir() {
                *dirs += 1;
                walk(&path, files, dirs, bytes);
            } else if meta.is_file() {
                *files += 1;
                *bytes += meta.len();
            }
        }
    }
    let mut files = 0;
    let mut dirs = 0;
    let mut bytes = 0;
    walk(path, &mut files, &mut dirs, &mut bytes);
    (files, dirs, bytes)
}

/// Count files, directories and bytes under `path` (see [`dir_stats`]).
fn subtree_stats(path: &Path) -> (u64, u64, u64) {
    dir_stats(path)
}
