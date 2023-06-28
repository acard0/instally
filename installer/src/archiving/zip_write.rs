use std::{fs::{File}, io, path};

use filepath::FilePath;
use walkdir::WalkDir;
use zip::{write::FileOptions, result::{ZipError, ZipResult}, ZipWriter};

pub fn compress_dirs<T>(it: &mut dyn Iterator<Item = walkdir::DirEntry>, prefix: &str, writer: T, method: zip::CompressionMethod,) 
    -> zip::result::ZipResult<Vec<std::path::PathBuf>>
where T: io::Write + io::Seek,
{
    let mut paths = vec![];
    let mut zip = ZipWriter::new(writer);
    let options = FileOptions::default()
        .compression_method(method)
        .unix_permissions(0o755);

    let mut buffer = Vec::new();
    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(path::Path::new(prefix)).unwrap();

        // Write file or directory explicitly
        // Some unzip tools unzip files with directory paths correctly, some do not!
        if path.is_file() {
            log::trace!("Archive: adding file {path:?} as {name:?} ...");
            #[allow(deprecated)]
            zip.start_file_from_path(name, options)?;
            let mut f = File::open(path)?;
            let path = f.path()?;

            io::Read::read_to_end(&mut f, &mut buffer)?;
            io::Write::write_all(&mut zip, &buffer)?;
            buffer.clear();

            paths.push(path)
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            log::trace!("Archive: adding dir {path:?} as {name:?} ...");
            #[allow(deprecated)]
            zip.add_directory_from_path(name, options)?;
        }
    }
    zip.finish()?;

    Ok(paths)
}

pub fn compress_dir(src_dir: &str, dst_file: &str, method: zip::CompressionMethod) 
    -> ZipResult<Vec<std::path::PathBuf>> {

    if !path::Path::new(src_dir).is_dir() {
        return Err(ZipError::FileNotFound);
    }

    let path = path::Path::new(dst_file);
    let file = File::create(path)?;

    let walkdir = WalkDir::new(src_dir);
    let it = walkdir.into_iter();

    let paths = compress_dirs(&mut it.filter_map(|e: Result<walkdir::DirEntry, walkdir::Error>| e.ok()), src_dir, file, method)?;
    Ok(paths)
}