use std::rc::Rc;

use deno_core::{op2, serde_v8, Extension, OpState, Resource};
use thiserror::Error;

use crate::{ResultJson, ToResultJson};

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
            Error::ModuleLoadError(..) | Error::ResourceError(..) | Error::SerdeError(..) => {
                Self::ExternalError(err.into())
            }
            Error::ExternalError(err) => Self::ExternalError(err),
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
        ErrorJson::Resource {
            types: error.to_string_variant(),
            rid: state.resource_table.add(error),
        }
    }

    pub fn custom(error: impl std::fmt::Display) -> ErrorJson {
        ErrorJson::Custom {
            message: error.to_string(),
        }
    }
}

pub fn error_to_deno(state: &mut OpState, error: Error) -> ErrorJson {
    ErrorJson::from_error(state, error)
}

#[op2]
#[serde]
pub fn op_error_invalid_url(state: &mut OpState, #[string] url: String) -> ErrorJson {
    ErrorJson::from_error(state, Error::InvalidUrl { url })
}

#[op2]
#[string]
pub fn op_error_to_string(state: &mut OpState, #[serde] error: ErrorJson) -> String {
    error.to_string(state)
}

#[op2]
#[string]
pub fn op_error_to_debug(state: &mut OpState, #[serde] error: ErrorJson) -> String {
    error.to_debug(state)
}

#[op2]
#[serde]
pub fn op_error_request_error(
    state: &mut OpState,
    #[string] url: String,
    #[string] message: String,
) -> ErrorJson {
    ErrorJson::from_error(state, Error::RequestError { url, message })
}

#[op2]
#[serde]
pub fn op_error_unexpected_error(
    state: &mut OpState,
    #[string] url: String,
    #[string] message: String,
) -> ErrorJson {
    ErrorJson::from_error(state, Error::UnexpectedError { url, message })
}

#[op2]
#[serde]
pub fn op_error_close(state: &mut OpState, #[serde] error: ErrorJson) -> ResultJson<()> {
    if let ErrorJson::Resource { rid, .. } = error {
        state
            .resource_table
            .take_any(rid)
            .map(|it| it.close())
            .map_err(|_| Error::resource_error(rid, "Error already closed"))
            .to_result_json(state)
    } else {
        ResultJson::Ok(())
    }
}

deno_core::extension!(
    extension,
    ops = [
        op_error_invalid_url,
        op_error_request_error,
        op_error_close,
        op_error_unexpected_error,
        op_error_to_string,
        op_error_to_debug,
    ]
);

pub fn init() -> Extension {
    extension::init_ops_and_esm()
}
