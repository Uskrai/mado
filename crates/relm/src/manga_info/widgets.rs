use super::*;
use gtk::prelude::*;
use relm4::{send, Model, Widgets};

#[relm4_macros::widget(pub)]
impl<T: Model> Widgets<MangaInfoModel, T> for MangaInfoWidgets {
    view! {
      gtk::Box {
        set_orientation: gtk::Orientation::Vertical,
        append = &gtk::Box {
          set_orientation: gtk::Orientation::Horizontal,
          append : url_entry = &gtk::Entry {
            // make the entry fill width
            set_hexpand: true,
            set_placeholder_text: Some("Enter Manga URL here"),
            // when user press enter, emit activate to enter button
            // using emit_activate instead of emit_clicked because
            // it doesn't animate the "press"
            connect_activate(enter_button) => move |_| {
              enter_button.emit_activate();
            }
          },
          // enter button
          append : enter_button = &gtk::Button {
            set_label: "⏎",
            connect_clicked(sender,url_entry) => move |_| {
              send!(sender, Msg::GetInfo(url_entry.text().to_string()))
            }
          }
        },

        append = &gtk::Box {
            set_vexpand: true,
            set_hexpand: true,
            append: components.chapters.root_widget(),
        },

        append = &gtk::Box {
          set_orientation: gtk::Orientation::Horizontal,

          append: download_path = &gtk::Entry {
            set_hexpand: true,
            set_placeholder_text: Some("Enter Download Path"),
          },

          append: download_button = &gtk::Button {
            set_label: "Download",
            connect_clicked(sender) => move |_| {
              send!(sender, Msg::Download);
            }
          }
        }
        //
      }
    }
}
