//! Génère `assets/ScreenCapture.ico` (Windows) et `assets/ScreenCapture.icns` (macOS)
//! à partir de l'icône dessinée par programme.
//! Usage : `cargo run --release --example make_icon`

#[path = "../src/icon.rs"]
mod icon;

use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};

fn main() -> anyhow::Result<()> {
    let sizes = [16u32, 20, 24, 32, 40, 48, 64, 128, 256];
    let pngs: Vec<Vec<u8>> = sizes
        .iter()
        .map(|&s| {
            let mut out = Vec::new();
            PngEncoder::new(&mut out).write_image(&icon::app_icon(s), s, s, ExtendedColorType::Rgba8)?;
            Ok(out)
        })
        .collect::<anyhow::Result<_>>()?;

    // Format ICO : en-tête, répertoire d'entrées, puis images PNG.
    let mut ico = Vec::new();
    ico.extend_from_slice(&[0, 0, 1, 0]);
    ico.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (&s, png) in sizes.iter().zip(&pngs) {
        let dim = if s >= 256 { 0 } else { s as u8 };
        ico.extend_from_slice(&[dim, dim, 0, 0]);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(png.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for png in &pngs {
        ico.extend_from_slice(png);
    }

    std::fs::create_dir_all("assets")?;
    std::fs::write("assets/ScreenCapture.ico", ico)?;
    println!("assets/ScreenCapture.ico écrit ({} tailles)", sizes.len());

    // Format ICNS : « icns » + taille totale, puis des entrées PNG typées (gros-boutiste).
    let entries: [(&[u8; 4], u32); 10] = [
        (b"icp4", 16),
        (b"icp5", 32),
        (b"ic11", 32),
        (b"icp6", 64),
        (b"ic12", 64),
        (b"ic07", 128),
        (b"ic08", 256),
        (b"ic13", 256),
        (b"ic09", 512),
        (b"ic14", 512),
    ];
    let mut body = Vec::new();
    for (kind, s) in entries {
        let mut png = Vec::new();
        PngEncoder::new(&mut png).write_image(&icon::app_icon(s), s, s, ExtendedColorType::Rgba8)?;
        body.extend_from_slice(kind);
        body.extend_from_slice(&(png.len() as u32 + 8).to_be_bytes());
        body.extend_from_slice(&png);
    }
    let mut icns = b"icns".to_vec();
    icns.extend_from_slice(&(body.len() as u32 + 8).to_be_bytes());
    icns.extend_from_slice(&body);
    std::fs::write("assets/ScreenCapture.icns", icns)?;
    println!("assets/ScreenCapture.icns écrit");
    Ok(())
}
