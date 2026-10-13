//! The system operations behind each tab, plus the status snapshot that each
//! tab displays.

pub mod cleaning;
pub mod downloader;
pub mod tweaks;

/// Turn an `Option<bool>` into a display string: `yes` for `Some(true)`, `no`
/// for `Some(false)`, and "unknown" for `None`.
pub(crate) fn label(value: Option<bool>, yes: &str, no: &str) -> String {
    match value {
        Some(true) => yes.to_string(),
        Some(false) => no.to_string(),
        None => "unknown".to_string(),
    }
}
