use crate::news::{MaterialExtractor, NewsCrawler};
use std::sync::Arc;

/// news source, where the news from and how handle it
pub struct NewsSource {
    pub crawler: Box<dyn NewsCrawler + Sync + Send + 'static>,
    pub extractor: ExtractorProxy,
}

#[derive(Clone)]
pub struct ExtractorProxy(pub Arc<Box<dyn MaterialExtractor + Sync + Send>>);

impl<T> From<T> for ExtractorProxy
where
    T: MaterialExtractor + Sync + Send + 'static,
{
    fn from(value: T) -> Self {
        Self(Arc::new(Box::new(value)))
    }
}
