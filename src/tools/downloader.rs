//! The Store backend: download an app's link and install it.
//!
//! Each Store row is a name and a link (see
//! [`crate::data::options::STORE_OPTIONS`]). The file lands in
//! `%LOCALAPPDATA%\PatchKit\<name>`, then an archive is unpacked there and an
//! installer is run. A link may also contain a `*` glob: a GitHub
//! `releases/latest/download` link resolves against the release API, any other
//! link against its directory listing.

use std::path::{Path, PathBuf};

use crate::data::options::StoreApp;
use crate::sys;

/// 7-Zip console, fetched once because the bundled `tar.exe` can't read `.7z`.
const SEVEN_ZIP_URL: &str = "https://www.7-zip.org/a/7zr.exe";

/// Download `app` and install it; returns the status-bar message.
pub fn install(app: &StoreApp) -> Result<String, String> {
    let dir = app_dir(app.name)?;
    let file = download(&resolve_link(app.link)?, &dir)?;
    let name = file.file_name().unwrap_or_default().to_string_lossy();

    if is_archive(&name) {
        extract(&file, &dir)?;
        Ok(format!("installed {} ({name})", app.name))
    } else if is_installer(&name) {
        run_installer(&file)?;
        Ok(format!("installed {} ({name})", app.name))
    } else {
        Ok(format!("downloaded {name}"))
    }
}

/// "Installed" / "Not installed" for the status column.
pub fn status(app: &StoreApp) -> String {
    let text = if is_installed(app.name) {
        "Installed"
    } else {
        "Not installed"
    };
    text.to_string()
}

/// `%LOCALAPPDATA%\PatchKit` — where PatchKit keeps its files.
fn tools_dir() -> Result<PathBuf, String> {
    let local = std::env::var_os("LocalAppData").ok_or("could not find %LOCALAPPDATA%")?;
    Ok(Path::new(&local).join("PatchKit"))
}

/// An app's download and install folder: `%LOCALAPPDATA%\PatchKit\<name>`.
fn app_dir(name: &str) -> Result<PathBuf, String> {
    Ok(tools_dir()?.join(name))
}

/// True when the app's folder exists and has content.
fn is_installed(name: &str) -> bool {
    app_dir(name)
        .ok()
        .and_then(|dir| std::fs::read_dir(dir).ok())
        .is_some_and(|mut entries| entries.next().is_some())
}

/// True for archive extensions that should be unpacked.
fn is_archive(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        ".zip", ".7z", ".tar.gz", ".tgz", ".tar.xz", ".txz", ".tar.bz2",
    ]
    .iter()
    .any(|ext| name.ends_with(ext))
}

/// True for installer extensions that should be run.
fn is_installer(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.ends_with(".exe") || name.ends_with(".msi")
}

/// Unpack an archive into `dest`.
fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    let archive = archive.to_string_lossy();
    let dest = dest.to_string_lossy();
    if archive.to_ascii_lowercase().ends_with(".7z") {
        let tool = seven_zip()?;
        sys::run(&tool, &["x", &archive, &format!("-o{dest}"), "-y"])?;
    } else {
        // bsdtar detects zip and tarballs from the file's content.
        sys::run("tar.exe", &["-xf", &archive, "-C", &dest])?;
    }
    Ok(())
}

/// The 7-Zip console, fetched once on first `.7z` use.
fn seven_zip() -> Result<String, String> {
    let tool = tools_dir()?.join("7zr.exe");
    if !tool.is_file() {
        download(SEVEN_ZIP_URL, &tools_dir()?)?;
    }
    Ok(tool.to_string_lossy().into_owned())
}

/// Run an installer: `.msi` through `msiexec`, `.exe` with the NSIS `/S` flag.
fn run_installer(file: &Path) -> Result<(), String> {
    let path = file.to_string_lossy().to_string();
    if path.to_ascii_lowercase().ends_with(".msi") {
        sys::run("msiexec.exe", &["/i", &path, "/passive", "/norestart"])?;
    } else {
        sys::run(&path, &["/S"])?;
    }
    Ok(())
}

/// Download `url` into `dir` and return the saved path. `-O -J` takes the name
/// from the Content-Disposition header or the URL, which also covers links
/// that only reveal their name after a redirect (e.g. VSCode's).
fn download(url: &str, dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    let saved = sys::run(
        "curl.exe",
        &[
            "-L",
            "--silent",
            "--show-error",
            "--fail",
            "-O",
            "-J",
            "--output-dir",
            &dir.to_string_lossy(),
            "-w",
            "%{filename_effective}",
            url,
        ],
    )?;
    if saved.is_empty() {
        return Err("the download produced no file".to_string());
    }
    Ok(PathBuf::from(saved))
}

