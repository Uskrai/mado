use std::{fmt::Display, sync::Arc};

use crate::deserializer;
use rune::{ContextError, Module};
use thiserror::Error;

use super::http::Url;

/// error happen when loading [`rune`] script with its diagnostics
#[derive(Debug)]
pub struct BuildError {
    sources: rune::Sources,
    diagnostics: rune::Diagnostics,
}

impl std::error::Error for BuildError {}

impl BuildError {
    pub fn new(diagnostics: rune::Diagnostics, sources: rune::Sources) -> Self {
        Self {
            diagnostics,
            sources,
        }
    }
}

impl Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut writer = rune::termcolor::Buffer::no_color();
        self.diagnostics.emit(&mut writer, &self.sources).unwrap();
        write!(f, "{}", String::from_utf8_lossy(writer.as_slice()))
    }
}

#[derive(Debug)]
pub struct VmError {
    sources: Arc<rune::Sources>,
    error: rune::runtime::VmError,
}

impl VmError {
    pub fn new(sources: Arc<rune::Sources>, error: rune::runtime::VmError) -> Self {
        Self { error, sources }
    }
}

impl std::error::Error for VmError {}
impl Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let buffer = rune::termcolor::Buffer::no_color();
        let mut writer = rune::termcolor::Ansi::new(buffer);
        self.error.emit(&mut writer, &self.sources).unwrap();
        write!(
            f,
            "{}",
            String::from_utf8_lossy(writer.get_ref().as_slice())
        )
    }
}

/// Error from [`rune`]
#[derive(Error, Debug)]
pub enum RuneError {
    #[error("{0}")]
    VmError(
        #[from]
        #[source]
        VmError,
    ),

    #[error("{0}")]
    AccessError(
        #[source]
        #[from]
        rune::runtime::AccessError,
    ),

    #[error("{0}")]
    LoadSourcesError(#[from] BuildError),

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
    ExternalError(
        #[source]
        #[from]
        anyhow::Error,
    ),

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
            Error::RuneError(_) => Self::ExternalError(err.into()),
            Error::UrlParseError { input, source } => Self::UrlParseError { input, source },
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
            | Error::ReqwestError(..) => Self::ExternalError(err.into()),
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
