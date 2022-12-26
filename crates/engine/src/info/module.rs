use std::{sync::Arc, time::Duration};

use futures::lock::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard};

use mado_core::{ArcMadoModule, ArcMadoModuleMap, MadoModule, Uuid};

#[derive(Clone)]
pub enum LateBindingModule {
    Module(ArcMadoModule),
    WaitModule(ArcMadoModuleMap, Uuid),
}

pub const LATE_BINDING_MODULE_SLEEP_TIME: Duration = Duration::from_millis(100);

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

impl From<ArcMadoModule> for LateBindingModule {
    fn from(v: ArcMadoModule) -> Self {
        Self::Module(v)
    }
}

impl<T> From<Arc<T>> for LateBindingModule
where
    T: MadoModule,
{
    fn from(v: Arc<T>) -> Self {
        Self::Module(v)
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

                    crate::timer::sleep(LATE_BINDING_MODULE_SLEEP_TIME).await;
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
    pub async fn lock(&self) -> AsyncMutexGuard<'_, LateBindingModule> {
        self.module.lock().await
    }

    pub fn uuid(&self) -> &Uuid {
        &self.uuid
    }
}

impl From<LateBindingModule> for ModuleInfo {
    fn from(v: LateBindingModule) -> Self {
        Self {
            uuid: v.uuid(),
            module: AsyncMutex::new(v),
        }
    }
}

impl From<ArcMadoModule> for ModuleInfo {
    fn from(v: ArcMadoModule) -> Self {
        Self {
            uuid: v.uuid(),
            module: AsyncMutex::new(v.into()),
        }
    }
}

impl<T: MadoModule> From<Arc<T>> for ModuleInfo {
    fn from(v: Arc<T>) -> Self {
        Self::from(v as ArcMadoModule)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mado_core::{
        DefaultMadoModuleMap, MockMadoModule, MutMadoModuleMap, MutexMadoModuleMap, Url,
    };

    use super::*;
    #[test]
    pub fn wait_test() {
        let map = MutexMadoModuleMap::<DefaultMadoModuleMap>::new(Default::default());

        let map = Arc::new(map);
        let mut wait_module = LateBindingModule::WaitModule(map.clone(), Uuid::from_u128(1));

        let mut module = MockMadoModule::new();
        module.expect_uuid().return_const(Uuid::from_u128(1));
        module
            .expect_domain()
            .return_const(Url::try_from("http://localhost").unwrap());

        let module = Arc::new(module) as ArcMadoModule;
        map.push_mut(module.clone()).unwrap();

        futures::executor::block_on(async {
            let wait_module_a = wait_module.wait().await;

            assert_eq!(wait_module_a.uuid(), module.uuid());

            let wait_module_b = wait_module.wait().await;
            assert_eq!(wait_module_b.uuid(), module.uuid());
        });
    }

    #[test]
    pub fn timeout_test() {
        let map = MutexMadoModuleMap::<DefaultMadoModuleMap>::new(Default::default());

        let map = Arc::new(map);
        let mut wait_module = LateBindingModule::WaitModule(map.clone(), Uuid::from_u128(1));

        let mut module = MockMadoModule::new();
        module.expect_uuid().return_const(Uuid::from_u128(2));
        module
            .expect_domain()
            .return_const(Url::try_from("http://localhost").unwrap());

        let module = Arc::new(module) as ArcMadoModule;
        map.push_mut(module.clone()).unwrap();

        futures::executor::block_on(async {
            crate::timer::timeout(
                std::time::Duration::from_millis(
                    (LATE_BINDING_MODULE_SLEEP_TIME.as_millis() * 2)
                        .try_into()
                        .unwrap(),
                ),
                async {
                    wait_module.wait().await;
                    unreachable!();
                },
            )
            .await
            .expect_err("this should error because no same uuid");
        });
    }
}
