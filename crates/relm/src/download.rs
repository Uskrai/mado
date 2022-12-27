use std::sync::Arc;

use gtk::prelude::*;
use mado::engine::path::Utf8PathBuf;
use mado::engine::DownloadInfo;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
};

use crate::list_model::{ListModel, ListModelBaseExt};
use crate::list_store::{ListStore, ListStoreIndex};
use crate::task::DownloadItem;
use crate::task_list::TaskListModel;

#[derive(Debug)]
pub enum DownloadMsg {
    CreateDownloadView(Arc<DownloadInfo>),
    OrderChanged(ListStoreIndex),
    PauseSelected,
    ResumeSelected,
    MoveUp,
    MoveDown,
    OpenMangaSelected,
}

#[derive(Debug)]
pub enum DownloadOutputMsg {
    OpenManga {
        url: mado_core::Url,
        path: Utf8PathBuf,
    },
}

#[derive(Copy, Clone)]
pub enum DownloadMoveDirection {
    Up,
    Down,
}

pub struct DownloadModel {
    list: ListStore<DownloadItem>,
    task_list: Controller<TaskListModel>,
}

impl DownloadModel {
    pub fn resume(&mut self, resume: bool) {
        let selection = &self.task_list.model().selection;

        let model = match selection.model() {
            Some(model) => model,
            None => return,
        };

        let selection = selection.selection();
        for (index, it) in model.into_iter().enumerate() {
            let it = it.unwrap();

            if selection.contains(index as u32) {
                if let Some(it) = self.list.get_by_object(&it) {
                    it.info().resume(resume);
                }
            }
        }
    }

    pub fn move_selected(&mut self, direction: DownloadMoveDirection) {
        let selection = &self.task_list.model().selection;

        let bitset = selection.selection().copy();

        let maximum: u64 = bitset.maximum().try_into().unwrap();
        let minimum: u64 = bitset.minimum().try_into().unwrap();
        let size = bitset.size();

        let model = match selection.model() {
            Some(model) => model,
            None => return,
        };

        let mut count = 0;

        use DownloadMoveDirection::*;

        let move_by = 1;
        let (minimum, maximum) = match direction {
            Up => (minimum.saturating_sub(move_by), maximum),
            Down => (minimum, maximum + move_by),
        };

        fn up_selection(bitset: &gtk::Bitset, index: u32) -> bool {
            bitset.contains(index)
        }

        fn down_selection(bitset: &gtk::Bitset, index: u32) -> bool {
            !bitset.contains(index)
        }

        let check_contains = match direction {
            Up => up_selection,
            Down => down_selection,
        };

        // check if
        let is_first = |index: u32| check_contains(&bitset, index);

        for (index, it) in model.into_iter().enumerate() {
            let index = index as u64;
            let it = it.unwrap();

            let it = match self.list.get_by_object(&it) {
                Some(it) => it,
                None => continue,
            };

            if index < minimum || index > maximum {
                continue;
            }

            let index = index as u32;

            if is_first(index) {
                let order = minimum + count;
                it.info().set_order((order).try_into().unwrap());
                count += 1;
            } else {
                let order = minimum + size + count;
                it.info().set_order((order).try_into().unwrap());
            }
        }

        let count: u32 = count.try_into().unwrap();
        let minimum: u32 = minimum.try_into().unwrap();
        selection.items_changed(minimum, count, count);
    }

    pub fn open_manga_selected_static<F, O>(
        selection: &gtk::MultiSelection,
        list: &ListStore<DownloadItem>,
        mut f: F,
    ) -> Option<O>
    where
        F: FnMut(&DownloadItem) -> Option<O>,
    {
        let selected = selection.iter::<gtk::glib::Object>();
        let bitset = selection.selection();

        if let Ok(iter) = selected {
            for (index, it) in iter.enumerate() {
                let it = it.unwrap();

                if !bitset.contains(index as u32) {
                    continue;
                }

                let it = match list.get_by_object(&it) {
                    Some(it) => it,
                    _ => continue,
                };

                if let Some(it) = f(&it) {
                    return Some(it);
                }
            }
        }

        None
    }

    pub fn open_manga_selected(&self) -> Option<DownloadOutputMsg> {
        Self::open_manga_selected_static(&self.task_list.model().selection, &self.list, |it| {
            let url = it.info().url().cloned()?;
            let path = it.info().path().clone();

            return Some(DownloadOutputMsg::OpenManga { url, path });
        })
    }

    pub fn is_selected_only(&self, size: u64) -> bool {
        self.task_list.model().selection.selection().size() == size
    }
}

#[relm4::component(pub)]
impl SimpleComponent for DownloadModel {
    type Widgets = DownloadWidgets;
    // type Components = DownloadComponents;

    type Init = ();

    type Input = DownloadMsg;
    type Output = DownloadOutputMsg;

