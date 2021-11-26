use mado_core::{url::Url, Error};
use mado_rune::WebsiteModule;
use tokio::task::JoinHandle;

use super::{
  chapter_list::{HasVecChapters, VecChapters},
  *,
};
use relm4::{send, ComponentUpdate, Model};

use gtk::prelude::WidgetExt;

#[derive(Debug)]
pub struct AbortOnDropHandle<R>(JoinHandle<R>);

impl<R> From<JoinHandle<R>> for AbortOnDropHandle<R> {
  fn from(v: JoinHandle<R>) -> Self {
    Self(v)
  }
}

impl<R> Drop for AbortOnDropHandle<R> {
  fn drop(&mut self) {
    println!("Drop");
    self.0.abort()
  }
}

pub struct MangaInfoModel {
  modules: Arc<WebsiteModuleMap>,
  chapters: VecChapters,
}

impl HasVecChapters for MangaInfoModel {
  fn get_vec_chapter_info(&self) -> chapter_list::VecChapters {
    self.chapters.clone()
  }
}

#[derive(Default, Debug)]
pub struct MangaInfoCell {
  current_info: Option<(Arc<WebsiteModule>, AbortOnDropHandle<()>)>,
}

impl Model for MangaInfoModel {
  type Msg = MangaInfoMsg;
  type Widgets = MangaInfoWidgets;
  type Components = MangaInfoComponents;
}

impl MangaInfoModel {
  fn get_module(&self, link: &str) -> Result<(Url, Arc<WebsiteModule>), Error> {
    let url = mado_core::url::fill_host(link)?;

    let module = self.modules.get(url.clone());

    match module {
      Some(module) => Ok((url, module)),
      None => Err(Error::UnsupportedUrl(link.to_string())),
    }
  }

  pub fn spawn_get_info(
    &self,
    components: &MangaInfoComponents,
    sender: relm4::Sender<Msg>,
    url: String,
  ) {
    let url = url.trim();

    // don't do anything when empty
    if url.is_empty() {
      return;
    }

    let result = self.get_module(url);

    let (url, module) = match result {
      Ok(item) => item,
      Err(err) => {
        return send!(sender, Msg::ShowError(err));
      }
    };

    components.set_url(url.as_str());

    // clear previous info
    send!(sender, Msg::Clear);

    let mut cell = components.get_cell_mut();

    let task = Self::get_info(module.clone(), url, sender);

    // reset current handle.
    // handle is automatically aborted when droped
    // so we just need to make it out of scope
    // by making it None first
    cell.current_info = None;
    // then we can spawn new task
    cell.current_info = Some((module, tokio::spawn(task).into()));
  }

  pub async fn get_info(
    module: Arc<WebsiteModule>,
    url: Url,
    sender: relm4::Sender<Msg>,
  ) {
    use mado_core::WebsiteModule;
    let manga = module.get_info(url).await;

    match manga {
      Ok(manga) => {
        send!(sender, Msg::Update(manga));
      }
      Err(err) => {
        send!(sender, Msg::ShowError(err));
      }
    }
  }
}

impl<T> ComponentUpdate<T> for MangaInfoModel
where
  T: Model + MangaInfoParentModel,
{
  fn init_model(parent_model: &T) -> Self {
    Self {
      modules: parent_model.get_website_module_map(),
      chapters: Default::default(),
    }
  }

  fn update(
    &mut self,
    msg: Self::Msg,
    components: &Self::Components,
    sender: relm4::Sender<Self::Msg>,
    _parent_sender: relm4::Sender<T::Msg>,
  ) {
    match msg {
      Msg::Download => {
        let module = match &components.get_cell_mut().current_info {
          Some((module, _)) => module.clone(),
          _ => {
            return;
          }
        };

        let mut ids = Vec::new();
        self.chapters.for_each_selected(|i, it| {
          ids.push(it.id.clone());
        });

        tokio::spawn(async move {
          // module.ge
          // module.get(ids).await;
        });
      }
      Msg::GetInfo(url) => {
        self.spawn_get_info(components, sender, url);
      }
      Msg::Update(manga) => {
        for it in manga.chapters {
          self.chapters.push(it);
        }
      }
      Msg::Clear => {
        self.chapters.clear();
      }

      Msg::ShowError(error) => {
        gtk::MessageDialog::builder()
          .message_type(gtk::MessageType::Error)
          .text(&error.to_string())
          .transient_for(&components.get_toplevel())
          .build()
          .show();
      }
    }
  }
}
