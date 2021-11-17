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

use std::time::Duration;

use reqwest::header::SET_COOKIE;
use runestick::{Any, ContextError, Module};

mod client;
mod url;

pub use self::url::Url;
pub use client::Client;

macro_rules! wrapper_fun {
  ($name:ident, $( $param:ident : $type:ty ), *) => {
    pub fn $name(self, $($param : $type), *) -> Self {
      Self {
        inner: self.inner.$name($($param), *)
      }
    }
  };

}

#[derive(Any, Debug)]
pub struct RequestBuilder {
  inner: reqwest::RequestBuilder,
}

impl RequestBuilder {
  pub fn query(self, name: String, value: String) -> Self {
    RequestBuilder {
      inner: self.inner.query(&[(name, value)]),
    }
  }

  wrapper_fun!(basic_auth, username: String, password: Option<String>);
  wrapper_fun!(bearer_auth, token: String);
  wrapper_fun!(timeout, timeout: Duration);

  pub fn header(self, name: String, value: String) -> Self {
    RequestBuilder {
      inner: self.inner.header(name, value),
    }
  }

  pub fn cookie(self, name: String, value: String) -> Self {
    self.header(SET_COOKIE.to_string(), format!("{}={}", name, value))
  }

  pub async fn send(self) -> Result<Response, crate::Error> {
    self
      .inner
      .send()
      .await
      .map(|inner| Response { inner })
      .map_err(|err| crate::Error::RequestError {
        url: err.url().unwrap().clone().into(),
        message: err.to_string(),
      })
  }
}

#[derive(Any, Debug)]
pub struct Response {
  inner: reqwest::Response,
}

impl Response {
  pub fn url(&self) -> String {
    self.inner.url().to_string()
  }

  pub fn status(&self) -> u16 {
    self.inner.status().as_u16()
  }
  pub async fn text(self) -> String {
    self.inner.text().await.unwrap()
  }

  pub async fn json(self) -> Result<super::Json, crate::Error> {
    Ok(self.inner.json::<serde_json::Value>().await?.into())
  }
}

pub fn load_module() -> Result<Module, ContextError> {
  let module = Module::with_crate_item("mado", &["http"]);

  mado_rune_macros::register_module! {
    (Client) => {
      inst => { get, post, put, delete, head }
      associated => { default: default_ }
    },
    (RequestBuilder) => {
      inst => { query, cookie, header },
      async_inst => { send }
    },
    (Response) => {
      inst => {
        url, status
      },
      async_inst => {
        text, json
      }
    },
    (Url) => {
      associated => { parse },
      inst => {
        to_string, query, clone, path
      }
      protocol => {
        to_string_debug: STRING_DEBUG
      }
    }
  }

  load_module_with(module)
}
