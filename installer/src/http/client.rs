use futures::StreamExt;

use std::{error::Error, fs::File, io::Write, cmp::min};
use bytes::Bytes;

pub async fn test() -> Result<String, Box<dyn std::error::Error>> {
    let resp = reqwest::get("http://www.gstatic.com/generate_204")
        .await?
        .text()
        .await?;

    Ok(resp)
}

pub async fn download<F, P>(
    url: &str,
    mut progress_callback: F,
    mut process_chunk: P,
) -> Result<(), Box<dyn Error + Send + Sync>>
where
    F: FnMut(f32),
    P: FnMut(Bytes) -> Result<(), Box<dyn Error + Send + Sync>>,
{
    // Reqwest setup
    let res = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;

    let total_size = res
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", &url))?;

    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Error while downloading file")))?;

        process_chunk(chunk.clone())?;

        downloaded = min(downloaded + (chunk.len() as u64), total_size);
        let progress = (downloaded as f32 / total_size as f32) * 100.0;
        progress_callback(progress);
    }

    Ok(())
}

pub async fn get_file<F>(url: &str, path: &str, progress_callback: F) -> Result<(), Box<dyn Error + Send + Sync>>
where
    F: FnMut(f32),
{
    let mut file = File::create(path).or(Err(format!("Failed to create file '{}'", path)))?;

    download(url, progress_callback, move |chunk| {
        file.write_all(&chunk)
            .map_err(|_| format!("Error while writing to file").into())
    }).await
}

pub async fn get_text<F>(url: &str, progress_callback: F) -> Result<String, Box<dyn Error + Send + Sync>>
where
    F: FnMut(f32),
{
    let mut result_string = String::new();
    let result_ref = &mut result_string;

    download(url, progress_callback, move |chunk| {
        if let Ok(chunk_str) = String::from_utf8(chunk.to_vec()) {
            result_ref.push_str(&chunk_str);
            Ok(())
        } else {
            Err(format!("Invalid UTF-8 data in chunk").into())
        }
    }).await?;

    Ok(result_string)
}
