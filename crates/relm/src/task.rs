use std::sync::Arc;

use gtk::prelude::*;
use mado::engine::{
    DownloadInfo, DownloadInfoMsg, DownloadProgressStatus, DownloadResumedStatus, DownloadStatus,
};

crate::gobject::struct_wrapper!(
    GDownloadItem,
    crate::task::DownloadItem,
    "MadoRelmDownloadInfo",
    info_wrapper
);
pub use info_wrapper::GDownloadItem;

#[derive(Debug)]
pub struct DownloadItem {
    pub info: Arc<DownloadInfo>,
}

#[derive(Debug, Clone)]
pub struct DownloadView {
    pub widget: gtk::Box,
    title: gtk::Label,
    status: gtk::Label,
    chapter_progress: gtk::Label,
    chapter_title: gtk::Label,
    manga_progress: gtk::Label,
}

const DOWNLOAD_RESUMED_CSS: &str = "download-resumed";
const DOWNLOAD_PAUSED_CSS: &str = "download-paused";
const DOWNLOAD_ERROR_CSS: &str = "download-error";

impl From<&DownloadInfo> for DownloadView {
    fn from(info: &DownloadInfo) -> Self {
        relm4::view! {
            #[name = "widget"]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                append = &gtk::Box {
                    set_spacing: 5,
                    #[name = "title"]
                    append = &gtk::Label {
                        set_markup: &gtk::glib::markup_escape_text(info.manga()),
                        set_halign: gtk::Align::Start,
                    },

                    #[name = "manga_progress"]
                    append = &gtk::Label {
                        set_text: "",
                    }
                },

                append = &gtk::Box {
                    set_spacing: 5,
                    #[name = "status"]
                    append = &gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_text: &status_to_string(&info.status()),
                    },

                    #[name = "chapter_title"]
                    append = &gtk::Label {
                        set_halign:gtk::Align::Start,
                    },

                    #[name = "chapter_progress"]
                    append = &gtk::Label {
                        set_halign: gtk::Align::Start,
                    }
                },
            }
        }

        let style = format!(
            r#"
            .download-resumed {{
                color: {};
            }}
            .download-paused {{
                color: {};
            }}
            .download-error {{
                color: RED;
            }}
        "#,
            Self::get_label_color(gtk::StateFlags::NORMAL),
            Self::get_label_color(gtk::StateFlags::INSENSITIVE)
        );

        let css = gtk::CssProvider::new();
        css.load_from_data(style.as_bytes());

        widget
            .style_context()
            .add_provider(&css, gtk::STYLE_PROVIDER_PRIORITY_USER);
        widget.add_css_class(status_to_class(&info.status()));

        Self {
            widget,
            status,
            title,
            chapter_progress,
            chapter_title,
            manga_progress,
        }
    }
}

pub fn status_to_string(status: &DownloadStatus) -> String {
    match status {
        DownloadStatus::Finished => "Finished".to_string(),
        DownloadStatus::InProgress(progress) => match progress {
            DownloadProgressStatus::Resumed(v) => match v {
                DownloadResumedStatus::Waiting => "Waiting".to_string(),
                DownloadResumedStatus::Downloading => "Downloading".to_string(),
            },
            DownloadProgressStatus::Paused => "Paused".to_string(),
            DownloadProgressStatus::Error(err) => format!("Error: {}", err),
        },
    }
}

pub fn status_to_class(status: &DownloadStatus) -> &'static str {
    match status {
        DownloadStatus::Finished => DOWNLOAD_RESUMED_CSS,
        DownloadStatus::InProgress(progress) => match progress {
            DownloadProgressStatus::Resumed(_) => DOWNLOAD_RESUMED_CSS,
            DownloadProgressStatus::Paused => DOWNLOAD_PAUSED_CSS,
            DownloadProgressStatus::Error(_) => DOWNLOAD_ERROR_CSS,
        },
    }
}

impl DownloadView {
    fn get_label_color(state: gtk::StateFlags) -> gtk::gdk::RGBA {
        thread_local! {
            static WIDGET: gtk::Label = gtk::Label::default();
        }

        WIDGET.with(|widget| {
            let ctx = widget.style_context();
            let old = ctx.state();
            ctx.set_state(state);
            let color = ctx.color();
            ctx.set_state(old);

            color
        })
    }

