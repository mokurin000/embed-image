use std::{
    borrow::Cow,
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    ops::Sub,
    path::Path,
    time::UNIX_EPOCH,
};

use chrono::{Datelike, Timelike};

use spdlog::{error, info, warn};
use zip::{DateTime, write::FileOptions};

use crate::overlay::write_overlayed_image;

mod args;
mod overlay;
mod walk;

fn main() -> Result<(), Box<dyn Error>> {
    let args::Args {
        img,
        path,
        password,
        qrcode_overlap,
        qr_position,
        has_quiet_zone,
        qrcode_fg_color,
        qrcode_bg_color,
    } = palc::Parser::parse();

    if !img.exists() {
        error!("input image not existing!");
    }

    let output_fn = output_filename(&img, qrcode_overlap).expect("failed to parse image filename");

    let mut output = OpenOptions::new()
        .append(false)
        .create(true)
        .write(true)
        .read(false)
        .open(&output_fn)?;

    info!("reading source image");

    if let Some(text) = password.as_deref()
        && qrcode_overlap
    {
        write_overlayed_image(
            img,
            &mut output,
            has_quiet_zone,
            qr_position,
            qrcode_fg_color,
            qrcode_bg_color,
            text,
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
        walk::visit_dirs_or_file(p, &mut path_to_pack)?;
    }

    for path in path_to_pack {
        let mtime = path.metadata()?.modified()?.duration_since(UNIX_EPOCH)?;
        let secs = mtime.as_secs();
        let nanos = mtime.subsec_nanos();
        let mtime_dt =
            chrono::DateTime::from_timestamp(secs as i64, nanos).expect("invalid mtime!");

        let mtime_zip = DateTime::from_date_and_time(
            mtime_dt.year().sub(1980).min(u16::MAX as _) as u16,
            mtime_dt.month() as u8,
            mtime_dt.day() as u8,
            mtime_dt.hour() as u8,
            mtime_dt.minute() as u8,
            mtime_dt.second() as u8,
        )?;
        writer.start_file_from_path(&path, options.last_modified_time(mtime_zip))?;
        let data = fs::read(&path)?;
        let size = humansize::format_size(data.len(), humansize::BINARY);
        info!("read {} of {size}, compressing...", path.to_string_lossy(),);
        writer.write_all(&data)?;
    }

    writer.finish()?;

    info!("all tasks finished without any error.");

    Ok(())
}

fn output_filename(img: impl AsRef<Path>, qrcode_overlap: bool) -> Option<String> {
    Some(
        img.as_ref()
            .file_name()?
            .to_string_lossy()
            .split(".")
            .nth(0)?
            .to_string()
            + "_merged."
            + &if qrcode_overlap {
                Cow::from("png")
            } else {
                img.as_ref().extension().unwrap().to_string_lossy()
            },
    )
}
