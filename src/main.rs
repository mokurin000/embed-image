use std::{
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use zip::write::FileOptions;

#[derive(Debug, palc::Parser)]
pub struct Args {
    img: PathBuf,
    path: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let Args { img, path } = palc::Parser::parse();

    let output_fn = img
        .file_name()
        .unwrap()
        .to_string_lossy()
        .split(".")
        .nth(0)
        .unwrap()
        .to_string()
        + "_merged."
        + &img.extension().unwrap().to_string_lossy();
    let img_data = fs::read(img)?;

    let mut output = OpenOptions::new()
        .append(false)
        .create(true)
        .write(true)
        .read(false)
        .open(&output_fn)?;

    output.write_all(&img_data)?;

    let mut writer = zip::ZipWriter::new(output);
    let options: FileOptions<'_, ()> = FileOptions::default()
        .large_file(false)
        .compression_level(None)
        .compression_method(zip::CompressionMethod::Deflated);

    let mut path_to_pack = Vec::new();

    for p in path {
        visit_dirs_or_file(p, &mut path_to_pack)?;
    }

    for path in path_to_pack {
        writer.start_file_from_path(&path, options)?;
        writer.write(&fs::read(&path)?)?;
    }

    writer.finish()?;

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
