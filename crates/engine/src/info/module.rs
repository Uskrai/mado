use futures::lock::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard};

use mado_core::{ArcMadoModule, ArcMadoModuleMap, Uuid};

#[derive(Clone)]
pub enum LateBindingModule {
    Module(ArcMadoModule),
    WaitModule(ArcMadoModuleMap, Uuid),
}

impl std::fmt::Debug for LateBindingModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LateBindingModule::WaitModule(_, uuid) => f
                .debug_struct("LateBindingModule")
                .field("uuid", uuid)
                .finish(),
            LateBindingModule::Module(module) => f
                .debug_struct("LateBindingModule")
                .field("module", module)
                .finish(),
        }
    }
}

impl LateBindingModule {
    pub async fn wait(&mut self) -> ArcMadoModule {
        match self {
            LateBindingModule::Module(module) => module.clone(),
            LateBindingModule::WaitModule(map, uuid) => {
                let module = loop {
                    let module = map.get_by_uuid(*uuid);
                    if let Some(module) = module {
                        break module;
                    }

                    crate::timer::sleep(std::time::Duration::from_secs(1)).await;
                };

                *self = Self::Module(module.clone());
                module
            }
        }
    }

    pub fn uuid(&self) -> Uuid {
        match self {
            LateBindingModule::Module(module) => module.uuid(),
            LateBindingModule::WaitModule(_, uuid) => *uuid,
        }
    }
}

#[derive(Debug)]
pub struct ModuleInfo {
    uuid: Uuid,
    module: AsyncMutex<LateBindingModule>,
}

impl ModuleInfo {
    pub fn new(module: LateBindingModule) -> Self {
        let uuid = module.uuid();
        Self {
            uuid,
            module: AsyncMutex::new(module),
        }
    }

    pub async fn lock(&self) -> AsyncMutexGuard<'_, LateBindingModule> {
        self.module.lock().await
    }

    pub fn uuid(&self) -> &Uuid {
        &self.uuid
    }
}
