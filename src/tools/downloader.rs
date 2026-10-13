//! Backend for the Store tab: download an app from its link and install it.
//!
//! A store entry is just a name, a description and a link (see
//! [`crate::data::options::STORE_OPTIONS`]); everything else is generic. The
//! file is downloaded into `%LOCALAPPDATA%\PatchKit\<name>`, then:
//!
//! * an archive (`.zip`, `.7z`, `.tar.*`) is unpacked there — a portable
//!   install, and
//! * an installer (`.exe`, `.msi`) is run from there.
//!
//! `.zip` and `.tar.*` are unpacked with the `tar.exe` Windows ships. `.7z`
//! needs the LZMA codec that `tar.exe` lacks, so it uses a `7z`/`7zr`/`7za`
//! already on `PATH`, or the standalone `7zr.exe` fetched from 7-zip.org once.
//!
//! A link may point at a file directly, or at a GitHub
//! `releases/latest/download/<name>` URL whose file name contains a `*` glob,
//! in which case the newest matching release asset is resolved first.

use std::path::{Path, PathBuf};

use crate::data::options::StoreApp;
use crate::sys;

use super::label;

/// The standalone 7-Zip console, used to unpack `.7z` when 7-Zip isn't on PATH.
const SEVEN_ZIP_URL: &str = "https://www.7-zip.org/a/7zr.exe";

/// PatchKit's own folder: `%LOCALAPPDATA%\PatchKit`.
fn tools_dir() -> Result<PathBuf, String> {
    let local = std::env::var_os("LocalAppData").ok_or("could not find %LOCALAPPDATA%")?;
    Ok(Path::new(&local).join("PatchKit"))
}

/// Where an app is downloaded and installed: `%LOCALAPPDATA%\PatchKit\<name>`.
fn app_dir(name: &str) -> Result<PathBuf, String> {
    Ok(tools_dir()?.join(name))
}

/// True when the app was installed: its folder exists and has content.
fn is_installed(name: &str) -> Option<bool> {
    let dir = app_dir(name).ok()?;
    let mut entries = std::fs::read_dir(&dir).ok()?;
    Some(entries.next().is_some())
}

/// Status text for a store row.
pub fn status(app: &StoreApp) -> String {
    label(is_installed(app.name), "Installed", "Not installed")
}

/// Download `app`'s link and install it, returning a status-bar message.
pub fn install(app: &StoreApp) -> Result<String, String> {
    let dir = app_dir(app.name)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create {}: {e}", dir.display()))?;

    let url = resolve_link(app.link)?;
    let file = download(&url, &dir)?;
    let name = file_name(&file);

    match install_kind(&name) {
        InstallKind::Portable => {
            extract(&file, &dir)?;
            Ok(format!("installed {} ({name})", app.name))
        }
        InstallKind::Installer => {
            run_installer(&file)?;
            Ok(format!("installed {} (ran {name})", app.name))
        }
        InstallKind::File => Ok(format!("downloaded {name} to {}", dir.display())),
    }
}

/// How a downloaded file should be installed.
enum InstallKind {
    /// Unpack into the app folder.
    Portable,
    /// Run the downloaded installer.
    Installer,
    /// Leave the file in the app folder untouched.
    File,
}

/// Pick the install method from the downloaded file's extension.
fn install_kind(name: &str) -> InstallKind {
    let name = name.to_ascii_lowercase();
    let archive = [
        ".zip", ".7z", ".tar.gz", ".tgz", ".tar.xz", ".txz", ".tar.bz2",
    ]
    .iter()
    .any(|ext| name.ends_with(ext));
    if archive {
        InstallKind::Portable
    } else if name.ends_with(".exe") || name.ends_with(".msi") {
        InstallKind::Installer
    } else {
        InstallKind::File
    }
}

