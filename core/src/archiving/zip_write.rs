use std::{io, path::{self, Path}};

use walkdir::WalkDir;
use zip::{write::SimpleFileOptions, ZipWriter};

use crate::helpers::{self, sha1::Sha1Error};

use super::error::ArchiveError;

pub fn compress_dirs<T>(it: &mut dyn Iterator<Item = walkdir::DirEntry>, prefix: &str, writer: &mut T, method: zip::CompressionMethod) 
    -> Result<Vec<std::path::PathBuf>, ArchiveError>
where T: io::Write + io::Seek,
{
    let mut paths = vec![];
    let mut zip = ZipWriter::new(writer);
    let options = SimpleFileOptions::default()
        .compression_method(method)
        .unix_permissions(0o755);

    let mut buffer = Vec::new();
    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(path::Path::new(prefix)).unwrap();

        // Write file or directory explicitly
        // Some unzip tools unzip files with directory paths correctly, some do not!
        if path.is_file() {
            log::trace!("archive: adding file {path:?} as {name:?} ...");
            #[allow(deprecated)]
            zip.start_file_from_path(name, options)?;
            let mut f = helpers::file::open(path)?;
            let path = helpers::file::path(&mut f)?;

            helpers::file::read_to_end(&mut f, &mut buffer)?;
            helpers::file::write_all_stream(&mut zip, &buffer)?;
            buffer.clear();

            paths.push(path)
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            log::trace!("archive: adding dir {path:?} as {name:?} ...");
            #[allow(deprecated)]
            zip.add_directory_from_path(name, options)?;
        }
    }

    zip.finish()?;

    Ok(paths)
}

pub fn compress_dir<P: AsRef<Path>>(src_dir: P, dst_file: P, method: zip::CompressionMethod, sha1: Option<&mut String>, writeout_sha1: bool) 
    -> Result<Vec<std::path::PathBuf>, ArchiveError> {

    if !src_dir.as_ref().is_dir() {
        log::error!("Supplied source path parameter is not a directory for compression. {:?}", src_dir.as_ref());
        return Err(ArchiveError::Io(std::io::Error::from(std::io::ErrorKind::NotFound).into()));
    }

    let mut file = helpers::file::create(&dst_file)?;
    let walkdir = WalkDir::new(&src_dir);
    let it = walkdir.into_iter();
    let paths = compress_dirs(&mut it.filter_map(|e: Result<walkdir::DirEntry, walkdir::Error>| e.ok()), src_dir.as_ref().to_str().unwrap(), &mut file, method)?;

    if let Some(out) = sha1 {
        *out = helpers::sha1::generate_sha1_file(&mut file)?;
        helpers::file::write_all(format!("{}.sha1", dst_file.as_ref().to_str().unwrap()), out.as_bytes())
            .map_err(|err| Sha1Error::from(err))?;
    }

    Ok(paths)
}