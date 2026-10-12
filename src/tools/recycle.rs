use std::{
    env::{self, temp_dir},
    path::PathBuf,
};

use crate::sys;
use windows::Win32::UI::Shell::{
    SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND, SHEmptyRecycleBinW, SHQUERYRBINFO,
    SHQueryRecycleBinW,
};

// ------------------------------------------------------------------- helpers

fn windir() -> PathBuf {
    PathBuf::from(env::var("windir").unwrap_or_else(|_| "C:\\Windows".to_string()))
}

fn local_app_data() -> Option<PathBuf> {
    env::var_os("LocalAppData").map(PathBuf::from)
}

// ---------------------------------------------------------------- recycle bin

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

pub fn preview_recycle_bin() -> Option<String> {
    let (size, items) = query_recycle_bin()?;
    Some(format!("{items} · {}", sys::format_bytes(size)))
}

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

fn preview_dir(path: &PathBuf) -> Option<String> {
    if !path.is_dir() {
        return None;
    }
    let (files, bytes) = sys::dir_stats(path);
    Some(format!("{files} file(s) · {}", sys::format_bytes(bytes)))
}

pub fn preview_windows_temp() -> Option<String> {
    preview_dir(&windir().join("Temp"))
}

pub fn preview_user_temp() -> Option<String> {
    preview_dir(&env::temp_dir())
}

pub fn clean_windows_temp() -> Result<String, String> {
    Ok(sys::clear_dir(&windir().join("Temp"))?.summary())
}

pub fn clean_user_temp() -> Result<String, String> {
    Ok(sys::clear_dir(&env::temp_dir())?.summary())
}
