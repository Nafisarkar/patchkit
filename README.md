# PatchKit

A small Windows cleanup and tweaks TUI. One terminal window, two tabs: free up
disk space and flip common Windows settings without digging through Control
Panel or the registry.

![Rust](https://img.shields.io/badge/rust-stable-blue)
![Platform](https://img.shields.io/badge/platform-Windows-blue)

## Download

Prebuilt binaries are on the
[Releases](https://github.com/Nafisarkar/patchkit/releases) page:

- **Latest build** — a rolling release rebuilt on every push to `master`
- **Versioned releases** (`v0.1.0`, `v0.2.0`, ...) — cut from version tags

Download `patchkit.exe` and run it in a terminal.

## Build from source

Requires Windows and the Rust toolchain (MSVC target).

```sh
git clone https://github.com/Nafisarkar/patchkit.git
cd patchkit
cargo build --release
```

The binary lands in `target/release/patchkit.exe`.

## Usage

The **Windows Activation** tweak needs PatchKit to run as administrator; every
other option works with normal user rights.
