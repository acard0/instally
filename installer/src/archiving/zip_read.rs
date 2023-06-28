use std::{io, path, fs};

pub fn extract_to<F>(input: &fs::File, output: &path::Path, progress_callback: &F) -> Result<Vec<std::path::PathBuf>, zip::result::ZipError> 
where F: Fn(f32)
{
    let mut paths = vec![];
    let mut archive = zip::ZipArchive::new(input)?;
    let length = archive.len();

    for i in 0..length {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => output.join(path),
            None => continue,
        };

        if (*file.name()).ends_with('/') {
            log::trace!("Archive: file {} extracted to \"{}\"", i, outpath.display());
            fs::create_dir_all(&outpath)?;
        } else {
            log::trace!(
                "Archive: file {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;

            paths.push(outpath);
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }

        let progress = (i as f32 / length as f32) * 100.0;
        progress_callback(progress);
    }

    Ok(paths)
}