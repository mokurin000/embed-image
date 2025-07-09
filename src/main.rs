use std::{
    borrow::Cow,
    error::Error,
    fs::{self, OpenOptions},
    io::{BufReader, Write},
    ops::Div,
    path::{Path, PathBuf},
};

use image::{EncodableLayout, ImageEncoder as _, Rgba, codecs::png::PngEncoder, imageops::overlay};
use qrencode::{EcLevel, QrCode};
use spdlog::{error, info, warn};
use zip::write::FileOptions;

#[derive(Debug, palc::Parser)]
pub struct Args {
    /// specify a password, optional
    #[arg(long, short = 'p')]
    password: Option<String>,

    /// add an QRCode overlap for password
    #[arg(long, short = 'Q')]
    qrcode_overlap: bool,

    /// has quiet zone of QR Code
    ///
    /// can be: `true`/`false`
    ///
    /// Quiet zone means the surrounding blank area
    #[arg(long, short = 'q', default_value_t = true)]
    has_quiet_zone: std::primitive::bool, // workaround: bypass `bool` match

    /// Position of QR code.
    ///
    /// Can be one of `top-left` (default), `top-right`, `bottom-left`, `bottom-right`, `center`
    ///
    /// will fallback to default on invalid input.
    #[arg(long, short = 'P')]
    qr_position: Option<String>,

    /// Color of QR Code foreground (the bar itself)
    ///
    /// format: CSS3 Color
    #[arg(long, default_value = "#000000ff")]
    qrcode_fg_color: String,
    /// Color of QR Code background (The blank background)
    ///
    /// format: CSS3 Color
    #[arg(long, default_value = "ffffffff")]
    qrcode_bg_color: String,

    /// target file. if enabled `qrcode_overlap, must be one of PNG, JPEG and WEBP.`
    img: PathBuf,
    path: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let Args {
        img,
        path,
        password,
        qr_position,
        qrcode_overlap,
        has_quiet_zone,
        qrcode_fg_color,
        qrcode_bg_color,
    } = palc::Parser::parse();

    if !img.exists() {
        error!("input image not existing!");
    }

    let output_fn = img
        .file_name()
        .unwrap()
        .to_string_lossy()
        .split(".")
        .nth(0)
        .unwrap()
        .to_string()
        + "_merged."
        + &if qrcode_overlap {
            Cow::from("png")
        } else {
            img.extension().unwrap().to_string_lossy()
        };

    let mut output = OpenOptions::new()
        .append(false)
        .create(true)
        .write(true)
        .read(false)
        .open(&output_fn)?;

    info!("reading source image");

    if let Some(pass) = password.as_deref()
        && qrcode_overlap
    {
        let file = OpenOptions::new().read(true).create(false).open(img)?;
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
        let qrcode_img = QrCode::with_error_correction_level(pass, EcLevel::H)?
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
        let encoder = PngEncoder::new(&mut output);
        encoder.write_image(
            orig_image.as_bytes(),
            orig_image.width(),
            orig_image.height(),
            image::ColorType::Rgba8,
        )?;
    } else {
        if qrcode_overlap {
            warn!("QR Code overlap does nothing if did not specify a password");
        }

        info!("copying original image");
        let img_data = fs::read(img)?;
        output.write_all(&img_data)?;
    }

    info!("writing ZIP, password: {password:?}");

    let mut writer = zip::ZipWriter::new(output);
    let mut options: FileOptions<'_, ()> = FileOptions::default()
        .large_file(false)
        .compression_level(None)
        .compression_method(zip::CompressionMethod::Deflated);

    if let Some(password) = password.as_deref() {
        options = options.with_aes_encryption(zip::AesMode::Aes256, password);
    }

    let mut path_to_pack = Vec::new();

    for p in path {
        visit_dirs_or_file(p, &mut path_to_pack)?;
    }

    for path in path_to_pack {
        writer.start_file_from_path(&path, options)?;
        let data = fs::read(&path)?;
        let size = humansize::format_size(data.len(), humansize::BINARY);
        info!("read {} of {size}, compressing...", path.to_string_lossy(),);
        writer.write_all(&data)?;
    }

    writer.finish()?;

    info!("all tasks finished without any error.");

    Ok(())
}

fn visit_dirs_or_file(
    path: impl AsRef<Path>,
    append_to: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    let path = path.as_ref();
    if path.is_file() {
        append_to.push(path.to_path_buf());
        return Ok(());
    }

    let dir = fs::read_dir(path)?;
    for entry in dir.flatten() {
        let path = entry.path();

        if path.is_dir() {
            visit_dirs_or_file(path, append_to)?;
        } else if path.is_file() {
            append_to.push(path);
        }
    }

    Ok(())
}
