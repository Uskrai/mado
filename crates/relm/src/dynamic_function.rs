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
