use crate::list_model::{
    ListModel, ListModelBase, ListModelBaseExt, ListModelBorrow, ListModelBorrowBase,
    ListModelMutBorrow, ListModelMutBorrowBase,
};
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use gtk::prelude::*;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ListIndex(usize);

impl ListIndex {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

struct Inner<T> {
    list: gtk::gio::ListStore,
    map_index: RefCell<HashMap<ListIndex, u32>>,
    container: RefCell<slab::Slab<T>>,
}

pub struct ListStore<T>(Arc<Inner<T>>);
impl<T> Clone for ListStore<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Default for ListStore<T> {
    fn default() -> Self {
        Self(Arc::new(Inner {
            list: gtk::gio::ListStore::new(gtk::glib::Type::OBJECT),
            container: Default::default(),
            map_index: Default::default(),
        }))
    }
}

pub struct RefGuard<'a, T> {
    guard: Ref<'a, slab::Slab<T>>,
    index: usize,
}

impl<'a, T> Deref for RefGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.get(self.index).unwrap()
    }
}

impl<'a, T> ListModelBorrowBase<T> for RefGuard<'a, T> {}

pub struct MutexGuard<'a, T> {
    guard: RefMut<'a, slab::Slab<T>>,
    index: usize,
}

impl<'a, T> ListModelMutBorrowBase<T> for MutexGuard<'a, T> {}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.get(self.index).unwrap()
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.get_mut(self.index).unwrap()
    }
}

crate::gobject::struct_wrapper!(GUsize, Option<usize>, "MadoRelmUsize", usize_wrapper);
use usize_wrapper::GUsize;

fn to_object(value: &ListIndex) -> GUsize {
    GUsize::to_gobject(Some(value.as_usize()))
}

pub fn object_gusize(object: &gtk::glib::Object) -> Option<GUsize> {
    object.clone().downcast().ok()
}

impl<T: 'static> ListStore<T> {
    fn container(&self) -> RefMut<slab::Slab<T>> {
        self.0.container.borrow_mut()
    }

    fn list(&self) -> &gtk::gio::ListStore {
        &self.0.list
    }

    fn map_index(&self) -> RefMut<HashMap<ListIndex, u32>> {
        self.0.map_index.borrow_mut()
    }

    pub fn push(&self, value: T) -> ListIndex {
        let index = ListIndex(self.container().insert(value));
        let list_position = self.list().n_items();
        self.list().append(&to_object(&index));
        self.map_index().insert(index.clone(), list_position);

        index
    }

    pub fn get(&self, &ListIndex(index): &ListIndex) -> Option<RefGuard<T>> {
        let guard = self.0.container.borrow();

        guard
            .get(index)
            .map(|_| ())
            .map(|_| RefGuard { guard, index })
    }

    pub fn get_mut(&self, &ListIndex(index): &ListIndex) -> Option<MutexGuard<T>> {
        let guard = self.container();

        guard
            .get(index)
            .map(|_| ())
            .map(|_| MutexGuard { guard, index })
    }

    pub fn get_gobject(&self, index: &ListIndex) -> Option<gtk::glib::Object> {
        let guard = self.list();
        let map_index = self.map_index();

        map_index.get(index).and_then(|it| guard.item(*it))
    }

    pub fn remove(&self, index: ListIndex) -> Option<T> {
        let it = self.container().try_remove(index.as_usize())?;

        let apply = |list_position: u32, slab_position: &ListIndex, gusize: GUsize| {
            if *gusize.borrow() == Some(slab_position.as_usize()) {
                *gusize.borrow_mut() = None;
                self.list().items_changed(list_position, 1, 1);
                true
            } else {
                false
            }
        };

        let mut map_index = self.map_index();
        if let Some(list_position) = map_index.get(&index).cloned() {
            let it = self
                .list()
                .item(list_position)
                .as_ref()
                .and_then(object_gusize);

            if let Some(it) = it {
                apply(list_position, &index, it);
                map_index.remove(&index);
            }
        }

        Some(it)
    }

    fn get_by_object(&self, index: &gtk::glib::Object) -> Option<RefGuard<T>> {
        let index = *object_gusize(index)?.borrow();

        index.and_then(|index| self.get(&ListIndex(index)))
    }

    fn get_mut_by_object(&self, index: &gtk::glib::Object) -> Option<MutexGuard<T>> {
        let index = *object_gusize(index)?.borrow();

        index.and_then(|index| self.get_mut(&ListIndex(index)))
    }

    fn notify_changed(&self, index: &ListIndex) {
        if let Some(it) = self.map_index().get(index) {
            self.list().items_changed(*it, 1, 1);
        }
    }

    fn base(&self) -> ListModel<T> {
        ListModel::new_with(self.clone())
    }
}

impl<T> ListModelBase<T> for ListStore<T>
where
    T: 'static,
{
    fn get_by_object(&self, object: &gtk::glib::Object) -> Option<ListModelBorrow<'_, T>> {
        self.get_by_object(object)
            .map(|it| ListModelBorrow::new_with(it))
    }
    fn get_mut_by_object(&self, object: &gtk::glib::Object) -> Option<ListModelMutBorrow<T>> {
        self.get_mut_by_object(object)
            .map(|it| ListModelMutBorrow::new_with(it))
    }

    fn notify_changed(&self, object: &gtk::glib::Object) {
        let index = object_gusize(object).and_then(|it| *it.borrow());

        if let Some(index) = index {
            self.notify_changed(&ListIndex(index));
        }
    }

    fn list_model(&self) -> gtk::gio::ListModel {
        let filter = gtk::BoolFilter::new(Some(&self.filter_closure(|_| true)));
        let model = gtk::FilterListModel::new(Some(self.list()), Some(&filter));

        model.into()
    }
}

