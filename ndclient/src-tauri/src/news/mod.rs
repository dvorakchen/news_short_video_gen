use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod pengpai_news;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewsTitle {
    pub title: String,
    pub url: String,
    pub pics: Vec<String>,
    pub videos: Vec<String>,
}

impl NewsTitle {
    pub async fn get_news_material<T>(&self, material_extractor: &T) -> NewsMaterialResult
    where
        T: MaterialExtractor,
    {
        material_extractor.get_material(self).await
    }
}

pub type NewsMaterialResult = Result<NewsMaterial, NewsMaterialError>;

#[derive(Error, Debug)]
pub enum NewsMaterialError {}

pub struct NewsMaterial {
    pub title: String,
    pub summary: Vec<String>,
    pub pics: Vec<String>,
    pub videos: Vec<String>,
}

/// crawler, indicats how to get the news title list
#[async_trait::async_trait]
pub trait NewsCrawler {
    async fn get_hot_news_list(&self) -> Vec<NewsTitle>;
}

/// Extractor, responsible for extractor news material: news summary, pictures, videos.
#[async_trait::async_trait]
pub trait MaterialExtractor {
    async fn get_material(&self, hot_news: &NewsTitle) -> NewsMaterialResult;
}
