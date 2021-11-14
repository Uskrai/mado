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
