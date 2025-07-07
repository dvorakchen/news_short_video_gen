use crate::news::NewsMaterial;
use std::{io, path::PathBuf, time::Duration};
use thiserror::Error;

pub mod junior_editor;

pub type VideoEditorResult<T> = Result<T, VideoEditorError>;

#[derive(Error, Debug)]
pub enum VideoEditorError {
    #[error("network request error: {0}")]
    NetWork(String),
    #[error("IO error: {0}")]
    IO(#[from] io::Error),
    #[error("Image error: {0}")]
    Image(String),
}

#[async_trait::async_trait]
pub trait VideoEditor {
    /// Edit the video Pics and return the path of the edited video.
    /// The `dur` is the duration of the video, which is used to determine how many pictures are needed.
    /// The `material` contains the information of the news material, including the pictures.
    /// The edited video will be in 9:16 aspect ratio.
    /// The Video without subtitle, dubbing
    async fn do_edit(&self, material: &NewsMaterial, dur: Duration) -> VideoEditorResult<PathBuf>;
}
