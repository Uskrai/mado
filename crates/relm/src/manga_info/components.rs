use std::cell::{RefCell, RefMut};

use crate::manga_info::chapter_list::ChapterListMsg;
use relm4::{send, Components, RelmComponent, Sender};

use super::{chapter_list::ChapterListModel, model::MangaInfoCell, *};

use gtk::prelude::*;

pub struct MangaInfoComponents {
  pub(super) chapters: RelmComponent<ChapterListModel, MangaInfoModel>,
  url_entry: gtk::Entry,
  cell: RefCell<MangaInfoCell>,
}

impl MangaInfoComponents {
  pub fn get_cell_mut(&self) -> RefMut<MangaInfoCell> {
    self.cell.borrow_mut()
  }
}

impl Components<MangaInfoModel> for MangaInfoComponents {
  fn init_components(
    parent: &MangaInfoModel,
    widget: &MangaInfoWidgets,
    sender: relm4::Sender<Msg>,
  ) -> Self {
    Self {
      url_entry: widget.url_entry.clone(),
      chapters: RelmComponent::new(parent, widget, sender),
      cell: RefCell::default(),
    }
  }
}

impl MangaInfoComponents {
  pub fn get_toplevel(&self) -> gtk::Window {
    crate::get_toplevel(self.url_entry.clone().upcast())
  }

  pub fn get_url(&self) -> String {
    self.url_entry.text().to_string()
  }

  pub fn set_url(&self, url: &str) {
    self.url_entry.set_text(url);
  }

  pub fn chapter_sender(&self) -> Sender<ChapterListMsg> {
    self.chapters.sender()
  }

  pub fn send_chapter(&self, msg: ChapterListMsg) {
    let sender = self.chapter_sender();
    send!(sender, msg);
  }
}
