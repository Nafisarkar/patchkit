// One-off generator for release assets:
//   * icons/Icon.ico          — multi-size ICO (16..256) for the exe + installer
//   * installer/welcome.bmp   — 164x314 NSIS MUI2 welcome/finish panel
//   * installer/header.bmp    — 150x57  NSIS MUI2 header banner
//
// Run from the repo root:  cargo run --release --manifest-path tools/gen-assets/Cargo.toml
// (kept out of the patchkit workspace members so the main build stays clean;
//  it just needs `image` on the lockfile, hence the tiny feature set)

use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

type BoxError = Box<dyn std::error::Error>;

const SRC: &str = "icons/Icon.png";

// Brand gradient endpoints, matching the fill in icons/Icon.svg. The panels
// reuse it so the composited logo blends into its backdrop instead of sitting
// on a mismatched colour.
const GRADIENT_TOP: [u8; 3] = [0x13, 0x1F, 0x3C]; // #131F3C
const GRADIENT_BOTTOM: [u8; 3] = [0x23, 0x50, 0xB9]; // #2350B9

/// Horizontal placement of the logo inside a panel.
enum Align {
    Center,
    Right,
}

fn main() -> Result<(), BoxError> {
    let img = image::open(SRC)?;
    println!(
        "source {} -> {}x{} {:?}",
        SRC,
        img.width(),
        img.height(),
        img.color()
    );

    std::fs::create_dir_all("installer")?;

    write_ico(&img, "icons/Icon.ico")?;
    // Welcome/finish panel: square logo centred at full width.
    write_panel(&img, "installer/welcome.bmp", 164, 314, 22, Align::Center)?;
    // Header banner: square logo on the right at full height.
    write_panel(&img, "installer/header.bmp", 150, 57, 8, Align::Right)?;

    Ok(())
}

// ------------------------------------------------------------------------ ico

fn write_ico(img: &DynamicImage, path: &str) -> Result<(), BoxError> {
    let sizes = [16u16, 24, 32, 48, 64, 128, 256];
    let mut entries: Vec<(u16, Vec<u8>)> = Vec::new();
    for &s in &sizes {
        let resized = img
            .resize_exact(s as u32, s as u32, FilterType::Lanczos3)
            .to_rgba8();
        let png = encode_png(&resized, ImageFormat::Png)?;
        entries.push((s, png));
    }

    let mut out = Vec::new();
    // ICONDIR: reserved, type, count
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());

    let mut offset = 6 + entries.len() * 16;
    for (s, png) in &entries {
        out.extend_from_slice(&[*s as u8, *s as u8]); // width, height (0 means 256)
        if *s == 256 {
            let last = out.len() - 2;
            out[last] = 0;
            out[last + 1] = 0;
        }
        out.extend_from_slice(&[0, 0]); // color palette, reserved
        out.extend_from_slice(&1u16.to_le_bytes()); // planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bpp
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += png.len();
    }
    for (_, png) in &entries {
        out.extend_from_slice(png);
    }

    std::fs::write(path, &out)?;
    println!("wrote {} ({} bytes)", path, out.len());
    Ok(())
}

// --------------------------------------------------------------------- panels

// NSIS MUI2 side/banner panels. The source logo is square, so scaling it to
// fill these non-square panels directly would stretch it; instead the logo is
// scaled to fit (preserving aspect ratio) and composited over a brand-gradient
// backdrop.
fn write_panel(
    img: &DynamicImage,
    path: &str,
    w: u32,
    h: u32,
    margin: u32,
    align: Align,
) -> Result<(), BoxError> {
    let mut canvas = gradient(w, h);

    let box_w = w.saturating_sub(margin * 2).max(1);
    let box_h = h.saturating_sub(margin * 2).max(1);
    let scale = (box_w as f64 / img.width() as f64).min(box_h as f64 / img.height() as f64);
    let logo_w = ((img.width() as f64 * scale).round() as u32).max(1);
    let logo_h = ((img.height() as f64 * scale).round() as u32).max(1);
    let logo = img.resize_exact(logo_w, logo_h, FilterType::Lanczos3).to_rgba8();

    let x = match align {
        Align::Center => (w - logo_w) / 2,
        Align::Right => w - margin - logo_w,
    };
    let y = (h - logo_h) / 2;
    image::imageops::overlay(&mut canvas, &logo, i64::from(x), i64::from(y));

    write_bmp(&canvas, path)?;
    Ok(())
}

/// Vertical linear gradient between the two brand colours.
fn gradient(w: u32, h: u32) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        let t = if h <= 1 {
            0.0
        } else {
            y as f32 / (h - 1) as f32
        };
        let pixel = lerp(GRADIENT_TOP, GRADIENT_BOTTOM, t);
        for x in 0..w {
            img.put_pixel(x, y, pixel);
        }
    }
    img
}

/// Blend two RGB colours; `t` is 0.0 at `a` and 1.0 at `b`.
fn lerp(a: [u8; 3], b: [u8; 3], t: f32) -> Rgba<u8> {
    let mix = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8;
    Rgba([mix(0), mix(1), mix(2), 255])
}

// ------------------------------------------------------------------------ bmp

// NSIS MUI2 wants 24-bit BMPs with rows stored bottom-up (negative height is
// not handled by all Windows dialogs), so write the DIB by hand.
fn write_bmp(img: &RgbaImage, path: &str) -> Result<(), BoxError> {
    let (w, h) = img.dimensions();

    let row = (w as usize * 3 + 3) & !3; // rows are 4-byte aligned
    let data_size = row * h as usize;
    let file_size = 14 + 40 + data_size;

    let mut out = Vec::with_capacity(file_size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(file_size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]); // reserved
    out.extend_from_slice(&54u32.to_le_bytes()); // pixel data offset

    // BITMAPINFOHEADER
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    // positive height => bottom-up rows, which MUI2 expects
    out.extend_from_slice(&(h as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // planes
    out.extend_from_slice(&24u16.to_le_bytes()); // bpp
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&(data_size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 16]); // ppm + colors used/important

    for y in (0..h).rev() {
        for x in 0..w {
            let Rgba([r, g, b, _a]) = img.get_pixel(x, y);
            out.push(*b);
            out.push(*g);
            out.push(*r);
        }
        for _ in w as usize * 3..row {
            out.push(0); // row padding
        }
    }

    std::fs::write(path, &out)?;
    println!("wrote {} ({}x{}, {} bytes)", path, w, h, out.len());
    Ok(())
}

fn encode_png(img: &image::RgbaImage, format: ImageFormat) -> Result<Vec<u8>, BoxError> {
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img.clone()).write_to(&mut buf, format)?;
    Ok(buf.into_inner())
}
