use std::{collections::HashMap, sync::Arc};

use crate::{ChapterInfo, DuplicateUUIDError, Error, MangaInfo, Uuid, WebsiteModuleMapError};

pub trait ChapterTask: Send {
    fn add(&mut self, name: Option<String>, id: String);
    fn get_chapter(&self) -> &ChapterInfo;
}

#[async_trait::async_trait]
pub trait WebsiteModule: Send + Send + 'static {
    /// Get UUID of module. this value should be const
    /// and should'nt be changed ever.
    fn get_uuid(&self) -> Uuid;

    fn get_domain(&self) -> crate::url::Url;

    /// Get Manga information from `url`
    async fn get_info(&self, url: crate::url::Url) -> Result<MangaInfo, Error>;

    /// Get Image of Chapter from `task::get_chapter`
    /// for each image `task::add` should be called
    async fn get_chapter_images(&self, task: Box<dyn ChapterTask>) -> Result<(), Error>;
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
    /// This operation should preserve old module if Error happen.
    fn push(&mut self, module: ArcWebsiteModule) -> Result<(), WebsiteModuleMapError>;
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

    fn push(&mut self, module: ArcWebsiteModule) -> std::result::Result<(), WebsiteModuleMapError> {
        match self.uuids.insert(module.get_uuid(), module.clone()) {
            Some(prev) => {
                let error = DuplicateUUIDError::new(prev.get_uuid(), prev.clone(), module);
                let uuid = prev.get_uuid();
                // restore previous module first.
                self.uuids.insert(uuid, prev.clone());
                // then return err
                Err(error.into())
            }
            None => {
                let mut url = module.get_domain();
                remove_domain(&mut url);
                self.domains.insert(url, module.clone());
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::{DefaultWebsiteModuleMap, WebsiteModuleMap};

    #[derive(Clone)]
    pub struct MockWebsiteModule {
        uuid: crate::Uuid,
        url: crate::url::Url,
    }

    impl MockWebsiteModule {
        pub fn new(uuid: super::Uuid, url: crate::url::Url) -> Self {
            Self { uuid, url }
        }
    }

    #[async_trait::async_trait]
    impl super::WebsiteModule for MockWebsiteModule {
        fn get_uuid(&self) -> uuid::Uuid {
            self.uuid
        }

        fn get_domain(&self) -> crate::url::Url {
            self.url.clone()
        }

        async fn get_info(&self, _: crate::url::Url) -> Result<crate::MangaInfo, crate::Error> {
            todo!()
        }

        async fn get_chapter_images(
            &self,
            _: Box<dyn crate::ChapterTask>,
        ) -> Result<(), crate::Error> {
            todo!()
        }
    }

    #[test]
    fn duplicate_insert() {
        let mut map = DefaultWebsiteModuleMap::default();
        let mock = Arc::new(MockWebsiteModule::new(
            super::Uuid::from_u128(123),
            crate::url::Url::parse("https://google.com").unwrap(),
        ));

        map.push(mock.clone()).unwrap();
        assert!(map.push(mock).is_err())
    }
}
