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

use std::sync::Arc;

use rune::{
  runtime::{
    Args, GuardedArgs, RuntimeContext, VmError as RuneVmError, VmSendExecution,
  },
  Context, FromValue, IntoTypeHash, Sources, Unit, Vm,
};

pub trait FromRuneValue: 'static + Sized {
  fn from_rune_value(rune: Rune, value: rune::Value) -> Result<Self, VmError>;
}

impl<T> FromRuneValue for T
where
  T: FromValue,
{
  fn from_rune_value(rune: Rune, value: rune::Value) -> Result<Self, VmError> {
    rune.convert_result(FromValue::from_value(value))
  }
}

use crate::VmError;

#[derive(Clone, Debug)]
pub struct Rune {
  pub sources: Arc<Sources>,
  pub unit: Arc<Unit>,
  pub context: Arc<Context>,
  pub runtime: Arc<RuntimeContext>,
}

const _: () = {
  pub fn assert_send<T: Send>() {}
  pub fn assert_sync<T: Sync>() {}

  fn assert_all() {
    assert_send::<Rune>();
    assert_sync::<Rune>();
  }
};

impl Rune {
  pub fn new(
    context: Arc<Context>,
    unit: Arc<Unit>,
    sources: Arc<Sources>,
  ) -> Self {
    let runtime = Arc::new(context.runtime());
    Self {
      runtime,
      context,
      unit,
      sources,
    }
  }
  pub fn vm(&self) -> Vm {
    Vm::new(self.runtime.clone(), self.unit.clone())
  }

  pub fn send_execute<N, A>(
    &self,
    name: N,
    args: A,
  ) -> Result<VmSendExecution, VmError>
  where
    N: IntoTypeHash,
    A: Send + Args,
  {
    self.convert_result(self.vm().send_execute(name, args))
  }

  /// convert [`rune::runtime::VmError`] to [`crate::VmError`]
  pub fn convert_vm_error(&self, error: RuneVmError) -> VmError {
    crate::error::VmError::new(self.sources.clone(), error)
  }

  /// convert [`Result<T, rune::runtime::VmError>`] to [`Result<T, crate::VmError>`]
  pub fn convert_result<T>(
    &self,
    result: Result<T, RuneVmError>,
  ) -> Result<T, VmError> {
    match result {
      Ok(value) => Ok(value),
      Err(err) => Err(self.convert_vm_error(err)),
    }
  }

  pub fn from_value<R, V>(&self, value: V) -> Result<R, VmError>
  where
    R: FromValue,
    V: ToValue,
  {
    let value = self.convert_result(V::to_value(value))?;
    self.convert_result(R::from_value(value))
  }

  pub fn to_value<V>(&self, value: V) -> Result<rune::Value, VmError>
  where
    V: rune::ToValue,
  {
    self.convert_result(V::to_value(value))
  }

  pub fn call<N, A>(&self, name: N, args: A) -> Result<rune::Value, VmError>
  where
    N: IntoTypeHash,
    A: GuardedArgs,
  {
    self.convert_result(self.vm().call(name, args))
  }

  pub async fn async_call<R, N, A>(
    &self,
    name: N,
    args: A,
  ) -> Result<R, VmError>
  where
    N: IntoTypeHash,
    A: GuardedArgs,
    R: FromValue,
  {
    let result = self.vm().async_call(name, args).await;
    let result = self.convert_result(result)?;
    self.convert_result(FromValue::from_value(result))
  }
}
