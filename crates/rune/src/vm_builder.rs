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

use crate::{error::LoadSourcesError, SendValue, SourceLoader};
use rune::SourceLoader as RuneSourceLoader;

use super::Error;
use rune::{CompileVisitor, NoopCompileVisitor};
use runestick::{
  Context, ContextError, FromValue, RuntimeContext, Source, Unit, Vm,
};

use super::WebsiteModule;
use std::{path::Path, sync::Arc};

pub fn create_context() -> Result<Context, ContextError> {
  let mut context = Context::with_default_modules()?;
  super::load_modules(&mut context)?;
  Ok(context)
}

pub struct VmBuilder {
  context: Context,
  options: rune::Options,
  source_loader: std::rc::Rc<dyn RuneSourceLoader>,
  compile_visitor: std::rc::Rc<dyn CompileVisitor>,
}

impl Default for VmBuilder {
  fn default() -> Self {
    Self::new()
  }
}

impl VmBuilder {
  /// construct new builder
  /// # Panics
  /// Panic if [`runestick::ContextError`] is raised from [`create_context`]
  pub fn new() -> VmBuilder {
    Self::try_new().unwrap()
  }

  pub fn try_new() -> Result<VmBuilder, ContextError> {
    let context = create_context()?;
    let options = rune::Options::default();
    let source_loader = std::rc::Rc::new(SourceLoader::new());
    let compile_visitor = std::rc::Rc::new(NoopCompileVisitor::new());

    Ok(Self {
      context,
      options,
      source_loader,
      compile_visitor,
    })
  }

  /// get mutable reference to [`runestick::Context`] that will be passed to [`rune::load_sources_with_visitor`]
  pub fn context(&mut self) -> &mut Context {
    &mut self.context
  }

  /// get [`runestick::RuntimeContext`] that can be passed to [`runestick::Vm`]
  pub fn context_runtime(&self) -> RuntimeContext {
    self.context.runtime()
  }

  /// get mutable reference to [`rune::Options`] that will be passed to [`rune::load_sources_with_visitor`]
  pub fn options(&mut self) -> &mut rune::Options {
    &mut self.options
  }

  pub fn set_compile_visitor(
    &mut self,
    visitor: std::rc::Rc<dyn CompileVisitor>,
  ) -> &mut Self {
    self.compile_visitor = visitor;
    self
  }

  /// change [`rune::SourceLoader`] passed to [`rune::load_sources_with_visitor()`]
  pub fn set_source_loader(
    &mut self,
    source_loader: std::rc::Rc<dyn RuneSourceLoader>,
  ) -> &mut Self {
    self.source_loader = source_loader;
    self
  }

  /// load multiple source
  pub fn load_sources(
    &self,
    mut sources: rune::Sources,
  ) -> Result<Unit, LoadSourcesError> {
    let mut diagnostics = rune::Diagnostics::new();
    let unit = rune::load_sources_with_visitor(
      &self.context,
      &self.options,
      &mut sources,
      &mut diagnostics,
      self.compile_visitor.clone(),
      self.source_loader.clone(),
    );

    match unit {
      Ok(unit) => Ok(unit),
      Err(err) => Err(LoadSourcesError::new(err, diagnostics, sources)),
    }
  }

  /// load single source
  pub fn load_source(&self, source: Source) -> Result<Unit, LoadSourcesError> {
    let mut sources = rune::Sources::new();
    sources.insert(source);
    self.load_sources(sources)
  }

  /// Load unit from path
  /// # Panics
  /// Panics if path doesn't exists
  pub fn load_path(&self, path: &Path) -> Result<Unit, Error> {
    let source = runestick::Source::from_path(path);

    let source = match source {
      Ok(v) => v,
      Err(err) => {
        return Err(Error::ExternalError(Box::new(err)));
      }
    };

    Ok(self.load_source(source)?)
  }

  pub fn load_vm_from_source(
    &self,
    source: Source,
  ) -> Result<Vm, LoadSourcesError> {
    let unit = self.load_source(source)?;
    Ok(Vm::new(Arc::new(self.context_runtime()), Arc::new(unit)))
  }

  pub fn load_vm_from_path(&self, path: &Path) -> Result<Vm, Error> {
    let unit = self.load_path(path)?;
    Ok(Vm::new(Arc::new(self.context_runtime()), Arc::new(unit)))
  }

  pub async fn load_website_module_from(
    &self,
    vm: Vm,
  ) -> Result<WebsiteModule, Error> {
    let execution = vm.send_execute(&["load_module"], ())?;
    let v = execution.async_complete().await?;
    let v = SendValue::from_value(v)?;
    let module = v.try_into()?;
    Ok(module)
  }
}

#[cfg(test)]
mod test {
  #[test]
  fn test_load() {
    super::VmBuilder::new();
  }
}
