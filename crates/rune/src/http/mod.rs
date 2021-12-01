use std::time::Duration;

use reqwest::header::SET_COOKIE;
use rune::{Any, ContextError, Module};

mod client;
mod url;

pub use self::url::Url;
pub use client::Client;

macro_rules! wrapper_fun {
  ($name:ident, $( $param:ident : $type:ty ),*) => {
    pub fn $name(self, $($param : $type),*) -> Self {
      Self {
        inner: self.inner.$name($($param),*)
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
        self.inner
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

#[derive(Any, Debug)]
pub struct StatusCode(reqwest::StatusCode);

impl StatusCode {
    pub fn as_string(&self) -> String {
        self.0.as_str().to_string()
    }

    pub fn as_u16(&self) -> u16 {
        self.0.as_u16()
    }

    pub fn is_client_error(&self) -> bool {
        self.0.is_client_error()
    }

    pub fn is_server_error(&self) -> bool {
        self.0.is_server_error()
    }

    pub fn is_informational(&self) -> bool {
        self.0.is_informational()
    }

    pub fn is_success(&self) -> bool {
        self.0.is_success()
    }

    pub fn is_redirection(&self) -> bool {
        self.0.is_redirection()
    }
}

impl Response {
    pub fn url(&self) -> String {
        self.inner.url().to_string()
    }

    pub fn status(&self) -> StatusCode {
        StatusCode(self.inner.status())
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
        inst => {
          get, post, put, delete, head
        }
        associated => {
          default: default_
        }
      },
      (RequestBuilder) => {
        inst => {
          query, cookie, header
        },
        async_inst => {
          send
        }
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
        associated => {
          parse
        }
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
