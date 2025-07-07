pub mod srt;

use std::{io, path::PathBuf, time::Duration};

use thiserror::Error;

pub type SubtitleResult = Result<PathBuf, SubtitleError>;

pub struct SingleSubtitle {
    pub text: String,
    pub duration: Duration,
}

#[derive(Error, Debug)]
pub enum SubtitleError {
    #[error("file error: {0}")]
    File(#[from] io::Error),
}

#[async_trait::async_trait]
pub trait Subtitle {
    async fn write_subtitle(&self, subtitles: &Vec<SingleSubtitle>) -> SubtitleResult;
}
