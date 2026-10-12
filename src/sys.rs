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

/// Set a Windows service startup type ("disabled", "demand", "auto").
pub fn sc_config(name: &str, start: &str) -> Result<(), String> {
    run("sc", &["config", name, "start=", start]).map(|_| ())
}

/// Total physical memory in kilobytes.
pub fn total_ram_kb() -> Option<u64> {
    use windows::Win32::System::SystemInformation::GetPhysicallyInstalledSystemMemory;
    let mut kb = 0u64;
    unsafe { GetPhysicallyInstalledSystemMemory(&mut kb) }.ok()?;
    Some(kb)
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

#[derive(Default)]
pub struct ClearReport {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub skipped: u64,
}

impl ClearReport {
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

fn clear_into(path: &Path, report: &mut ClearReport) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            report.skipped += 1;
            let _ = e;
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
            let (files, bytes) = subtree_stats(&path);
            match fs::remove_dir_all(&path) {
                Ok(()) => {
                    report.files += files;
                    report.bytes += bytes;
                    report.dirs += 1;
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

/// Delete matching entries in a single directory (non-recursive).
pub fn clear_matching<F: Fn(&str) -> bool>(path: &Path, pred: F) -> Result<ClearReport, String> {
    if !path.is_dir() {
        return Err(format!("{} does not exist", path.display()));
    }
    let mut report = ClearReport::default();
    let entries = fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !pred(&name) {
            continue;
        }
        let path = entry.path();
        let Ok(meta) = entry.metadata() else {
            report.skipped += 1;
            continue;
        };
        if meta.is_dir() {
            let (files, bytes) = subtree_stats(&path);
            match fs::remove_dir_all(&path) {
                Ok(()) => {
                    report.files += files;
                    report.bytes += bytes;
                    report.dirs += 1;
                }
                Err(_) => report.skipped += 1,
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
    Ok(report)
}

/// Total size and file count of a directory tree (best effort).
pub fn dir_stats(path: &Path) -> (u64, u64) {
    fn walk(path: &Path, files: &mut u64, bytes: &mut u64) {
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.metadata() {
                Ok(m) if m.is_dir() => walk(&path, files, bytes),
                Ok(m) if m.is_file() => {
                    *files += 1;
                    *bytes += m.len();
                }
                _ => {}
            }
        }
    }
    let mut files = 0;
    let mut bytes = 0;
    walk(path, &mut files, &mut bytes);
    (files, bytes)
}

fn subtree_stats(path: &Path) -> (u64, u64) {
    let (files, bytes) = dir_stats(path);
    (files, bytes)
}
