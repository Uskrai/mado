use std::{io::Write, path::PathBuf, sync::Arc};

use futures::{Future, StreamExt};
use mado_core::{ArcMadoModule, ChapterImageInfo, ChapterInfo};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub fn create(
    chapter: Arc<ChapterInfo>,
    module: ArcMadoModule,
) -> (ChapterTask, ChapterTaskReceiver) {
    let (sender, recv) = tokio::sync::mpsc::unbounded_channel();

    let task = ChapterTask {
        sender,
        chapter: chapter.clone(),
    };

    let receiver = ChapterTaskReceiver {
        module,
        recv,
        chapter,
    };

    (task, receiver)
}

#[derive(Debug)]
pub struct ChapterTask {
    sender: UnboundedSender<mado_core::ChapterImageInfo>,
    chapter: Arc<ChapterInfo>,
}

#[derive(Debug)]
pub struct ChapterTaskReceiver {
    module: ArcMadoModule,
    chapter: Arc<ChapterInfo>,
    recv: UnboundedReceiver<mado_core::ChapterImageInfo>,
}

impl ChapterTaskReceiver {
    pub async fn run(mut self) {
        tracing::trace!("Start downloading chapter {:?}", self.chapter);
        let title = self
            .chapter
            .title
            .clone()
            .unwrap_or_else(|| "0000".to_string());

        let chapter_path = PathBuf::from(title.clone());

        std::fs::create_dir_all(&chapter_path).unwrap();

        let mut i = 0;
        while let Some(image) = self.recv.recv().await {
            let mut path = chapter_path.join(i.to_string());
            path.set_extension(image.extension.clone());

            self.download_image(path, image).await;
            i += 1;
        }
        tracing::trace!("Finished downloading chapter {:?}", self.chapter);
    }

    fn download_image(
        &self,
        path: PathBuf,
        image: ChapterImageInfo,
    ) -> impl Future<Output = impl Send> {
        let module = self.module.clone();
        async move {
            tracing::trace!("Start downloading {} {:?}", path.display(), image);
            let mut stream = module.download_image(image.clone()).await.unwrap();
            let mut file = std::fs::File::create(path.clone()).unwrap();
            let mut retry = 0;

            while let Some(bytes) = stream.next().await {
                let bytes: Result<_, mado_core::Error> = bytes;
                match bytes {
                    Ok(bytes) => {
                        file.write_all(&bytes).unwrap();
                    }
                    Err(err) => {
                        tracing::error!(
                            "error downloading {} {:?} because:{}",
                            path.display(),
                            image,
                            err
                        );
                        match err {
                            mado_core::Error::RequestError { .. }
                            | mado_core::Error::UrlParseError { .. } => {
                                retry += 1;
                                if retry > 10 {
                                    break;
                                }
                                tracing::info!("Retry downloading {} {:?}", path.display(), image);
                                stream = module.download_image(image.clone()).await.unwrap();
                            }
                            mado_core::Error::UnsupportedUrl(_) => {
                                todo!();
                            }
                            mado_core::Error::ExternalError(_) => {
                                break;
                            }
                        }
                    }
                }
            }
            tracing::trace!("Finished downloading {} {:?}", path.display(), image);
        }
    }
}

impl mado_core::ChapterTask for ChapterTask {
    fn add(&mut self, image: mado_core::ChapterImageInfo) {
        tracing::trace!("Sending image info {:?}", image);
        self.sender.send(image).unwrap();
    }

    fn get_chapter(&self) -> &ChapterInfo {
        &self.chapter
    }
}
