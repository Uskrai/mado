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

use std::{collections::HashMap, ops::Range};

use runestick::{Any, ContextError, Module, Value, VmError};

#[derive(Any, Clone, Debug)]
pub struct Regex {
  inner: regex::Regex,
}

#[derive(Any, Clone, Debug)]
pub struct Match {
  range: Option<Range<usize>>,
  text: String,
}

#[derive(Any, Debug, Clone)]
pub struct Captures {
  text: String,
  named: HashMap<String, Option<Range<usize>>>,
  captured: Vec<Option<Range<usize>>>,
}

impl Regex {
  fn compile(pattern: &str) -> Self {
    regex::Regex::new(pattern)
      .map(|v| Regex { inner: v })
      .unwrap()
  }

  fn is_match(&self, text: &str) -> bool {
    self.inner.is_match(text)
  }

  fn find(&self, text: Value) -> Result<Match, VmError> {
    let text = text.into_string()?.take()?;

    let range = match self.inner.find(&text) {
      Some(val) => Some(val.range()),
      None => None,
    };

    Ok(Match { text, range })
  }

  fn find_at(&self, text: Value, index: usize) -> Result<Match, VmError> {
    let text = text.into_string()?.take()?;

    let range = {
      match self.inner.find_at(&text, index) {
        Some(val) => Some(val.range()),
        None => None,
      }
    };

    Ok(Match { text, range })
  }

  fn captures(&self, text: Value) -> Result<Option<Captures>, VmError> {
    let text = text.into_string()?.take().unwrap();

    let fun = || {
      let captures = self.inner.captures(&text)?;

      let mut named = HashMap::new();
      let mut captured = Vec::new();

      let cap = self.inner.capture_names();
      for (name, ma) in cap.zip(captures.iter()) {
        let ma = ma.map(|v| v.range());
        if let Some(name) = name {
          named.insert(name.to_owned(), ma.clone());
        }

        captured.push(ma);
      }

      Some(Captures {
        text,
        named,
        captured,
      })
    };

    Ok(fun())
  }
}

impl Match {
  fn range(&self) -> Option<Range<usize>> {
    self.range.clone()
  }

  fn to_string_debug(&self, s: &mut String) -> std::fmt::Result {
    use std::fmt::Write;
    write!(s, "{:?}", self)
  }

  fn get_match(&self) -> Option<String> {
    match self.range.clone() {
      Some(range) => Some(self.text[range].to_owned()),
      None => None,
    }
  }

  fn into_original(self) -> String {
    self.text
  }
}

impl Captures {
  pub fn get(&self, i: usize) -> Option<String> {
    let range = self.captured.get(i)?.clone()?;
    Some(self.text[range].to_owned())
  }

  pub fn name(&self, name: &str) -> Option<String> {
    let range = self.named.get(name)?.clone()?;
    Some(self.text[range].to_owned())
  }

  pub fn to_string_debug(&self, s: &mut String) -> std::fmt::Result {
    use std::fmt::Write;
    write!(s, "{:?}", self)
  }
}

pub fn load_module() -> Result<Module, ContextError> {
  let module = Module::with_crate_item("mado", &["regex"]);
  mado_rune_macros::register_module! {
    (Regex) => {
      associated => {
        compile
      },
      inst => { is_match, find, find_at, captures }
    },
    (Captures) => {
      inst => {
        get, name
      },
      protocol => {
        to_string_debug : STRING_DEBUG
      }
    },
    (Match) => {
      inst => { range, get_match, into_original}
      protocol => {
        to_string_debug : STRING_DEBUG
      }
    }
  }

  load_module_with(module)
}