/// Resolve a link to a file URL. A `*` link picks the newest file whose name
/// matches: a GitHub `releases/latest/download` link uses the release API, any
/// other link is matched against its directory listing.
fn resolve_link(link: &str) -> Result<String, String> {
    if !link.contains('*') {
        return Ok(link.to_string());
    }
    let (base, pattern) = link
        .rsplit_once('/')
        .ok_or("a '*' link needs a file name")?;
    if let Some(repo) = base
        .strip_prefix("https://github.com/")
        .and_then(|rest| rest.strip_suffix("/releases/latest/download"))
    {
        let json = curl(&format!(
            "https://api.github.com/repos/{repo}/releases/latest"
        ))?;
        return asset_url_in(&json, pattern)
            .ok_or_else(|| format!("no asset matched '{pattern}' in {repo}"));
    }
    let listing = curl(&format!("{base}/"))?;
    let href = href_matching(&listing, pattern)
        .ok_or_else(|| format!("no file matched '{pattern}' at {base}"))?;
    Ok(if href.starts_with("http") {
        href.to_string()
    } else if href.starts_with('/') {
        format!("{}{href}", origin(base))
    } else {
        format!("{base}/{href}")
    })
}

/// Scheme and host of a URL, e.g. `https://nodejs.org`.
fn origin(url: &str) -> &str {
    let start = url.find("://").map_or(0, |i| i + 3);
    match url[start..].find('/') {
        Some(i) => &url[..start + i],
        None => url,
    }
}

/// Curl's text output for a URL.
fn curl(url: &str) -> Result<String, String> {
    sys::run(
        "curl.exe",
        &["-L", "--silent", "--show-error", "--fail", url],
    )
}

/// First `href="..."` link in an HTML listing whose file name matches `pattern`.
fn href_matching<'a>(listing: &'a str, pattern: &str) -> Option<&'a str> {
    listing
        .split("href=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .find(|href| glob_match(pattern, href.rsplit('/').next().unwrap_or("")))
}

/// First `browser_download_url` in a release JSON whose file name matches the
/// `*`/`?` glob `pattern`. The JSON is usually minified onto one line.
fn asset_url_in(json: &str, pattern: &str) -> Option<String> {
    json.split("\"browser_download_url\":")
        .skip(1)
        .filter_map(|rest| rest.trim_start().strip_prefix('"')?.split('"').next())
        .find(|url| glob_match(pattern, url.rsplit('/').next().unwrap_or("")))
        .map(str::to_string)
}

/// Case-insensitive `*`/`?` glob match.
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

/// Cached "Status" column for the Store tab.
#[derive(Default)]
pub struct StoreStatus {
    rows: Vec<String>,
}

impl StoreStatus {
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
    fn glob_match_supports_wildcards() {
        assert!(glob_match(
            "mpv-x86_64-*.7z",
            "mpv-x86_64-20261013-git-abc.7z"
        ));
        assert!(glob_match(
            "Dorion_*_x64-setup.exe",
            "Dorion_6.13.2_x64-setup.exe"
        ));
        assert!(!glob_match(
            "Dorion_*_x64-setup.exe",
            "Dorion_6.13.2.arm64-setup.exe"
        ));
        assert!(glob_match("a?c", "abc"));
    }

    #[test]
    fn asset_url_in_matches_the_glob() {
        let json = concat!(
            r#"{"assets":[{"browser_download_url":"https://example.com/mpv-aarch64-abc.7z"},"#,
            r#"{"browser_download_url":"https://example.com/mpv-x86_64-abc.7z"}]}"#,
        );
        assert_eq!(
            asset_url_in(json, "mpv-x86_64-*.7z").as_deref(),
            Some("https://example.com/mpv-x86_64-abc.7z")
        );
    }

    #[test]
    fn kind_follows_the_extension() {
        assert!(is_archive("app.7z") && is_archive("app.ZIP") && is_archive("a.tar.gz"));
        assert!(is_installer("setup.exe") && is_installer("setup.msi"));
        assert!(!is_archive("readme.txt") && !is_installer("readme.txt"));
    }

    #[test]
    fn href_matching_reads_a_directory_listing() {
        let listing = r#"<a href="node-v26.11.1-x64.msi">x</a> <a href="../">up</a>"#;
        assert_eq!(
            href_matching(listing, "node-v*-x64.msi"),
            Some("node-v26.11.1-x64.msi")
        );
        assert_eq!(href_matching(listing, "node-v*-x64.zip"), None);
    }
}
