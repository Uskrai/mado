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

use std::{collections::VecDeque, path::PathBuf};

use rune::{
  compile::{CompileError, CompileErrorKind, Component},
  Source,
};

//
// /// ```
// /// use rune::compile::Item;
// /// use mado_rune::compile::SourceLoader;
// ///
// /// #   use tempfile::TempDir;
// /// # pub fn create_environment(
// /// #   list_path: &[&str],
// /// # ) -> Result<TempDir, Box<dyn std::error::Error>> {
// /// #   let dir = tempfile::tempdir()?;
// /// #   let path = dir.path();
// /// #   println!("{}", path.display());
// /// #   for it in list_path {
// /// #     std::fs::create_dir_all(path.join(it).parent().unwrap())?;
// /// #     std::fs::File::create(path.join(it))?;
// /// #   }
// /// #   Ok(dir)
// /// # }
// /// # pub fn main() -> Result<(), Box<dyn std::error::Error>> {
// /// let dir = create_environment(&[
// ///   "root.rn",
// ///   "foo.rn",
// ///   "foo/foo.rn",
// ///   "foo/bar.rn",
// ///   "foo/baz.rn",
// ///   "baz.rn",
// ///   "bar.rn",
// /// ])?;
// /// let root = dir.path().join("root.rn");
// ///
// /// let assert = |name, expected| -> Result<(), Box<dyn std::error::Error>> {
// ///   let path = SourceLoader::search_path(root.clone(), item(name))?;
// ///   assert_eq!(path, dir.path().join(expected));
// ///   Ok(())
// /// };
// ///
// /// assert("foo", "foo.rn")?;
// /// assert("foo::foo", "foo/foo.rn")?;
// /// assert("foo::bar", "bar.rn")?;
// /// assert("foo::foo::bar", "foo/bar.rn")?;
// /// assert("foo::bar::baz", "baz.rn")?;
// /// assert("bar::foo::foo::baz", "foo/baz.rn")?;
// ///
// /// // these work but will cause infinite loop when called from rune
// /// assert("foo::bar::foo", "foo.rn")?;
// /// assert("foo::bar::foo::bar", "bar.rn")?;
// /// Ok(())
// /// # }
// ///
// /// # fn item(name: &str) -> Item {
// /// #   Item::with_item(name.split("::").into_iter())
// /// # }
// /// ```
#[derive(Default)]
pub struct SourceLoader {
  //
}

impl rune::compile::SourceLoader for SourceLoader {
  fn load(
    &mut self,
    root: &std::path::Path,
    item: &rune::compile::Item,
    span: rune::ast::Span,
  ) -> Result<rune::Source, rune::compile::CompileError> {
    let path = Self::search_path(root.to_owned(), item.clone());

    let path = match path {
      Ok(path) => path,
      Err(error) => {
        return Err(CompileError::new(span, error));
      }
    };

    match Source::from_path(&path) {
      Ok(source) => Ok(source),
      Err(error) => Err(CompileError::new(
        span,
        CompileErrorKind::ModFileError {
          path: path.to_owned(),
          error,
        },
      )),
    }

    // panic!("{} {} {}", base.display(), item, span);
  }
}

impl SourceLoader {
  pub fn new() -> Self {
    Self {}
  }

  pub fn search_path(
    mut root: std::path::PathBuf,
    base_item: rune::compile::Item,
  ) -> Result<std::path::PathBuf, CompileErrorKind> {
    if !root.pop() {
      return Err(CompileErrorKind::UnsupportedModuleRoot {
        root: root.to_owned(),
      });
    };

    // convert to deque for efficient front popping
    let mut item: VecDeque<_> = base_item.clone().into_iter().collect();

    loop {
      match item.pop_front() {
        Some(Component::Str(string)) => {
          root = Self::search_module(root, &string)?;
        }
        Some(Component::Id(..) | Component::Crate(..)) => {
          return Err(CompileErrorKind::UnsupportedModuleItem {
            item: base_item,
          })
        }
        // break when empty
        None => {
          break;
        }
      }
    }

    let candidates = [root.join("mod.rn"), root.with_extension("rn")];
    let mut found = None;

    for it in candidates {
      if it.exists() && it.is_file() {
        found = Some(it);
        break;
      }
    }

    let path = match found {
      Some(path) => path,
      None => {
        return Err(CompileErrorKind::ModNotFound { path: root });
      }
    };

    Ok(path)
  }

  fn search_module(
    mut root: PathBuf,
    name: &str,
  ) -> Result<PathBuf, CompileErrorKind> {
    if root.is_file() {
      let filestem = root.file_stem().unwrap().to_string_lossy();
      if name == filestem {
        root = root.parent().unwrap().join(name);
      } else {
        let res = root.parent();
        root = match res {
          Some(path) => path.to_owned(),
          None => {
            return Err(CompileErrorKind::ModNotFound {
              path: root.join(name),
            })
          }
        };
      }
    }

    let path = root.join(name).with_extension("rn");
    if path.exists() {
      Ok(path)
    } else {
      Err(CompileErrorKind::ModNotFound {
        path: root.join(name),
      })
    }
  }
}

#[cfg(test)]
mod test {
  use super::SourceLoader;
  use rune::compile::Item;
  use tempfile::TempDir;

  pub fn create_environment(
    list_path: &[&str],
  ) -> Result<TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path();
    for it in list_path {
      std::fs::create_dir_all(path.join(it).parent().unwrap())?;
      std::fs::File::create(path.join(it))?;
    }
    Ok(dir)
  }

  #[test]
  pub fn test_same_in_directory() -> Result<(), Box<dyn std::error::Error>> {
    let dir = create_environment(&[
      "root.rn",
      "foo.rn",
      "foo/foo.rn",
      "foo/baz.rn",
      "foo/bar.rn",
      "baz.rn",
      "bar.rn",
    ])?;
    let root = dir.path().join("root.rn");

    let assert = |name, expected| -> Result<(), Box<dyn std::error::Error>> {
      let path = SourceLoader::search_path(root.clone(), item(name))?;
      assert_eq!(path, dir.path().join(expected));
      Ok(())
    };

    assert("foo", "foo.rn")?;
    assert("foo::foo", "foo/foo.rn")?;
    assert("foo::bar", "bar.rn")?;
    assert("foo::foo::bar", "foo/bar.rn")?;
    assert("foo::bar::baz", "baz.rn")?;
    assert("bar::foo::baz", "baz.rn")?;
    assert("bar::foo::foo::baz", "foo/baz.rn")?;

    // these will cause infinite loop
    assert("foo::bar::foo", "foo.rn")?;
    assert("foo::bar::foo::bar", "bar.rn")?;
    Ok(())
  }

  fn item(name: &str) -> Item {
    Item::with_item(name.split("::").into_iter())
  }
}
