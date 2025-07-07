use core::panic;
use std::time::Duration;

use ndclient_lib::{
    director::{Director, source::NewsSource},
    news::{MaterialExtractor, NewsCrawler, NewsMaterial, NewsMaterialResult, NewsTitle},
    subtitle::srt::SrtSubtitle,
    tts::{TTSError, TTSFile, TTSService},
    video::junior_editor::JuniorEditor,
};
use tokio::fs;

const MOCK_TITLE: &str = "第五人格启动";

struct MockNewsCrawler;

#[async_trait::async_trait]
impl NewsCrawler for MockNewsCrawler {
    async fn get_hot_news_list(&self) -> Vec<NewsTitle> {
        vec![
            NewsTitle{
                title: MOCK_TITLE.to_owned(),
                url: "https://example.com/".to_owned(),
                pics: vec!["https://plus.unsplash.com/premium_photo-1675337267945-3b2fff5344a0?fm=jpg&q=60&w=300".to_owned()],
                videos: vec![]
            }
        ]
    }
}

struct MockNewsMaterialExtractor;

#[async_trait::async_trait]
impl MaterialExtractor for MockNewsMaterialExtractor {
    async fn get_material(&self, hot_news: &NewsTitle) -> NewsMaterialResult {
        Ok(NewsMaterial {
            title: hot_news.title.clone(),
            summary: vec![
                "鸡块狗".to_owned(),
                "闺蜜闺蜜要不要跟我玩第五人格喵喵喵".to_owned(),
                "兄弟兄弟要不要跟我玩第五人格喵喵喵".to_owned(),
            ],
                pics: vec![
                    "https://plus.unsplash.com/premium_photo-1675337267945-3b2fff5344a0?fm=jpg&q=60&w=300".to_owned(),
                    "https://images.unsplash.com/photo-1750748305395-5fc18cb35f2a?fm=jpg&q=60&w=300".to_owned()
                ],
                videos: vec![]
        })
    }
}

struct MockTTSSer;

#[async_trait::async_trait]
impl TTSService for MockTTSSer {
    async fn tts(&self, _text_list: &Vec<String>) -> Result<Vec<TTSFile>, TTSError> {
        let temp_dir = fs::canonicalize("./tests").await.unwrap();

        let mock_file = temp_dir.join("mock_voice.wav");

        let path = temp_dir.join("fake_tts_output.wav");

        if path.exists() {
            return Ok(vec![TTSFile {
                path,
                text: String::new(),
                duration: Duration::from_secs(2),
            }]);
        }

        fs::copy(mock_file, &path).await.unwrap();

        if !path.exists() {
            panic!("has not test wav file: {}", path.display());
        }
        Ok(vec![TTSFile {
            path,
            text: String::new(),
            duration: Duration::from_secs(2),
        }])
    }
}

#[tokio::test]
async fn director_shot_single() {
    let mut director = Director::new(NewsSource {
        crawler: Box::new(MockNewsCrawler),
        extractor: MockNewsMaterialExtractor.into(),
    })
    .with_tts(MockTTSSer)
    .with_subtitle(SrtSubtitle::new())
    .with_video_editor(JuniorEditor::new());

    let list = director.get_hot_news_list().await;

    assert!(list.len() != 0);

    let first = list.get(0).unwrap();

    let news_short_video = director.shot_single(first).await;

    assert!(news_short_video.is_ok());

    _ = fs::remove_file(news_short_video.unwrap().path).await;
}
