use crate::{error::BuildError, Rune, RuneError, SendValue, SourceLoader};
use rune::{
  compile::{CompileVisitor, SourceLoader as RuneSourceLoader},
  Context, ContextError, Diagnostics, Options, Source, Sources,
};

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
  context: Option<Arc<Context>>,
  visitor: Option<&'a mut dyn CompileVisitor>,
  options: Option<&'a Options>,
  source_loader: Option<&'a mut dyn RuneSourceLoader>,
}

impl<'a> Build<'a> {
  #[inline(always)]
  pub fn context(mut self, context: Arc<Context>) -> Self {
    self.context = Some(context);
    self
  }

  pub fn get_context_or_default(&self) -> Arc<Context> {
    match &self.context {
      Some(c) => c.clone(),
      None => Self::default_context(),
    }
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
  fn build_unit_diagnostics(
    self,
    sources: &mut Sources,
  ) -> Result<Unit, Diagnostics> {
    let build = rune::prepare(sources);

    // initialize context first to make borrow checker happy
    let context = self.get_context_or_default();
    let build = build.with_context(&context);

    let Build {
      // we don't need context anymore
      context: _,
      visitor,
      options,
      source_loader,
    } = self;

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

    let mut diagnostics = Diagnostics::new();
    build
      .with_diagnostics(&mut diagnostics)
      .build()
      .map_err(|_| diagnostics)
  }

  pub fn build_unit(self, mut sources: Sources) -> Result<Unit, BuildError> {
    self
      .build_unit_diagnostics(&mut sources)
      .map_err(|e| BuildError::new(e, sources))
  }

  #[inline(always)]
  pub fn build_vm(self, sources: Sources) -> Result<Vm, BuildError> {
    let unit = self.build_unit(sources)?;

    Ok(Vm::new(
      Arc::new(Self::default_context_runtime()),
      Arc::new(unit),
    ))
  }

  pub fn build(self, mut sources: Sources) -> Result<Rune, BuildError> {
    let context = self.get_context_or_default();

    // make borrow checker happy
    let unit = match self.build_unit_diagnostics(&mut sources) {
      Ok(unit) => unit,
      Err(d) => return Err(BuildError::new(d, sources)),
    };

    let unit = Arc::new(unit);
    let sources = Arc::new(sources);

    Ok(Rune::new(context, unit, sources))
  }

  #[inline(always)]
  pub fn build_for_module(
    self,
    sources: Sources,
  ) -> Result<ModuleBuild, BuildError> {
    Ok(ModuleBuild::new(self.build(sources)?))
  }

  #[inline(always)]
  pub fn default_source_loader() -> &'static SourceLoader {
    lazy_static::lazy_static! {
      static ref LOADER: SourceLoader = SourceLoader::default();
    }
    &LOADER
  }

  #[inline(always)]
  pub fn default_context() -> Arc<Context> {
    lazy_static::lazy_static! {
      static ref CONTEXT: Arc<Context> = Arc::new(create_context().unwrap());
    }
    CONTEXT.clone()
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

  pub fn build(self) -> Result<Rune, BuildError> {
    self.build.build(self.sources)
  }

  #[inline(always)]
  pub fn build_for_module(self) -> Result<ModuleBuild, BuildError> {
    self.build.build_for_module(self.sources)
  }
}

#[derive(Clone)]
pub struct ModuleBuild {
  rune: Rune,
  error_missing_load_module: bool,
}

impl ModuleBuild {
  #[inline(always)]
  pub fn new(rune: Rune) -> Self {
    Self {
      rune,
      error_missing_load_module: false,
    }
  }

  #[inline(always)]
  pub fn error_missing_load_module(mut self, error: bool) -> Self {
    self.error_missing_load_module = error;
    self
  }

  #[inline(always)]
  pub fn build(self) -> Result<Vec<WebsiteModule>, crate::VmError> {
    let rune = self.rune;
    let hash = rune::Hash::type_hash(&["load_module"]);
    let fun = rune.unit.lookup(hash);

    if fun.is_none() {
      if self.error_missing_load_module {
        let error = RuneError::MissingLoadModuleFn;
        let error = rune::runtime::VmError::panic(error);
        return Err(rune.convert_vm_error(error));
      } else {
        return Ok(Vec::new());
      }
    }

    let value = rune.call(hash, ())?;
    let value: SendValue = rune.from_value(value)?;

    rune.convert_result(WebsiteModule::from_value_vec(rune.clone(), value))
  }
}
