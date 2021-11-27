mod error;
mod http_error;
#[allow(dead_code)]
mod manga;

use std::{collections::HashMap, sync::Arc};

pub use error::Error;
pub use manga::*;

pub mod url;

pub trait ChapterTask: Send {
  fn add(&mut self, name: Option<String>, id: String);
  fn get_chapter(&self) -> &ChapterInfo;
}

pub use uuid::Uuid;

#[async_trait::async_trait]
pub trait WebsiteModule: Send + Send + 'static {
  /// Get UUID of module. this value should be const
  /// and should'nt be changed ever.
  fn get_uuid(&self) -> Uuid;

  fn get_domain(&self) -> self::url::Url;

  /// Get Manga information from `url`
  async fn get_info(&self, url: self::url::Url) -> Result<MangaInfo, Error>;

  /// Get Image of Chapter from `task::get_chapter`
  /// for each image `task::add` should be called
  async fn get_chapter_images(
    &self,
    task: Box<dyn ChapterTask>,
  ) -> Result<(), Error>;
}

pub type ArcWebsiteModule = Arc<dyn WebsiteModule + Sync>;
pub type ArcWebsiteModuleMap = Arc<dyn WebsiteModuleMap + Sync>;

/// Collection of [`WebsiteModule`]
pub trait WebsiteModuleMap: Send + 'static {
  /// Get module corresponding to the [`WebsiteModule::get_domain`]
  ///
  /// `url` doesn't need to be domain. implementor should remove non-domain part from
  /// url first with [`remove_domain`] before attempting to search Module.
  fn get_by_url(&self, url: crate::url::Url) -> Option<ArcWebsiteModule>;

  /// Get module corresponsing to the [`WebsiteModule::get_uuid`]
  fn get_by_uuid(&self, uuid: Uuid) -> Option<ArcWebsiteModule>;

  /// Push module to collection that can be retreived later.
  ///
  /// this should preserve first element if there is duplicate.
  fn push(&mut self, module: ArcWebsiteModule);
}

pub fn remove_domain(url: &mut crate::url::Url) {
  url.set_path("");
  url.set_query(None);
  url.set_fragment(None);
  url.set_password(None).ok();
  url.set_username("").ok();
}

#[derive(Default)]
pub struct DefaultWebsiteModuleMap {
  domains: HashMap<crate::url::Url, ArcWebsiteModule>,
  uuids: HashMap<Uuid, ArcWebsiteModule>,
}

impl DefaultWebsiteModuleMap {
  pub fn new() -> Self {
    Self::default()
  }
}

impl WebsiteModuleMap for DefaultWebsiteModuleMap {
  fn get_by_url(&self, mut url: crate::url::Url) -> Option<ArcWebsiteModule> {
    remove_domain(&mut url);
    self.domains.get(&url).cloned()
  }

  fn get_by_uuid(&self, uuid: Uuid) -> Option<ArcWebsiteModule> {
    self.uuids.get(&uuid).cloned()
  }

  fn push(&mut self, module: ArcWebsiteModule) {
    let mut url = module.get_domain();
    remove_domain(&mut url);
    self.domains.insert(url, module.clone());
    self.uuids.insert(module.get_uuid(), module);
  }
}
