pub mod ali_tts;
pub mod http_tts_ser;

use hound::WavReader;
use std::{io, path::PathBuf, time::Duration};
use thiserror::Error;

#[async_trait::async_trait]
pub trait TTSService {
    async fn tts(&self, text_list: &Vec<String>) -> Result<Vec<TTSFile>, TTSError>;
}

pub struct TTSFile {
    // the audio file path
    pub path: PathBuf,
    // text
    pub text: String,
    // this audio duartion
    pub duration: Duration,
}

#[derive(Error, Debug)]
pub enum TTSError {
    #[error("handle text_list failed: {0}")]
    HandleFailed(String),
    #[error("cannot pass empty test")]
    EmptyText,
    #[error("network error: {0}")]
    Network(String),
    #[error("IO error: {0}")]
    IO(#[from] io::Error),
}

pub(crate) async fn get_wav_len(path: &PathBuf) -> Result<Duration, TTSError> {
    let reader = WavReader::open(path).map_err(|e| TTSError::HandleFailed(e.to_string()))?;

    let seconds = reader.duration() as f64 / reader.spec().sample_rate as f64;
    let duration = Duration::from_secs_f64(seconds);

    Ok(duration)
}
