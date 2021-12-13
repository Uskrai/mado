use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("error parsing {input} : {source}")]
    UrlParseError {
        input: String,
        source: crate::url::ParseError,
    },

    #[error("Request error from {url}: {message}")]
    RequestError {
        url: crate::url::Url,
        message: String,
    },

    #[error("\"{0}\" are not supported")]
    UnsupportedUrl(String),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    ExternalError(#[from] anyhow::Error),
}
