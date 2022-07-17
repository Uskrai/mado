use mado_core::MadoModule;
use std::{collections::HashMap, rc::Rc};
use tokio::task::LocalSet;

#[test]
pub fn main() {
    let tokio = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    tokio.block_on(async {
        let local = LocalSet::new();
        let options = deno_core::RuntimeOptions {
            module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
            extensions: mado_deno::extensions(),
            ..Default::default()
        };

        let mut runtime = mado_deno::ModuleLoader::new(options);

        let mut map = HashMap::new();
        println!("{:?}", std::env::current_dir());

        for it in std::fs::read_dir("./script").unwrap() {
            let it = it.unwrap();
            let path = it.path();

            if path.extension() == Some(std::ffi::OsStr::new("js")) {
                let module = runtime.load_file(&path).await.unwrap();

                let module = runtime.init_module(module).await;

                if let Ok(module) = module {
                    for it in module {
                        let (module, looper) = it.unwrap();

                        local.spawn_local(looper.start());
                        map.insert(module.name().to_string(), module);
                    }
                }
            }
        }

        map.clear();
        local.await;
    });
}