    fn init(
        _: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list = ListStore::<DownloadItem>::default();
        let model = ListModel::new_with_model(list.clone(), |model, gtkmodel| {
            let sorter = model
                .custom_sorter(|first, second| first.info().order().cmp(&second.info().order()));

            gtk::SortListModel::new(Some(&gtkmodel), Some(&sorter)).into()
        });

        let task_list = TaskListModel::builder().launch(model).detach();

        let model = Self { list, task_list };
        let widgets = view_output!();

        let open_manga = widgets.open_manga.clone();

        let list = model.list.clone();
        model
            .task_list
            .model()
            .selection
            .connect_selection_changed(move |selection, _, _| {
                let is_sensitive = selection.selection().size() == 1
                    && Self::open_manga_selected_static(selection, &list, |it| {
                        Some(it.info().url().is_some())
                    })
                    .unwrap_or(false);

                open_manga.set_sensitive(is_sensitive);
            });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DownloadMsg::CreateDownloadView(info) => {
                let index = self.list.push(DownloadItem::new(info.clone()));

                info.connect_only(move |msg| match msg {
                    mado::engine::DownloadInfoMsg::StatusChanged(_) => {}
                    mado::engine::DownloadInfoMsg::OrderChanged(_) => {
                        sender.input(DownloadMsg::OrderChanged(index.clone()));
                    }
                });
            }
            DownloadMsg::OrderChanged(index) => {
                self.list.notify_changed(&index);
            }
            DownloadMsg::PauseSelected => {
                self.resume(false);
            }
            DownloadMsg::ResumeSelected => {
                self.resume(true);
            }
            DownloadMsg::MoveUp => {
                self.move_selected(DownloadMoveDirection::Up);
            }
            DownloadMsg::MoveDown => {
                self.move_selected(DownloadMoveDirection::Down);
            }
            DownloadMsg::OpenMangaSelected => {
                if let Some(msg) = self.open_manga_selected() {
                    sender.output(msg).ok();
                }
            }
        }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            append = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,

                #[name = "resume_button"]
                append = &gtk::Button {
                    set_label: "Resume",
                    connect_clicked[sender] => move |_| {
                        sender.input(DownloadMsg::ResumeSelected);
                    }
                },

                #[name = "pause_button"]
                append = &gtk::Button {
                    set_label: "Pause",
                    connect_clicked[sender] => move |_| {
                        sender.input(DownloadMsg::PauseSelected);
                    }
                },

                #[name = "move_up_button"]
                append = &gtk::Button {
                    set_label: "Move Up",
                    connect_clicked[sender] => move |_| {
                        sender.input(DownloadMsg::MoveUp);
                    }
                },

                #[name = "move_down_button"]
                append = &gtk::Button {
                    set_label: "Move Down",
                    connect_clicked[sender] => move |_| {
                        sender.input(DownloadMsg::MoveDown);
                    }
                },

                #[name = "open_manga"]
                append = &gtk::Button {
                    set_label: "Open Manga",
                    #[track(model.is_selected_only(1) == open_manga.is_sensitive())]
                    set_sensitive: model.is_selected_only(1),
                    connect_clicked[sender] => move |_| {
                        sender.input(DownloadMsg::OpenMangaSelected);
                    }
                }
            },

            append = &gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,
                set_child: Some(model.task_list.widget())
            }
        }
    }
}

impl DownloadModel {
    pub fn task_len(&self) -> usize {
        self.list.len()
    }
}

#[cfg(test)]
mod tests {
    use mado::engine::LateBindingModule;
    use mado_core::{DefaultMadoModuleMap, Url, Uuid};

    use super::*;
    use crate::tests::*;

    pub struct State {
        model: Controller<DownloadModel>,
        modulemap: Arc<DefaultMadoModuleMap>,
    }

    impl State {
        pub fn new(model: Controller<DownloadModel>) -> Self {
            Self {
                model,
                modulemap: Arc::new(DefaultMadoModuleMap::new()),
            }
        }

        pub fn emit_create(&self, info: Arc<DownloadInfo>) {
            self.model.emit(DownloadMsg::CreateDownloadView(info));
        }

        pub fn create_info(&self, order: usize) -> Arc<DownloadInfo> {
            let module = LateBindingModule::WaitModule(
                self.modulemap.clone(),
                Uuid::from_u128(order.try_into().unwrap()),
            );

            Arc::new(
                DownloadInfo::builder()
                    .order(order)
                    .module(module)
                    .chapters(vec![])
                    .status(mado::engine::DownloadStatus::paused())
                    .build(),
            )
        }

        pub fn selection(&self) -> gtk::MultiSelection {
            self.model.model().task_list.model().selection.clone()
        }
    }

    #[gtk::test]
    pub fn resume_test() {
        let model = DownloadModel::builder().launch(()).detach();

        let state = State::new(model);

        let first = state.create_info(1);
        let second = state.create_info(2);

        state.emit_create(first.clone());
        state.emit_create(second.clone());
        run_loop();

        state.selection().select_item(0, false);

        state.model.widgets().resume_button.emit_clicked();
        run_loop();

        assert!(first.status().is_resumed());
        assert!(second.status().is_paused());

        state.model.widgets().pause_button.emit_clicked();
        // model.emit(DownloadMsg::PauseSelected);
        run_loop();

        assert!(first.status().is_paused());
        assert!(second.status().is_paused());
    }

