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

use runestick::{Any, Named, Protocol, RawStr};
use std::fmt::{Debug, Display, Write};

#[derive(Any, thiserror::Error, Debug)]
pub struct ErrorWrapper<T>
where
  T: std::error::Error + runestick::Named + Send + Sync + 'static,
{
  #[source]
  #[from]
  inner: T,
}

#[derive(thiserror::Error, Debug)]
#[error("{inner}")]
pub struct NamedErrorWrapper<T>
where
  T: std::error::Error + Send + Sync + 'static,
{
  #[source]
  #[from]
  inner: T,
}

impl<T> Display for ErrorWrapper<T>
where
  T: std::error::Error + runestick::Named + Send + Sync + 'static,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Display::fmt(&self.inner, f)
  }
}

impl<T> Named for NamedErrorWrapper<T>
where
  T: std::error::Error + Send + Sync + 'static,
{
  const BASE_NAME: runestick::RawStr = RawStr::from_str("Any");
  fn full_name() -> String {
    "Any".to_string()
  }
}

impl<T> From<T> for ErrorWrapper<NamedErrorWrapper<T>>
where
  T: std::error::Error + Send + Sync + 'static,
{
  fn from(v: T) -> Self {
    Self { inner: v.into() }
  }
}

impl<T> ErrorWrapper<T>
where
  T: std::error::Error + runestick::Named + Send + Sync + 'static,
{
  pub fn display(&self, buf: &mut String) -> std::fmt::Result {
    write!(buf, "{}", self)
  }
}

pub fn register_error<T>(
  module: &mut runestick::Module,
) -> Result<(), runestick::ContextError>
where
  T: std::error::Error + runestick::Named + Send + Sync + 'static,
{
  module.ty::<ErrorWrapper<T>>()?;
  module.inst_fn(Protocol::STRING_DISPLAY, ErrorWrapper::<T>::display)?;
  module.inst_fn(Protocol::STRING_DEBUG, ErrorWrapper::<T>::display)?;

  Ok(())
}

pub fn register_unnamed_error<T>(
  module: &mut runestick::Module,
) -> Result<(), runestick::ContextError>
where
  T: std::error::Error + Send + Sync + 'static,
{
  register_error::<NamedErrorWrapper<T>>(module)
}
