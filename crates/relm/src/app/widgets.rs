use gtk::prelude::*;
use mado_core::MadoModuleMap;
use relm4::Widgets;

use super::AppModel;

#[relm4_macros::widget(pub)]
impl<Map: MadoModuleMap> Widgets<AppModel<Map>, ()> for AppWidgets {
    view! {
      gtk::ApplicationWindow {
        set_title: Some("Mado"),
        set_child = Some(&gtk::Box) {
          set_orientation: gtk::Orientation::Vertical,

          append = &gtk::StackSwitcher {
            set_stack: Some(&stack)
          },

          // Download tab
          append: stack = &gtk::Stack {
            add_titled(Some("Download"), "Download") = &gtk::Frame {
              set_child = Some(&gtk::Box) {
                set_orientation: gtk::Orientation::Vertical,
                append = &gtk::Button {
                  set_label: "Yer download"
                }
              }
            },
            // Manga Info tab
            add_titled(Some("Manga Info"), "Manga Info") = &gtk::Box {
              set_orientation: gtk::Orientation::Vertical,
              append: component!(components.manga_info.root_widget())
            },
            set_visible_child_name: component!("Manga Info"),
          },

        }
      }
    }
}
