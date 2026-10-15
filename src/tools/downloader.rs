//! The Store backend: download an app's link and install it.
//!
//! Each Store row is a name and a link (see
//! [`crate::data::options::STORE_CATEGORIES`]). The file lands in
//! `%USERPROFILE%\Downloads\<name>`, then an archive is unpacked there and an
//! installer is run. A link may also contain a `*` glob: a GitHub
//! `releases/latest/download` link resolves against the release API, any other
//! link against its directory listing.

use std::path::{Path, PathBuf};

use crate::data::options::{STORE_CATEGORIES, StoreApp, StoreCategory, StoreScript};
use crate::sys;

/// 7-Zip console, fetched once because the bundled `tar.exe` can't read `.7z`.
const SEVEN_ZIP_URL: &str = "https://www.7-zip.org/a/7zr.exe";

/// The Chocolatey StoreApp, the one first-run bootstrap installs.
fn chocolatey_app() -> Option<&'static StoreApp> {
    STORE_CATEGORIES
        .iter()
        .flat_map(|category| category.apps)
        .find(|app| app.name == "Chocolatey")
}

/// True when Chocolatey is missing and should be auto-installed at first run.
pub fn bootstrap_needed() -> bool {
    chocolatey_app().is_some_and(|app| !is_installed(app))
}

/// Install Chocolatey in the background; runs on a worker thread.
pub fn bootstrap() -> Result<String, String> {
    chocolatey_app()
        .ok_or("no Chocolatey entry in the Store list".to_string())
        .and_then(install)
}

/// Download `app` and install it; returns the status-bar message.
pub fn install(app: &StoreApp) -> Result<String, String> {
    // Name every error after the app so the status bar says what failed.
    download_and_install(app)
        .map_err(|error| if error.contains(app.name) {
            error
        } else {
            format!("{}: {error}", app.name)
        })
}

/// The plain install flow, without the app-name prefix.
fn download_and_install(app: &StoreApp) -> Result<String, String> {
    match &app.script {
        Some(StoreScript::Command(program, args)) => {
            sys::run(program, args)?;
            return Ok(format!("installed {}", app.name));
        }
        Some(StoreScript::GlobalPackage(package)) => {
            global_install(app, package)?;
            return Ok(format!("installed {} ({package})", app.name));
        }
        None => {}
    }
    let dir = app_dir(app.name)?;
    let file = download(&resolve_link(app.link)?, &dir)?;
    let name = file.file_name().unwrap_or_default().to_string_lossy();

    if is_archive(&name) {
        extract(&file, &dir)?;
        Ok(format!("installed {} ({name})", app.name))
    } else if is_installer(&name) {
        run_installer(&file, app.args)?;
        Ok(format!("installed {} ({name})", app.name))
    } else {
        Ok(format!("downloaded {} ({name})", app.name))
    }
}

/// "Installed" when the app's marker is on PATH or under a standard install
/// location, "Install" otherwise. Checked in-process (no subprocesses), so a
/// full status refresh stays instant.
pub fn status(app: &StoreApp) -> String {
    if is_installed(app) {
        "Installed".to_string()
    } else {
        "Install".to_string()
    }
}

/// True once `app` is installed: its marker should be found on PATH, or exist
/// under a standard install location or its own download folder.
fn is_installed(app: &StoreApp) -> bool {
    let exe = app.exe;
    // A marker with a folder part is a file check only: matching its bare
    // name on PATH would false-positive on built-in aliases (the stock
    // `wt.exe` alias before the app was installed).
    if exe.contains('/') || exe.contains('\\') {
        return install_roots(app).iter().any(|root| root.join(exe).exists());
    }
    on_path(exe)
        || install_roots(app).iter().any(|root| root.join(exe).is_file())
}

/// The PATH directory list, read once.
fn path_dirs() -> &'static Vec<PathBuf> {
    static PATH_DIRS: std::sync::OnceLock<Vec<PathBuf>> = std::sync::OnceLock::new();
    PATH_DIRS.get_or_init(|| {
        std::env::var_os("Path")
            .unwrap_or_default()
            .to_string_lossy()
            .split(';')
            .filter(|dir| !dir.is_empty())
            .map(PathBuf::from)
            .collect()
    })
}

/// True when `name` resolves on PATH like a shell would, using the common
/// PATHEXT extensions so a bare name ("pi", "command-code") matches its
/// .exe or .cmd shim. scanned in-process — fast enough to run during every
/// status refresh, unlike spawning `where.exe` per app.
fn on_path(name: &str) -> bool {
    // Already carries an extension (e.g. "pi.cmd") vs a bare command name.
    let exts: &[&str] = if name.contains('.') {
        &[""]
    } else {
        &["exe", "cmd", "bat", "com"]
    };
    path_dirs()
        .iter()
        .any(|dir| {
            exts.iter()
                .any(|ext| dir.join(if ext.is_empty() { name.to_string() } else { format!("{name}.{ext}") }).is_file())
        })
}

/// Locations the install marker is searched under.
fn install_roots(app: &StoreApp) -> Vec<PathBuf> {
    let mut roots = vec![];
    for var in ["ProgramFiles", "ProgramFiles(x86)", "LocalAppData", "ProgramData", "USERPROFILE"] {
        if let Some(base) = std::env::var_os(var) {
            roots.push(PathBuf::from(&base));
        }
    }
    if let Some(local) = std::env::var_os("LocalAppData") {
        roots.push(Path::new(&local).join("Programs"));
    }
    if let Ok(dir) = app_dir(app.name) {
        roots.push(dir);
    }
    roots
}

