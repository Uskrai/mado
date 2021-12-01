use std::sync::Arc;

use mado_core::{url::Url, ArcMadoModule, Error, MangaInfo};

use crate::AbortOnDropHandle;

use super::{
    chapter_list::{ChapterListParentModel, VecChapters},
    *,
};
use relm4::{send, ComponentUpdate, Model};

use gtk::prelude::WidgetExt;

#[derive(Debug)]
pub enum MangaInfoMsg {
    Download,
    ShowError(mado_core::Error),
    /// Get info from string
    /// string should be convertible to URL
    GetInfo(String),
    Update(mado_core::MangaInfo),
    Clear,
}

pub struct MangaInfoModel {
    modules: ArcMadoModuleMap,
    chapters: VecChapters,
    current_handle: Option<(ArcMadoModule, AbortOnDropHandle<()>)>,
    manga_info: Option<Arc<MangaInfo>>,
}

impl ChapterListParentModel for MangaInfoModel {
    fn get_vec_chapter_info(&self) -> chapter_list::VecChapters {
        self.chapters.clone()
    }
}

impl Model for MangaInfoModel {
    type Msg = MangaInfoMsg;
    type Widgets = MangaInfoWidgets;
    type Components = MangaInfoComponents;
}

impl MangaInfoModel {
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

        let task = Self::get_info(module.clone(), url, sender);

        // reset current handle.
        // handle is automatically aborted when droped
        // so we just need to make it out of scope
        // by making it None first
        self.current_handle = None;
        // then we can spawn new task
        self.current_handle = Some((module, tokio::spawn(task).into()));
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
    T: Model + MangaInfoParentModel,
{
    fn init_model(parent_model: &T) -> Self {
        Self {
            modules: parent_model.get_website_module_map(),
            chapters: Default::default(),
            current_handle: None,
            manga_info: None,
        }
    }

    fn update(
        &mut self,
        msg: Self::Msg,
        components: &Self::Components,
        sender: relm4::Sender<Self::Msg>,
        _parent_sender: relm4::Sender<T::Msg>,
    ) {
        match msg {
            Msg::Download => {
                let module = match &self.current_handle {
                    Some((module, _)) => module.clone(),
                    _ => {
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

                println!("{:#?}", selected);
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
            Msg::Clear => {
                self.chapters.clear();
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