    pub fn set_download_status(&self, status: &DownloadStatus) {
        let remove_css = |title| {
            self.widget.remove_css_class(title);
        };

        let add_css = |title| {
            self.widget.add_css_class(title);
        };

        let set_text = |text| {
            self.status.set_text(text);
        };

        remove_css(DOWNLOAD_RESUMED_CSS);
        remove_css(DOWNLOAD_PAUSED_CSS);
        remove_css(DOWNLOAD_ERROR_CSS);

        add_css(status_to_class(status));
        set_text(&status_to_string(status));
    }

    pub fn update_info(&self, info: &DownloadInfo) {
        let countfinished = info
            .chapters()
            .iter()
            .filter(|it| it.status().is_finished())
            .count();
        let total = info.chapters().len();

        self.manga_progress
            .set_text(&format!("[{countfinished}/{total}]"));
    }

    pub fn update_chapter_info(&self, info: &DownloadInfo) {
        let chapter = info.chapters().iter().find(|it| it.status().is_resumed());
        if let Some(chapter) = chapter {
            let countfinished = chapter
                .images()
                .iter()
                .filter(|it| it.status().is_finished())
                .count();
            let total = chapter.images().len();
            let title = chapter.title();

            self.chapter_title.set_text(title);
            self.chapter_progress
                .set_text(&format!("[{countfinished}/{total}]"));
        } else {
            self.chapter_title.set_text("");
            self.chapter_progress.set_text("");
        }
    }
}

#[derive(Debug, Clone)]
pub struct DownloadViewController {
    sender: gtk::glib::Sender<DownloadMsg>,
}

#[derive(Debug)]
pub enum DownloadMsg {
    StatusChanged,
    ChapterChanged,
}

impl DownloadViewController {
    pub fn connect(view: DownloadView, info: Arc<DownloadInfo>) -> Self {
        use gtk::glib;

        let (sender, recv) = gtk::glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let this = Self { sender };
        let mut handles = vec![];

        let sender = this.sender.clone();
        let handle = info.connect(move |msg| match msg {
            DownloadInfoMsg::StatusChanged(_) => sender.send(DownloadMsg::StatusChanged).unwrap(),
        });
        handles.push(handle.send_handle_any());

        for it in info.chapters().iter() {
            let sender = this.sender.clone();
            let handle = it.connect(move |msg| match msg {
                mado::engine::DownloadChapterInfoMsg::StatusChanged(_) => {
                    sender.send(DownloadMsg::ChapterChanged).unwrap()
                }
                mado::engine::DownloadChapterInfoMsg::DownloadImagesChanged(images) => {
                    for it in images.iter() {
                        let sender = sender.clone();
                        it.connect(move |msg| match msg {
                            mado::engine::DownloadChapterImageInfoMsg::StatusChanged(_) => {
                                sender.send(DownloadMsg::ChapterChanged).unwrap()
                            }
                        });
                    }
                }
            });

            handles.push(handle.send_handle_any());
        }

        let widget = view.widget.downgrade();
        let status = view.status.downgrade();
        let title = view.title.downgrade();
        let chapter_progress = view.chapter_progress.downgrade();
        let manga_progress = view.manga_progress.downgrade();
        let chapter_title = view.chapter_title.downgrade();

        let upgrade_view = move || {
            widget.upgrade().and_then(|widget| {
                title.upgrade().and_then(|title| {
                    chapter_title.upgrade().and_then(|chapter_title| {
                        chapter_progress.upgrade().and_then(|chapter_progress| {
                            manga_progress.upgrade().and_then(|manga_progress| {
                                status.upgrade().map(|status| DownloadView {
                                    widget,
                                    title,
                                    status,
                                    chapter_progress,
                                    manga_progress,
                                    chapter_title,
                                })
                            })
                        })
                    })
                })
            })
        };

        recv.attach(None, move |msg| {
            let widget = upgrade_view();

            let con = if let Some(view) = widget {
                match msg {
                    DownloadMsg::StatusChanged => {
                        view.set_download_status(&info.status());
                        view.update_info(&info);
                    }
                    DownloadMsg::ChapterChanged => {
                        view.update_info(&info);
                        view.update_chapter_info(&info);
                    }
                };

                true
            } else {
                for it in handles.iter() {
                    it.clone().disconnect();
                }
                false
            };

            gtk::glib::Continue(con)
        });

        this
    }
}

