use std::{fs, io, path::{self, Path}, fs::File};
use zip::result::ZipError;

pub fn extract_to<F>(
    input: &File,
    output: &Path,
    progress_callback: &F,
) -> Result<Vec<path::PathBuf>, ZipError>
where
    F: Fn(f32),
{
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
            log::trace!("Archive: file {} extracted to \"{}\"", i, outpath_full.display());
            fs::create_dir_all(&outpath_full)?;
        } else {
            log::trace!(
                "Archive: file {} extracted to \"{}\" ({} bytes)",
                i,
                outpath_full.display(),
                file.size()
            );
            if let Some(p) = outpath_full.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath_full)?;
            io::copy(&mut file, &mut outfile)?;
            paths.push(outpath.to_path_buf());
        }

        // Get and Set permissions
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