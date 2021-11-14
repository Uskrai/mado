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

use super::*;
use runestick::Any;

#[derive(Any, Clone, Debug)]
pub struct Client {
  inner: reqwest::Client,
}

macro_rules! impl_client {
  ($name:ident) => {
    pub fn $name(&self, url: Url) -> RequestBuilder {
      RequestBuilder {
        inner: self.inner.$name::<reqwest::Url>(url.into()),
      }
    }
  };
}

impl Client {
  pub fn default() -> Self {
    Self::new(reqwest::Client::new())
  }

  pub fn new(value: reqwest::Client) -> Self {
    Self { inner: value }
  }

  impl_client!(get);
  impl_client!(post);
  impl_client!(put);
  impl_client!(delete);
  impl_client!(head);
}
