use std::{
    collections::HashMap,
    fmt::Debug,
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
};

use bytes::Bytes;

use crate::{ChapterImageInfo, DuplicateUUIDError, Error, MadoModuleMapError, MangaInfo, Uuid};

pub trait ChapterTask: Send {
    fn add(&mut self, image: ChapterImageInfo);
    fn get_chapter_id(&self) -> &str;
}

pub trait BytesStream: futures_core::stream::Stream<Item = Result<Bytes, Error>> + Send {}
impl<T> BytesStream for T where T: futures_core::stream::Stream<Item = Result<Bytes, Error>> + Send {}

#[async_trait::async_trait]
pub trait MadoModule: Send + Sync + Debug + 'static {
    /// Get UUID of module. this value should be const
    /// and should'nt be changed ever.
    fn get_uuid(&self) -> Uuid;

    /// Get module's user readable name.
    fn get_name(&self) -> &str;

    fn get_domain(&self) -> crate::url::Url;

    /// Get Manga information from `url`
    async fn get_info(&self, url: crate::url::Url) -> Result<MangaInfo, Error>;

    /// Get Image of Chapter from `task::get_chapter`
    /// for each image `task::add` should be called
    async fn get_chapter_images(&self, task: Box<dyn ChapterTask>) -> Result<(), Error>;

    async fn download_image(
        &self,
        image: ChapterImageInfo,
    ) -> Result<Pin<Box<dyn BytesStream>>, crate::Error>;
}

pub type ArcMadoModule = Arc<dyn MadoModule + Sync>;
pub type ArcMadoModuleMap = Arc<dyn MadoModuleMap + Sync>;

/// Collection of [`MadoModule`]
pub trait MadoModuleMap: Send + 'static {
    /// Get module corresponding to the [`MadoModule::get_domain`]
    ///
    /// `url` doesn't need to be domain. implementor should remove non-domain part from
    /// url first with [`remove_domain`] before attempting to search Module.
    fn get_by_url(&self, url: crate::url::Url) -> Option<ArcMadoModule>;

    /// Get module corresponsing to the [`MadoModule::get_uuid`]
    fn get_by_uuid(&self, uuid: Uuid) -> Option<ArcMadoModule>;

    /// Push module to collection that can be retreived later.
    ///
    /// This operation should preserve old module if Error happen.
    fn push(&mut self, module: ArcMadoModule) -> Result<(), MadoModuleMapError>;
}

pub fn remove_domain(url: &mut crate::url::Url) {
    url.set_path("");
    url.set_query(None);
    url.set_fragment(None);
    url.set_password(None).ok();
    url.set_username("").ok();
}

#[derive(Default, Debug)]
pub struct DefaultMadoModuleMap {
    domains: HashMap<crate::url::Url, ArcMadoModule>,
    uuids: HashMap<Uuid, ArcMadoModule>,
    vec: Vec<ArcMadoModule>,
}

impl DefaultMadoModuleMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a reference to the default mado module map's vec.
    pub fn vec(&self) -> &[Arc<dyn MadoModule + Sync>] {
        self.vec.as_ref()
    }
}

impl MadoModuleMap for DefaultMadoModuleMap {
    fn get_by_url(&self, mut url: crate::url::Url) -> Option<ArcMadoModule> {
        remove_domain(&mut url);
        self.domains.get(&url).cloned()
    }

    fn get_by_uuid(&self, uuid: Uuid) -> Option<ArcMadoModule> {
        self.uuids.get(&uuid).cloned()
    }

    fn push(&mut self, module: ArcMadoModule) -> std::result::Result<(), MadoModuleMapError> {
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
                self.vec.push(module);
                Ok(())
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct MutexMadoModuleMap<Map: MadoModuleMap> {
    map: Mutex<Map>,
}

impl<Map: MadoModuleMap> MutexMadoModuleMap<Map> {
    pub fn new(map: Map) -> Self {
        Self {
            map: Mutex::new(map),
        }
    }

    pub fn lock(&self) -> Result<MutexGuard<Map>, std::sync::PoisonError<MutexGuard<Map>>> {
        self.map.lock()
    }
}

impl<Map: MadoModuleMap> MadoModuleMap for MutexMadoModuleMap<Map> {
    fn push(&mut self, module: ArcMadoModule) -> Result<(), MadoModuleMapError> {
        self.push_mut(module)
    }

    fn get_by_url(&self, url: crate::url::Url) -> Option<ArcMadoModule> {
        self.map.lock().unwrap().get_by_url(url)
    }

    fn get_by_uuid(&self, uuid: Uuid) -> Option<ArcMadoModule> {
        self.map.lock().unwrap().get_by_uuid(uuid)
    }
}

/// Interior Mutable [`MadoModuleMap`]
pub trait MutMadoModuleMap: MadoModuleMap {
    fn push_mut(&self, module: ArcMadoModule) -> Result<(), MadoModuleMapError>;
}

impl<Map: MadoModuleMap> MutMadoModuleMap for MutexMadoModuleMap<Map> {
    fn push_mut(&self, module: ArcMadoModule) -> Result<(), MadoModuleMapError> {
        self.map.lock().unwrap().push(module)
    }
}

#[cfg(test)]
mod test {
    use std::{pin::Pin, sync::Arc};

    use crate::{BytesStream, DefaultMadoModuleMap, MadoModuleMap};

    #[derive(Clone, Debug)]
    pub struct MockMadoModule {
        uuid: crate::Uuid,
        url: crate::url::Url,
    }

    impl MockMadoModule {
        pub fn new(uuid: super::Uuid, url: crate::url::Url) -> Self {
            Self { uuid, url }
        }
    }

    #[async_trait::async_trait]
    impl super::MadoModule for MockMadoModule {
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

        fn get_name(&self) -> &str {
            "test"
        }

        async fn download_image(
            &self,
            _: crate::ChapterImageInfo,
        ) -> Result<Pin<Box<dyn BytesStream>>, crate::Error> {
            todo!()
        }
    }

    #[test]
    fn duplicate_insert() {
        let mut map = DefaultMadoModuleMap::default();
        let mock = Arc::new(MockMadoModule::new(
            super::Uuid::from_u128(123),
            crate::url::Url::parse("https://google.com").unwrap(),
        ));

        map.push(mock.clone()).unwrap();
        assert!(map.push(mock).is_err())
    }
}
