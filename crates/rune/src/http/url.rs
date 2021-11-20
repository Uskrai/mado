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

use rune::Any;
use std::{fmt::Display, str::FromStr};

use crate::Error;

#[derive(Any, Debug, Clone)]
pub struct Url {
  inner: reqwest::Url,
}

impl Url {
  pub fn new(url: reqwest::Url) -> Self {
    Self { inner: url }
  }
  pub fn parse(input: &str) -> Result<Self, Error> {
    let url = input.parse();
    match url {
      Ok(url) => Ok(Self::new(url)),
      Err(err) => Err(Error::url_parse_error(input.to_string(), err)),
    }
  }

  pub fn parse_resolve_domain(input: &str) -> Result<Self, Error> {
    use ::url::ParseError;
    // parsing as reqwest::Url because runes::Error will complicate the process
    let url = reqwest::Url::parse(input);
    url
      .or_else(|err| match err {
        ParseError::RelativeUrlWithoutBase => {
          let input = "https://".to_owned() + input;
          reqwest::Url::parse(&input)
            // we should return the first error here.
            .map_err(|_| Error::url_parse_error(input.to_string(), err))
        }
        _ => Err(Error::url_parse_error(input.to_string(), err)),
      })
      .map(|v| v.into())
  }

  pub fn query(mut self, key: &str, value: &str) -> Self {
    self.inner.query_pairs_mut().append_pair(key, value);
    self
  }

  pub fn path(&self) -> String {
    self.inner.path().to_string()
  }

  pub fn into_inner(self) -> url::Url {
    self.inner
  }

  pub fn to_string_debug(&self, s: &mut String) -> std::fmt::Result {
    use std::fmt::Write;
    write!(s, "{:?}", self.inner)
  }
}

impl Display for Url {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.inner)
  }
}

impl From<Url> for mado_core::url::Url {
  fn from(v: Url) -> Self {
    v.inner
  }
}

impl From<reqwest::Url> for Url {
  fn from(v: reqwest::Url) -> Self {
    Self::new(v)
  }
}

impl FromStr for Url {
  type Err = Error;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Self::parse(s)
  }
}
