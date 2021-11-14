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

use crate::deserializer;
use rune::EmitDiagnostics;
use runestick::{ContextError, Module};
use thiserror::Error;

use super::http::Url;

/// error happen when loading [`rune`] script with its diagnostics
#[derive(Debug)]
pub struct LoadSourcesError {
  sources: rune::Sources,
  diagnostics: rune::Diagnostics,
  error: rune::LoadSourcesError,
}

impl std::error::Error for LoadSourcesError {}

impl LoadSourcesError {
  pub fn new(
    error: rune::LoadSourcesError,
    diagnostics: rune::Diagnostics,
    sources: rune::Sources,
  ) -> Self {
    Self {
      error,
      diagnostics,
      sources,
    }
  }
}

impl Display for LoadSourcesError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut writer = rune::termcolor::Buffer::no_color();
    self
      .diagnostics
      .emit_diagnostics(&mut writer, &self.sources)
      .unwrap();
    write!(f, "{}", String::from_utf8_lossy(writer.as_slice()))
  }
}

/// Error from [`rune`]
#[derive(Error, Debug)]
pub enum RuneError {
  #[error("{0}")]
  VmError(
    #[from]
    #[source]
    runestick::VmError,
  ),

  #[error("{0}")]
  AccessError(
    #[source]
    #[from]
    runestick::AccessError,
  ),

  #[error("{0}")]
  LoadSourcesError(#[from] LoadSourcesError),

  #[error("load_module function doesn't exists inside vm")]
  MissingLoadModuleFn,

  #[error("Expected {0}, found {1}")]
  Expected(String, String),
}

#[derive(Error, Debug)]
pub enum Error {
  #[error("{0}")]
  RuneError(#[source] RuneError),

  #[error("{input}: {source}")]
  UrlParseError {
    input: String,
    source: mado_core::url::ParseError,
  },

  #[error("{url} is invalid")]
  InvalidUrl { url: Url },

  /// request is success but the response format is not
  /// as you want
  /// example: server respond request with json key named "result"
  /// but when using serde_json::Value::pointer the value does not exists
  /// then this should be used.
  #[error("Unexpected response from {url}: {message}")]
  UnexpectedError { url: Url, message: String },

  #[error("Request error from {url}: {message}")]
  RequestError { url: Url, message: String },

  #[error(transparent)]
  DeserializeError {
    #[from]
    source: deserializer::Error,
  },

  #[error("{0}")]
  ExternalError(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),

  #[error("{0}")]
  JsonPathError(
    #[source]
    #[from]
    jsonpath_lib::JsonPathError,
  ),

  #[error("{0}")]
  SerdeJsonError(
    #[source]
    #[from]
    serde_json::Error,
  ),

  #[error("{0}")]
  ReqwestError(
    #[source]
    #[from]
    reqwest::Error,
  ),
}

impl From<Error> for mado_core::Error {
  fn from(err: Error) -> Self {
    match err {
      Error::RuneError(_) => Self::ExternalError(Box::new(err)),
      Error::UrlParseError { input, source } => {
        Self::UrlParseError { input, source }
      }
      Error::InvalidUrl { url } => Self::RequestError {
        url: url.into(),
        message: "Invalid Link".into(),
      },
      Error::UnexpectedError { url, message } => Self::RequestError {
        url: url.into(),
        message,
      },
      Error::RequestError { url, message } => Self::RequestError {
        url: url.into(),
        message,
      },
      Error::DeserializeError { .. }
      | Error::ExternalError(..)
      | Error::JsonPathError(..)
      | Error::SerdeJsonError(..)
      | Error::ReqwestError(..) => Self::ExternalError(Box::new(err)),
    }
  }
}

impl<T> From<T> for Error
where
  T: Into<RuneError>,
{
  fn from(v: T) -> Self {
    Self::RuneError(v.into())
  }
}

impl From<runestick::VmErrorKind> for Error {
  fn from(v: runestick::VmErrorKind) -> Self {
    Self::RuneError(runestick::VmError::from(v).into())
  }
}

impl Error {
  pub fn url_parse_error(input: String, source: url::ParseError) -> Error {
    Self::UrlParseError { input, source }
  }

  pub fn invalid_url(url: Url) -> Error {
    Self::InvalidUrl { url }
  }

  pub fn unexpected_response(url: Url, message: String) -> Error {
    Self::UnexpectedError { url, message }
  }

  pub fn expected(expected: String, found: String) -> Self {
    Self::RuneError(RuneError::Expected(expected, found))
  }

  pub fn request_error(url: Url, message: String) -> Self {
    Self::RequestError { url, message }
  }

  pub fn to_string_debug(&self, s: &mut String) -> std::fmt::Result {
    use std::fmt::Write;
    write!(s, "{:?}", self)
  }

  pub fn to_string_display(&self, s: &mut String) -> std::fmt::Result {
    use std::fmt::Write;
    write!(s, "{}", self)
  }

  pub fn to_string_variant(&self) -> String {
    macro_rules! match_var {
      ($id:ident (..)) => {
        Self::$id(..)
      };
      ($id:ident {..}) => {
        Self::$id { .. }
      };
    }

    macro_rules! variant {
      ($($name:ident $tt:tt),+) => {
        match self {
          $(match_var!($name $tt) => {
            stringify!($name).to_string()
          }),+
        }
      };
    }

    variant! {
      RuneError(..), ExternalError(..),
      UrlParseError{..}, InvalidUrl{..}, UnexpectedError{..},
      RequestError{..}, DeserializeError{..},
      JsonPathError(..), SerdeJsonError(..), ReqwestError(..)
    }
  }
}

pub fn load_module() -> Result<Module, ContextError> {
  let module = Module::with_crate_item("mado", &["error"]);

  mado_rune_macros::register_module! {
    (Error) => {
      associated => {
        invalid_url, unexpected_response, request_error
      }
      inst => {
        to_string, to_string_variant
      }
      protocol => {
        to_string_debug: STRING_DEBUG,
        to_string_display: STRING_DISPLAY
      }
    }
  };

  load_module_with(module)
}
