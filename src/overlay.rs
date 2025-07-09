use std::{
    error::Error,
    fs::OpenOptions,
    io::{BufReader, Write},
    ops::{Deref, Div},
    path::Path,
};

use image::{EncodableLayout, ImageEncoder as _, Rgba, codecs::png::PngEncoder, imageops::overlay};
use qrencode::{EcLevel, QrCode};
use spdlog::{info, warn};

pub fn write_overlayed_image(
    img: impl AsRef<Path>,
    output: impl Write,
    has_quiet_zone: bool,
    qr_position: Option<String>,
    qrcode_fg_color: impl Deref<Target = str>,
    qrcode_bg_color: impl Deref<Target = str>,
    text: impl AsRef<str>,
) -> Result<(), Box<dyn Error>> {
    let file = OpenOptions::new()
        .read(true)
        .create(false)
        .open(img.as_ref())?;
    let bufreader = BufReader::new(file);

    info!("start pixel converting");

    let mut orig_image = image::io::Reader::new(bufreader)
        .with_guessed_format()?
        .decode()?
        .to_rgba8(); // use RGBA8 to better save space

    info!("start QR Code generation");

    let orig_width = orig_image.width();
    let orig_height = orig_image.height();
    let pixel_len = orig_width.min(orig_height).div(3).max(200);

    let fg_color = csscolorparser::parse(&qrcode_fg_color)?.to_rgba8();
    let bg_color = csscolorparser::parse(&qrcode_bg_color)?.to_rgba8();
    let qrcode_img = QrCode::with_error_correction_level(text.as_ref(), EcLevel::H)?
        .render::<image::Rgba<u8>>()
        .max_dimensions(pixel_len, pixel_len)
        .quiet_zone(has_quiet_zone)
        .light_color(Rgba(bg_color))
        .dark_color(Rgba(fg_color))
        .build();
    let real_pixel_len = qrcode_img.width();

    let (x, y) = match qr_position.as_deref() {
        Some("top-right") => (orig_width - real_pixel_len, 0),
        Some("bottom-left") => (0, orig_height - real_pixel_len),
        Some("bottom-right") => (orig_width - real_pixel_len, orig_height - real_pixel_len),
        Some("center") => (
            (orig_width - real_pixel_len) / 2,
            (orig_height - real_pixel_len) / 2,
        ),
        Some(pos) => {
            if pos != "top-left" {
                warn!("unknown position {pos}, falling back to top-left");
            }
            (0, 0)
        }
        _ => (0, 0),
    };

    info!("overlapping QR Code on original image");
    overlay(&mut orig_image, &qrcode_img, x.into(), y.into());

    info!("writing overlapped image");
    let encoder = PngEncoder::new(output);
    encoder.write_image(
        orig_image.as_bytes(),
        orig_image.width(),
        orig_image.height(),
        image::ColorType::Rgba8,
    )?;

    Ok(())
}
