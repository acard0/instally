use error_stack::{IntoReport, ResultExt, Context, Result, Report};
use futures::StreamExt;
use std::{fs::File, cmp::min, fmt, io::Write};
use bytes::Bytes;

#[derive(Debug)]
pub enum HttpStreamError {
    NetworkError(String),
    ChunkProcessError(String)
}
impl fmt::Display for HttpStreamError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str("Failed to complete the http request")
    }
}
impl Context for HttpStreamError {}

pub async fn test() -> Result<String, HttpStreamError> {
    let resp = reqwest::get("http://www.gstatic.com/generate_204").await
        .into_report()
        .change_context(HttpStreamError::NetworkError("Failed to fetch http://www.gstatic.com/generate_204".to_owned()))?
        .text().await
        .into_report()
        .change_context(HttpStreamError::ChunkProcessError("Failed to read stream at http://www.gstatic.com/generate_204".to_owned()))?;

    Ok(resp)
}

pub async fn download<F, P>(
    url: &str,
    mut progress_callback: F,
    mut process_chunk: P,
) -> Result<(), HttpStreamError>
where
    F: FnMut(f32),
    P: FnMut(Bytes) -> Result<(), HttpStreamError>,
{
    // Reqwest setup
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .into_report()
        .change_context(HttpStreamError::NetworkError(format!("Failed to fetch {}", url)))?;

    let total_size = match response.content_length() {
        Some(r) => r,
        _ => Err(Report::new(HttpStreamError::ChunkProcessError(format!("Failed to read content length at {}", url))))
            .attach_printable(format!("Failed to resolve content size for {}", url))?
    };

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item
            .into_report()
            .change_context(HttpStreamError::ChunkProcessError("Failed to read chunk from the stream".to_owned()))
            .attach_printable("Failed to read chunk from the stream")?;

        process_chunk(chunk.clone())?;

        downloaded = min(downloaded + (chunk.len() as u64), total_size);
        let progress = (downloaded as f32 / total_size as f32) * 100.0;
        progress_callback(progress);
    }

    Ok(())
}

pub async fn get_file<F>(url: &str, file: &mut File, progress_callback: F) -> Result<(), HttpStreamError>
where
    F: FnMut(f32),
{
    download(url, progress_callback, move |chunk| {
        file.write_all(&chunk).or_else(|err| {
            let str = format!("Failed to write stream chunk to the file.");
            let report = Report::from(HttpStreamError::ChunkProcessError(str.clone()))
                .attach_printable(str.clone());
            Err(report)
        })
    }).await
}

pub async fn get_text<F>(url: &str, progress_callback: F) -> Result<String, HttpStreamError>
where
    F: FnMut(f32),
{
    let mut result_string = String::new();
    let result_ref = &mut result_string;

    download(url, progress_callback, move |chunk| {
        let chunk_result = String::from_utf8(chunk.to_vec()).into_report();

        match chunk_result {
            Ok(chunk_str) => {
                result_ref.push_str(&chunk_str);
                Ok(())
            }
            Err(_) => {
                let str = "Failed to parse byte chunk into string".to_owned();
                chunk_result
                    .change_context(HttpStreamError::ChunkProcessError(str.clone()))
                    .attach_printable(str.clone())
                    .map(|s| ())
            }
        }
    }).await?;

    Ok(result_string)
}
