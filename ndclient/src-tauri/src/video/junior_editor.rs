use reqwest::Client;
use std::{path::PathBuf, time::Duration};
use tokio::fs;

use crate::{
    news::NewsMaterial,
    // resize_img::resize_to_9_16,
    video::{VideoEditor, VideoEditorError, VideoEditorResult},
};

const DEFAULT_TEMP_DIR: &str = "./temp";

pub struct JuniorEditor {
    temp_dir: &'static str,
    http: Client,
}

impl JuniorEditor {
    pub fn new() -> Self {
        Self {
            temp_dir: DEFAULT_TEMP_DIR,
            http: Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl VideoEditor for JuniorEditor {
    async fn do_edit(&self, material: &NewsMaterial, dur: Duration) -> VideoEditorResult<PathBuf> {
        let pic_urls = Self::get_need_pics(&material.pics, dur);
        let pic_files = self.save_pics(pic_urls).await?;
        // let resized_pics = self.resize_pics(&pic_files).await?;

        let path = self.compose_pics(&pic_files).await?.canonicalize()?;

        Ok(path)
    }
}

impl JuniorEditor {
    fn get_need_pics(pics: &Vec<String>, dur: Duration) -> Vec<String> {
        let mut pic_need = (dur.as_secs() / 2).max(1) as usize;
        if dur.subsec_millis() > 0 {
            pic_need += 1;
        }

        let pic_count = pics.len();

        let pics = if pic_need <= pic_count {
            pics[..pic_need].to_vec()
        } else {
            let (full_cycles, remainder) = (pic_need / pic_count, pic_need % pic_count);

            let mut list = Vec::with_capacity(pic_need);

            for _ in 0..full_cycles {
                list.extend_from_slice(pics);
            }

            if remainder > 0 {
                list.extend_from_slice(&pics[..remainder]);
            }

            list
        };

        pics
    }

    async fn save_pics(&self, pics: Vec<String>) -> VideoEditorResult<Vec<PathBuf>> {
        let id = nanoid::nanoid!(10);

        let mut pic_files = vec![];

        let path = PathBuf::from(format!("{}", self.temp_dir));
        fs::create_dir_all(&path).await?;
        let path = path.canonicalize().unwrap();

        for (i, pic_url) in pics.iter().enumerate() {
            let response = self
                .http
                .get(pic_url)
                .send()
                .await
                .map_err(|e| VideoEditorError::NetWork(e.to_string()))?;

            let format = {
                let content_type = response
                    .headers()
                    .get("content-type")
                    .ok_or(VideoEditorError::NetWork(
                        "cannot find image format".to_owned(),
                    ))?
                    .to_str()
                    .unwrap();

                content_type
                    .split('/')
                    .nth(1)
                    .ok_or(VideoEditorError::NetWork(
                        "malformed content-type".to_owned(),
                    ))?
                    .to_string()
            };

            let bytes = response
                .bytes()
                .await
                .map_err(|e| VideoEditorError::NetWork(e.to_string()))?;

            let file_path = path.join(format!("{}{:03}.{}", id, i, format));
            fs::write(&file_path, bytes).await?;

            pic_files.push(file_path);
        }

        Ok(pic_files)
    }

    // async fn resize_pics(&self, pics: &Vec<PathBuf>) -> VideoEditorResult<Vec<PathBuf>> {
    //     let mut resized_pics = vec![];

    //     for pic in pics {
    //         let old_name = pic.file_stem().unwrap().to_str().unwrap();
    //         let ext_name = pic.extension().unwrap().to_str().unwrap();

    //         let new_name = PathBuf::from(format!("{}916.{}", old_name, ext_name));

    //         let mut new_path = pic.clone();
    //         new_path.set_file_name(new_name);
    //         resize_to_9_16(pic, &new_path).map_err(|e| VideoEditorError::Image(e.to_string()))?;

    //         resized_pics.push(new_path);
    //     }

    //     Ok(resized_pics)
    // }

    async fn compose_pics(&self, pics: &Vec<PathBuf>) -> VideoEditorResult<PathBuf> {
        if pics.is_empty() {
            return Err(VideoEditorError::Image("no pics to compose".to_owned()));
        }

        let output_path = PathBuf::from(format!(
            "{}/{}-composed.mp4",
            self.temp_dir,
            nanoid::nanoid!(10)
        ));

        let file_list_path = self.build_file_list(pics).await?;

        let mut cmd = tokio::process::Command::new("ffmpeg");
        cmd.arg("-y")
            .arg("-f")
            .arg("concat")
            .arg("-i")
            .arg(&file_list_path)
            .arg("-vf")
            .arg("scale=720:1280:force_original_aspect_ratio=decrease,pad=720:1280:(ow-iw)/2:(oh-ih)/2:color=black")
            .arg("-c:v")
            .arg("libx264")
            .arg("-r")
            .arg("30")
            .arg("-pix_fmt")
            .arg("yuv420p")
            .arg(output_path.to_str().unwrap());

        cmd.output().await?;
        _ = fs::remove_file(file_list_path).await;

        for pic in pics {
            _ = fs::remove_file(pic).await;
        }
        Ok(output_path)
    }

    async fn build_file_list(&self, pics: &Vec<PathBuf>) -> VideoEditorResult<PathBuf> {
        if pics.is_empty() {
            return Err(VideoEditorError::Image("no pics to compose".to_owned()));
        }

        let file_list_path = PathBuf::from(format!(
            "{}/{}-file-list.txt",
            self.temp_dir,
            nanoid::nanoid!(10)
        ));

        let mut file_list_content = String::new();
        for pic in pics {
            file_list_content.push_str(&format!(
                "file '{}'\n",
                pic.file_name().unwrap().to_str().unwrap()
            ));
            file_list_content.push_str(&format!("duration 2\n\n"));
        }

        file_list_content.push_str(&format!(
            "file '{}'\n",
            pics.get(0).unwrap().file_name().unwrap().to_str().unwrap()
        ));
        file_list_content.push_str(&format!("duration 0\n\n"));

        fs::write(&file_list_path, file_list_content).await?;

        Ok(file_list_path.canonicalize()?)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn edit_video_need_three() {
        let material = NewsMaterial {
            title: "TITLE".to_owned(),
            summary: vec![],
            pics: vec!["pic1".to_owned(), "pic2".to_owned()],
            videos: vec![],
        };

        let res = JuniorEditor::get_need_pics(&material.pics, Duration::from_millis(5300));
        assert_eq!(res.len(), 3);

        let expected_pics = vec!["pic1".to_owned(), "pic2".to_owned(), "pic1".to_owned()];

        assert_eq!(expected_pics, res);
    }

    #[tokio::test]
    async fn edit_video_need_two() {
        let material = NewsMaterial {
            title: "TITLE".to_owned(),
            summary: vec![],
            pics: vec!["pic1".to_owned(), "pic2".to_owned()],
            videos: vec![],
        };

        let res = JuniorEditor::get_need_pics(&material.pics, Duration::from_millis(4000));
        assert_eq!(2, res.len());

        let expected_pics = vec!["pic1".to_owned(), "pic2".to_owned()];

        assert_eq!(expected_pics, res);
    }

    #[tokio::test]
    async fn edit_video_need_one() {
        let material = NewsMaterial {
            title: "TITLE".to_owned(),
            summary: vec![],
            pics: vec!["pic1".to_owned(), "pic2".to_owned()],
            videos: vec![],
        };

        let res = JuniorEditor::get_need_pics(&material.pics, Duration::from_millis(1000));
        assert_eq!(1, res.len());

        let expected_pics = vec!["pic1".to_owned()];

        assert_eq!(expected_pics, res);
    }

    #[tokio::test]
    async fn edit_video_need_many() {
        let material = NewsMaterial {
            title: "TITLE".to_owned(),
            summary: vec![],
            pics: vec![
                "pic1".to_owned(),
                "pic2".to_owned(),
                "pic3".to_owned(),
                "pic4".to_owned(),
                "pic5".to_owned(),
                "pic6".to_owned(),
                "pic7".to_owned(),
            ],
            videos: vec![],
        };

        let res = JuniorEditor::get_need_pics(&material.pics, Duration::from_millis(8_300));
        assert_eq!(5, res.len());

        let expected_pics = vec![
            "pic1".to_owned(),
            "pic2".to_owned(),
            "pic3".to_owned(),
            "pic4".to_owned(),
            "pic5".to_owned(),
        ];

        assert_eq!(expected_pics, res);
    }

    #[tokio::test]
    async fn save_http_pics_two_png() {
        let mock_response = vec![0x52, 0x49, 0x46, 0x46];

        let mut server = mockito::Server::new_async().await;

        let url = server.url();

        // Create a mock
        let _mock = server
            .mock("GET", "/images")
            .with_status(200)
            .with_header("content-type", "image/png")
            .with_body(mock_response)
            .create();

        let editor = JuniorEditor::new();

        let pics = vec![format!("{url}/images"), format!("{url}/images")];

        let paths = editor.save_pics(pics).await;
        assert!(paths.is_ok());

        let paths = paths.unwrap();

        for path in &paths {
            _ = fs::remove_file(path).await;
        }

        assert_eq!(2, paths.len());
    }

    #[tokio::test]
    async fn save_http_pics_mix_format() {
        let mock_response = vec![0x52, 0x49, 0x46, 0x46];

        let mut server = mockito::Server::new_async().await;

        let url = server.url();

        // Create a mock
        server
            .mock("GET", "/image_png")
            .with_status(200)
            .with_header("content-type", "image/png")
            .with_body(&mock_response)
            .create();

        // Create a mock
        server
            .mock("GET", "/image_jpg")
            .with_status(200)
            .with_header("content-type", "image/jpg")
            .with_body(&mock_response)
            .create();

        let editor = JuniorEditor::new();

        let pics = vec![format!("{url}/image_png"), format!("{url}/image_jpg")];

        let paths = editor.save_pics(pics).await;
        assert!(paths.is_ok());

        let paths = paths.unwrap();

        for path in &paths {
            _ = fs::remove_file(path).await;
        }

        assert_eq!(2, paths.len());
    }

    // #[tokio::test]
    // async fn resize_pic_to_9_16() {
    //     let pic_path = fs::canonicalize("./tests/mock_pic_1.jpeg").await.unwrap();

    //     let editor = JuniorEditor::new();
    //     let resized_pics = editor.resize_pics(&vec![pic_path]).await;

    //     assert!(resized_pics.is_ok());

    //     let resized_pics = resized_pics.unwrap();

    //     for pic in &resized_pics {
    //         _ = fs::remove_file(pic).await;
    //     }

    //     let mut expected_file = fs::canonicalize("./tests/").await.unwrap();
    //     expected_file.push("mock_pic_1916.jpeg");
    //     assert_eq!(vec![expected_file], resized_pics);
    // }
}