#[cfg(test)]
mod tests {
    use gtk::FilterListModel;

    use crate::list_model::ListModel;

    use super::*;

    #[gtk::test]
    pub fn list_test() {
        let store = ListStore::<usize>::default();
        let base = ListModel::new_with(store.clone());
        let model = store.list_model();
        let count = || model.into_iter().count();

        assert_eq!(count(), 0);
        let first = store.push(1);
        assert_eq!(count(), 1);
        let second = store.push(1);
        assert_eq!(count(), 2);

        store.remove(first);
        assert_eq!(count(), 1);
        store.remove(second);
        assert_eq!(count(), 0);

        let mut indexes = vec![];
        for it in 0..10 {
            indexes.push(store.push(it * 5));
        }
        assert_eq!(count(), 10);
        for (index, it) in model.into_iter().enumerate() {
            let it = it.unwrap();

            assert_eq!(*base.get_mut_by_object(&it).unwrap(), index * 5);
        }

        for (index, it) in indexes.into_iter().skip(1).step_by(2).enumerate() {
            assert_eq!(store.remove(it).unwrap(), index * 10 + 5);
        }

        for (index, it) in model.into_iter().enumerate() {
            let it = it.unwrap();

            assert_eq!(*base.get_mut_by_object(&it).unwrap(), index * 10);
        }
    }

    #[gtk::test]
    pub fn guard_test() {
        let store = ListStore::<usize>::default();
        let base = ListModel::new_with(store.clone());
        let model = base.list_model();

        let boolfilter = store.bool_filter(|it| *it <= 100);

        let model = FilterListModel::new(Some(&model), Some(&boolfilter));
        let count = || model.iter::<gtk::glib::Object>().unwrap().count();

        let first = store.push(100);
        let second = store.push(200);
        let first = store.get_gobject(&first).unwrap();
        let second = store.get_gobject(&second).unwrap();

        assert_eq!(count(), 1);

        {
            let mut second = base.get_mut_by_object(&second).unwrap();
            *second = 50;
        };
        base.notify_changed(&second);

        assert_eq!(count(), 2);

        {
            let mut first = base.get_mut_by_object(&first).unwrap();
            *first = 101;
        };
        base.notify_changed(&first);

        assert_eq!(count(), 1);
    }

    #[gtk::test]
    pub fn sort_test() {
        let store = ListStore::<usize>::default();

        let model = store.list_model();
        let sorter = store.custom_sorter(|first, second| second.cmp(&first));
        let model = gtk::SortListModel::new(Some(&model), Some(&sorter));

        let collect = || {
            model
                .iter()
                .unwrap()
                .map(|it| *store.get_by_object(&it.unwrap()).unwrap())
                .collect::<Vec<_>>()
        };

        let first = store.push(4);
        assert_eq!(collect(), [4]);

        let second = store.push(2);
        assert_eq!(collect(), [4, 2]);

        store.remove(second);
        assert_eq!(collect(), [4]);

        store.remove(first);
        assert_eq!(collect(), vec![] as Vec<usize>);

        let first = store.push(1);
        assert_eq!(collect(), [1]);

        let second = store.push(2);
        assert_eq!(collect(), [2, 1]);

        store.remove(first);
        assert_eq!(collect(), [2]);

        store.remove(second);
        assert_eq!(collect(), vec![] as Vec<usize>);
    }

    #[gtk::test]
    pub fn sort_reverse_option_test() {
        let store = ListStore::<usize>::default();
        let sorter = store
            .base()
            .custom_sorter(|first, second| second.cmp(&first));
        let model = gtk::SortListModel::new(Some(store.list()), Some(&sorter));
        let collect = || {
            model
                .iter()
                .unwrap()
                .map(|it| store.get_by_object(&it.unwrap()).map(|it| *it))
                .collect::<Vec<_>>()
        };

        let first = store.push(1);
        assert_eq!(collect(), [Some(1)]);
        let second = store.push(2);
        assert_eq!(collect(), [Some(2), Some(1)]);

        let third = store.push(4);
        let fourth = store.push(3);
        assert_eq!(collect(), [4, 3, 2, 1].map(Some));

        store.remove(second);
        assert_eq!(collect(), [Some(4), Some(3), Some(1), None]);

        store.remove(third);
        store.remove(fourth);
        assert_eq!(collect(), [Some(1), None, None, None]);

        store.remove(first);
        assert_eq!(collect(), [None; 4]);
    }

    #[gtk::test]
    pub fn sort_option_test() {
        let store = ListStore::<usize>::default();
        let sorter = store
            .base()
            .custom_sorter(|first, second| first.cmp(&second));
        let model = gtk::SortListModel::new(Some(store.list()), Some(&sorter));
        let collect = || {
            model
                .iter()
                .unwrap()
                .map(|it| store.get_by_object(&it.unwrap()).map(|it| *it))
                .collect::<Vec<_>>()
        };

        let first = store.push(1);
        assert_eq!(collect(), [Some(1)]);
        let second = store.push(2);
        assert_eq!(collect(), [Some(1), Some(2)]);

        let third = store.push(4);
        let fourth = store.push(3);
        assert_eq!(collect(), [1, 2, 3, 4].map(Some));

        store.remove(second);
        assert_eq!(collect(), [Some(1), Some(3), Some(4), None]);

        store.remove(third);
        store.remove(fourth);
        assert_eq!(collect(), [Some(1), None, None, None]);

        store.remove(first);
        assert_eq!(collect(), [None; 4]);
    }
}