    #[gtk::test]
    pub fn sort_test() {
        let model = DownloadModel::builder().launch(()).detach();

        let state = State::new(model);

        let first = state.create_info(1);
        let second = state.create_info(2);

        state.emit_create(second);
        state.emit_create(first);
        run_loop();

        // item at 0 should be from `first` because `first` has higher order
        // even if second is added first
        let gfirst = state.selection().item(0).unwrap();
        let gsecond = state.selection().item(1).unwrap();

        let gmodel = state.model.model();

        let gfirst = gmodel.list.get_by_object(&gfirst).unwrap();
        let gsecond = gmodel.list.get_by_object(&gsecond).unwrap();

        assert_eq!(*gfirst.info().module_uuid(), Uuid::from_u128(1));
        assert_eq!(*gsecond.info().module_uuid(), Uuid::from_u128(2));
    }

    #[gtk::test]
    pub fn order_test() {
        let model = DownloadModel::builder().launch(()).detach();
        let state = State::new(model);

        let vec = (0..8).map(|it| state.create_info(it)).collect::<Vec<_>>();

        let collect = || {
            let model = state.model.model();
            let tmodel = model.task_list.model();

            tmodel
                .selection
                .model()
                .unwrap()
                .into_iter()
                .flat_map(|it| it.ok())
                .flat_map(|it| model.list.get_by_object(&it))
                .map(|it| (it.info().order(), *it.info().module_uuid()))
                .collect::<Vec<_>>()
        };

        let map_vec_uuid = |vec: Vec<(usize, Uuid)>| vec.into_iter().map(|(_, it)| it.as_u128());

        let collect_uuid = || map_vec_uuid(collect()).collect::<Vec<_>>();

        let collect_selected = || {
            let model = state.model.model();
            let tmodel = model.task_list.model();

            let bitset = tmodel.selection.selection();

            tmodel
                .selection
                .model()
                .unwrap()
                .into_iter()
                .enumerate()
                .filter(|(i, _)| bitset.contains(*i as u32))
                .map(|(_, it)| it.unwrap())
                .flat_map(|it| model.list.get_by_object(&it))
                .map(|it| (it.info().order(), *it.info().module_uuid()))
                .collect::<Vec<_>>()
        };

        let collect_selected_uuid = || map_vec_uuid(collect_selected()).collect::<Vec<_>>();

        for it in &vec {
            state.emit_create(it.clone());
        }

        run_loop();
        state.selection().select_item(5, false);
        state.selection().select_item(3, false);
        run_loop();

        assert_eq!(collect_uuid(), [0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(collect_selected_uuid(), [3, 5]);
        state.model.widgets().move_up_button.emit_clicked();
        run_loop();
        assert_eq!(collect_uuid(), [0, 1, 3, 5, 2, 4, 6, 7]);
        assert_eq!(collect_selected_uuid(), [3, 5]);

        state.model.widgets().move_down_button.emit_clicked();
        run_loop();
        assert_eq!(collect_uuid(), [0, 1, 2, 3, 5, 4, 6, 7]);

        state.selection().unselect_item(3);
        assert_eq!(collect_selected_uuid(), [5]);
        run_loop();
        state.model.widgets().move_down_button.emit_clicked();
        run_loop();
        assert_eq!(collect_uuid(), [0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[gtk::test]
    pub fn open_manga_test() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();

        let (tx, rx) = relm4::channel();
        let model = DownloadModel::builder().launch(()).forward(&tx, |msg| msg);
        let state = State::new(model);

        state.create_info(5);
        let module = LateBindingModule::WaitModule(
            state.modulemap.clone(),
            Uuid::from_u128(1.try_into().unwrap()),
        );

        let dl1 = Arc::new(
            DownloadInfo::builder()
                .url(Some(Url::parse("https://localhost/").unwrap()))
                .path("path-1")
                .order(1)
                .module(module.clone())
                .chapters(vec![])
                .status(mado::engine::DownloadStatus::paused())
                .build(),
        );

        let dl2 = Arc::new(
            DownloadInfo::builder()
                .url(Some(Url::parse("https://127.0.0.1").unwrap()))
                .path("path-2")
                .order(2)
                .module(module)
                .chapters(vec![])
                .status(mado::engine::DownloadStatus::paused())
                .build(),
        );

        state.model.emit(DownloadMsg::OpenMangaSelected);
        run_loop();
        rt.block_on(try_recv(&rx)).expect_err("should not exists");

        state.emit_create(dl1.clone());
        state.emit_create(dl2.clone());
        run_loop();

        state.selection().select_item(1, true);
        run_loop();

        state.model.emit(DownloadMsg::OpenMangaSelected);
        run_loop();

        let (url, path) = rt.block_on(async {
            match try_recv(&rx).await.unwrap() {
                DownloadOutputMsg::OpenManga { url, path } => (url, path),
                // _ => unreachable!(),
            }
        });

        assert_eq!(url.to_string(), "https://127.0.0.1/");
        assert_eq!(path.to_string(), "path-2");
    }
}
