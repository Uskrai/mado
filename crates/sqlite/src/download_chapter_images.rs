use mado_engine::path::Utf8PathBuf;

use crate::download_chapters::DownloadChapterPK;

#[derive(Debug)]
pub struct DownloadChapterImage {
    pub pk: DownloadChapterImagePK,
    pub title: String,
    pub chapter_id: String,
    pub path: Utf8PathBuf,
}

#[derive(Debug)]
pub struct DownloadChapterImagePK {
    pub id: i64,
    pub ch_pk: DownloadChapterPK,
}
