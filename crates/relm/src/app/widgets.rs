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

          append: stack = &gtk::Stack {
            // Download tab
            add_titled(Some("Download"), "Download") = &gtk::Box {
              set_orientation: gtk::Orientation::Vertical,
              append: components.download.root_widget()
            },
            // Manga Info tab
            add_titled(Some("Manga Info"), "Manga Info") = &gtk::Box {
              set_orientation: gtk::Orientation::Vertical,
              append: components.manga_info.root_widget()
            },
            set_visible_child_name: "Download",
          },

        }
      }
    }
}
