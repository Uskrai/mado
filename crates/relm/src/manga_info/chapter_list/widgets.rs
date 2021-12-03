use gtk::prelude::WidgetExt;
use relm4::{send, Model, Widgets};

use super::{ChapterListModel, ChapterListMsg};

fn create_selection_model(model: &ChapterListModel) -> gtk::MultiSelection {
    gtk::MultiSelection::new(Some(&model.chapters.views.inner))
}

/// Widget that show Chapter with checkbox
#[relm4_macros::widget(pub)]
impl<ParentModel: Model> Widgets<ChapterListModel, ParentModel> for ChapterListWidgets {
    view! {
        gtk::ScrolledWindow {
            set_vexpand : true,
            set_child = Some(&gtk::ListView) {
                set_factory = Some(&gtk::SignalListItemFactory) {
                    connect_setup(sender) => move |_, item| {
                        send!(sender, ChapterListMsg::Setup(item.clone().into()))
                    },

                    connect_bind(sender) => move |_, item| {
                        send!(sender, ChapterListMsg::Change(item.clone().into()))
                    }
                },
                set_single_click_activate: false,
                connect_activate(sender) => move |view, _| {
                    send!(sender, ChapterListMsg::Activate(view.clone()))
                },

                set_model: Some(&create_selection_model(model))
            },
        }
    }
}
