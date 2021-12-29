use std::sync::Arc;

use mado_core::{url::Url, ArcMadoModule, Error, MangaInfo};
use mado_engine::{
    path::{Utf8Path, Utf8PathBuf},
    DownloadRequest, DownloadRequestStatus,
};

use crate::AbortOnDropHandle;

use super::*;
use crate::chapter_list::{ChapterListParentModel, VecChapters};
use relm4::{send, ComponentUpdate, Model};

use gtk::prelude::WidgetExt;

#[derive(Debug)]
pub enum MangaInfoMsg {
    Download,
    DownloadPathChanged(String),
    ShowError(mado_core::Error),
    /// Get info from string
    /// string should be convertible to URL
    GetInfo(String),
    Update(mado_core::MangaInfo),
    Clear,
}

pub trait MangaInfoParentMsg {
    fn download_request(request: DownloadRequest) -> Self;
}

pub trait MangaInfoParentModel
where
    Self: Model,
    Self::Msg: MangaInfoParentMsg,
{
    fn get_website_module_map(&self) -> ArcMadoModuleMap;
}

pub struct MangaInfoModel {
    modules: ArcMadoModuleMap,
    chapters: VecChapters,
    current_handle: Option<(ArcMadoModule, Url, AbortOnDropHandle<()>)>,
    manga_info: Option<Arc<MangaInfo>>,
    path: Utf8PathBuf,
}

impl ChapterListParentModel for MangaInfoModel {
    fn get_vec_chapter_info(&self) -> VecChapters {
        self.chapters.clone()
    }
}

impl Model for MangaInfoModel {
    type Msg = MangaInfoMsg;
    type Widgets = MangaInfoWidgets;
    type Components = MangaInfoComponents;
}

impl MangaInfoModel {
    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    fn get_module(&self, link: &str) -> Result<(Url, ArcMadoModule), Error> {
        let url = mado_core::url::fill_host(link)?;

        let module = self.modules.get_by_url(url.clone());

        match module {
            Some(module) => Ok((url, module)),
            None => Err(Error::UnsupportedUrl(link.to_string())),
        }
    }

    pub fn spawn_get_info(
        &mut self,
        components: &MangaInfoComponents,
        sender: relm4::Sender<Msg>,
        url: String,
    ) {
        let url = url.trim();

        // don't do anything when empty
        if url.is_empty() {
            return;
        }

        let result = self.get_module(url);

        let (url, module) = match result {
            Ok(item) => item,
            Err(err) => {
                return send!(sender, Msg::ShowError(err));
            }
        };

        components.set_url(url.as_str());

        // clear previous info
        send!(sender, Msg::Clear);

        let task = Self::get_info(module.clone(), url.clone(), sender);

        // reset current handle.
        // handle is automatically aborted when droped
        // so we just need to make it out of scope
        // by making it None first
        self.current_handle = None;
        // then we can spawn new task
        self.current_handle = Some((module, url, tokio::spawn(task).into()));
    }

    pub async fn get_info(module: ArcMadoModule, url: Url, sender: relm4::Sender<Msg>) {
        let manga = module.get_info(url).await;

        match manga {
            Ok(manga) => {
                send!(sender, Msg::Update(manga));
            }
            Err(err) => {
                send!(sender, Msg::ShowError(err));
            }
        }
    }
}

impl<T> ComponentUpdate<T> for MangaInfoModel
where
    T: MangaInfoParentModel,
    T::Msg: MangaInfoParentMsg,
{
    fn init_model(parent_model: &T) -> Self {
        Self {
            modules: parent_model.get_website_module_map(),
            chapters: Default::default(),
            current_handle: None,
            manga_info: None,
            path: Utf8PathBuf::from("downloads/"),
        }
    }

    fn update(
        &mut self,
        msg: Self::Msg,
        components: &Self::Components,
        sender: relm4::Sender<Self::Msg>,
        parent_sender: relm4::Sender<T::Msg>,
    ) {
        match msg {
            Msg::Download => {
                let (module, url) = match &self.current_handle {
                    Some((module, url, _)) => (module.clone(), url.clone()),
                    _ => {
                        return;
                    }
                };

                let manga_info = match &self.manga_info {
                    Some(info) => info.clone(),
                    None => {
                        return;
                    }
                };

                let mut selected = Vec::new();
                self.chapters.for_each_selected(|_, it| {
                    selected.push(it.clone());
                });

                if selected.is_empty() {
                    return;
                }

                let path = self.path.join(&manga_info.title);

                let request = DownloadRequest::new(
                    module,
                    manga_info,
                    selected,
                    path,
                    Some(url),
                    DownloadRequestStatus::Resume,
                );

                let msg = T::Msg::download_request(request);

                parent_sender.send(msg).unwrap();
            }
            Msg::GetInfo(url) => {
                self.spawn_get_info(components, sender, url);
            }
            Msg::Update(manga) => {
                let manga = Arc::new(manga);
                self.manga_info.replace(manga);
                let chapters = &self.manga_info.as_ref().unwrap().chapters;
                for it in chapters {
                    self.chapters.push(it.clone());
                }
            }
            Msg::DownloadPathChanged(path) => {
                self.path = path.into();
            }
            Msg::Clear => {
                self.chapters.clear();
                self.manga_info = None;
            }

            Msg::ShowError(error) => {
                gtk::MessageDialog::builder()
                    .message_type(gtk::MessageType::Error)
                    .text(&error.to_string())
                    .transient_for(&components.get_toplevel())
                    .build()
                    .show();
            }
        }
    }
}
