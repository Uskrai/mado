pub use ::url::*;

pub fn fill_host(input: &str) -> Result<Url, crate::Error> {
  Url::parse(input).or_else(|err| match err {
    ParseError::RelativeUrlWithoutBase => {
      let input = format!("https://{}", input);
      Url::parse(&input).map_err(|_| {
        // return first error
        crate::Error::UrlParseError { input, source: err }
      })
    }
    _ => Err(crate::Error::UrlParseError {
      input: input.to_string(),
      source: err,
    }),
  })
}

pub fn parse_resolve_domain(input: &str) -> Result<Url, ParseError> {
  Url::parse(input).or_else(|err| match err {
    ParseError::RelativeUrlWithoutBase => {
      let input = format!("https://{}", input);
      Url::parse(&input).map_err(|_| {
        // return the first error instead
        err
      })
    }
    _ => Err(err),
  })
}
