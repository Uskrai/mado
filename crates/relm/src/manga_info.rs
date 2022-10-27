use mado::core::ArcMadoModuleMap;

use gtk::prelude::*;
use std::sync::Arc;

use crate::AbortOnDropHandle;
use mado::core::{url::Url, ArcMadoModule, Error, MangaAndChaptersInfo};
use mado::engine::{
    path::{Utf8Path, Utf8PathBuf},
    DownloadRequest, DownloadRequestStatus,
};

use crate::chapter_list::ChapterListModel;
use crate::vec_chapters::VecChapters;
use relm4::{Component, ComponentController, ComponentParts, ComponentSender, SimpleComponent};

#[derive(Debug)]
pub enum MangaInfoMsg {
    Download,
    DownloadPathChanged(String),
    Error(mado::core::Error),
    /// Get info from string
    /// string should be convertible to URL
    GetInfo(String),
    Update(MangaAndChaptersInfo),
    Clear,
}

#[derive(Debug)]
pub enum MangaInfoOutput {
    DownloadRequest(DownloadRequest),
    Error(mado::core::Error),
}

pub struct MangaInfoModel {
    modules: ArcMadoModuleMap,
    chapters: VecChapters,
    chapter_list: relm4::Controller<ChapterListModel>,
    current_handle: Option<(ArcMadoModule, Url, AbortOnDropHandle<()>)>,
    manga_info: Option<Arc<MangaAndChaptersInfo>>,
    url: String,
    path: Utf8PathBuf,
}

impl MangaInfoModel {
    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    fn get_module(&self, link: &str) -> Result<(Url, ArcMadoModule), Error> {
        let url = mado::core::url::fill_host(link)?;

        let module = self.modules.get_by_url(url.clone());

        match module {
            Some(module) => Ok((url, module)),
            None => Err(Error::UnsupportedUrl(link.to_string())),
        }
    }

    pub fn spawn_get_info(&mut self, sender: ComponentSender<Self>, url: String) {
        self.url = url.to_string();

        let url = url.trim();

        // don't do anything when empty
        if url.is_empty() {
            return;
        }

        let result = self.get_module(url);

        let (url, module) = match result {
            Ok(item) => item,
            Err(err) => {
                return sender.input(MangaInfoMsg::Error(err));
            }
        };

        // components.set_url(url.as_str());

        // clear previous info
        sender.input(MangaInfoMsg::Clear);

        self.url = url.to_string();

        let task = Self::get_info(module.clone(), url.clone(), sender);

        // reset current handle.
        // handle is automatically aborted when droped
        // so we just need to make it out of scope
        // by making it None first
        self.current_handle = None;
        // then we can spawn new task
        self.current_handle = Some((module, url, tokio::spawn(task).into()));
    }

    pub fn create_download_request(&self) -> Option<DownloadRequest> {
        let (module, url, _) = self.current_handle.as_ref()?;

        let manga_info = self.manga_info.as_ref()?;

        let mut selected = Vec::new();
        self.chapters.for_each_selected(|_, it| {
            selected.push(it.clone());
        });

        if selected.is_empty() {
            return None;
        }

        let path = self.path.join(&manga_info.manga.title);

        let request = DownloadRequest::new(
            module.clone(),
            manga_info.manga.clone(),
            selected,
            path,
            Some(url.clone()),
            DownloadRequestStatus::Resume,
        );

        Some(request)
    }

