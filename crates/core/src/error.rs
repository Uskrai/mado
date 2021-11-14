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
  ExternalError(Box<dyn std::error::Error + Send + Sync>),
}
