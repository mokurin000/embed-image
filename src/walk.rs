use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn visit_dirs_or_file(
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
