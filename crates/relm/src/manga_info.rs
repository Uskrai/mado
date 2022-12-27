use mado::core::ArcMadoModuleMap;

use gtk::prelude::*;
use mado::engine::DownloadOption;
use std::sync::Arc;

use crate::list_model::ListModelBaseExt;
use crate::list_store::ListStore;
use crate::AbortOnDropHandle;
use mado::core::{url::Url, ArcMadoModule, Error, MangaAndChaptersInfo};
use mado::engine::{path::Utf8PathBuf, DownloadRequest, DownloadRequestStatus};

use crate::chapter_list::{ChapterListModel, CheckChapterInfo};
use relm4::{Component, ComponentController, ComponentParts, ComponentSender, SimpleComponent};

#[derive(Debug)]
pub enum MangaInfoMsg {
    Download,
    DownloadPathChanged(String),
    Error(mado::core::Error),
    /// Get info from string
    /// string should be convertible to URL
    GetInfo {
        url: String,
        path: Option<String>,
    },
    Update {
        module: ArcMadoModule,
        url: Url,
        path: Option<String>,
        manga: MangaAndChaptersInfo,
    },
    Clear,
}

#[derive(Debug)]
pub enum MangaInfoOutput {
    DownloadRequest(DownloadRequest),
    Error(mado::core::Error),
}

pub struct MangaInfoModel {
    modules: ArcMadoModuleMap,
    option: DownloadOption,
    chapters: ListStore<CheckChapterInfo>,
    download_path: relm4::Controller<DownloadPathModel>,
    chapter_list: relm4::Controller<ChapterListModel>,
    manga_info: Option<(ArcMadoModule, Url, Arc<MangaAndChaptersInfo>)>,
    url: String,
    default_download_path: Utf8PathBuf,

    current_handle: Option<AbortOnDropHandle<()>>,
}

#[derive(Clone, Debug)]
pub enum DownloadPath {
    FromGetInfo(String),
    FromUser(String),
}

impl DownloadPath {
    pub fn join(&self, v: &str, option: &DownloadOption) -> Utf8PathBuf {
        match self {
            DownloadPath::FromGetInfo(path) => Utf8PathBuf::from(path),
            DownloadPath::FromUser(path) => {
                Utf8PathBuf::from(path).join(option.sanitize_filename(v))
            }
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            DownloadPath::FromGetInfo(v) => &v,
            DownloadPath::FromUser(v) => &v,
        }
    }
}

impl MangaInfoModel {
    pub fn manga_and_chapters(&self) -> Option<&MangaAndChaptersInfo> {
        self.manga_info.as_ref().map(|it| it.2.as_ref())
    }

    fn get_module(&self, link: &str) -> Result<(Url, ArcMadoModule), Error> {
        let url = mado::core::url::fill_host(link)?;

        let module = self.modules.get_by_url(url.clone());

        match module {
            Some(module) => Ok((url, module)),
            None => Err(Error::UnsupportedUrl(link.to_string())),
        }
    }

    pub fn spawn_get_info(
        &mut self,
        sender: ComponentSender<Self>,
        url: String,
        path: Option<String>,
    ) {
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

        let task = Self::get_info(module, url, path, sender);

        // reset current handle.
        // handle is automatically aborted when droped
        // so we just need to make it out of scope
        // by making it None first
        self.current_handle = None;
        // then we can spawn new task
        self.current_handle = Some(tokio::spawn(task).into());
    }

    pub fn create_download_request(&self) -> Option<DownloadRequest> {
        let (module, url, manga_info) = self.manga_info.as_ref()?;

        let mut selected = Vec::new();
        self.chapters.for_each(|it| {
            if it.active() {
                selected.push(it.info().clone());
            }
        });

        if selected.is_empty() {
            return None;
        }

        let path = self
            .download_path
            .model()
            .path
            .join(&manga_info.manga.title, &self.option);

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

    pub async fn get_info(
        module: ArcMadoModule,
        url: Url,
        path: Option<String>,
        sender: relm4::ComponentSender<Self>,
    ) {
        let manga = module.get_info(url.clone()).await;

        match manga {
            Ok(manga) => {
                sender.input(MangaInfoMsg::Update {
                    module,
                    url,
                    path,
                    manga,
                });
            }
            Err(err) => {
                sender.input(MangaInfoMsg::Error(err));
            }
        }
    }

    pub fn set_download_path(&self, path: DownloadPath) {
        self.download_path
            .widgets()
            .download_path
            .set_text(path.as_str());
        self.download_path
            .sender()
            .send(DownloadPathMsg::ChangeDownloadPath(path))
            .ok();
    }

    pub fn path(&self) -> DownloadPath {
        self.download_path.model().path.clone()
    }
}

pub struct MangaInfoInit {
    pub modules: ArcMadoModuleMap,
    pub default_download_path: Utf8PathBuf,
    pub option: DownloadOption,
}

#[relm4::component(pub)]
impl Component for MangaInfoModel {
    type Widgets = MangaInfoWidgets;
    type Init = MangaInfoInit;

