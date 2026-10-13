//! Backend for the Cleaning tab: empty the Recycle Bin, delete the Windows and
//! per-user temp folders, and launch the Windows Disk Cleanup tool.
//!
//! The `preview_*` functions are read-only and fill the "Status" column; the
//! `clean_*` functions actually delete files.

use std::{
    env::{self, temp_dir},
    path::{Path, PathBuf},
};

use crate::sys;
use windows::Win32::UI::Shell::{
    SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHEmptyRecycleBinW, SHQUERYRBINFO,
    SHQueryRecycleBinW,
};

// ------------------------------------------------------------------- helpers

/// The Windows install directory (`%windir%`), falling back to `C:\Windows`.
fn windir() -> PathBuf {
    PathBuf::from(env::var_os("windir").unwrap_or_else(|| "C:\\Windows".into()))
}

/// The current user's `%LOCALAPPDATA%` folder, if it is set.
fn local_app_data() -> Option<PathBuf> {
    env::var_os("LocalAppData").map(PathBuf::from)
}

/// The per-user Temp folder shown in the UI. Prefer `%LOCALAPPDATA%\Temp` and
/// fall back to the system temp directory when `LOCALAPPDATA` is not set.
fn user_temp_dir() -> PathBuf {
    match local_app_data() {
        Some(dir) => dir.join("Temp"),
        None => temp_dir(),
    }
}

// ---------------------------------------------------------------- recycle bin

/// Total size in bytes and number of items in the Recycle Bin, or an error
/// describing why the query failed.
fn query_recycle_bin() -> Result<(u64, u64), String> {
    let mut info = SHQUERYRBINFO {
        cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32,
        i64Size: 0,
        i64NumItems: 0,
    };
    unsafe { SHQueryRecycleBinW(None, &mut info) }
        .map_err(|e| format!("could not query the recycle bin: {e}"))?;
    // The API reports i64 values but never negative ones; clamp defensively.
    Ok((
        u64::try_from(info.i64Size).unwrap_or(0),
        u64::try_from(info.i64NumItems).unwrap_or(0),
    ))
}

/// Status text for the Recycle Bin row, e.g. "3 · 1.20 MB".
pub fn preview_recycle_bin() -> Option<String> {
    let (size, items) = query_recycle_bin().ok()?;
    Some(format!("{items} · {}", sys::format_bytes(size)))
}

/// Empty the Recycle Bin and describe what was removed.
pub fn clear_recycle_bin() -> Result<String, String> {
    let (size, items) = query_recycle_bin()?;
    if items == 0 {
        // Do not call the shell API on an empty bin: some builds fail it with
        // a spurious 0x8000FFFF ("catastrophic failure").
        return Ok("already empty".to_string());
    }
    // Emptying is irreversible, so the result text is the user's only
    // confirmation.
    let shell_result = unsafe {
        SHEmptyRecycleBinW(
            None,
            None,
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        )
    };
    match shell_result {
        Ok(()) => Ok(format!(
            "emptied {items} item(s), {} freed",
            sys::format_bytes(size)
        )),
        Err(e) => {
            // The raw API sometimes fails with 0x8000FFFF even when the bin
            // is fine; the modern cmdlet usually still works there.
            sys::powershell("Clear-RecycleBin -Force -ErrorAction Stop").map_err(
                |fallback| {
                    format!(
                        "emptying failed ({e}); fallback failed ({fallback}) — \
                         the Recycle Bin may be corrupted"
                    )
                },
            )?;
            Ok("recycle bin emptied (via PowerShell fallback)".to_string())
        }
    }
}

// ----------------------------------------------------------------- temp dirs

/// Status text for a temp folder ("N · SIZE"), or `None` if the folder does
/// not exist.
fn preview_dir(path: &Path) -> Option<String> {
    if !path.is_dir() {
        return None;
    }
    let (files, _dirs, bytes) = sys::dir_stats(path);
    Some(format!("{files} · {}", sys::format_bytes(bytes)))
}

fn preview_windows_temp() -> Option<String> {
    preview_dir(&windir().join("Temp"))
}

fn preview_user_temp() -> Option<String> {
    preview_dir(&user_temp_dir())
}

fn clean_dir(path: &Path) -> Result<String, String> {
    Ok(sys::clear_dir(path)?.summary())
}

/// Delete the contents of the Windows temp folder. Unelevated, files owned by
/// SYSTEM or other accounts are skipped, so mention the admin caveat.
pub fn clean_windows_temp() -> Result<String, String> {
    let summary = clean_dir(&windir().join("Temp"))?;
    if sys::is_admin() {
        Ok(summary)
    } else {
        Ok(format!("{summary} — run as administrator for full access"))
    }
}

pub fn clean_user_temp() -> Result<String, String> {
    clean_dir(&user_temp_dir())
}

// --------------------------------------------------------------------- tools

/// Launch the Windows Disk Cleanup tool (`cleanmgr`) for the system drive.
pub fn run_disk_cleanup() -> Result<String, String> {
    let drive = env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
    sys::spawn("cleanmgr.exe", &["/d", &drive])?;
    Ok("disk cleanup launched".to_string())
}

// -------------------------------------------------------------------- status

/// Snapshot of the Cleaning tab's "Status" column. It is refreshed on demand
/// instead of inside `draw`, because scanning the temp folders is expensive.
#[derive(Default)]
pub struct CleaningStatus {
    /// Text for the "Recycle Bin" row.
    pub recycle_bin: String,
    /// Text for the "Windows Temp" row.
    pub windows_temp: String,
    /// Text for the "User Temp" row.
    pub user_temp: String,
}

impl CleaningStatus {
    /// Build a status snapshot with the current sizes filled in.
    pub fn new() -> Self {
        let mut status = Self::default();
        status.refresh();
        status
    }

    /// Re-scan the current sizes. Slow (walks the temp folders), so call it
    /// only at startup and after a cleaning action.
    pub fn refresh(&mut self) {
        self.recycle_bin = preview_recycle_bin().unwrap_or_else(|| "unknown".to_string());
        self.windows_temp = preview_windows_temp().unwrap_or_else(|| "unknown".to_string());
        self.user_temp = preview_user_temp().unwrap_or_else(|| "unknown".to_string());
    }
}
