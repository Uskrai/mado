use uuid::Uuid;

use crate::ArcMadoModule;

#[derive(thiserror::Error, Clone)]
#[error(
    "{} and {} have conflicting uuid: {}", previous.domain(), current.domain(), uuid
)]
pub struct DuplicateUUIDError {
    uuid: Uuid,
    previous: ArcMadoModule,
    current: ArcMadoModule,
}

impl DuplicateUUIDError {
    pub fn new(uuid: Uuid, previous: ArcMadoModule, current: ArcMadoModule) -> Self {
        Self {
            uuid,
            previous,
            current,
        }
    }
}

impl std::fmt::Debug for DuplicateUUIDError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DuplicateUUIDError")
            .field("uuid", &self.uuid)
            .field("previous_domain", &self.previous.domain())
            .field("current_domain", &self.current.domain())
            .finish()
    }
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum MadoModuleMapError {
    /// TODO: Change Url to Name
    #[error(transparent)]
    DuplicateUUID(#[from] DuplicateUUIDError),
}
