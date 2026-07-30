//! Regenerate `assets/icon.ico` (and a BMP preview) from the procedural
//! renderer in `argb_core::icon` — the same code the app uses live for its
//! window and tray, so every surface stays pixel-identical.
//!
//! Run from the workspace root:  cargo run -p argb_core --example make_icon

use std::io::Write;

const SIZES: [u32; 6] = [256, 128, 64, 48, 32, 16];

fn main() -> std::io::Result<()> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let assets = root.join("assets");
    std::fs::create_dir_all(&assets)?;

    // ICO container: classic 32-bit BMP entries (universally supported).
    let images: Vec<(u32, Vec<u8>)> = SIZES
        .iter()
        .map(|&s| (s, argb_core::icon::render(s)))
        .collect();

    let mut ico: Vec<u8> = Vec::new();
    ico.extend_from_slice(&0u16.to_le_bytes()); // reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // type: icon
    ico.extend_from_slice(&(images.len() as u16).to_le_bytes());

    let mut blobs: Vec<Vec<u8>> = Vec::new();
    for (s, rgba) in &images {
        blobs.push(bmp_entry(*s, rgba));
    }
    let mut offset = 6 + 16 * images.len() as u32;
    for ((s, _), blob) in images.iter().zip(&blobs) {
        let dim = if *s >= 256 { 0u8 } else { *s as u8 };
        ico.push(dim); // width (0 = 256)
        ico.push(dim); // height
        ico.push(0); // palette colors
        ico.push(0); // reserved
        ico.extend_from_slice(&1u16.to_le_bytes()); // planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        ico.extend_from_slice(&(blob.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += blob.len() as u32;
    }
    for blob in &blobs {
        ico.extend_from_slice(blob);
    }
    let ico_path = assets.join("icon.ico");
    std::fs::File::create(&ico_path)?.write_all(&ico)?;
    println!("wrote {} ({} bytes)", ico_path.display(), ico.len());

    // A plain BMP preview of the 256 px render for quick visual checks.
    let preview = bmp_file(256, &images[0].1);
    let bmp_path = assets.join("icon-preview.bmp");
    std::fs::File::create(&bmp_path)?.write_all(&preview)?;
    println!("wrote {}", bmp_path.display());
    Ok(())
}

/// One ICO image entry: BITMAPINFOHEADER + bottom-up BGRA rows + AND mask.
fn bmp_entry(size: u32, rgba: &[u8]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&40u32.to_le_bytes()); // header size
    b.extend_from_slice(&(size as i32).to_le_bytes());
    b.extend_from_slice(&((size * 2) as i32).to_le_bytes()); // XOR + AND
    b.extend_from_slice(&1u16.to_le_bytes()); // planes
    b.extend_from_slice(&32u16.to_le_bytes()); // bpp
    b.extend_from_slice(&[0u8; 24]); // compression .. unused
    for y in (0..size).rev() {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            b.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    // AND mask: all zero (alpha channel rules), rows padded to 32 bits.
    let row_bytes = ((size + 31) / 32) * 4;
    b.extend(std::iter::repeat(0u8).take((row_bytes * size) as usize));
    b
}

/// A standalone .bmp file (32-bit, bottom-up) for previewing.
fn bmp_file(size: u32, rgba: &[u8]) -> Vec<u8> {
    let data_size = size * size * 4;
    let mut b = Vec::new();
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&(14 + 40 + data_size).to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&(14u32 + 40).to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(size as i32).to_le_bytes());
    b.extend_from_slice(&(size as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&32u16.to_le_bytes());
    b.extend_from_slice(&[0u8; 24]);
    for y in (0..size).rev() {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            // Composite onto dark gray so transparency is visible in viewers.
            let a = rgba[i + 3] as f32 / 255.0;
            let bg = 40.0;
            let px = |c: u8| (c as f32 * a + bg * (1.0 - a)) as u8;
            b.extend_from_slice(&[px(rgba[i + 2]), px(rgba[i + 1]), px(rgba[i]), 255]);
        }
    }
    b
}
