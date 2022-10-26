use std::sync::Arc;

use gtk::{gio, prelude::*};
use mado::engine::{DownloadInfo, DownloadInfoMsg, DownloadStatus};

#[derive(Debug)]
pub struct DownloadItem {
    pub info: Arc<DownloadInfo>,
}

crate::gobject::struct_wrapper!(
    GDownloadItem,
    crate::download::task_list::DownloadItem,
    "MadoRelmDownloadInfo",
    info_wrapper
);
pub use info_wrapper::GDownloadItem;

use relm4::{Component, ComponentParts, ComponentSender, SimpleComponent};

#[derive(Debug)]
pub enum TaskListMsg {
    Setup(gtk::ListItem),
    Bind(gtk::ListItem),
}

pub trait TaskListParentModel: Component {
    fn get_list(&self) -> gio::ListStore;
}

#[derive(Clone)]
pub struct TaskListModel {
    tasks: gio::ListStore,
}

fn create_selection_model(model: &TaskListModel) -> gtk::MultiSelection {
    gtk::MultiSelection::new(Some(&model.tasks))
}

#[relm4::component(pub)]
impl SimpleComponent for TaskListModel {
    type Widgets = TaskListWidgets;

    type Init = gio::ListStore;
    type Input = TaskListMsg;
    type Output = ();

    fn init(
        tasks: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { tasks };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _: ComponentSender<Self>) {
        match msg {
            TaskListMsg::Setup(item) => {
                let download = item.item().unwrap().downcast::<GDownloadItem>().unwrap();
                let view = DownloadView::from(download.borrow().info.as_ref());
                let _ = DownloadViewController::connect(view.clone(), &mut download.borrow_mut());
                item.set_child(Some(&view.widget));
            }
            TaskListMsg::Bind(_) => {
                //
            }
        }
    }

    view! {
        gtk::ListView {
            set_model: Some(&create_selection_model(&model)),

            #[wrap(Some)]
            set_factory = &gtk::SignalListItemFactory {
                connect_setup[sender] => move |_,item| {
                    sender.input(TaskListMsg::Setup(item.clone()));
                },
                connect_bind[sender] => move |_, item| {
                    sender.input(TaskListMsg::Bind(item.clone()));
                },
            }
        }
    }
}

#[derive(Debug, Clone)]
struct DownloadView {
    widget: gtk::Box,
    title: gtk::Label,
    status: gtk::Label,
}

const DOWNLOAD_RESUMED_CSS: &str = "download-resumed";
const DOWNLOAD_PAUSED_CSS: &str = "download-paused";
const DOWNLOAD_ERROR_CSS: &str = "download-error";

impl From<&DownloadInfo> for DownloadView {
    fn from(info: &DownloadInfo) -> Self {
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 5);

        let title = gtk::Label::builder()
            .use_markup(true)
            .label(&format!(
                "<span size='large'>{}</span>",
                gtk::glib::markup_escape_text(info.manga())
            ))
            .halign(gtk::Align::Start)
            .build();

        let status = gtk::Label::builder().halign(gtk::Align::Start).build();

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

        let register_css = |w: &gtk::Label| {
            w.style_context()
                .add_provider(&css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        };

        register_css(&title);
        register_css(&status);

        widget.append(&title);
        widget.append(&status);

        Self {
            widget,
            title,
            status,
        }
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
            self.title.remove_css_class(title);
            self.status.remove_css_class(title);
        };

        let add_css = |title| {
            self.title.add_css_class(title);
            self.status.add_css_class(title);
        };

        let set_text = |text| {
            self.status.set_text(text);
        };

        remove_css(DOWNLOAD_RESUMED_CSS);
        remove_css(DOWNLOAD_PAUSED_CSS);
        remove_css(DOWNLOAD_ERROR_CSS);

        match status {
            DownloadStatus::Finished => {
                add_css(DOWNLOAD_RESUMED_CSS);
                set_text("Finished");
            }

            DownloadStatus::InProgress(progress) => match progress {
                mado::engine::DownloadProgressStatus::Resumed(v) => {
                    add_css(DOWNLOAD_RESUMED_CSS);
                    match v {
                        mado::engine::DownloadResumedStatus::Waiting => {
                            set_text("Waiting");
                        }
                        mado::engine::DownloadResumedStatus::Downloading => {
                            set_text("Downloading");
                        }
                    }
                }
                mado::engine::DownloadProgressStatus::Paused => {
                    add_css(DOWNLOAD_PAUSED_CSS);
                    set_text("Paused");
                }
                mado::engine::DownloadProgressStatus::Error(error) => {
                    add_css(DOWNLOAD_ERROR_CSS);
                    set_text(error);
                }
            },
        }
    }
}

#[derive(Debug, Clone)]
struct DownloadViewController {
    sender: gtk::glib::Sender<DownloadMsg>,
}

pub enum DownloadMsg {
    StatusChanged,
}

impl DownloadViewController {
    pub fn connect(view: DownloadView, download: &mut DownloadItem) -> Self {
        use gtk::glib;

        let (sender, recv) = gtk::glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let this = Self { sender };

        let info = download.info.clone();
        recv.attach(None, move |msg| {
            match msg {
                DownloadMsg::StatusChanged => {
                    view.set_download_status(&info.status());
                }
            }

            gtk::glib::Continue(true)
        });

        let sender = this.sender.clone();
        download.info.connect(move |msg| match msg {
            DownloadInfoMsg::StatusChanged(_) => sender.send(DownloadMsg::StatusChanged).unwrap(),
        });

        this
    }
}
