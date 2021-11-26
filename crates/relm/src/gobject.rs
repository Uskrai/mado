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
          pub inner: RefCell<Option<$dest>>,
        }

        #[glib::object_subclass]
        impl ObjectSubclass for $name {
          const NAME: &'static str = $gname;
          type ParentType = glib::Object;
          type Type = super::$name;
        }

        impl ObjectImpl for GChapterInfo {}
      }

      glib::wrapper! {
        pub struct $name(ObjectSubclass<imp::$name>);
      }

      impl $name {
        pub fn to_gobject(dest: $dest) -> Self {
          let this = glib::Object::new(&[]).unwrap();
          let r = imp::$name::from_instance(&this);
          r.inner.replace(Some(dest));
          this
        }

        pub fn to_inner(self) -> Option<$dest> {
          let r = imp::$name::from_instance(&self);
          r.inner.replace(None)
        }

        pub fn borrow(&self) -> std::cell::Ref<'_, Option<$dest>> {
          let r = imp::$name::from_instance(&self);
          r.inner.borrow()
        }

        pub fn borrow_mut(&self) -> std::cell::RefMut<'_, Option<$dest>> {
          let r = imp::$name::from_instance(&self);
          r.inner.borrow_mut()
        }
      }
    }
  };
}

pub(crate) use struct_wrapper;
