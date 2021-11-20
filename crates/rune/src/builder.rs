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

use crate::{error::BuildError, RuneError, SendValue, SourceLoader};
use rune::{
  compile::{CompileVisitor, SourceLoader as RuneSourceLoader},
  Context, ContextError, Diagnostics, FromValue, Options, Source, Sources,
};

use super::Error;
use rune::runtime::{RuntimeContext, Unit, Vm};

use super::WebsiteModule;
use std::{path::Path, sync::Arc};

pub struct NoopCompileVisitor;
impl CompileVisitor for NoopCompileVisitor {}

pub fn create_context() -> Result<Context, ContextError> {
  let mut context = Context::with_default_modules()?;
  super::load_modules(&mut context)?;
  Ok(context)
}

#[derive(Default)]
pub struct Build<'a> {
  context: Option<&'a Context>,
  visitor: Option<&'a mut dyn CompileVisitor>,
  options: Option<&'a Options>,
  source_loader: Option<&'a mut dyn RuneSourceLoader>,
}

impl<'a> Build<'a> {
  #[inline(always)]
  pub fn context(mut self, context: &'a Context) -> Self {
    self.context = Some(context);
    self
  }

  #[inline(always)]
  pub fn visitor(mut self, visitor: &'a mut dyn CompileVisitor) -> Self {
    self.visitor = Some(visitor);
    self
  }

  #[inline(always)]
  pub fn options(mut self, options: &'a Options) -> Self {
    self.options = Some(options);
    self
  }

  #[inline(always)]
  pub fn source_loader(
    mut self,
    source_loader: &'a mut dyn RuneSourceLoader,
  ) -> Self {
    self.source_loader = Some(source_loader);
    self
  }

  #[inline(always)]
  pub fn with_source(self, source: Source) -> SourceBuild<'a> {
    SourceBuild::new(self, source)
  }

  #[inline(always)]
  pub fn with_path(
    self,
    path: &Path,
  ) -> Result<SourceBuild<'a>, std::io::Error> {
    Ok(self.with_source(Source::from_path(path)?))
  }

  #[inline(always)]
  pub fn build_unit(self, mut sources: Sources) -> Result<Unit, BuildError> {
    let Build {
      context,
      visitor,
      options,
      source_loader,
    } = self;
    let mut diagnostics = Diagnostics::new();

    let build = rune::prepare(&mut sources);
    let build = match context {
      Some(context) => build.with_context(context),
      None => build.with_context(Self::default_context()),
    };

    let build = match visitor {
      Some(visitor) => build.with_visitor(visitor),
      None => build,
    };

    let build = match options {
      Some(options) => build.with_options(options),
      None => build,
    };

    let mut loader: SourceLoader = SourceLoader::default();

    let build = match source_loader {
      Some(source_loader) => build.with_source_loader(source_loader),
      None => build.with_source_loader(&mut loader),
    };

    build
      .with_diagnostics(&mut diagnostics)
      .build()
      .map_err(|_| BuildError::new(diagnostics, sources))
  }

  #[inline(always)]
  pub fn build_vm(self, sources: Sources) -> Result<Vm, BuildError> {
    let unit = self.build_unit(sources)?;

    Ok(Vm::new(
      Arc::new(Self::default_context_runtime()),
      Arc::new(unit),
    ))
  }

  #[inline(always)]
  pub fn build_for_module(
    self,
    sources: Sources,
  ) -> Result<ModuleBuild, BuildError> {
    let vm = self.build_vm(sources)?;
    Ok(ModuleBuild::new(vm))
  }

  #[inline(always)]
  pub fn default_source_loader() -> &'static SourceLoader {
    lazy_static::lazy_static! {
      static ref LOADER: SourceLoader = SourceLoader::default();
    }
    &LOADER
  }

  #[inline(always)]
  pub fn default_context() -> &'static Context {
    lazy_static::lazy_static! {
      static ref CONTEXT: Context = create_context().unwrap();
    }
    &CONTEXT
  }

  #[inline(always)]
  pub fn default_context_runtime() -> RuntimeContext {
    Self::default_context().runtime()
  }
}

pub struct SourceBuild<'a> {
  build: Build<'a>,
  sources: Sources,
}

impl<'a> SourceBuild<'a> {
  #[inline(always)]
  pub fn new(build: Build<'a>, source: Source) -> Self {
    let mut sources = Sources::new();
    sources.insert(source);
    Self { build, sources }
  }

  #[inline(always)]
  pub fn build_unit(self) -> Result<Unit, BuildError> {
    self.build.build_unit(self.sources)
  }

  #[inline(always)]
  pub fn build_vm(self) -> Result<Vm, BuildError> {
    self.build.build_vm(self.sources)
  }

  #[inline(always)]
  pub fn build_for_module(self) -> Result<ModuleBuild, BuildError> {
    self.build.build_for_module(self.sources)
  }
}

#[derive(Clone)]
pub struct ModuleBuild {
  vm: Vm,
  error_missing_load_module: bool,
}

impl ModuleBuild {
  #[inline(always)]
  pub fn new(vm: Vm) -> Self {
    Self {
      vm,
      error_missing_load_module: true,
    }
  }

  #[inline(always)]
  pub fn error_missing_load_module(mut self, error: bool) -> Self {
    self.error_missing_load_module = error;
    self
  }

  #[inline(always)]
  pub fn build(self) -> Result<Vec<WebsiteModule>, Error> {
    let mut vm = self.vm;
    let hash = rune::Hash::type_hash(&["load_module"]);
    let fun = vm.unit().lookup(hash);

    if fun.is_none() {
      if self.error_missing_load_module {
        return Err(RuneError::MissingLoadModuleFn.into());
      } else {
        return Ok(Vec::new());
      }
    }

    let result = vm.execute(hash, ())?.complete()?;

    SendValue::from_value(result)?.try_into()
  }
}
