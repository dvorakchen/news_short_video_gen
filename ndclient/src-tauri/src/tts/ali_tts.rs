use std::{fs, path::PathBuf, time::Duration};

use hound::WavReader;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};

use crate::tts::{TTSError, TTSFile, TTSService};

const DEFAULT_TEMP_DIR: &str = "temp";

pub struct AliTTS {
    http: Client,
    url: String,
    key: String,
    temp_dir: String,
}

impl AliTTS {
    /// Create a new instance of `AliTTS` with the given URL and key.
    /// url: The API endpoint for the Ali TTS service.
    /// key: The ALI API key for authentication.
    pub fn new(url: String, key: String) -> Self {
        Self {
            http: Client::new(),
            url,
            key,
            temp_dir: DEFAULT_TEMP_DIR.to_owned(),
        }
    }
}

#[async_trait::async_trait]
impl TTSService for AliTTS {
    async fn tts(&self, text_list: &Vec<String>) -> Result<Vec<TTSFile>, TTSError> {
        let mut list = vec![];

        let to_network_err = |e: reqwest::Error| TTSError::Network(e.to_string());

        let tmp_path = PathBuf::from(&self.temp_dir);
        fs::create_dir_all(&tmp_path).map_err(|e| TTSError::HandleFailed(e.to_string()))?;

        let tmp_path = tmp_path
            .canonicalize()
            .map_err(|e| TTSError::HandleFailed(e.to_string()))?;

        let mut index = 1usize;

        for text in text_list {
            let body = build_body(text);
            let response = self
                .http
                .post(&self.url)
                .header(header::AUTHORIZATION, format!("Bearer {}", self.key))
                .header(header::CONTENT_TYPE, "application/json")
                .body(body)
                .send()
                .await
                .map_err(to_network_err)?;

            if response.status() != 200 {
                let data = response.text().await.unwrap();
                return Err(TTSError::Network(format!(
                    "request ali tts failed: {}",
                    data
                )));
            }

            let data: ApiResponse = response.json().await.map_err(to_network_err)?;

            let audio_url = data.output.audio.url;

            let response = self
                .http
                .get(audio_url)
                .send()
                .await
                .map_err(to_network_err)?;
            let bytes = response.bytes().await.map_err(to_network_err)?;

            let file = tmp_path.clone().join(format!("voice_{:03}.wav", index));
            index += 1;
            fs::write(&file, bytes)?;
            let reader =
                WavReader::open(&file).map_err(|e| TTSError::HandleFailed(e.to_string()))?;

            let seconds = reader.duration() as f64 / reader.spec().sample_rate as f64;
            let duration = Duration::from_secs_f64(seconds);

            list.push(TTSFile {
                path: file,
                text: text.clone(),
                duration,
            });
        }

        fn build_body(text: &String) -> String {
            let len = text.chars().count();
            let mut body = String::with_capacity(100 + len);

            body.push_str(
                r#"{
    "model": "qwen-tts",
    "input": {
        "text": ""#,
            );
            body.push_str(&text);
            body.push_str(
                r#"",
        "voice": "Serena"
    }"#,
            );

            body
        }

        Ok(list)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Audio {
    pub expires_at: i64,
    pub data: String,
    pub id: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Output {
    pub finish_reason: String,
    pub audio: Audio,
}

#[derive(Debug, Serialize, Deserialize)]
struct InputTokensDetails {
    pub text_tokens: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutputTokensDetails {
    pub audio_tokens: i32,
    pub text_tokens: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Usage {
    pub input_tokens_details: InputTokensDetails,
    pub total_tokens: i32,
    pub output_tokens: i32,
    pub input_tokens: i32,
    pub output_tokens_details: OutputTokensDetails,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse {
    pub output: Output,
    pub usage: Usage,
    pub request_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wav_result_one() {
        let wav_data = vec![
            0x52, 0x49, 0x46, 0x46, 0x24, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6D,
            0x74, 0x20, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x40, 0x1F, 0x00, 0x00,
            0x40, 0x1F, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x64, 0x61, 0x74, 0x61, 0x00, 0x00,
            0x00, 0x00,
        ];

        let mut server = mockito::Server::new_async().await;
        let url = server.url();

        let mut first_body = String::from(
            r#"{"output":{"finish_reason":"stop","audio":{"expires_at":1751529865,"data":"","id":"audio_f51d7b99-deab-9785-8599-73705c774fa3",
        "url":""#,
        );
        first_body.push_str(&format!("{}/second", url));
        first_body.push_str(
        r#""
        }},"usage":{"input_tokens_details":{"text_tokens":19},"total_tokens":224,"output_tokens":205,"input_tokens":19,"output_tokens_details":{"audio_tokens":205,"text_tokens":0}},"request_id":"f51d7b99-deab-9785-8599-73705c774fa3"}"#);

        // Create a mock
        let _mock = server
            .mock("GET", "/first")
            .with_status(200)
            .with_header(header::CONTENT_TYPE, "application/json")
            .with_body(first_body)
            .create();

        let _mock = server
            .mock("GET", "/second")
            .with_status(200)
            .with_header(header::CONTENT_TYPE, "application/json")
            .with_body(wav_data)
            .create();

        let ali_tts = AliTTS::new(format!("{}/first", url), "test_key".to_string());
        let tts_files = ali_tts.tts(&vec!["测试".to_owned()]).await;

        assert!(tts_files.is_ok());
        let tts_files = tts_files.unwrap();

        assert_eq!(tts_files.len(), 1);
        assert_eq!(tts_files[0].text, "测试");
        assert!(tts_files[0].path.exists());
    }

    #[tokio::test]
    async fn test_wav_result_two() {
        let wav_data = vec![
            0x52, 0x49, 0x46, 0x46, 0x24, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6D,
            0x74, 0x20, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x40, 0x1F, 0x00, 0x00,
            0x40, 0x1F, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x64, 0x61, 0x74, 0x61, 0x00, 0x00,
            0x00, 0x00,
        ];

        let mut server = mockito::Server::new_async().await;
        let url = server.url();

        let mut first_body = String::from(
            r#"{"output":{"finish_reason":"stop","audio":{"expires_at":1751529865,"data":"","id":"audio_f51d7b99-deab-9785-8599-73705c774fa3",
        "url":""#,
        );
        first_body.push_str(&format!("{}/second", url));
        first_body.push_str(
        r#""
        }},"usage":{"input_tokens_details":{"text_tokens":19},"total_tokens":224,"output_tokens":205,"input_tokens":19,"output_tokens_details":{"audio_tokens":205,"text_tokens":0}},"request_id":"f51d7b99-deab-9785-8599-73705c774fa3"}"#);

        // Create a mock
        let _mock = server
            .mock("GET", "/first")
            .with_status(200)
            .with_header(header::CONTENT_TYPE, "application/json")
            .with_body(first_body)
            .create();

        let _mock = server
            .mock("GET", "/second")
            .with_status(200)
            .with_header(header::CONTENT_TYPE, "application/json")
            .with_body(wav_data)
            .create();

        let ali_tts = AliTTS::new(format!("{}/first", url), "test_key".to_string());
        let tts_files = ali_tts
            .tts(&vec!["测试".to_owned(), "测试".to_owned()])
            .await;

        assert!(tts_files.is_ok());
        let tts_files = tts_files.unwrap();

        assert_eq!(tts_files.len(), 2);
        assert_eq!(tts_files[0].text, "测试");
        assert!(tts_files[0].path.exists());
        assert_eq!(tts_files[1].text, "测试");
        assert!(tts_files[1].path.exists());
    }
}
