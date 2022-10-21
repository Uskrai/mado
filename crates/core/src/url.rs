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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fill_host_test() {
        assert_eq!(
            fill_host("localhost").unwrap(),
            Url::parse("https://localhost").unwrap()
        );

        assert_eq!(
            fill_host("https://localhost").unwrap(),
            Url::parse("https://localhost").unwrap()
        );

        assert!(matches!(
            fill_host("https://lo:calhost"),
            Err(crate::Error::UrlParseError { .. })
        ));

        assert!(matches!(
            fill_host("://localhost"),
            Err(crate::Error::UrlParseError { .. })
        ))
    }
}
