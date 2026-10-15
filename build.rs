// Embeds icons/Icon.ico and version info into patchkit.exe so the exe and its
// shortcuts show the app icon in Explorer, the taskbar and UAC prompts.
use std::env;
use std::path::PathBuf;

use winresource::{VersionInfo, WindowsResource};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    // Re-run only when the icon or version metadata actually changes.
    println!("cargo:rerun-if-changed=assets/app.ico");
    println!("cargo:rerun-if-changed=build.rs");

    let manifest = env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let mut parts = manifest.split('.').map(|p| p.parse::<u64>().unwrap_or(0));
    let (major, minor, patch, revision) = (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    );

    // Copy the icon into OUT_DIR so the resource compiler always sees a
    // stable, absolute path. icons/Icon.ico is generated from icons/Icon.png
    // by tools/gen-assets and committed to the repo.
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let icon_src = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("icons/Icon.ico");
    std::fs::copy(&icon_src, out_dir.join("app.ico"))
        .expect("icons/Icon.ico missing — run gen-assets");

    let mut res = WindowsResource::new();
    res.set_icon(out_dir.join("app.ico").to_str().unwrap());
    res.set_version_info(VersionInfo::FILEVERSION, major << 48 | minor << 32 | patch << 16 | revision);
    res.set_version_info(VersionInfo::PRODUCTVERSION, major << 48 | minor << 32 | patch << 16 | revision);
    res.set("FileDescription", "PatchKit - Windows cleanup, tweaks and downloads TUI");
    res.set("FileVersion", &format!("{manifest}.0"));
    res.set("ProductName", "PatchKit");
    res.set("ProductVersion", &format!("{manifest}.0"));
    res.set("OriginalFilename", "patchkit.exe");
    res.compile()
        .expect("compile Windows resources (icon + version info)");
}
