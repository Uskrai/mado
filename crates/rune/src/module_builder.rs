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

use std::path::Path;

use runestick::{FromValue, Source, Vm};

use crate::{Error, RuneError, SendValue, VmBuilder, WebsiteModule};

#[derive(Default)]
pub struct WebsiteModuleBuilder {
  vm_builder: VmBuilder,
  raise_missing_load_module: bool,
}

impl WebsiteModuleBuilder {
  pub fn new(vm_builder: VmBuilder) -> Self {
    Self {
      vm_builder,
      ..Default::default()
    }
  }

  pub fn vm_builder(&mut self) -> &mut VmBuilder {
    &mut self.vm_builder
  }

  /// Raise error when load_module function inside rune script doesn't exists
  /// default to false which ignore the script and return empty vector
  pub fn error_on_missing_load_module(&mut self, cond: bool) -> &mut Self {
    self.raise_missing_load_module = cond;
    self
  }

  pub fn load_source(
    &self,
    source: Source,
  ) -> Result<Vec<WebsiteModule>, Error> {
    let vm = self.vm_builder.load_vm_from_source(source)?;

    self.load_vm(vm)
  }

  pub fn load_path(&self, path: &Path) -> Result<Vec<WebsiteModule>, Error> {
    let vm = self.vm_builder.load_vm_from_path(path)?;

    self.load_vm(vm)
  }

  pub fn load_vm(&self, mut vm: Vm) -> Result<Vec<WebsiteModule>, Error> {
    let hash = runestick::Hash::type_hash(&["load_module"]);
    let fun = vm.unit().lookup(hash);

    if fun.is_none() {
      if self.raise_missing_load_module {
        return Err(RuneError::MissingLoadModuleFn.into());
      } else {
        return Ok(Vec::new());
      }
    }

    let result = vm.execute(hash, ())?.complete()?;

    let result = SendValue::from_value(result)?;
    result.try_into()
  }
}
