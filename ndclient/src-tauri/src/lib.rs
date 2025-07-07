use crate::{
    director::{source::NewsSource, Director}, news::{pengpai_news::{PengPaiNews, PengPaiNewsMaterialExtractor}, NewsTitle}, subtitle::srt::SrtSubtitle, tts::ali_tts::AliTTS, video::junior_editor::JuniorEditor
};

pub mod director;
pub mod news;
pub mod subtitle;
pub mod tts;
pub mod video;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn get_hot_news() -> Vec<NewsTitle> {
    let director = director::Director::new(NewsSource {
        crawler: Box::new(news::pengpai_news::PengPaiNews::new()),
        extractor: PengPaiNewsMaterialExtractor::from_deepseek().into(),
    });

    director.get_hot_news_list().await
}

#[tauri::command]
async fn gen_video(news_title: NewsTitle) -> String {
    
    let tts_url = dotenv::var("TTS_URL").unwrap();
    let ali_key = dotenv::var("ALI_DASHSCOPE_API_KEY").unwrap();

    let mut director = Director::new(NewsSource {
        crawler: Box::new(PengPaiNews::new()),
        extractor: PengPaiNewsMaterialExtractor::from_deepseek().into(),
    })
    .with_tts(AliTTS::new(tts_url, ali_key))
    .with_subtitle(SrtSubtitle::new())
    .with_video_editor(JuniorEditor::new());


    director
        .shot_single(&news_title)
        .await
        .unwrap()
        .path
        .display()
        .to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, get_hot_news, gen_video])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
