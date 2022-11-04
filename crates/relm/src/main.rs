use futures::{SinkExt, StreamExt};
use mado::core::{ArcMadoModule, DefaultMadoModuleMap, MutexMadoModuleMap};
use mado::engine::{
    path::Utf8PathBuf, MadoEngine, MadoEngineState, MadoModuleLoader, ModuleLoadError,
};
use mado_relm::AppModel;
use relm4::RelmApp;
use tracing_subscriber::{util::SubscriberInitExt, EnvFilter};

use std::sync::Arc;

pub enum LoaderMsg {
    Load(
        Utf8PathBuf,
        futures::channel::oneshot::Sender<Result<Vec<ArcMadoModule>, ModuleLoadError>>,
    ),
}
pub struct Loader(futures::channel::mpsc::Sender<LoaderMsg>);
#[async_trait::async_trait]
impl MadoModuleLoader for Loader {
    async fn get_paths(&self) -> Vec<Utf8PathBuf> {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../deno/dist/module");
        let mut dir = tokio::fs::read_dir(dir).await.unwrap();

        let mut paths = Vec::new();
        loop {
            let it = dir.next_entry().await;
            match it {
                Ok(Some(it)) => {
                    if it.path().is_file() {
                        let it = Utf8PathBuf::from_path_buf(it.path());
                        match it {
                            Ok(it) => paths.push(it),
                            Err(it) => tracing::error!("{:?} is not a valid utf8 path", it),
                        }
                    } else {
                        continue;
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    tracing::error!("error loading: {}", err);
                    continue;
                }
            };
        }

        paths
    }

    async fn load(
        &self,
        path: Utf8PathBuf,
    ) -> Result<Vec<mado::core::ArcMadoModule>, ModuleLoadError> {
        let (tx, rx) = futures::channel::oneshot::channel();

        self.0
            .clone()
            .send(LoaderMsg::Load(path, tx))
            .await
            .map_err(anyhow::Error::from)?;

        rx.await.map_err(anyhow::Error::from)?
    }
}

async fn handle_loader_msg(loader: &mut mado_deno::ModuleLoader, msg: LoaderMsg) {
    match msg {
        LoaderMsg::Load(path, rx) => {
            let fun = async {
                let num = loader.load_file(path.as_std_path()).await?;

                let mut vec = Vec::new();

                let object = loader.init_module(num).await.map_err(anyhow::Error::from)?;

                for (sender, looper) in object.into_iter().flatten() {
                    tokio::task::spawn_local(looper.start());

                    vec.push(Arc::new(sender) as ArcMadoModule);
                }

                Ok(vec)
            };

            let result = fun.await;

            rx.send(result).ok();
        }
    }
}

pub fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .init();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();

    let db = mado_sqlite::Database::open("data.db").unwrap();
    let channel = mado_sqlite::channel(db);

    let map = Arc::new(MutexMadoModuleMap::new(DefaultMadoModuleMap::new()));
    let downloads = channel.load_connect(map.clone()).unwrap();

    let state = MadoEngineState::new(map, downloads);
    channel.connect_only(&state);

    let mado = MadoEngine::new(state);
    let state = mado.state();

    let (loader_tx, mut loader_rx) = futures::channel::mpsc::channel(5);
    let deno_loader = Loader(loader_tx);

    let handle = runtime.handle().clone();

    std::thread::Builder::new()
        .name("deno-runtime".to_string())
        .spawn(move || {
            let handle = handle;
            let task = tokio::task::LocalSet::new();
            let deno_runtime = mado_deno::Runtime::default();
            let mut deno_loader = mado_deno::ModuleLoader::from_runtime(deno_runtime);

            task.spawn_local(async move {
                while let Some(msg) = loader_rx.next().await {
                    handle_loader_msg(&mut deno_loader, msg).await;
                }
            });

            handle.block_on(task);
        })
        .unwrap();

    tokio::spawn(mado.load_module(deno_loader));
    tokio::spawn(mado.run());

    let _guard = scopeguard::guard(channel.sender(), |sender| {
        sender.send(mado_sqlite::DbMsg::Close).unwrap();
    });
    runtime.spawn_blocking(|| {
        let mut channel = channel;
        channel.run().unwrap();
    });

    RelmApp::new("").run::<AppModel>(state);
}
