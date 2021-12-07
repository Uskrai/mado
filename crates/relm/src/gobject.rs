use std::fmt::Debug;
use std::mem::MaybeUninit as StdMaybeUninit;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Initialized {
    Initialized,
    Uninitalized,
}

/// Struct to wrap glib object inside struct_wrapper.
/// #Safety:
/// This Struct should only be used at struct_wrapper.
pub struct MaybeUninit<T> {
    initialized: Initialized,
    inner: StdMaybeUninit<T>,
}

macro_rules! check_init {
    ($this:expr) => {
        assert!($this.initialized == Initialized::Initialized);
    };
}

impl<T> MaybeUninit<T> {
    pub fn new(t: T) -> Self {
        Self {
            initialized: Initialized::Initialized,
            inner: StdMaybeUninit::new(t),
        }
    }

    pub fn write(&mut self, t: T) {
        self.inner.write(t);
        self.initialized = Initialized::Initialized;
    }
}

impl<T> AsRef<T> for MaybeUninit<T> {
    fn as_ref(&self) -> &T {
        check_init!(self);
        unsafe { self.inner.assume_init_ref() }
    }
}

impl<T> AsMut<T> for MaybeUninit<T> {
    fn as_mut(&mut self) -> &mut T {
        check_init!(self);
        unsafe { self.inner.assume_init_mut() }
    }
}

impl<T> Clone for MaybeUninit<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        MaybeUninit::new(self.as_ref().clone())
    }
}

impl<T> Default for MaybeUninit<T> {
    fn default() -> Self {
        Self {
            initialized: Initialized::Uninitalized,
            inner: StdMaybeUninit::uninit(),
        }
    }
}

impl<T> Debug for MaybeUninit<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        check_init!(self);
        f.debug_struct("MaybeUninit")
            .field("initialized", &self.initialized)
            .field("inner", self.as_ref())
            .finish()
    }
}

impl<T> Drop for MaybeUninit<T> {
    fn drop(&mut self) {
        check_init!(self);
        unsafe {
            drop_in_place(self.inner.as_mut_ptr());
        }
    }
}

pub struct Ref<'a, T> {
    inner: std::cell::Ref<'a, MaybeUninit<T>>,
}

impl<'a, T> Ref<'a, T> {
    pub fn new(cell: std::cell::Ref<'a, MaybeUninit<T>>) -> Self {
        Self { inner: cell }
    }
}

impl<'a, T> Deref for Ref<'a, T> {
    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }

    type Target = T;
}

pub struct RefMut<'a, T> {
    inner: std::cell::RefMut<'a, MaybeUninit<T>>,
}

impl<'a, T> RefMut<'a, T> {
    pub fn new(cell: std::cell::RefMut<'a, MaybeUninit<T>>) -> Self {
        Self { inner: cell }
    }
}

impl<'a, T> Deref for RefMut<'a, T> {
    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }

    type Target = T;
}

impl<'a, T> DerefMut for RefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut()
    }
}

macro_rules! struct_wrapper {
    ($name:ident, $dest:ty, $gname:literal, $module:ident) => {
        mod $module {
            use gtk::glib;
            use gtk::subclass::prelude::*;
            mod imp {
                use gtk::glib;
                use gtk::subclass::prelude::*;
                use std::cell::RefCell;

                #[derive(Default, Debug)]
                pub struct $name {
                    pub inner: RefCell<$crate::gobject::MaybeUninit<$dest>>,
                }

                #[glib::object_subclass]
                impl ObjectSubclass for $name {
                    const NAME: &'static str = $gname;
                    type ParentType = glib::Object;
                    type Type = super::$name;
                }

                impl ObjectImpl for $name {}
            }

            glib::wrapper! {
              pub struct $name(ObjectSubclass<imp::$name>);
            }

            impl $name {
                pub fn to_gobject(dest: $dest) -> Self {
                    let this = glib::Object::new(&[]).unwrap();
                    let r = imp::$name::from_instance(&this);
                    r.inner.borrow_mut().write(dest);
                    this
                }

                pub fn borrow(&self) -> $crate::gobject::Ref<$dest> {
                    let r = imp::$name::from_instance(self);
                    $crate::gobject::Ref::new(r.inner.borrow())
                }

                pub fn borrow_mut<'a>(&'a self) -> $crate::gobject::RefMut<$dest> {
                    let r = imp::$name::from_instance(self);
                    $crate::gobject::RefMut::new(r.inner.borrow_mut())
                }
            }
        }
    };
}

use std::ops::{Deref, DerefMut};
use std::ptr::drop_in_place;

pub(crate) use struct_wrapper;
