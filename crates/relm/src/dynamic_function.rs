#[allow(unused_macros)]
macro_rules! create_dynamic_function {
  ($name:ident, ($($args:ident : $types:ty),+) -> $return:ty) => {
    pub struct $name {
      inner: Box<dyn Fn($($types),+) -> $return>
    }

    impl $name {
      pub fn call(&self, $($args : $types),+) -> $return {
        (*self.inner)($($args),+)
      }
    }

    impl<T> From<T> for $name
      where T: Fn($($types),+) -> $return + 'static
    {
      fn from(v: T) -> Self {
        Self { inner: Box::new(v) }
      }
    }


    impl std::fmt::Debug for $name {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ptr = &*self.inner as *const dyn Fn($($types),+) -> $return;
        f.debug_struct(stringify!($name))
          .field("inner", &ptr)
          .finish()
      }
    }
  };
}

pub(crate) use create_dynamic_function;
