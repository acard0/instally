
use futures::StreamExt;
use std::{fs::File, io::{Seek, SeekFrom, Write}, time::Duration};
use bytes::Bytes;
use once_cell::sync::Lazy;

use rust_i18n::error::*;
use convert_case::*;
use crate::*;

#[derive(thiserror::Error, rust_i18n::AsDetails, strum::AsRefStr, Debug)]
pub enum HttpStreamError {
    #[error("network")]
    Network(#[from] reqwest::Error),

    #[error("status-code")]
    StatusCode(u16),

    #[error("content-length")]
    ContentLength,

    #[error("pull-to-file.{}", .0.kind().to_string().to_case(Case::Kebab))]
    PullToFile(#[from] std::io::Error),

    #[error("pull-to-string-utf8")]
    PullToString(#[from] std::string::FromUtf8Error)
}

const MAX_RETRIES: u32 = 4;
const RETRY_BASE_DELAY: Duration = Duration::from_millis(500);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(30);

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent(concat!("instally/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .pool_idle_timeout(Duration::from_secs(90))
        .build()
        .expect("failed to build reqwest client")
});

fn is_retryable(err: &HttpStreamError) -> bool {
    match err {
        HttpStreamError::Network(e) => {
            e.is_timeout() || e.is_connect() || e.is_request() || e.is_body() || e.is_decode()
        }
        HttpStreamError::StatusCode(code) => {
            matches!(code, 408 | 425 | 429 | 500 | 502 | 503 | 504)
        }
        HttpStreamError::ContentLength => true,
        HttpStreamError::PullToFile(_) | HttpStreamError::PullToString(_) => false,
    }
}

fn backoff_delay(attempt: u32) -> Duration {
    RETRY_BASE_DELAY * 2u32.pow(attempt.saturating_sub(1))
}

pub async fn test() -> Result<String, HttpStreamError> {
    let resp = CLIENT.get("http://www.gstatic.com/generate_204")
        .send()
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
    let response = CLIENT.get(url)
        .send()
        .await?;

    if response.status().is_success() == false {
        return Err(HttpStreamError::StatusCode(response.status().as_u16()))
    }

    let total_size = response.content_length().unwrap_or(0);

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;

        process_chunk(chunk.clone())?;

        downloaded += chunk.len() as u64;
        if total_size > 0 {
            let progress = (downloaded as f32 / total_size as f32) * 100.0;
            progress_callback(progress.clamp(0.0, 100.0));
        }
    }

    Ok(())
}

/// Downloads `url` into `file`, retrying transient failures with backoff.
pub async fn get_file<F>(url: &str, file: &mut File, mut progress_callback: F) -> Result<(), HttpStreamError>
where
    F: FnMut(f32),
{
    let mut attempt: u32 = 0;
    loop {
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;

        let result = download(url, &mut progress_callback, |chunk| {
            file.write_all(&chunk).map_err(HttpStreamError::PullToFile)
        }).await;

        match result {
            Ok(()) => {
                file.flush().map_err(HttpStreamError::PullToFile)?;
                return Ok(());
            }
            Err(err) if is_retryable(&err) && attempt < MAX_RETRIES => {
                attempt += 1;
                let delay = backoff_delay(attempt);
                log::warn!("Download of '{}' failed (attempt {}/{}): {}. Retrying in {:?}.", url, attempt, MAX_RETRIES, err, delay);
                progress_callback(0.0);
                tokio::time::sleep(delay).await;
            }
            Err(err) => return Err(err),
        }
    }
}

/// Downloads `url` as a UTF-8 string, retrying transient failures with backoff.
pub async fn get_text<F>(url: &str, mut progress_callback: F) -> Result<String, HttpStreamError>
where
    F: FnMut(f32),
{
    let mut attempt: u32 = 0;
    loop {
        let mut buffer: Vec<u8> = Vec::new();

        let result = {
            let buffer = &mut buffer;
            download(url, &mut progress_callback, |chunk| {
                buffer.extend_from_slice(chunk.as_ref());
                Ok(())
            }).await
        };

        match result {
            Ok(()) => return String::from_utf8(buffer).map_err(HttpStreamError::PullToString),
            Err(err) if is_retryable(&err) && attempt < MAX_RETRIES => {
                attempt += 1;
                let delay = backoff_delay(attempt);
                log::warn!("Fetch of '{}' failed (attempt {}/{}): {}. Retrying in {:?}.", url, attempt, MAX_RETRIES, err, delay);
                progress_callback(0.0);
                tokio::time::sleep(delay).await;
            }
            Err(err) => return Err(err),
        }
    }
}