    type Output = MangaInfoOutput;
    type Input = MangaInfoMsg;

    type CommandOutput = ();

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let chapters = ListStore::default();
        let Self::Init {
            modules,
            default_download_path,
            option,
        } = init;

        let chapter_list = ChapterListModel::builder().launch(chapters.base()).detach();
        let download_path = DownloadPathModel::builder()
            .launch(default_download_path.to_string())
            .forward(sender.input_sender(), |msg| match msg {
                DownloadPathOutput::Download => MangaInfoMsg::Download,
            });

        let model = Self {
            modules,
            option,
            chapters,
            chapter_list,
            current_handle: None,
            manga_info: None,
            url: "".to_string(),
            default_download_path,
            download_path,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _: &Self::Root) {
        match msg {
            MangaInfoMsg::Download => {
                let request = match self.create_download_request() {
                    Some(it) => it,
                    None => return,
                };

                sender
                    .output(MangaInfoOutput::DownloadRequest(request))
                    .ok();
            }
            MangaInfoMsg::GetInfo { url, path } => {
                self.spawn_get_info(sender, url, path);
            }
            MangaInfoMsg::Update {
                module,
                url,
                path,
                manga,
            } => {
                let manga = Arc::new(manga);
                self.manga_info.replace((module, url, manga.clone()));

                if let Some(path) = path {
                    let path = DownloadPath::FromGetInfo(path);
                    self.set_download_path(path);
                } else {
                    if !matches!(self.download_path.model().path, DownloadPath::FromUser(..)) {
                        let path = DownloadPath::FromUser(self.default_download_path.to_string());
                        self.set_download_path(path);
                    }
                }
                for it in manga.chapters.iter() {
                    self.chapters.push(CheckChapterInfo::new(it.clone(), false));
                }
            }
            MangaInfoMsg::DownloadPathChanged(path) => {
                self.set_download_path(DownloadPath::FromUser(path));
            }
            MangaInfoMsg::Clear => {
                self.chapters.clear();
                self.manga_info = None;
            }

            MangaInfoMsg::Error(error) => {
                sender.output(MangaInfoOutput::Error(error)).ok();
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
                        sender.input(MangaInfoMsg::GetInfo{ url: url_entry.text().to_string(), path: None });
                    }
                }
            },

            append = &gtk::Box {
                set_vexpand: true,
                set_hexpand: true,
                append: model.chapter_list.widget(),
            },

            append: model.download_path.widget(),
        }
    }
}

pub struct DownloadPathModel {
    path: DownloadPath,
}

#[derive(Debug)]
pub enum DownloadPathMsg {
    ChangeDownloadPath(DownloadPath),
}

#[derive(Debug)]
pub enum DownloadPathOutput {
    Download,
}

#[relm4::component(pub)]
impl SimpleComponent for DownloadPathModel {
    type Widgets = DownloadPathWidget;
    type Init = String;