#[cfg(test)]
mod tests {
    use mado::engine::{DownloadChapterImageInfo, DownloadChapterInfo, LateBindingModule};
    use mado_core::{DefaultMadoModuleMap, Uuid};

    use super::*;
    use crate::tests::*;

    #[gtk::test]
    fn test_status() {
        macro_rules! assert_status {
            ($status:expr, $class:expr, $title:expr) => {{
                let item = $status;
                assert_eq!(status_to_class(&item), $class);
                assert_eq!(status_to_string(&item), $title);
            }};
        }

        assert_status!(DownloadStatus::Finished, DOWNLOAD_RESUMED_CSS, "Finished");
        assert_status!(DownloadStatus::waiting(), DOWNLOAD_RESUMED_CSS, "Waiting");
        assert_status!(
            DownloadStatus::downloading(),
            DOWNLOAD_RESUMED_CSS,
            "Downloading"
        );
        assert_status!(DownloadStatus::paused(), DOWNLOAD_PAUSED_CSS, "Paused");
        assert_status!(
            DownloadStatus::error("Error"),
            DOWNLOAD_ERROR_CSS,
            "Error: Error"
        );
    }

    #[gtk::test]
    fn test_view() {
        let map = Arc::new(DefaultMadoModuleMap::new());
        let latebinding = LateBindingModule::WaitModule(map, Uuid::from_u128(1));

        let title = "title".to_string();
        let chapter = Arc::new(DownloadChapterInfo::new(
            latebinding.clone(),
            "1".to_string(),
            title.clone(),
            "path".into(),
            DownloadStatus::Finished,
        ));
        let info = Arc::new(DownloadInfo::new(
            latebinding,
            title.clone(),
            vec![chapter.clone()],
            "path".into(),
            None,
            DownloadStatus::Finished,
        ));

        let view = DownloadView::from(info.as_ref());
        assert_eq!(view.title.text().to_string(), title);
        assert_eq!(view.status.text().as_str(), "Finished");

        let controller = DownloadViewController::connect(view.clone(), info.clone());

        for i in [
            DownloadStatus::waiting(),
            DownloadStatus::downloading(),
            DownloadStatus::error("error"),
            DownloadStatus::finished(),
        ] {
            info.set_status(i.clone());

            run_loop();

            assert_eq!(view.status.text().as_str(), status_to_string(&i));
        }

        info.set_status(DownloadStatus::downloading());
        run_loop();
        assert_eq!(view.manga_progress.text(), "[1/1]");
        assert_eq!(view.chapter_progress.text(), "");

        chapter.set_status(DownloadStatus::downloading());
        run_loop();
        assert_eq!(view.manga_progress.text(), "[0/1]");
        assert_eq!(view.chapter_title.text(), "title");
        assert_eq!(view.chapter_progress.text(), "[0/0]");

        chapter.set_images(vec![
            Arc::new(DownloadChapterImageInfo::new(
                Default::default(),
                Default::default(),
                DownloadStatus::downloading(),
            )),
            Arc::new(DownloadChapterImageInfo::new(
                Default::default(),
                Default::default(),
                DownloadStatus::downloading(),
            )),
        ]);
        run_loop();
        assert_eq!(view.chapter_progress.text(), "[0/2]");

        chapter.images()[0].set_status(DownloadStatus::finished());
        run_loop();
        assert_eq!(view.chapter_progress.text(), "[1/2]");

        // check that dropping view should stop controller receiver
        drop(view);

        info.set_status(DownloadStatus::waiting());
        run_loop();
        controller
            .sender
            .send(DownloadMsg::StatusChanged)
            .expect_err("should error");
    }
}
