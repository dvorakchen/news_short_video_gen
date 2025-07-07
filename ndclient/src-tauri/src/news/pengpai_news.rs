use openai::{
    Credentials,
    chat::{ChatCompletion, ChatCompletionMessage, ChatCompletionMessageRole},
};
use reqwest::Client;
use scraper::Selector;
use std::{cell::RefCell, sync::Mutex};

use crate::news::{MaterialExtractor, NewsCrawler, NewsMaterial, NewsMaterialResult, NewsTitle};

pub struct PengPaiNews {
    client: Client,
    hot_news_resp: Mutex<RefCell<Vec<PengPaiHotNews>>>,
    // pub articles: Vec<Article>,
}

impl PengPaiNews {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            hot_news_resp: Mutex::new(RefCell::new(vec![])),
            // articles: vec![],
        }
    }
}

#[async_trait::async_trait]
impl NewsCrawler for PengPaiNews {
    async fn get_hot_news_list(&self) -> Vec<super::NewsTitle> {
        let url = "https://cache.thepaper.cn/contentapi/wwwIndex/rightSidebar";

        let response = self.client.get(url).send().await.unwrap();
        let json: ResponseContent = response.json().await.unwrap();

        let resp_data = json.data.hot_news;

        {
            let mut lock = self.hot_news_resp.lock().unwrap();
            let list = lock.get_mut();
            *list = resp_data.clone();
        }

        resp_data
            .into_iter()
            .map(PengPaiHotNews::to_hot_news)
            .collect()
    }
}

#[derive(Debug, serde::Deserialize)]
struct ResponseContent {
    pub data: Data,
}

#[derive(Debug, serde::Deserialize)]
struct Data {
    #[serde(rename = "hotNews")]
    pub hot_news: Vec<PengPaiHotNews>,
}

#[derive(Debug, serde::Deserialize, Clone)]
struct PengPaiHotNews {
    #[serde(rename = "contId")]
    pub cont_id: String,
    pub name: String,
    #[serde(rename = "pic")]
    pub cover: String,
    pub videos: Option<Video>,
}

impl PengPaiHotNews {
    pub fn to_hot_news(self) -> NewsTitle {
        let mut pics = vec![self.cover];
        let mut videos = vec![];

        if let Some(video) = self.videos {
            videos.push(video.url);
            pics.push(video.cover);
        }

        NewsTitle {
            title: self.name,
            url: format!(
                "https://www.thepaper.cn/newsDetail_forward_{}",
                self.cont_id
            ),
            pics,
            videos,
        }
    }
}

#[derive(Debug, serde::Deserialize, Clone)]
struct Video {
    #[serde(rename = "url")]
    pub url: String,
    #[serde(rename = "coverUrlFirstFrame")]
    pub cover: String,
}

pub struct PengPaiNewsMaterialExtractor {
    credentials: Credentials,
}

impl PengPaiNewsMaterialExtractor {
    pub fn from_deepseek() -> Self {
        Self {
            credentials: Credentials::from_env(),
        }
    }
}

#[async_trait::async_trait]
impl MaterialExtractor for PengPaiNewsMaterialExtractor {
    async fn get_material(&self, hot_news: &NewsTitle) -> NewsMaterialResult {
        let http = Client::new();
        let response = http.get(hot_news.url.clone()).send().await.unwrap();
        let raw_content = response.text().await.unwrap();

        let body = Self::get_body_inner_text(raw_content);

        let deepseek_result = self.ask_deepseek(&body).await;

        let mut pics = hot_news.pics.clone();
        pics.extend(deepseek_result.images);

        Ok(NewsMaterial {
            title: hot_news.title.clone(),
            summary: deepseek_result.summary.clone(),
            videos: hot_news.videos.clone(),
            pics,
        })
    }
}

#[derive(Debug, serde::Deserialize, Clone)]
struct DeepSeekResult {
    pub summary: Vec<String>,
    pub images: Vec<String>,
}

impl PengPaiNewsMaterialExtractor {
    fn get_body_inner_text(raw_content: String) -> String {
        use scraper::Html;
        let document = Html::parse_document(&raw_content);
        let body = Selector::parse("body").unwrap();

        let mut bodys = document.select(&body);
        let body = bodys.next().unwrap();
        body.inner_html()
    }

    async fn ask_deepseek(&self, news_content: &str) -> DeepSeekResult {
        let messages = vec![
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::System,
                content: Some(Self::get_prompt().to_string()),
                name: None,
                function_call: None,
                tool_call_id: None,
                tool_calls: None,
            },
            ChatCompletionMessage {
                role: ChatCompletionMessageRole::User,
                content: Some(news_content.to_string()),
                name: None,
                function_call: None,
                tool_call_id: None,
                tool_calls: None,
            },
        ];

        let chat_completion = ChatCompletion::builder("deepseek-chat", messages.clone())
            .credentials(self.credentials.clone())
            .create()
            .await
            .unwrap();

        let returned_message = chat_completion.choices.first().unwrap().message.clone();

        let raw_json = returned_message
            .content
            .unwrap()
            .trim_start_matches("```json")
            .trim_start_matches("```JSON")
            .trim_end_matches("```")
            .trim()
            .to_string();

        println!("raw_json: {}", raw_json);
        serde_json::from_str(&raw_json).unwrap()
    }

    fn get_prompt() -> &'static str {
        r#"
你是一个爆款短视频的作者，我会给你一个 HTML 格式的新闻稿，你要根据要求总结里面的新闻，并提取正文的图片，具体要求为：
1. 将新闻内容浓缩为200字内的短视频风格摘要，严格控制在200字以内，使用吸引眼球的短视频的风格夸张语气和俏皮。
风格夸张俏皮，喜欢使用网络热词和热梗，保持事实准确，突出核心事件、关键人物和戏剧性细节，纯文字输出，禁止使用表情符号，时间地点人物等关键信息必须准确，注意中文标点符号使用规范。正文要根据逗号、句号分割，放在数组内，如：

```json
["句子1", "句子2"]
```

2. 从新闻稿HTML中提取仅正文部分的图片链接（排除封面、视频缩略图、图标等非正文内容），并去除URL中的querystring参数。严格限定在正文内容区域，排除所有非正文图片（封面/视频缩略图/广告等），清除URL中?及后面的参数，结果以JSON数组格式返回，若无符合条件图片则返回空数组[]。如：

```json
[
        "https://imgpai.cn/newpai/image/175423202.jpg",
        "https://imgpai.cn/newpai/image/17540.jpg"
      ]
```

上面两点要求按照 JSON 格式输出，如：

{
  "summary": ["句子1", "句子2"],
  "images": ["图片1", "图片2"]
}
        "#
    }
}
