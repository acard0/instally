

pub fn create_tmp_file() -> std::io::Result<tempfile::NamedTempFile> {
    let path = std::env::temp_dir();
    std::fs::create_dir_all(&path)?; // ensure path is created.
                                     // this is required on some Windows builds
    tempfile::NamedTempFile::new_in(path)
}