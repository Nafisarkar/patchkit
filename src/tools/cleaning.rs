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
    PathBuf::from(env::var("windir").unwrap_or_else(|_| "C:\\Windows".to_string()))
}

/// The current user's `%LOCALAPPDATA%` folder, if it is set.
fn local_app_data() -> Option<PathBuf> {
    env::var_os("LocalAppData").map(PathBuf::from)
}

/// The per-user Temp folder shown in the UI. Prefer `%LOCALAPPDATA%\Temp` and
/// fall back to `%TEMP%` when `LOCALAPPDATA` is not set.
fn user_temp_dir() -> PathBuf {
    match local_app_data() {
        Some(dir) => dir.join("Temp"),
        None => temp_dir(),
    }
}

// ---------------------------------------------------------------- recycle bin

/// Total size in bytes and number of items in the Recycle Bin, or `None` if it
/// could not be queried.
pub fn query_recycle_bin() -> Option<(u64, u64)> {
    let mut info = SHQUERYRBINFO {
        cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32,
        i64Size: 0,
        i64NumItems: 0,
    };
    let hr = unsafe { SHQueryRecycleBinW(None, &mut info) };
    hr.is_ok()
        .then_some((info.i64Size as u64, info.i64NumItems as u64))
}

/// Status text for the Recycle Bin row, e.g. "3 · 1.20 MB".
pub fn preview_recycle_bin() -> Option<String> {
    let (size, items) = query_recycle_bin()?;
    Some(format!("{items} · {}", sys::format_bytes(size)))
}

/// Empty the Recycle Bin and describe what was removed.
pub fn clear_recycle_bin() -> Result<String, String> {
    let (size, items) = query_recycle_bin().ok_or("could not query the recycle bin")?;
    if items == 0 {
        return Ok("already empty".to_string());
    }
    // We already confirmed ourselves, so skip the Windows dialog/sound.
    unsafe {
        SHEmptyRecycleBinW(
            None,
            None,
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        )
    }
    .map_err(|e| format!("failed: {e}"))?;
    Ok(format!(
        "emptied {items} item(s), {} freed",
        sys::format_bytes(size)
    ))
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

/// Status text for the Windows temp folder.
pub fn preview_windows_temp() -> Option<String> {
    preview_dir(&windir().join("Temp"))
}

/// Status text for the per-user temp folder.
pub fn preview_user_temp() -> Option<String> {
    preview_dir(&user_temp_dir())
}

/// Delete the contents of the Windows temp folder.
pub fn clean_windows_temp() -> Result<String, String> {
    Ok(sys::clear_dir(&windir().join("Temp"))?.summary())
}

/// Delete the contents of the per-user temp folder.
pub fn clean_user_temp() -> Result<String, String> {
    Ok(sys::clear_dir(&user_temp_dir())?.summary())
}

// --------------------------------------------------------------------- tools

/// Launch the Windows Disk Cleanup tool (`cleanmgr`) for the C: drive.
pub fn run_disk_cleanup() -> Result<String, String> {
    sys::spawn("cleanmgr.exe", &["/d", "C:"])?;
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
