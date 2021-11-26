mod error;
mod http_error;
#[allow(dead_code)]
mod manga;

pub use error::Error;
pub use manga::*;

pub mod url;

pub trait ChapterTask: Send {
  fn add(&mut self, name: Option<String>, id: String);
  fn get_chapter(&self) -> &ChapterInfo;
}

pub use uuid::Uuid;

#[async_trait::async_trait]
pub trait WebsiteModule: Send {
  /// Get UUID of module. this value should be const
  /// and should'nt be changed ever.
  fn get_uuid(&self) -> Uuid;

  /// Get Manga information from `url`
  async fn get_info(&self, url: self::url::Url) -> Result<MangaInfo, Error>;

  /// Get Image of Chapter from `task::get_chapter`
  /// for each image `task::add` should be called
  async fn get_chapter_images(
    &self,
    task: Box<dyn ChapterTask>,
  ) -> Result<(), Error>;
}
