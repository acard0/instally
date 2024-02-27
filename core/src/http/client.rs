
use futures::StreamExt;
use std::{fs::File, cmp::min, io::Write};
use bytes::Bytes;

use rust_i18n::error::*;
use convert_case::*;
use crate::*;

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum HttpStreamError {
    #[error("network")]
    Network(#[from] reqwest::Error),

    #[error("content-length")]
    ContentLength,

    #[error("pull-to-file.{}", .0.kind().to_string().to_case(Case::Kebab))]
    PullToFile(#[from] std::io::Error),

    #[error("pull-to-string-utf8")]
    PullToString(#[from] std::string::FromUtf8Error)
}

pub async fn test() -> Result<String, HttpStreamError> {
    let resp = reqwest::get("http://www.gstatic.com/generate_204")
        .await?
        .text().await?;

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
        .await?;

    let total_size = match response.content_length() {
        Some(r) => r,
        _ => Err(HttpStreamError::ContentLength)?
    };

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;

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
            Err(HttpStreamError::PullToFile(err))?
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
        let chunk_str = String::from_utf8(chunk.to_vec())?;
        result_ref.push_str(&chunk_str);
        Ok(())
    }).await?;

    Ok(result_string)
}
