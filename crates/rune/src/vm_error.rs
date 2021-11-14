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

use std::fmt::Display;

use thiserror::Error;

use runestick::VmError as RuneVmError;

/// this is Error to make [runestick::VmError] more readable
#[derive(Error, Debug)]
pub struct VmError {
  error: RuneVmError,
}

impl From<RuneVmError> for VmError {
  fn from(error: RuneVmError) -> Self {
    Self { error }
  }
}

impl Display for VmError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // match self.error.kind() {
    //   VmErrorKind::Unwound {
    //     kind,
    //     unit,
    //     ip,
    //     frames: _,
    //   } => {
    //     match &**kind {
    //       VmErrorKind::MissingInstanceFunction { hash, instance } => {
    //         return write!(
    //           f,
    //           "missing instance function `{}` for `{}`",
    //           hash, instance
    //         );
    //       }
    //       _ => {}
    //     }
    //     //
    //   }
    //
    //   _ => {}
    // }
    write!(f, "{}", self.error)
  }
}
