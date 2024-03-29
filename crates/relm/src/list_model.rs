use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub struct ListModel<T>(
    Arc<dyn ListModelBase<T>>,
    Option<Arc<dyn Fn(&ListModel<T>, gtk::gio::ListModel) -> gtk::gio::ListModel + Send>>,
);

impl<T> Clone for ListModel<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

impl<T> ListModel<T> {
    pub fn new<R>(val: R) -> ListModel<T>
    where
        R: ListModelBase<T> + 'static,
    {
        ListModel(Arc::new(val), None)
    }

    pub fn new_with_model<R, F>(val: R, fun: F) -> ListModel<T>
    where
        R: ListModelBase<T> + 'static,
        F: Fn(&ListModel<T>, gtk::gio::ListModel) -> gtk::gio::ListModel + Send + 'static,
    {
        ListModel(Arc::new(val), Some(Arc::new(fun)))
    }
}

pub trait ListModelBorrowBase<T>: Deref<Target = T> {}
pub struct ListModelBorrow<'a, T>(Box<dyn ListModelBorrowBase<T> + 'a>);

impl<'borrow, T> ListModelBorrow<'borrow, T> {
    pub fn new<R>(val: R) -> Self
    where
        R: ListModelBorrowBase<T> + 'borrow,
    {
        Self(Box::new(val))
    }
}

impl<T> Deref for ListModelBorrow<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait ListModelMutBorrowBase<T>: DerefMut<Target = T> {}
pub struct ListModelMutBorrow<'a, T>(Box<dyn ListModelMutBorrowBase<T> + 'a>);

impl<'borrow, T> ListModelMutBorrow<'borrow, T> {
    pub fn new<R>(val: R) -> Self
    where
        R: ListModelMutBorrowBase<T> + 'borrow,
    {
        Self(Box::new(val))
    }
}

impl<T> Deref for ListModelMutBorrow<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ListModelMutBorrow<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> ListModelMutBorrowBase<T> for ListModelMutBorrow<'_, T> {}

pub trait ListModelBase<T> {
    /// Return a BorrowMut of  Item that represent object
    ///
    /// Object should be from list_model.
    fn get_by_object(&self, object: &gtk::glib::Object) -> Option<ListModelBorrow<'_, T>>;
    fn get_mut_by_object(&self, object: &gtk::glib::Object) -> Option<ListModelMutBorrow<'_, T>>;
    fn notify_changed(&self, object: &gtk::glib::Object);
    fn list_model(&self) -> gtk::gio::ListModel;
}

impl<T> ListModelBase<T> for ListModel<T> {
    fn get_by_object(&self, object: &gtk::glib::Object) -> Option<ListModelBorrow<'_, T>> {
        self.0.get_by_object(object)
    }

    fn get_mut_by_object(&self, object: &gtk::glib::Object) -> Option<ListModelMutBorrow<'_, T>> {
        self.0.get_mut_by_object(object)
    }

    fn notify_changed(&self, object: &gtk::glib::Object) {
        self.0.notify_changed(object)
    }

    fn list_model(&self) -> gtk::gio::ListModel {
        let model = self.0.list_model();
        if let Some(it) = &self.1 {
            it(self, model)
        } else {
            model
        }
    }
}

pub trait ListModelBaseExt<T>: ListModelBase<T> + Clone + 'static {
    fn filter_closure<F>(&self, closure: F) -> gtk::ClosureExpression
    where
        F: Fn(ListModelBorrow<T>) -> bool + 'static,
    {
        use gtk::glib;
        let this = self.clone();

        gtk::ClosureExpression::new::<bool>(
            &[] as &[gtk::Expression],
            gtk::glib::closure_local!(|number: gtk::glib::Object| {
                if let Some(it) = this.get_by_object(&number) {
                    closure(it)
                } else {
                    false
                }
            }),
        )
    }

    fn bool_filter<F>(&self, closure: F) -> gtk::BoolFilter
    where
        F: Fn(ListModelBorrow<T>) -> bool + 'static,
    {
        let closure = self.filter_closure(closure);

        gtk::BoolFilter::new(Some(&closure))
    }

    fn custom_sorter<F>(&self, closure: F) -> gtk::CustomSorter
    where
        F: Fn(ListModelBorrow<T>, ListModelBorrow<T>) -> std::cmp::Ordering + 'static,
    {
        let this = self.clone();
        gtk::CustomSorter::new(move |first, second| {
            let first = this.get_by_object(first);
            let second = this.get_by_object(second);

            match (first, second) {
                (None, None) => gtk::Ordering::Equal,
                (None, Some(_)) => gtk::Ordering::Larger,
                (Some(_), None) => gtk::Ordering::Smaller,
                (Some(first), Some(second)) => closure(first, second).into(),
            }
        })
    }

    fn for_each<F>(&self, mut closure: F)
    where
        F: FnMut(ListModelBorrow<T>),
    {
        for it in self.list_model().into_iter() {
            let it = it.unwrap();

            if let Some(it) = self.get_by_object(&it) {
                closure(it)
            }
        }
    }
}
impl<L, T> ListModelBaseExt<T> for L where L: ListModelBase<T> + Clone + 'static {}
