use std::{fs::File, path::{self, Path}};

use filepath::FilePath;

use crate::helpers;

use super::error::ArchiveError;

pub fn extract_to<F>(input: &mut File, output: &Path, progress_callback: &F, sha1: Option<&str>) 
    -> Result<Vec<path::PathBuf>, ArchiveError>
where F: Fn(f32),
{
    if let Some(checksum) = sha1 {
        let mut sha1 = String::new();
        if !(helpers::sha1::verify_sha1_file(input, checksum, Some(&mut sha1))?) {
            log::error!("Sha1 of downloaded package {:?} (sha: {:?}) does not match with {:?}", input.path(), sha1, checksum);
            return Err(ArchiveError::Sha1Mismatch);
        }
    }

    let mut paths = vec![];
    let mut archive = zip::ZipArchive::new(input)?;
    let length = archive.len();

    for i in 0..length {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };
        let outpath_full = output.join(outpath.clone());

        if (*file.name()).ends_with('/') {
            log::trace!("archive: file {} extracted to \"{}\"", i, outpath_full.display());
            helpers::file::create_dir_all(&outpath_full)?;
        } else {
            log::trace!(
                "archive: file {} extracted to \"{}\" ({} bytes)",
                i,
                outpath_full.display(),
                file.size()
            );
            if let Some(p) = outpath_full.parent() {
                if !p.exists() {
                    helpers::file::create_dir_all(p)?;
                }
            }
            let mut outfile = helpers::file::create(&outpath_full)?;
            helpers::file::copy_stream(&mut file, &mut outfile)?;
            paths.push(outpath.to_path_buf());
        }

        // Get and set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath_full, fs::Permissions::from_mode(mode))?;
            }
        }

        let progress = (i as f32 / length as f32) * 100.0;
        progress_callback(progress);
    }

    progress_callback(100.0);
    Ok(paths)
}