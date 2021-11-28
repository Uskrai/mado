use uuid::Uuid;

use crate::ArcWebsiteModule;

pub struct DuplicateUUIDError {
    uuid: Uuid,
    previous: ArcWebsiteModule,
    current: ArcWebsiteModule,
}

impl DuplicateUUIDError {
    pub fn new(uuid: Uuid, previous: ArcWebsiteModule, current: ArcWebsiteModule) -> Self {
        Self {
            uuid,
            previous,
            current,
        }
    }
}

impl std::error::Error for DuplicateUUIDError {}

impl std::fmt::Display for DuplicateUUIDError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::write!(
            f,
            "{} and {} have conflicting uuid: {}",
            self.previous.get_domain(),
            self.current.get_domain(),
            self.uuid
        )
    }
}

impl std::fmt::Debug for DuplicateUUIDError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DuplicateUUIDError")
            .field("uuid", &self.uuid)
            .field("previous_domain", &self.previous.get_domain())
            .field("current_domain", &self.current.get_domain())
            .finish()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum WebsiteModuleMapError {
    /// TODO: Change Url to Name
    #[error(transparent)]
    DuplicateUUID(#[from] DuplicateUUIDError),
}
