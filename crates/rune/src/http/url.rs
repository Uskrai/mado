use rune::Any;
use std::{fmt::Display, str::FromStr};

use crate::Error;

#[derive(Any, Debug, Clone)]
pub struct Url {
    inner: mado_core::Url,
}

impl Url {
    pub fn new(url: mado_core::Url) -> Self {
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
        // parsing as mado_core::Url because runes::Error will complicate the process
        let url = mado_core::Url::parse(input);
        url.or_else(|err| match err {
            ParseError::RelativeUrlWithoutBase => {
                let input = "https://".to_owned() + input;
                mado_core::Url::parse(&input)
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

    pub fn extension(self) -> Option<String> {
        let path = self.inner.path();
        let path = std::path::Path::new(path);
        path.extension().map(|it| it.to_string_lossy().to_string())
    }
}

impl Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl From<Url> for mado_core::Url {
    fn from(v: Url) -> Self {
        v.inner
    }
}

impl From<mado_core::Url> for Url {
    fn from(v: mado_core::Url) -> Self {
        Self::new(v)
    }
}

impl FromStr for Url {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
