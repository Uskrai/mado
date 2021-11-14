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

/// this is runestick::Any implementation of Error
use super::Error;

impl runestick::Any for Error {
  fn type_hash() -> runestick::Hash {
    runestick::Hash::from_type_id(std::any::TypeId::of::<Self>())
  }
}
impl runestick::InstallWith for Error {
  fn install_with(
    _: &mut runestick::Module,
  ) -> ::std::result::Result<(), runestick::ContextError> {
    Ok(())
  }
}
impl runestick::Named for Error {
  const BASE_NAME: runestick::RawStr = runestick::RawStr::from_str("Error");
}
impl runestick::TypeOf for Error {
  fn type_hash() -> runestick::Hash {
    <Self as runestick::Any>::type_hash()
  }
  fn type_info() -> runestick::TypeInfo {
    runestick::TypeInfo::Any(runestick::RawStr::from_str(
      std::any::type_name::<Self>(),
    ))
  }
}
impl runestick::UnsafeFromValue for &Error {
  type Output = *const Error;
  type Guard = runestick::RawRef;
  fn from_value(
    value: runestick::Value,
  ) -> ::std::result::Result<(Self::Output, Self::Guard), runestick::VmError>
  {
    value.into_any_ptr()
  }
  unsafe fn unsafe_coerce(output: Self::Output) -> Self {
    &*output
  }
}
impl runestick::UnsafeFromValue for &mut Error {
  type Output = *mut Error;
  type Guard = runestick::RawMut;
  fn from_value(
    value: runestick::Value,
  ) -> ::std::result::Result<(Self::Output, Self::Guard), runestick::VmError>
  {
    value.into_any_mut()
  }
  unsafe fn unsafe_coerce(output: Self::Output) -> Self {
    &mut *output
  }
}
impl runestick::UnsafeToValue for &Error {
  type Guard = runestick::SharedPointerGuard;
  unsafe fn unsafe_to_value(
    self,
  ) -> ::std::result::Result<(runestick::Value, Self::Guard), runestick::VmError>
  {
    let (shared, guard) = runestick::Shared::from_ref(self);
    Ok((runestick::Value::from(shared), guard))
  }
}
impl runestick::UnsafeToValue for &mut Error {
  type Guard = runestick::SharedPointerGuard;
  unsafe fn unsafe_to_value(
    self,
  ) -> ::std::result::Result<(runestick::Value, Self::Guard), runestick::VmError>
  {
    let (shared, guard) = runestick::Shared::from_mut(self);
    Ok((runestick::Value::from(shared), guard))
  }
}
