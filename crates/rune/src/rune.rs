use std::sync::Arc;

use rune::{
  runtime::{
    Args, GuardedArgs, RuntimeContext, VmError as RuneVmError, VmSendExecution,
  },
  Context, FromValue, IntoTypeHash, Sources, ToValue, Unit, Vm,
};

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

  /// create new [`rune::Vm`]
  pub fn vm(&self) -> Vm {
    Vm::new(self.runtime.clone(), self.unit.clone())
  }

  /// call [`rune::Vm::send_execute`] then map error with [`Self::convert_result`]
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

  /// convert `value` to R and map error with [`Self::convert_result`]
  pub fn from_value<R, V>(&self, value: V) -> Result<R, VmError>
  where
    R: FromValue,
    V: ToValue,
  {
    let value = self.convert_result(V::to_value(value))?;
    self.convert_result(R::from_value(value))
  }

  /// convert `value` to `rune::Value` and map error with [`Self::convert_result`]
  pub fn to_value<V>(&self, value: V) -> Result<rune::Value, VmError>
  where
    V: rune::ToValue,
  {
    self.convert_result(V::to_value(value))
  }

  /// call [`rune::Vm::call`] and map error with [`Self::convert_result`]
  pub fn call<N, A>(&self, name: N, args: A) -> Result<rune::Value, VmError>
  where
    N: IntoTypeHash,
    A: GuardedArgs,
  {
    self.convert_result(self.vm().call(name, args))
  }

  /// call [`rune::Vm::async_call`] and map error with [`Self::convert_result`]
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
