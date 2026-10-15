# PatchKit

A small Windows cleanup, tweaks and download TUI. One terminal window, three
tabs: free up disk space, flip common Windows settings, and download useful apps,
all without digging through Control Panel or the registry.

![Rust](https://img.shields.io/badge/rust-stable-blue)
![Platform](https://img.shields.io/badge/platform-Windows-blue)

## Download

Prebuilt binaries are on the
[Releases](https://github.com/Nafisarkar/patchkit/releases) page:

- **Latest build** — a rolling release rebuilt on every push to `master`
- **Versioned releases** (`v0.1.0`, `v0.2.0`, ...) — cut from version tags

Two artifacts are published for each release:

- **`patchkit-setup.exe`** — Windows installer. Installs to `Program Files`,
  creates Start Menu and desktop shortcuts, adds PatchKit to the system PATH
  (so `patchkit` works in any terminal), and registers an uninstaller in
  Apps & Features. Requires administrator rights.
- **`patchkit.exe`** — standalone portable executable; just drop it anywhere.

## Build from source

Requires Windows and the Rust toolchain (MSVC target).

```sh
git clone https://github.com/Nafisarkar/patchkit.git
cd patchkit
cargo build --release
```

The binary lands in `target/release/patchkit.exe`, with the app icon and
version info embedded.

### Build the installer

1. Install [NSIS 3](https://nsis.sourceforge.io/) (or run the build on the
   GitHub Actions workflow, which has it preinstalled).
2. Regenerate installer assets from `icons/Icon.png` when the icon changes:

   ```sh
   cargo run --locked --manifest-path tools/gen-assets/Cargo.toml --release
   ```

   This writes `icons/Icon.ico` (exe + installer icon) and the NSIS
   `installer/*.bmp` side panels.

3. Compile the installer:

   ```sh
   makensis installer\patchkit.nsi                # uses version from Cargo.toml default
   makensis /DVERSION=0.2.0 installer\patchkit.nsi  # or pin one explicitly
   ```

   `dist/patchkit-setup.exe` is the finished installer.

## Usage

The **Windows Activation** tweak needs PatchKit to run as administrator; every
other option works with normal user rights.
