use std::{fs, path::PathBuf, time::Duration};

use crate::subtitle::{SingleSubtitle, Subtitle, SubtitleResult};

const DEFAULT_TEMP_DIR: &str = "./temp";

pub struct SrtSubtitle {
    temp_dir: &'static str,
}

#[async_trait::async_trait]
impl Subtitle for SrtSubtitle {
    async fn write_subtitle(&self, subtitles: &Vec<SingleSubtitle>) -> SubtitleResult {
        let mut content = String::with_capacity(subtitles.len() * 13 * 10);

        let mut index = 1usize;
        let mut time = Duration::from_secs(0);
        let gap_time = Duration::from_millis(200);

        for subtitle in subtitles {
            let (single, end_time) =
                self.gen_single_subtitle(&subtitle.text, index, time, subtitle.duration);

            content.push_str(&single);
            time = end_time + gap_time;
            index += 1;
        }

        let path = PathBuf::from(self.temp_dir);
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        let mut path = path.canonicalize()?;

        path = path.join(format!("{}.srt", nanoid::nanoid!(10).to_string()));

        fs::write(&path, content)?;

        Ok(path)
    }
}

impl SrtSubtitle {
    pub fn new() -> Self {
        Self {
            temp_dir: DEFAULT_TEMP_DIR,
        }
    }

    fn gen_single_subtitle(
        &self,
        text: &str,
        index: usize,
        start: Duration,
        time: Duration,
    ) -> (String, Duration) {
        let mut content = String::new();

        content.push_str(&index.to_string());
        content.push('\n');

        let start_time = Self::to_subtitle_time_string(&start);
        content.push_str(&start_time);
        content.push_str(" --> ");

        let end_time = start + time;
        let end_time_str = Self::to_subtitle_time_string(&end_time);

        content.push_str(&end_time_str);
        content.push('\n');
        content.push_str(text);
        content.push_str("\n\n");

        (content, end_time)
    }

    fn to_subtitle_time_string(time: &Duration) -> String {
        let total_secs = time.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        let millis = time.subsec_millis();

        format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, time::Duration};

    use crate::subtitle::{SingleSubtitle, Subtitle, srt::SrtSubtitle};

    #[tokio::test]
    async fn write_subtitle_success() {
        let writer = SrtSubtitle::new();

        let list = vec![
            SingleSubtitle {
                text: "闺蜜闺蜜想不想玩第五人格喵喵喵".to_owned(),
                duration: Duration::from_secs(1),
            },
            SingleSubtitle {
                text: "兄弟兄弟想不想玩第五人格喵喵喵".to_owned(),
                duration: Duration::from_secs(1),
            },
            SingleSubtitle {
                text: "鸡块狗".to_owned(),
                duration: Duration::from_millis(200),
            },
        ];

        let path = writer.write_subtitle(&list).await;

        assert!(path.is_ok());

        let path = path.unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path);

        fs::remove_file(path).unwrap();

        assert!(content.is_ok());

        let content = content.unwrap();
        assert_eq!(
            content,
            r#"1
00:00:00,000 --> 00:00:01,000
闺蜜闺蜜想不想玩第五人格喵喵喵

2
00:00:01,200 --> 00:00:02,200
兄弟兄弟想不想玩第五人格喵喵喵

3
00:00:02,400 --> 00:00:02,600
鸡块狗

"#
        );
    }
}
