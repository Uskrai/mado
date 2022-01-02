use crate::{core::ChapterImageInfo, path::Utf8PathBuf};
#[derive(Debug)]
pub struct DownloadChapterImageInfo {
    image: ChapterImageInfo,
    path: Utf8PathBuf,
}

impl DownloadChapterImageInfo {
    pub fn new(image: ChapterImageInfo, path: Utf8PathBuf) -> Self {
        Self { image, path }
    }

    pub fn image(&self) -> &ChapterImageInfo {
        &self.image
    }

    pub fn path(&self) -> &crate::path::Utf8Path {
        &self.path
    }
}
