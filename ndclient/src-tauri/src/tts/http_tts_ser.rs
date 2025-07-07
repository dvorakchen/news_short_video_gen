use std::{path::PathBuf, time::Duration};

use hound::WavReader;
use reqwest::Client;
use tokio::fs;

use crate::tts::{TTSError, TTSFile, TTSService};

const DEFAULT_TEMP_DIR: &str = "./temp";

pub struct HttpTTSSer {
    url: String,
    temp_dir: &'static str,
}

impl HttpTTSSer {
    pub fn new(url: String) -> Self {
        Self {
            url,
            temp_dir: DEFAULT_TEMP_DIR,
        }
    }
}

#[async_trait::async_trait]
impl TTSService for HttpTTSSer {
    async fn tts(&self, text_list: &Vec<String>) -> Result<Vec<TTSFile>, TTSError> {
        let text_list: Vec<&String> = text_list
            .into_iter()
            .filter(|item| !item.is_empty())
            .collect();

        if text_list.len() == 0 {
            return Err(TTSError::EmptyText);
        }

        let http = Client::new();
        let response = http
            .post(&self.url)
            .json(&text_list)
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| TTSError::HandleFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(TTSError::HandleFailed(format!(
                "Server returned error: {}",
                response.status()
            )));
        }

        let dir = PathBuf::from(self.temp_dir);

        if !dir.exists() {
            fs::create_dir(&dir)
                .await
                .map_err(|e| TTSError::HandleFailed(e.to_string()))?;
        }

        let path = dir.join(format!("{}.wav", nanoid::nanoid!(10)));

        let bytes = response
            .bytes()
            .await
            .map_err(|e| TTSError::HandleFailed(e.to_string()))?;

        tokio::fs::write(&path, bytes)
            .await
            .map_err(|e| TTSError::HandleFailed(e.to_string()))?;

        let reader = WavReader::open(&path).map_err(|e| TTSError::HandleFailed(e.to_string()))?;

        let seconds = reader.duration() as f64 / reader.spec().sample_rate as f64;
        let duration = Duration::from_secs_f64(seconds);

        Ok(vec![TTSFile {
            path,
            duration,
            text: String::new(),
        }])
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    // #[tokio::test]
    // async fn get_wav_file() {
    //     let wav_data = vec![
    //         0x52, 0x49, 0x46, 0x46, 0x24, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6D,
    //         0x74, 0x20, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x40, 0x1F, 0x00, 0x00,
    //         0x40, 0x1F, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x64, 0x61, 0x74, 0x61, 0x00, 0x00,
    //         0x00, 0x00,
    //     ];

    //     let mut server = mockito::Server::new_async().await;

    //     let url = server.url();

    //     // Create a mock
    //     let _mock = server
    //         .mock("POST", "/tts")
    //         .with_status(200)
    //         .with_header("content-type", "audio/wav")
    //         .with_body(wav_data)
    //         .create();

    //     let service = HttpTTSSer::new(format!("{}/tts", url));
    //     let tts_file = service
    //         .tts(&vec!["第一".to_owned(), "第二".to_owned()])
    //         .await;

    //     assert!(tts_file.is_ok());
    //     let tts_file = tts_file.unwrap();

    //     assert!(tts_file.path.exists());

    //     fs::remove_file(tts_file.path).await.unwrap();
    // }

    #[tokio::test]
    async fn empty_text_list() {
        let service = HttpTTSSer::new("".to_owned());
        let tts_file = service.tts(&vec!["".to_owned()]).await;

        assert!(tts_file.is_err());
    }
}
