//! Backend for the Store tab: resolve and download apps into the user's
//! Downloads folder.
//!
//! For now a store entry is a download, not an install: the resolved file is
//! saved to `%USERPROFILE%\Downloads`. Downloads use the bundled `curl.exe`,
//! matching the rest of the app, so no extra dependency is needed.

use std::path::{Path, PathBuf};

use crate::sys;

use super::label;

/// GitHub API endpoint for the newest mpv Windows build (shinchiro's
/// `mpv-winbuild-cmake`). The asset names carry a date and git hash, so the
/// download URL is resolved from the API instead of being a constant.
const MPV_RELEASES_API: &str =
    "https://api.github.com/repos/shinchiro/mpv-winbuild-cmake/releases/latest";

/// The user's Downloads folder (`%USERPROFILE%\Downloads`).
fn downloads_dir() -> Result<PathBuf, String> {
    let profile =
        std::env::var_os("USERPROFILE").ok_or("could not find your user profile folder")?;
    Ok(Path::new(&profile).join("Downloads"))
}

/// True for the plain 64-bit mpv build (`mpv-x86_64-<date>-git-<hash>.7z`), and
/// not the `-v3-` variant or the `-dev-` headers.
fn is_mpv_asset(file: &str) -> bool {
    file.starts_with("mpv-x86_64-") && file.ends_with(".7z") && !file.contains("-v3-")
}

/// "In Downloads" / "Not downloaded" for the mpv row.
fn mpv_downloaded() -> Option<bool> {
    has_download(is_mpv_asset)
}

/// Whether the Downloads folder already holds a file matching `want`.
fn has_download(want: impl Fn(&str) -> bool) -> Option<bool> {
    let entries = std::fs::read_dir(downloads_dir().ok()?).ok()?;
    Some(
        entries
            .flatten()
            .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
            .any(|name| want(&name)),
    )
}

pub fn download_mpv() -> Result<String, String> {
    let url = github_asset_url(MPV_RELEASES_API, is_mpv_asset)?;
    let path = download_to_downloads(&url)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("mpv");
    Ok(format!("downloaded {name} to your Downloads folder"))
}

/// Download `url` into the Downloads folder, keeping the file name from the URL,
/// and return the path it was saved to.
fn download_to_downloads(url: &str) -> Result<PathBuf, String> {
    let name = url
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .ok_or("the download URL has no file name")?;
    let dir = downloads_dir()?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let dest = dir.join(name);
    sys::run(
        "curl.exe",
        &[
            "-L",
            "--silent",
            "--show-error",
            "--fail",
            "-o",
            &dest.to_string_lossy(),
            url,
        ],
    )?;
    Ok(dest)
}

/// Resolve the download URL of the newest release asset whose file name
/// satisfies `want`, via the GitHub REST API.
fn github_asset_url(api: &str, want: impl Fn(&str) -> bool) -> Result<String, String> {
    // The API asks callers to identify themselves; curl's default agent works,
    // but sending our own keeps the request identifiable.
    let json = sys::run(
        "curl.exe",
        &[
            "-L",
            "--silent",
            "--show-error",
            "--fail",
            "-H",
            "User-Agent: PatchKit",
            api,
        ],
    )?;
    asset_url_in(&json, want).ok_or_else(|| "no matching file in the latest release".to_string())
}

/// Pull the first `browser_download_url` out of a GitHub release JSON whose file
/// name satisfies `want`. The response may be minified onto one line, so this
/// scans for the key instead of assuming one field per line.
fn asset_url_in(json: &str, want: impl Fn(&str) -> bool) -> Option<String> {
    const KEY: &str = "\"browser_download_url\"";
    let mut search = json;
    while let Some(start) = search.find(KEY) {
        // Everything after the key, so the next iteration scans later assets.
        let tail = &search[start + KEY.len()..];
        search = tail;
        let Some(colon) = tail.find(':') else {
            continue;
        };
        // The value is a quoted string; whitespace after the colon is optional.
        let Some(url) = tail[colon + 1..]
            .trim_start()
            .strip_prefix('"')
            .and_then(|value| value.split('"').next())
        else {
            continue;
        };
        let file = url.rsplit('/').next().unwrap_or("");
        if want(file) {
            return Some(url.to_string());
        }
    }
    None
}

/// Snapshot of the Store tab's "Status" column.
#[derive(Default)]
pub struct StoreStatus {
    /// Text for the "mpv" row.
    pub mpv: String,
}

impl StoreStatus {
    /// Build a status snapshot with the current Downloads contents filled in.
    pub fn new() -> Self {
        let mut status = Self::default();
        status.refresh();
        status
    }

    /// Re-check which store downloads are already present.
    pub fn refresh(&mut self) {
        self.mpv = label(mpv_downloaded(), "In Downloads", "Not downloaded");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_url_in_picks_the_plain_build() {
        // The release lists the `-dev-` headers, a `-v3-` variant and the plain
        // build; only the plain 64-bit one should be chosen.
        //
        // The JSON is minified onto one line, exactly as curl.exe receives it
        // from the GitHub API — a line-based parse would find nothing.
        let json = concat!(
            r#"{"assets":[{"browser_download_url":"https://example.com/mpv-dev-x86_64-20261013-git-abc.7z"},"#,
            r#"{"browser_download_url":"https://example.com/mpv-x86_64-v3-20261013-git-abc.7z"},"#,
            r#"{"browser_download_url":"https://example.com/mpv-aarch64-20261013-git-abc.7z"},"#,
            r#"{"browser_download_url":"https://example.com/mpv-x86_64-20261013-git-abc.7z"}]}"#,
        );
        assert_eq!(
            asset_url_in(json, is_mpv_asset).as_deref(),
            Some("https://example.com/mpv-x86_64-20261013-git-abc.7z")
        );
    }

    #[test]
    fn asset_url_in_returns_none_without_a_match() {
        assert_eq!(asset_url_in("[]", is_mpv_asset), None);
    }

    #[test]
    fn is_mpv_asset_excludes_variants() {
        assert!(is_mpv_asset("mpv-x86_64-20261013-git-9d144e57b6.7z"));
        assert!(!is_mpv_asset("mpv-x86_64-v3-20261013-git-9d144e57b6.7z"));
        assert!(!is_mpv_asset("mpv-dev-x86_64-20261013-git-9d144e57b6.7z"));
        assert!(!is_mpv_asset("mpv-x86_64-20261013-git-9d144e57b6.zip"));
    }
}
