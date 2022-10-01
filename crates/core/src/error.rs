use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("error parsing {input} : {source}")]
    UrlParseError {
        input: String,
        source: crate::url::ParseError,
    },

    #[error("Request error from {url}: {message}")]
    RequestError { url: String, message: String },

    #[error("\"{0}\" are not supported")]
    UnsupportedUrl(String),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    HttpClientError(#[from] crate::http::Error),

    #[error(transparent)]
    ExternalError(#[from] anyhow::Error),
}

impl Error {
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
            UnsupportedUrl { ..},
            RequestError { .. },
            ExternalError(..),
            IOError(..),
            HttpClientError(..)
        }
        .to_string()
    }
}
