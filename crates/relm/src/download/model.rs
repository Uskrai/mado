use std::sync::Arc;

use mado_engine::{DownloadInfo, DownloadSender};
use relm4::{ComponentUpdate, Model, Widgets};

pub enum DownloadMsg {
    CreateDownloadView(Arc<DownloadInfo>, DownloadSender),
}

pub struct DownloadModel {
    //
}

impl Model for DownloadModel {
    type Msg = DownloadMsg;

    type Widgets = DownloadWidgets;

    type Components = ();
}

impl<ParentModel> ComponentUpdate<ParentModel> for DownloadModel
where
    ParentModel: Model,
{
    fn init_model(_: &ParentModel) -> Self {
        Self {}
    }

    fn update(
        &mut self,
        msg: Self::Msg,
        _: &Self::Components,
        _: relm4::Sender<Self::Msg>,
        _: relm4::Sender<ParentModel::Msg>,
    ) {
        match msg {
            DownloadMsg::CreateDownloadView(download, _) => {
                //TODO
            }
        }
    }
    //
}

#[relm4_macros::widget(pub)]
impl<ParentModel> Widgets<DownloadModel, ParentModel> for DownloadWidgets
where
    ParentModel: Model,
{
    view! {
      gtk::Box {
        //
      }
    }
}
