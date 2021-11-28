use mado_core::WebsiteModuleMap;
use relm4::RelmApp;

use std::sync::Arc;

#[tokio::main]
pub async fn main() {
    let mut modules = mado_core::DefaultWebsiteModuleMap::default();

    let load_module = |path: &std::path::Path| -> Result<
    Vec<mado_rune::WebsiteModule>,
    Box<dyn std::error::Error>,
  > {
    mado_rune::Build::default()
      .with_path(path)?
      .build_for_module()?
      .error_missing_load_module(false)
      .build()
      .map_err(Into::into)
  };
    for it in std::fs::read_dir("../rune/script").unwrap() {
        let it = it.unwrap();
        if it.path().is_file() {
            let vec = load_module(&it.path());

            match vec {
                Ok(vec) => {
                    for it in vec {
                        modules.push(Arc::new(it.clone())).unwrap();
                    }
                }

                Err(err) => {
                    println!("{}", err);
                }
            }
        }
    }

    let model = mado_relm::AppModel::new(modules);

    let app = RelmApp::new(model);
    app.run()
}
