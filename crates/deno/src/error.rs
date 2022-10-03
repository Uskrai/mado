use std::rc::Rc;

use deno_core::{op, Extension, ExtensionBuilder, OpState};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{input}: {source}")]
    UrlParseError {
        input: String,
        source: mado_core::url::ParseError,
    },

    #[error("{url} is invalid")]
    InvalidUrl { url: String },

    /// request is success but the response format is not
    /// as you want
    /// example: server respond request with json key named "result"
    /// but when using serde_json::Value::pointer the value does not exists
    /// then this should be used.
    #[error("Unexpected response from {url}: {message}")]
    UnexpectedError { url: String, message: String },

    #[error("Request error from {url}: {message}")]
    RequestError { url: String, message: String },

    #[error("{0}")]
    ExternalError(
        #[source]
        #[from]
        anyhow::Error,
    ),

    #[error("{0}")]
    SerdeError(
        #[source]
        #[from]
        serde_v8::Error,
    ),

    #[error("Bad resource ID ({1}): {0}")]
    ResourceError(u32, String),

    #[error("{0}")]
    ModuleLoadError(#[from] crate::runtime::ModuleLoadError),

    #[error("{0}")]
    MadoError(#[from] mado_core::Error),
}
impl deno_core::Resource for Error {}

impl Error {
    pub fn resource_error(rid: u32, message: impl ToString) -> Self {
        Self::ResourceError(rid, message.to_string())
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
                    stringify!($name)
                  }),+
                }
          };
        }

        variant! {
            UrlParseError { .. },
            InvalidUrl { ..},
            UnexpectedError { .. },
            RequestError { .. },
            ExternalError(..),
            SerdeError(..),
            ModuleLoadError(..),
            ResourceError(..),
            MadoError(..)
        }
        .to_string()
    }
}

impl From<mado_core::http::Error> for Error {
    fn from(e: mado_core::http::Error) -> Self {
        Self::MadoError(e.into())
    }
}

impl From<Error> for mado_core::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::UrlParseError { input, source } => Self::UrlParseError { input, source },
            Error::InvalidUrl { url } => Self::RequestError {
                url,
                message: "Invalid Link".into(),
            },
            Error::UnexpectedError { url, message } => Self::RequestError { url, message },
            Error::RequestError { url, message } => Self::RequestError { url, message },
            Error::MadoError(err) => err,
            Error::ExternalError(..)
            | Error::ModuleLoadError(..)
            | Error::ResourceError(..)
            | Error::SerdeError(..) => Self::ExternalError(err.into()),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(tag = "type", content = "content")]
pub enum ErrorJson {
    Resource { rid: u32, types: String },
    Custom { message: String },
}

impl ErrorJson {
    pub fn take(self, state: &mut OpState) -> Error {
        match self {
            Self::Resource { rid, .. } => {
                let it = state
                    .resource_table
                    .take::<Error>(rid)
                    .map_err(Into::into)
                    .and_then(|it| {
                        Rc::try_unwrap(it).map_err(|_| {
                            Error::ExternalError(anyhow::anyhow!("cannot unwrap error"))
                        })
                    });

                match it {
                    Ok(it) | Err(it) => it,
                }
            }
            Self::Custom { message } => Error::ExternalError(anyhow::anyhow!(message)),
        }
    }

    pub fn to_string(self, state: &mut OpState) -> String {
        match self {
            Self::Resource { rid, .. } => state
                .resource_table
                .get::<Error>(rid)
                .map_err(|err| err.to_string())
                .map(|it| it.to_string())
                .unwrap_or_else(|it| it),
            Self::Custom { message } => message,
        }
    }

    pub fn to_debug(self, state: &mut OpState) -> String {
        match self {
            Self::Resource { rid, .. } => state
                .resource_table
                .get::<Error>(rid)
                .map_err(|err| format!("{:?}", err))
                .map(|it| format!("{:?}", it))
                .unwrap_or_else(|it| it),
            Self::Custom { message } => message,
        }
    }

    pub fn from_error(state: &mut OpState, error: Error) -> ErrorJson {
        error_to_deno(state, error)
    }
}

pub fn error_to_deno(state: &mut OpState, error: Error) -> ErrorJson {
    ErrorJson::Resource {
        types: error.to_string_variant(),
        rid: state.resource_table.add(error),
    }
}

#[op]
pub fn op_error_invalid_url(state: &mut OpState, url: String) -> ErrorJson {
    error_to_deno(state, Error::InvalidUrl { url })
}

#[op]
pub fn op_error_to_string(state: &mut OpState, error: ErrorJson) -> String {
    error.to_string(state)
}

#[op]
pub fn op_error_to_debug(state: &mut OpState, error: ErrorJson) -> String {
    error.to_debug(state)
}

#[op]
pub fn op_error_request_error(state: &mut OpState, url: String, message: String) -> ErrorJson {
    error_to_deno(state, Error::RequestError { url, message })
}

#[op]
pub fn op_error_unexpected_error(state: &mut OpState, url: String, message: String) -> ErrorJson {
    error_to_deno(state, Error::UnexpectedError { url, message })
}

#[op]
pub fn op_error_close(state: &mut OpState, error: ErrorJson) {
    if let ErrorJson::Resource { rid, .. } = error {
        state.resource_table.close(rid).unwrap();
    }
}

pub fn init() -> Extension {
    ExtensionBuilder::default()
        .ops(vec![
            op_error_invalid_url::decl(),
            op_error_request_error::decl(),
            op_error_close::decl(),
            op_error_unexpected_error::decl(),
            op_error_to_string::decl(),
            op_error_to_debug::decl(),
        ])
        .build()
}
