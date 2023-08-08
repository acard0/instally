use std::{path::PathBuf, io};


pub fn open<P: Into<PathBuf>>(path: P) -> io::Result<fs_err::File> {
    let file = fs_err::OpenOptions::new()
        .read(true).write(true).open(path)?;

    Ok(file)
}

pub fn open_create<P: Into<PathBuf>>(path: P) -> io::Result<fs_err::File> {
    let file = fs_err::OpenOptions::new()
        .create(true).read(true).write(true).open(path)?;

    Ok(file)
}
