# PatchKit

A small Windows cleanup and tweaks TUI. One terminal window, two tabs: free up
disk space and flip common Windows settings without digging through Control
Panel or the registry.

![Rust](https://img.shields.io/badge/rust-stable-blue)
![Platform](https://img.shields.io/badge/platform-Windows-blue)

## Features

### Cleaning

| Option       | What it does                                          |
| ------------ | ----------------------------------------------------- |
| Recycle Bin  | Empty the Recycle Bin to free up disk space           |
| Windows Temp | Delete system temp files from `C:\Windows\Temp`       |
| User Temp    | Delete per-user temp files from `%LOCALAPPDATA%\Temp` |
| Disk Clean   | Open the built-in Windows Disk Cleanup utility        |

### Tweaks

| Option                | What it does                                                                              |
| --------------------- | ----------------------------------------------------------------------------------------- |
| Toggle Dark Mode      | Switch system apps between light and dark theme                                           |
| Show File Extensions  | Show file extensions (.exe, .png, etc.) in File Explorer                                  |
| Show Hidden Files     | Show or hide hidden files and folders in File Explorer                                    |
| Windows Activation    | Activate Windows with MAS (requires administrator)                                        |
| Wallpaper Compression | Disable JPEG wallpaper compression for full-quality backgrounds (applies after a restart) |

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

| Key                      | Action                 |
| ------------------------ | ---------------------- |
| `↑` / `↓` (or `k` / `j`) | Move in the table      |
| `Enter`                  | Run the selected tweak |
| `←` / `→` (or `h` / `l`) | Switch tabs            |
| `q` or `Esc`             | Quit                   |

The **Windows Activation** tweak needs PatchKit to run as administrator; every
other option works with normal user rights.
