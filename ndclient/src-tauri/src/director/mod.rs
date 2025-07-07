use std::{io, path::PathBuf, process::Command, sync::Arc, time::Duration};

use hound::WavReader;
use nanoid::nanoid;
use thiserror::Error;
use tokio::fs;
pub mod source;

use crate::{
    director::source::NewsSource,
    news::{MaterialExtractor, NewsMaterial, NewsMaterialError, NewsMaterialResult, NewsTitle},
    subtitle::{SingleSubtitle, Subtitle},
    tts::{TTSFile, TTSService, get_wav_len},
    video::VideoEditor,
};

#[derive(Error, Debug)]
pub enum DirectorError {
    #[error("fail occurred: {0}")]
    Failed(String),
    #[error("get material failed: {0}")]
    Material(#[from] NewsMaterialError),
    #[error("tts error: {0}")]
    TTS(String),
    #[error("subtitle error: {0}")]
    Subtitle(String),
    #[error("file error")]
    File(#[from] io::Error),
    #[error("video editor error: {0}")]
    VideoEditor(String),
}

pub type DirectorResult<T> = Result<T, DirectorError>;

pub struct Director {
    cur_id: String,
    source: NewsSource,
    tts: Option<Box<dyn TTSService + Sync + Send + 'static>>,
    subtitle: Option<Box<dyn Subtitle + Sync + Send + 'static>>,
    video_editor: Option<Box<dyn VideoEditor + Sync + Send + 'static>>,
}

impl Director {
    pub fn new(source: NewsSource) -> Self {
        Self {
            cur_id: nanoid::nanoid!(10),
            source,
            tts: None,
            subtitle: None,
            video_editor: None,
        }
    }

    pub fn with_tts(mut self, tts: impl TTSService + Sync + Send + 'static) -> Self {
        let tts = Box::new(tts);
        self.tts = Some(tts);
        self
    }

    pub fn with_subtitle(mut self, subtitle: impl Subtitle + Sync + Send + 'static) -> Self {
        let subtitle = Box::new(subtitle);
        self.subtitle = Some(subtitle);
        self
    }

    pub fn with_video_editor(
        mut self,
        video_editor: impl VideoEditor + Sync + Send + 'static,
    ) -> Self {
        let video_editor = Box::new(video_editor);
        self.video_editor = Some(video_editor);
        self
    }

    pub async fn get_hot_news_list(&self) -> Vec<NewsTitle> {
        self.source.crawler.get_hot_news_list().await
    }

    pub async fn shot_single(&mut self, news_title: &NewsTitle) -> DirectorResult<NewsShortVideo> {
        self.cur_id = nanoid::nanoid!(10);

        struct WrapExtractor {
            inner: Arc<Box<dyn MaterialExtractor + Sync + Send>>,
        }

        #[async_trait::async_trait]
        impl MaterialExtractor for WrapExtractor {
            async fn get_material(&self, hot_news: &NewsTitle) -> NewsMaterialResult {
                self.inner.get_material(hot_news).await
            }
        }

        let material = {
            let wrap_extractor = WrapExtractor {
                inner: Arc::clone(&self.source.extractor.0),
            };

            news_title.get_news_material(&wrap_extractor).await?
        };

        let dubbing_path = if self.tts.is_some() {
            Some(self.gen_dubbing(&material).await?)
        } else {
            None
        };

        let subtitle_path = if let Some(ref subtitle_handler) = self.subtitle
            && let Some(ref dubbing_subtitle) = dubbing_path
        {
            Some(
                subtitle_handler
                    .write_subtitle(&dubbing_subtitle.tts_files)
                    .await
                    .map_err(|e| DirectorError::Subtitle(e.to_string()))?,
            )
        } else {
            None
        };

        let video_path = {
            let dur = if let Some(ref dubbing) = dubbing_path {
                let reader = WavReader::open(&dubbing.dubbing_path)
                    .map_err(|e| DirectorError::TTS(e.to_string()))?;

                let seconds = reader.duration() as f64 / reader.spec().sample_rate as f64;
                let duration = Duration::from_secs_f64(seconds);

                Some(duration)
            } else {
                None
            };

            if self.video_editor.is_some() {
                Some(self.gen_video(&material, dur).await?)
            } else {
                None
            }
        };

        let video_path = video_path
            .ok_or_else(|| DirectorError::VideoEditor("has not video editor setted".to_string()))?;

        let final_path = self
            .compose_all(
                video_path,
                dubbing_path.unwrap().dubbing_path,
                subtitle_path.unwrap(),
            )
            .await
            .map_err(|e| DirectorError::VideoEditor(e.to_string()))?;

        Ok(NewsShortVideo {
            title: material.title.clone(),
            path: final_path,
        })
    }

    async fn gen_dubbing(&mut self, material: &NewsMaterial) -> DirectorResult<DubbingSubtitle> {
        if self.tts.is_none() {
            return Err(DirectorError::TTS("has no TTS setted".to_owned()));
        }

        let mut tts_files = self
            .tts
            .as_ref()
            .unwrap()
            .tts(&material.summary)
            .await
            .map_err(|e| DirectorError::TTS(e.to_string()))?;

        // carton tts files
        for tts_file in tts_files.iter_mut() {
            let mut new_file_path = tts_file.path.clone();
            new_file_path.set_file_name(format!(
                "{}-cartoned.{}",
                new_file_path.file_stem().unwrap().to_str().unwrap(),
                new_file_path.extension().unwrap().to_str().unwrap()
            ));

            Command::new("ffmpeg")
                .args(&[
                    "-i",
                    tts_file.path.to_str().unwrap(),
                    "-af",
                    "asetrate=30000, aresample=22050, atempo=1",
                    new_file_path.to_str().unwrap(),
                ])
                .output()
                .map_err(|e| DirectorError::TTS(e.to_string()))?;

            _ = fs::remove_file(&tts_file.path).await;

            tts_file.duration = get_wav_len(&new_file_path)
                .await
                .map_err(|e| DirectorError::TTS(e.to_string()))?;
            tts_file.path = new_file_path;
        }

        // compose up
        let compose_path = self.compose_audio(&tts_files).await?;

        let subtitles: Vec<SingleSubtitle> = tts_files
            .into_iter()
            .map(|tts| {
                _ = std::fs::remove_file(&tts.path);
                return SingleSubtitle {
                    text: tts.text.clone(),
                    duration: tts.duration,
                };
            })
            .collect();

        Ok(DubbingSubtitle {
            dubbing_path: compose_path,
            tts_files: subtitles,
        })
    }

    async fn compose_audio(&self, tts_files: &Vec<TTSFile>) -> DirectorResult<PathBuf> {
        if tts_files.is_empty() {
            return Err(DirectorError::TTS("has no tts_files".to_owned()));
        }

        let spec = {
            let first = tts_files.get(0).unwrap();
            let reader =
                WavReader::open(&first.path).map_err(|e| DirectorError::TTS(e.to_string()))?;
            reader.spec()
        };

        let silence = {
            let silence_duration = 0.3;
            let num_samples = (spec.sample_rate as f32 * silence_duration) as usize;
            vec![0; num_samples * spec.channels as usize]
        };

        let mut compose_wav = vec![];

        for tts_file in tts_files {
            let mut reader = WavReader::open(&tts_file.path).unwrap();
            let samples: Vec<i32> = reader.samples::<i16>().map(|s| s.unwrap() as i32).collect();

            compose_wav.extend(samples.clone());
            compose_wav.extend(silence.clone());
        }

        let final_wav = self
            .get_temp_dir()
            .await?
            .join(format!("{}-final.wav", nanoid!()));

        let mut writer = hound::WavWriter::create(&final_wav, spec)
            .map_err(|e| DirectorError::TTS(e.to_string()))?;

        for sample in compose_wav {
            writer.write_sample(sample).unwrap();
        }

        Ok(final_wav)
    }

    async fn compose_all(
        &self,
        video: PathBuf,
        dubbing: PathBuf,
        subtitle: PathBuf,
    ) -> DirectorResult<PathBuf> {
        let output_path = self
            .get_temp_dir()
            .await?
            .join(format!("{}-final.mp4", nanoid::nanoid!()));

        Command::new("ffmpeg")
            .args(&[
                "-i",
                video.to_str().unwrap(),
                "-i",
                dubbing.to_str().unwrap(),
                "-vf",
                &format!("subtitles='{}'", subtitle.to_str().unwrap()),
                "-c:v",
                "libx264",
                "-c:a",
                "copy",
                "-shortest",
                "-y",
                output_path.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| DirectorError::VideoEditor(e.to_string()))?;

        _ = fs::remove_file(video).await;
        _ = fs::remove_file(dubbing).await;
        _ = fs::remove_file(subtitle).await;

        Ok(output_path)
    }

    async fn gen_video(
        &self,
        material: &NewsMaterial,
        dur: Option<Duration>,
    ) -> DirectorResult<PathBuf> {
        if self.video_editor.is_none() {
            return Err(DirectorError::VideoEditor(
                "has not video editor setted".to_string(),
            ));
        }

        let time = if let Some(time) = dur {
            time
        } else {
            Duration::from_secs((material.pics.len() * 2) as u64)
        };

        self.video_editor
            .as_ref()
            .unwrap()
            .do_edit(material, time)
            .await
            .map_err(|e| DirectorError::VideoEditor(e.to_string()))
    }

    async fn get_temp_dir(&self) -> DirectorResult<PathBuf> {
        let temp = PathBuf::from(format!("./temp/{}", self.cur_id.to_string()));
        if !temp.exists() {
            fs::create_dir_all(&temp).await.unwrap();
        }

        let temp = temp.canonicalize().unwrap();

        return Ok(temp);
    }
}

struct DubbingSubtitle {
    dubbing_path: PathBuf,
    tts_files: Vec<SingleSubtitle>,
}

pub struct NewsShortVideo {
    pub title: String,
    pub path: PathBuf,
}

#[cfg(test)]
mod tests {
    use reqwest::header;

    use crate::{
        news::NewsCrawler, subtitle::srt::SrtSubtitle, tts::ali_tts::AliTTS,
        video::junior_editor::JuniorEditor,
    };

    use super::*;

    #[tokio::test]
    async fn dubbing_tts_none() {
        struct MockCrawler;

        #[async_trait::async_trait]
        impl NewsCrawler for MockCrawler {
            async fn get_hot_news_list(&self) -> Vec<NewsTitle> {
                unimplemented!()
            }
        }

        struct MockMaterialExtractor;

        #[async_trait::async_trait]
        impl MaterialExtractor for MockMaterialExtractor {
            async fn get_material(&self, _hot_news: &NewsTitle) -> NewsMaterialResult {
                unimplemented!()
            }
        }

        let material = NewsMaterial {
            title: "这是一个标题".to_owned(),
            summary: vec![
                "闺蜜闺蜜想不想玩第五人格喵喵喵".to_owned(),
                "兄弟兄弟想不想玩第五人格喵喵喵".to_owned(),
            ],
            pics: vec![
                "./tests/mock_pic_1.jpeg".to_owned(),
                "./tests/mock_pic_2.jpeg".to_owned(),
            ],
            videos: vec![],
        };

        let mut director = Director::new(NewsSource {
            crawler: Box::new(MockCrawler),
            extractor: MockMaterialExtractor.into(),
        });

        let path = director.gen_dubbing(&material).await;

        assert!(path.is_err());
    }

    #[tokio::test]
    async fn director_shot_single() {
        struct MockCrawler;

        #[async_trait::async_trait]
        impl NewsCrawler for MockCrawler {
            async fn get_hot_news_list(&self) -> Vec<NewsTitle> {
                vec![NewsTitle {
                    title: "这是一个标题".to_owned(),
                    url: "https://example.com/news/1".to_owned(),
                    pics: vec![],
                    videos: vec![],
                }]
            }
        }

        let mut server = mockito::Server::new_async().await;
        let url = server.url();

        server
            .mock("GET", "/mock_pic_1")
            .with_status(200)
            .with_header(header::CONTENT_TYPE, "image/jpeg")
            .with_body(fs::read("./tests/mock_pic_1.jpeg").await.unwrap())
            .create();

        server
            .mock("GET", "/mock_pic_2")
            .with_status(200)
            .with_header(header::CONTENT_TYPE, "image/jpeg")
            .with_body(fs::read("./tests/mock_pic_2.jpeg").await.unwrap())
            .create();

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
            .with_body(fs::read("./tests/mock_voice.wav").await.unwrap())
            .create();

        let ali_tts = AliTTS::new(format!("{}/first", url), "test_key".to_string());

        struct MockMaterialExtractor(String);

        #[async_trait::async_trait]
        impl MaterialExtractor for MockMaterialExtractor {
            async fn get_material(&self, _hot_news: &NewsTitle) -> NewsMaterialResult {
                let url = self.0.clone();
                Ok(NewsMaterial {
                    title: "这是一个标题".to_owned(),
                    summary: vec![
                        "闺蜜闺蜜想不想玩第五人格喵喵喵".to_owned(),
                        "兄弟兄弟想不想玩第五人格喵喵喵".to_owned(),
                    ],
                    pics: vec![
                        format!("{}/mock_pic_1", url.clone()),
                        format!("{}/mock_pic_2", url.clone()),
                    ],
                    videos: vec![],
                })
            }
        }

        let mock = MockCrawler.get_hot_news_list().await;
        let material = mock.get(0).unwrap();

        let mut director = Director::new(NewsSource {
            crawler: Box::new(MockCrawler),
            extractor: MockMaterialExtractor(url).into(),
        })
        .with_video_editor(JuniorEditor::new())
        .with_tts(ali_tts)
        .with_subtitle(SrtSubtitle::new());

        let path = director.shot_single(&material).await.unwrap();

        assert!(path.path.exists());
    }
}
