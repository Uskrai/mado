/*
 *  Copyright (c) 2021 Uskrai
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use relm4::RelmApp;

use std::sync::Arc;

#[tokio::main]
pub async fn main() {
  // let app = gtk::Application::builder().application_id("MADO").build();
  //
  // app.connect_activate(|app| {
  //   let window = gtk::ApplicationWindow::builder()
  //     .application(app)
  //     .title("MADO")
  //     .build();
  //
  //   let list = gio::ListStore::builder().build();]
  //   for it in 1..200 {
  //     list.append(&gtk::Label::builder().label(&it.to_string()).build());
  //   }
  //   let selection = gtk::MultiSelection::new(Some(&list));
  //   selection.connect_items_changed(|f, a, b, c| {
  //     // dbg!(f, a, b, c);
  //     // dbg!(a.name());
  //   });
  //
  //   let factory = gtk::SignalListItemFactory::new();
  //   factory.connect_bind(|_, item| {
  //     let it = item.item().unwrap();
  //     let it = it.downcast::<gtk::Label>().unwrap();
  //     item.set_child(Some(&it));
  //
  //     let action = gtk::ShortcutAction::parse_string("activate");
  //     let trigger = gtk::ShortcutTrigger::parse_string("space");
  //     let shortcut = gtk::Shortcut::new(trigger.as_ref(), action.as_ref());
  //
  //     let controller = gtk::ShortcutController::new();
  //     controller.add_shortcut(&shortcut);
  //
  //     it.add_controller(&controller);
  //
  //     // add_controller = &gtk::ShortcutController {
  //     //   set_name: "mine",
  //     //   add_shortcut = &gtk::Shortcut {
  //     //     set_action: Option::as_ref(&gtk::ShortcutAction::parse_string("action(list.activate-item)")),
  //     //     set_trigger: Option::as_ref(&gtk::ShortcutTrigger::parse_string("space")),
  //     //   }
  //     // }
  //   });
  //
  //   // let view = gtk::ListView::new(None, None);
  //   let view = gtk::ListView::builder()
  //     //   .model(&selection)
  //     //   .factory(&factory)
  //     .build();
  //
  //   // debug_controller(view.observe_controllers());
  //
  //   view.set_model(Some(&selection));
  //   view.set_factory(Some(&factory));
  //
  //   view
  //     .connect("activate", false, |f| {
  //       dbg!(f);
  //       None
  //     })
  //     .unwrap();
  //
  //   // debug_controller(view.observe_controllers());
  //
  //   let scrolled = gtk::ScrolledWindow::builder().child(&view).build();
  //
  //   window.set_child(Some(&scrolled));
  //   window.present()
  // });

  // app.run();

  let module = mado_rune::WebsiteModuleBuilder::default();
  let mut modules = mado_rune::WebsiteModuleMap::default();
  for it in std::fs::read_dir("../rune/script/module").unwrap() {
    let it = it.unwrap();
    if it.path().is_file() {
      for it in module.load_path(&it.path()).unwrap() {
        modules.insert(it);
      }
    }
  }

  let model = mado_relm::AppModel {
    modules: Arc::new(modules),
  };

  let app = RelmApp::new(model);
  app.run()
}
