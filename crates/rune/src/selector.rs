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

use std::fmt::Display;

use mado_rune_macros::register_module;
use runestick::{Any, AnyObj, ContextError, Module, Shared, Value};

#[derive(Any)]
pub struct Document {
  inner: nipper::Document,
}

#[derive(Any)]
pub struct Selection {
  inner: nipper::Selection<'static>,
}

#[derive(Any)]
pub struct Node {
  inner: nipper::Node<'static>,
}

#[derive(Any)]
pub struct StrTendril {
  inner: tendril::StrTendril,
}

impl Document {
  pub fn new(from: &str) -> Self {
    Self {
      inner: nipper::Document::from(from),
    }
  }

  pub fn find(&'static self, selector: &str) -> Selection {
    Selection {
      inner: self.inner.select(selector),
    }
  }

  pub fn html(&self) -> StrTendril {
    StrTendril {
      inner: self.inner.html(),
    }
  }

  pub fn text(&self) -> StrTendril {
    StrTendril {
      inner: self.inner.text(),
    }
  }
}

impl Selection {
  pub fn new(selection: nipper::Selection<'static>) -> Self {
    Self { inner: selection }
  }
  pub fn text(&self) -> StrTendril {
    StrTendril {
      inner: self.inner.text(),
    }
  }

  pub fn html(&self) -> StrTendril {
    StrTendril::new(self.inner.html())
  }

  pub fn find(&'static self, selector: &str) -> Self {
    self.inner.select(selector).into()
  }

  pub fn remove(&mut self) {
    self.inner.remove()
  }

  pub fn first(&self) -> Self {
    self.inner.first().into()
  }

  pub fn last(&self) -> Self {
    self.inner.last().into()
  }

  pub fn attr(&self, name: &str) -> Option<StrTendril> {
    Some(self.inner.attr(name)?.into())
  }

  pub fn attr_or(&self, name: &str, default: &str) -> StrTendril {
    self.inner.attr_or(name, default).into()
  }

  pub fn parent(&self) -> Selection {
    self.inner.parent().into()
  }

  pub fn children(&self) -> Selection {
    self.inner.children().into()
  }

  pub fn to_string_debug(&self, s: &mut String) -> std::fmt::Result {
    use std::fmt::Write;
    write!(s, "{:?}", self.inner)
  }

  pub fn len(&self) -> usize {
    self.inner.length()
  }

  pub fn iter_of_value(&'static self) -> impl Iterator<Item = Value> {
    self
      .inner
      .iter()
      .map(Self::from)
      .map(|v| Value::Any(Shared::new(AnyObj::new(v))))
  }

  pub fn iter(&'static self) -> Value {
    let nodes = self.iter_of_value();

    let iter = runestick::Iterator::from("nodes", nodes);

    Value::Iterator(Shared::new(iter))
  }

  pub fn to_vec(&'static self) -> Value {
    let nodes = self.iter_of_value();
    let vec = runestick::Vec::from(nodes.collect::<Vec<_>>());
    Value::Vec(Shared::new(vec))
  }
}

impl From<nipper::Selection<'static>> for Selection {
  fn from(v: nipper::Selection<'static>) -> Self {
    Self::new(v)
  }
}

impl Node {
  pub fn new(node: nipper::Node<'static>) -> Self {
    Self { inner: node }
  }

  pub fn attr(&self, name: &str) -> Option<StrTendril> {
    Some(self.inner.attr(name)?.into())
  }

  pub fn text(&self) -> StrTendril {
    self.inner.text().into()
  }

  pub fn html(&self) -> StrTendril {
    self.inner.html().into()
  }

  pub fn to_string_debug(&self, s: &mut String) -> std::fmt::Result {
    use std::fmt::Write;
    write!(s, "{:#?}", self.inner)
  }
}

impl StrTendril {
  pub fn new(s: tendril::StrTendril) -> Self {
    Self { inner: s }
  }

  pub fn to_string_rune(&self, s: &mut String) -> std::fmt::Result {
    use std::fmt::Write;
    write!(s, "{}", self.inner)
  }

  pub fn to_string_debug(&self, s: &mut String) -> std::fmt::Result {
    use std::fmt::Write;
    write!(s, "{:?}", self.inner)
  }
}

impl Display for StrTendril {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.inner)
  }
}

impl From<tendril::StrTendril> for StrTendril {
  fn from(s: tendril::StrTendril) -> Self {
    Self::new(s)
  }
}

pub fn load_module() -> Result<Module, ContextError> {
  register_module! {
    (Document) => {
      associated => {
        new
      },
      inst => {
        find, html, text
      }
    },
    (Selection) => {
      inst => {
        find, remove, first, last,
        text, html, attr, attr_or,
        len, to_vec,iter
        parent, children
      },
    },
    (Node) => {
      inst => {
        attr, html, text
      }
    },
    (StrTendril) => {
      inst => {
        to_string
      }
      protocol => {
        to_string_rune: STRING_DISPLAY
      }
    },
    //debug
    (Selection : skip, Node : skip, StrTendril : skip) => {
      protocol => {
        to_string_debug: STRING_DEBUG
      }
    }
  }

  load_module_with(Module::with_crate_item("mado", &["html"]))
}
