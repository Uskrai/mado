use rune::{runtime::SyncFunction, FromValue};

use crate::{Rune, VmError};

#[derive(Clone)]
pub struct DebugSyncFunction {
  inner: SyncFunction,
}

impl std::ops::Deref for DebugSyncFunction {
  type Target = SyncFunction;
  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl std::fmt::Debug for DebugSyncFunction {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let ptr = &self.inner as *const SyncFunction;
    f.debug_struct("SyncFunction").field("inner", &ptr).finish()
  }
}

impl From<SyncFunction> for DebugSyncFunction {
  fn from(inner: SyncFunction) -> Self {
    Self { inner }
  }
}

impl DebugSyncFunction {
  pub fn into_inner(self) -> SyncFunction {
    self.inner
  }
}

impl FromValue for DebugSyncFunction {
  fn from_value(value: rune::Value) -> Result<Self, rune::runtime::VmError> {
    Ok(value.into_function()?.take()?.into_sync()?.into())
  }
}

/// Rune Function that is send and return human readable error
#[derive(Debug, Clone)]
pub struct RuneFunction {
  rune: Rune,
  fun: DebugSyncFunction,
}

impl RuneFunction {
  pub fn new(rune: Rune, fun: DebugSyncFunction) -> Self {
    Self { rune, fun }
  }
  pub async fn async_call<A, R>(&self, args: A) -> Result<R, VmError>
  where
    R: rune::FromValue,
    A: rune::runtime::Args + Send,
  {
    let exec = self.rune.send_execute(self.fun.type_hash(), args)?;
    let result = exec.async_complete().await;
    let value = self.rune.convert_result(result)?;

    let value = self
      .rune
      .convert_result(rune::FromValue::from_value(value))?;

    Ok(value)
  }
}