/// Unpack an archive into `dest`.
fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    let name = file_name(archive).to_ascii_lowercase();
    let archive = archive.to_string_lossy();
    let dest = dest.to_string_lossy();
    if name.ends_with(".7z") {
        let seven_zip = seven_zip()?;
        sys::run(&seven_zip, &["x", &archive, &format!("-o{dest}"), "-y"])?;
    } else {
        // bsdtar detects zip and tarballs from the file's content.
        sys::run("tar.exe", &["-xf", &archive, "-C", &dest])?;
    }
    Ok(())
}

/// A 7-Zip executable: one already on `PATH`, else the standalone `7zr.exe`
/// fetched once into PatchKit's folder.
fn seven_zip() -> Result<String, String> {
    for program in ["7z", "7zr", "7za"] {
        if let Some(path) = on_path(program) {
            return Ok(path);
        }
    }
    let tool = tools_dir()?.join("7zr.exe");
    if !tool.is_file() {
        let dir = tools_dir()?;
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("could not create {}: {e}", dir.display()))?;
        sys::run(
            "curl.exe",
            &[
                "-L",
                "--silent",
                "--show-error",
                "--fail",
                "-o",
                &tool.to_string_lossy(),
                SEVEN_ZIP_URL,
            ],
        )?;
    }
    Ok(tool.to_string_lossy().into_owned())
}

/// The full path of `program.exe` on `PATH`, if it exists.
fn on_path(program: &str) -> Option<String> {
    let exe = format!("{program}.exe");
    std::env::split_paths(&std::env::var_os("PATH")?).find_map(|dir| {
        let candidate = dir.join(&exe);
        candidate
            .is_file()
            .then(|| candidate.to_string_lossy().into_owned())
    })
}

/// Run a downloaded installer. `.msi` goes through `msiexec`; `.exe` uses the
/// NSIS `/S` silent flag, which the common one-click installers (NSIS, Tauri)
/// support. An installer that doesn't understand it simply shows its wizard.
fn run_installer(file: &Path) -> Result<(), String> {
    let path = file.to_string_lossy().to_string();
    if path.to_ascii_lowercase().ends_with(".msi") {
        sys::run("msiexec.exe", &["/i", &path, "/passive", "/norestart"])?;
    } else {
        sys::run(&path, &["/S"])?;
    }
    Ok(())
}