    type Output = DownloadPathOutput;
    type Input = DownloadPathMsg;

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            path: DownloadPath::FromUser(init),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _: ComponentSender<Self>) {
        match message {
            DownloadPathMsg::ChangeDownloadPath(string) => {
                self.path = string;
            }
        }
    }

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,

            append: download_path = &gtk::Entry {
                set_hexpand: true,
                set_placeholder_text: Some("Enter Download Path"),
                connect_changed[sender] => move |path| {
                    let path = DownloadPath::FromUser(path.text().to_string());
                    sender.input(DownloadPathMsg::ChangeDownloadPath(path));
                }
            },

            append: download_button = &gtk::Button {
                set_label: "Download",
                connect_clicked[sender] => move |_| {
                    sender.output(DownloadPathOutput::Download).ok();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{list_model::ListModelBaseExt, tests::*};
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

        let default_download_path = Utf8PathBuf::from("downloads");

        let (tx, rx) = relm4::channel();
        let model = MangaInfoModel::builder()
            .launch(MangaInfoInit {
                modules: map.clone(),
                default_download_path: default_download_path.clone(),
                option: Default::default(),
            })
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
        let get_info_link = domain.join("test").unwrap();
        let info_ = info.clone();
        let (tx_waiter_get_info, rx_waiter_get_info) = relm4::channel();
        module
            .expect_get_info()
            .with(mockall::predicate::eq(get_info_link.clone()))
            .returning(move |_| {
                tx_waiter_get_info.send(());
                Ok(info_.clone())
            });

        // duplicate because cannot clone mado_core::Error
        let errrr = mado_core::Error::RequestError {
            url: "error".to_string(),
            message: "error".to_string(),
        };

        let get_info_error_link = domain.join("error").unwrap();
        let (tx_waiter_get_info_err, rx_waiter_get_info_err) = relm4::channel();
        module
            .expect_get_info()
            .with(mockall::predicate::eq(get_info_error_link.clone()))
            .returning(move |_| {
                tx_waiter_get_info_err.send(());
                Err(mado_core::Error::RequestError {
                    url: "error".to_string(),
                    message: "error".to_string(),
                })
            });

        let module: ArcMadoModule = Arc::new(module);
        map.push_mut(module.clone()).unwrap();
        {
            let path = Utf8PathBuf::from("download_path");
            model
                .model()
                .set_download_path(DownloadPath::FromUser(path.to_string()));

            run_loop();
            model.model().path();
            assert_eq!(model.model().path().as_str(), path.as_str());
        }

        // start of test Download
        {
            let path = Utf8PathBuf::from("set_path");
            model.emit(MangaInfoMsg::GetInfo {
                url: get_info_link.to_string(),
                path: Some("set_path".to_string()),
            });

            run_loop();

            model
                .model()
                .current_handle
                .as_ref()
                .expect("handle should exist");

            rt.block_on(rx_waiter_get_info.recv()).unwrap();

            run_loop();

            assert_eq!(model.model().path().as_str(), path.as_str());
            assert_eq!(
                model
                    .model()
                    .download_path
                    .widgets()
                    .download_path
                    .text()
                    .as_str(),
                model.model().path().as_str()
            );
            assert!(Arc::ptr_eq(
                &model.model().manga_and_chapters().unwrap().manga,
                &info.manga
            ));

            assert!(Arc::ptr_eq(
                &model.model().manga_and_chapters().unwrap().chapters,
                &info.chapters
            ));

            model
                .model()
                .download_path
                .widgets()
                .download_button
                .emit_clicked();

            run_loop();

            model.model().chapters.for_each(|info| {
                info.set_active(true);
            });

            model
                .model()
                .download_path
                .widgets()
                .download_button
                .emit_clicked();

            run_loop();

            let request = match rt.block_on(try_recv(&rx)).unwrap() {
                MangaInfoOutput::DownloadRequest(request) => request,
                _ => unreachable!(),
            };

            assert_eq!(request.path(), path);
            assert_eq!(request.url(), Some(&get_info_link));
            assert_eq!(request.chapters().len(), 1);
            assert_eq!(request.module().domain(), module.domain());

            run_loop();
        }
        // end of test Download

        // test DownloadPath::FromUser join with title
        {
            model.emit(MangaInfoMsg::GetInfo {
                url: get_info_link.to_string(),
                path: None,
            });
            run_loop();

            rt.block_on(rx_waiter_get_info.recv()).unwrap();

            run_loop();

            model.model().chapters.for_each(|info| {
                info.set_active(true);
            });

            model
                .model()
                .download_path
                .widgets()
                .download_button
                .emit_clicked();

            run_loop();

            let request = match rt.block_on(try_recv(&rx)).unwrap() {
                MangaInfoOutput::DownloadRequest(request) => request,
                _ => unreachable!(),
            };

            assert_eq!(request.path(), default_download_path.join("test title"));
            assert_eq!(request.url(), Some(&get_info_link));
            assert_eq!(request.chapters().len(), 1);
            assert_eq!(request.module().domain(), module.domain());
        }
        // end of test DownloadPath::FromUser join with title

        // start of test GetInfo Error
        {
            model.emit(MangaInfoMsg::GetInfo {
                url: get_info_error_link.to_string(),
                path: None,
            });

            run_loop();

            model
                .model()
                .current_handle
                .as_ref()
                .expect("handle should exist");

            rt.block_on(rx_waiter_get_info_err.recv()).unwrap();

            run_loop();

            let it = rt.block_on(try_recv(&rx)).unwrap();

            assert!(matches!(
                it,
                MangaInfoOutput::Error(mado_core::Error::RequestError { .. })
            ));

            match (it, errrr) {
                (
                    MangaInfoOutput::Error(mado_core::Error::RequestError { url, message }),
                    mado_core::Error::RequestError {
                        url: eurl,
                        message: emessage,
                    },
                ) => {
                    assert_eq!(url, eurl);
                    assert_eq!(message, emessage);
                }
                _ => unreachable!(),
            };
        }
    }
}
