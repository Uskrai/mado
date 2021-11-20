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
  runtime::{Args, RuntimeContext, VmError, VmSendExecution},
  Context, IntoTypeHash, Sources, Unit, Vm,
};

#[derive(Clone)]
pub struct Rune {
  sources: Arc<Sources>,
  unit: Arc<Unit>,
  context: Arc<Context>,
  runtime: Arc<RuntimeContext>,
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
    self,
    name: N,
    args: A,
  ) -> Result<VmSendExecution, VmError>
  where
    N: IntoTypeHash,
    A: Send + Args,
  {
    self.vm().send_execute(name, args)
  }
}