/// Download `url` into `dir`, keeping the file name from the URL.
fn download(url: &str, dir: &Path) -> Result<PathBuf, String> {
    let name = url
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .ok_or("the link has no file name")?;
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

/// Turn a link into a concrete file URL. A GitHub
/// `releases/latest/download/<name>` link whose file name contains a `*` glob
/// is resolved to the newest matching release asset; other links are returned
/// unchanged.
fn resolve_link(link: &str) -> Result<String, String> {
    if !link.contains('*') {
        return Ok(link.to_string());
    }
    let (repo, pattern) = github_latest_pattern(link).ok_or(
        "a link with a '*' must look like \
         https://github.com/<owner>/<repo>/releases/latest/download/<name>",
    )?;
    let api = format!("https://api.github.com/repos/{repo}/releases/latest");
    let json = fetch(&api)?;
    asset_url_in(&json, |file| glob_match(pattern, file))
        .ok_or_else(|| format!("no file matched '{pattern}' in the newest {repo} release"))
}

/// Split a GitHub `releases/latest/download/<pattern>` link into `owner/repo`
/// and the `<pattern>` file name.
fn github_latest_pattern(link: &str) -> Option<(&str, &str)> {
    let rest = link.strip_prefix("https://github.com/")?;
    let (repo, pattern) = rest.split_once("/releases/latest/download/")?;
    let valid = repo.contains('/') && !pattern.is_empty() && !pattern.contains('/');
    valid.then_some((repo, pattern))
}

/// Fetch a URL's text output (used for the GitHub API).
fn fetch(url: &str) -> Result<String, String> {
    sys::run(
        "curl.exe",
        &[
            "-L",
            "--silent",
            "--show-error",
            "--fail",
            "-H",
            "User-Agent: PatchKit",
            url,
        ],
    )
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

/// Case-insensitive glob match supporting `*` (any run) and `?` (one character).
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.to_ascii_lowercase().chars().collect();
    let text: Vec<char> = text.to_ascii_lowercase().chars().collect();
    let (mut pi, mut ti) = (0, 0);
    let (mut star, mut mark) = (None, 0);
    while ti < text.len() {
        if pi < pattern.len() && (pattern[pi] == '?' || pattern[pi] == text[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < pattern.len() && pattern[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(star_index) = star {
            pi = star_index + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < pattern.len() && pattern[pi] == '*' {
        pi += 1;
    }
    pi == pattern.len()
}

/// File name of a path, for display and extension checks.
fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_string()
}

/// Cached "Status" column for the Store tab.
#[derive(Default)]
pub struct StoreStatus {
    rows: Vec<String>,
}

impl StoreStatus {
    /// Build the status snapshot for the given apps.
    pub fn new(apps: &[StoreApp]) -> Self {
        let mut status = Self::default();
        status.refresh(apps);
        status
    }

    /// Re-check which apps are installed.
    pub fn refresh(&mut self, apps: &[StoreApp]) {
        self.rows = apps.iter().map(status).collect();
    }

    /// Status text for the row at `index`, or empty if out of range.
    pub fn value(&self, index: usize) -> &str {
        self.rows.get(index).map(String::as_str).unwrap_or("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_match_handles_wildcards() {
        assert!(glob_match(
            "mpv-x86_64-*.7z",
            "mpv-x86_64-20261013-git-abc.7z"
        ));
        assert!(glob_match(
            "Dorion_*_x64-setup.exe",
            "Dorion_6.13.2_x64-setup.exe"
        ));
        // The arm64 installer has a dot, not an underscore, before x64.
        assert!(!glob_match(
            "Dorion_*_x64-setup.exe",
            "Dorion_6.13.2.arm64-setup.exe"
        ));
        assert!(!glob_match(
            "Dorion_*_x64-setup.exe",
            "Dorion_6.13.2_win64_portable.zip"
        ));
        assert!(glob_match("a?c", "abc"));
        assert!(!glob_match("a?c", "abbc"));
    }

    #[test]
    fn github_latest_pattern_parses_owner_and_pattern() {
        let link =
            "https://github.com/SpikeHD/Dorion/releases/latest/download/Dorion_*_x64-setup.exe";
        assert_eq!(
            github_latest_pattern(link),
            Some(("SpikeHD/Dorion", "Dorion_*_x64-setup.exe"))
        );
        assert_eq!(github_latest_pattern("https://example.com/file.zip"), None);
    }

    #[test]
    fn asset_url_in_picks_the_first_match() {
        // Minified onto one line, exactly as curl.exe receives it.
        let json = concat!(
            r#"{"assets":[{"browser_download_url":"https://example.com/mpv-aarch64-20261013-git-abc.7z"},"#,
            r#"{"browser_download_url":"https://example.com/mpv-x86_64-20261013-git-abc.7z"}]}"#,
        );
        assert_eq!(
            asset_url_in(json, |file| glob_match("mpv-x86_64-*.7z", file)).as_deref(),
            Some("https://example.com/mpv-x86_64-20261013-git-abc.7z")
        );
    }

    #[test]
    fn install_kind_follows_the_extension() {
        assert!(matches!(install_kind("app.7z"), InstallKind::Portable));
        assert!(matches!(install_kind("app.tar.gz"), InstallKind::Portable));
        assert!(matches!(install_kind("app.ZIP"), InstallKind::Portable));
        assert!(matches!(install_kind("setup.exe"), InstallKind::Installer));
        assert!(matches!(install_kind("setup.msi"), InstallKind::Installer));
        assert!(matches!(install_kind("readme.txt"), InstallKind::File));
    }
}