/// Install a global npm-style package: bun when it is installed, npm when
/// node/npm is installed, otherwise an error for the status bar.
fn global_install(app: &StoreApp, package: &str) -> Result<(), String> {
    if on_path("bun.exe") {
        sys::run("bun", &["add", "-g", package])?;
    } else if on_path("node.exe") {
        sys::run("npm", &["i", "-g", package])?;
    } else {
        return Err(format!(
            "{} needs bun or node — install one from the IDEs & Dev Tools section first",
            app.name
        ));
    }
    Ok(())
}

/// `%LOCALAPPDATA%\PatchKit` — where PatchKit keeps its own files (the 7-Zip
/// helper), separate from the apps it downloads.
fn tools_dir() -> Result<PathBuf, String> {
    let local = std::env::var_os("LocalAppData").ok_or("could not find %LOCALAPPDATA%")?;
    Ok(Path::new(&local).join("PatchKit"))
}

/// The user's Downloads folder: `%USERPROFILE%\Downloads`.
fn downloads_dir() -> Result<PathBuf, String> {
    let profile =
        std::env::var_os("USERPROFILE").ok_or("could not find your user profile folder")?;
    Ok(Path::new(&profile).join("Downloads"))
}

/// An app's download and install folder: `Downloads\<name>`.
fn app_dir(name: &str) -> Result<PathBuf, String> {
    Ok(downloads_dir()?.join(name))
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
    let dir = tools_dir()?;
    let tool = dir.join("7zr.exe");
    if !tool.is_file() {
        // The helper's URL always serves "7zr.exe"; download with a fixed
        // name so the saved path is the one checked above.
        std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
        let saved = sys::run(
            "curl.exe",
            &[
                "-L", "--silent", "--show-error", "--fail",
                "-o", &tool.to_string_lossy(), SEVEN_ZIP_URL,
            ],
        )?;
        let _ = saved;
    }
    Ok(tool.to_string_lossy().into_owned())
}

/// Run an installer: `.msi` through `msiexec` (passive, so it can raise the
/// elevation prompt), `.exe` with the app's `args` or the NSIS `/S` fallback.
/// Codes for "installed" and "already installed" all count as success.
fn run_installer(file: &Path, extra_args: &[&str]) -> Result<(), String> {
    let path = file.to_string_lossy().to_string();
    let code = if path.to_ascii_lowercase().ends_with(".msi") {
        // MSI installs accept their own /passive flag; app.args are only
        // meant for .exe installers.
        sys::exit_code("msiexec.exe", &["/i", &path, "/passive", "/norestart"])?
    } else if extra_args.is_empty() {
        sys::exit_code(&path, &["/S"])?
    } else {
        sys::exit_code(&path, extra_args)?
    };
    // 0 = installed, 3010 = installed (reboot needed), 1638 = already installed.
    if matches!(code, 0 | 3010 | 1638) {
        Ok(())
    } else {
        Err(format!("installer exited with code {code}"))
    }
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

/// The newest release asset whose file name matches the `*`/`?` glob `pattern`.
/// When several match, the shortest name wins — the plain build rather than a
/// `-v3`/`-lt20`/CLI variant. The JSON is usually minified onto one line.
fn asset_url_in(json: &str, pattern: &str) -> Option<String> {
    json.split("\"browser_download_url\":")
        .skip(1)
        .filter_map(|rest| rest.trim_start().strip_prefix('"')?.split('"').next())
        .filter(|url| glob_match(pattern, url.rsplit('/').next().unwrap_or("")))
        .min_by_key(|url| url.rsplit('/').next().unwrap_or("").len())
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
    /// Build the status snapshot for the given categories.
    pub fn new(categories: &[StoreCategory]) -> Self {
        let mut status = Self::default();
        status.refresh(categories);
        status
    }

    /// Re-check which apps are downloaded, in the categories' flat order.
    pub fn refresh(&mut self, categories: &[StoreCategory]) {
        self.rows = categories
            .iter()
            .flat_map(|category| category.apps)
            .map(status)
            .collect();
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
    fn on_path_matches_extensions_and_shims() {
        // Present in System32 on every Windows box.
        assert!(on_path("curl.exe"));
        assert!(on_path("notepad.exe"));
        // A bare name probes PATHEXT extensions.
        assert!(!on_path("definitely-not-a-real-command-name-xyz"));
    }

    #[test]
    fn is_installed_marks_unknown_apps_missing() {
        let app = StoreApp::from_script(
            "DefinitelyNotInstalledApp",
            "marker for tests",
            "definitely-not-installed-xyz.exe",
            "echo",
            &[],
        );
        assert!(!is_installed(&app));
    }

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
    fn asset_url_in_prefers_the_shortest_name() {
        let json = concat!(
            r#"{"assets":[{"browser_download_url":"https://example.com/qbittorrent_5.2.4_lt20_x64_setup.exe"},"#,
            r#"{"browser_download_url":"https://example.com/qbittorrent_5.2.4_x64_setup.exe"}]}"#,
        );
        assert_eq!(
            asset_url_in(json, "qbittorrent_*_x64_setup.exe").as_deref(),
            Some("https://example.com/qbittorrent_5.2.4_x64_setup.exe")
        );
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