    pub async fn get_info(module: ArcMadoModule, url: Url, sender: relm4::ComponentSender<Self>) {
        let manga = module.get_info(url).await;

        match manga {
            Ok(manga) => {
                sender.input(MangaInfoMsg::Update(manga));
            }
            Err(err) => {
                sender.input(MangaInfoMsg::Error(err));
            }
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for MangaInfoModel {
    type Widgets = MangaInfoWidgets;
    type Init = ArcMadoModuleMap;

    type Output = MangaInfoOutput;
    type Input = MangaInfoMsg;

    fn init(
        modules: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let chapters = VecChapters::default();

        let chapter_list = ChapterListModel::builder()
            .launch(chapters.clone())
            .detach();

        let model = Self {
            modules,
            chapters,
            chapter_list,
            current_handle: None,
            manga_info: None,
            url: "".to_string(),
            path: Utf8PathBuf::from("downloads/"),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            MangaInfoMsg::Download => {
                let request = match self.create_download_request() {
                    Some(it) => it,
                    None => return,
                };

                sender.output(MangaInfoOutput::DownloadRequest(request));
            }
            MangaInfoMsg::GetInfo(url) => {
                self.spawn_get_info(sender, url);
            }
            MangaInfoMsg::Update(manga) => {
                let manga = Arc::new(manga);
                self.manga_info.replace(manga);
                let chapters = &self.manga_info.as_ref().unwrap().chapters;
                for it in chapters.iter() {
                    self.chapters.push(it.clone());
                }
            }
            MangaInfoMsg::DownloadPathChanged(path) => {
                self.path = path.into();
            }
            MangaInfoMsg::Clear => {
                self.chapters.clear();
                self.manga_info = None;
            }

            MangaInfoMsg::Error(error) => {
                sender.output(MangaInfoOutput::Error(error));
            }
        }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            append = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                append : url_entry = &gtk::Entry {
                    // make the entry fill width
                    set_hexpand: true,
                    set_placeholder_text: Some("Enter Manga URL here"),
                    #[track = "model.url != url_entry.text()"]
                    set_text: &model.url,
                    // when user press enter, emit activate to enter button
                    // using emit_activate instead of emit_clicked because
                    // it doesn't animate the "press"
                    connect_activate[enter_button] => move |_| {
                        enter_button.emit_activate();
                    }
                },
                // enter button
                append : enter_button = &gtk::Button {
                    set_label: "⏎",
                    connect_clicked[sender,url_entry] => move |_| {
                        sender.input(MangaInfoMsg::GetInfo(url_entry.text().to_string()))
                    }
                }
            },

            append = &gtk::Box {
                set_vexpand: true,
                set_hexpand: true,
                append: model.chapter_list.widget(),
            },

            append = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,

                append: download_path = &gtk::Entry {
                    set_hexpand: true,
                    set_placeholder_text: Some("Enter Download Path"),
                    connect_changed[sender] => move |path| {
                        sender.input(MangaInfoMsg::DownloadPathChanged(path.text().to_string()));
                    }
                },

                append: download_button = &gtk::Button {
                    set_label: "Download",
                    connect_clicked[sender] => move |_| {
                        sender.input(MangaInfoMsg::Download);
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;
    use mado::core::{DefaultMadoModuleMap, MutexMadoModuleMap};
    use mado_core::{ChapterInfo, ChaptersInfo, MangaInfo, MutMadoModuleMap};

    #[gtk::test]
    fn test_test() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();

        let _g = rt.enter();

        let map = DefaultMadoModuleMap::new();
        let map = MutexMadoModuleMap::new(map);
        let map = Arc::new(map);

        let (tx, rx) = relm4::channel();
        let model = MangaInfoModel::builder()
            .launch(map.clone())
            .forward(&tx, |msg| msg);

        run_loop();

        {
            let link = "https".to_string();
            model.widgets().url_entry.set_text(&link);

            // url_entry.emit_activate doesn't do anything in test
            // so make sure to call emit_clicked too
            // and assert that it doesn't run twice below
            model.widgets().url_entry.emit_activate();
            model.widgets().enter_button.emit_clicked();

            run_loop();

            rt.block_on(async {
                assert!(matches!(
                    try_recv(&rx).await.unwrap(),
                    MangaInfoOutput::Error(mado::core::Error::UnsupportedUrl(..))
                ));

                try_recv(&rx).await.expect_err("should not exist");
            });

            assert_eq!(model.model().url, link);
        };

        let mut module = mado_core::MockMadoModule::default();
        let domain = mado_core::Url::parse("https://localhost").unwrap();
        module
            .expect_uuid()
            .return_const(mado_core::Uuid::from_u128(1));

        module.expect_domain().return_const(domain.clone());
        {
            let path = Utf8PathBuf::from("download_path");
            model.widgets().download_path.set_text(path.as_str());

            run_loop();

            assert_eq!(model.model().path(), path);

            let info = MangaAndChaptersInfo {
                manga: Arc::new(MangaInfo {
                    id: "test".to_string(),
                    title: "test title".to_string(),
                    ..Default::default()
                }),
                chapters: Arc::new(ChaptersInfo(vec![Arc::new(ChapterInfo {
                    index: Some(1),
                    id: "1".to_string(),
                    title: Some("ch title".to_string()),
                    ..Default::default()
                })])),
            };

            let (tx_waiter, rx_waiter) = relm4::channel();
            let link = domain.join("test").unwrap();
            let info_ = info.clone();
            let tx_waiter_get_info = tx_waiter;
            module
                .expect_get_info()
                .with(mockall::predicate::eq(link.clone()))
                .returning(move |_| {
                    tx_waiter_get_info.send(());
                    Ok(info_.clone())
                });

            let module: ArcMadoModule = Arc::new(module);
            map.push_mut(module.clone()).unwrap();

            model.emit(MangaInfoMsg::GetInfo(link.to_string()));

            run_loop();

            model
                .model()
                .current_handle
                .as_ref()
                .expect("handle should exist");

            rt.block_on(rx_waiter.recv()).unwrap();

            run_loop();

            assert!(Arc::ptr_eq(
                &model.model().manga_info.as_ref().unwrap().manga,
                &info.manga
            ));

            assert!(Arc::ptr_eq(
                &model.model().manga_info.as_ref().unwrap().chapters,
                &info.chapters
            ));

            model.widgets().download_button.emit_clicked();

            run_loop();

            model.model().chapters.for_each(|_, info| {
                info.borrow_mut().set_active(true);
            });

            model.widgets().download_button.emit_clicked();

            run_loop();

            let request = match rt.block_on(try_recv(&rx)).unwrap() {
                MangaInfoOutput::DownloadRequest(request) => request,
                _ => unreachable!(),
            };

            assert_eq!(request.path(), path.join("test title"));
            assert_eq!(request.url(), Some(&link));
            assert_eq!(request.chapters().len(), 1);
            assert_eq!(request.module().domain(), module.domain());
        }
    }
}
