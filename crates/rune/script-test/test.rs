use std::{cell::RefCell, rc::Rc};

use futures::Future;
use mado_rune::DeserializeResult;
use rune::CompileVisitor;
use runestick::{Any, CompileMetaKind, FromValue, Hash, VmError, VmErrorKind};

// implement convert from value for
// type that is a result and Ok is deserialize
// and error on Err
#[allow(dead_code)]
struct OkDeserilizeValue<T> {
  inner: T,
}

impl<T> FromValue for OkDeserilizeValue<T>
where
  T: 'static + Send + serde::de::Deserialize<'static>,
{
  fn from_value(value: runestick::Value) -> Result<Self, VmError> {
    let deser = DeserializeResult::<T>::from_value(value)?;

    let inner = deser.get().map_err(|err| VmErrorKind::Panic {
      reason: runestick::Panic::custom(err),
    })?;

    Ok(Self { inner })
  }
}

#[allow(dead_code)]
struct OkAnyValue<T> {
  inner: T,
}

impl<T> FromValue for OkAnyValue<T>
where
  T: 'static + Any,
{
  fn from_value(value: runestick::Value) -> Result<Self, VmError> {
    let inner = T::from_value(value)?;

    Ok(Self { inner })
  }
}

#[derive(Default)]
struct TestVisitor {
  function: RefCell<Vec<(String, Hash)>>,
}

impl CompileVisitor for TestVisitor {
  fn register_meta(&self, meta: &runestick::CompileMeta) {
    let item = meta.kind.clone();

    if let CompileMetaKind::Function {
      type_hash,
      is_test: true,
    } = item
    {
      let name = meta.item.item.to_string();
      let name = name.trim_start_matches("test::");
      let source = meta.source.clone().unwrap();

      let path = source.path.unwrap();
      let path = path.file_stem().unwrap().to_string_lossy();

      self
        .function
        .borrow_mut()
        .push((format!("{}::{}", path, name), type_hash));
    }
  }
}

impl TestVisitor {
  /// Clone function, clear internal, then return function
  pub fn to_function(&self) -> Vec<(String, Hash)> {
    let res = self.function.clone().into_inner();
    self.function.borrow_mut().clear();
    res
  }
}

#[tokio::main]
async fn main() {
  let mut vm_builder = mado_rune::VmBuilder::new();
  let visitor = Rc::new(TestVisitor::default());

  vm_builder.set_source_loader(Rc::new(mado_rune::SourceLoader::new()));
  vm_builder.options().test(true);
  vm_builder.set_compile_visitor(visitor.clone());

  let mut tests = Vec::new();

  let entry = std::fs::read_dir("script").unwrap();
  for it in entry {
    let it = it.unwrap();
    if it.path().is_file() {
      let source = runestick::Source::from_path(&it.path()).unwrap();
      let vm = vm_builder.load_vm_from_source(source);

      match vm {
        Ok(vm) => {
          let vm_function = visitor.to_function();
          tests.push((vm, vm_function));
        }
        Err(err) => {
          println!("{}", err);
          continue;
        }
      }
    }
  }

  let mut future = Vec::new();

  let pattern = std::env::var("MADO_RUNE_TEST").unwrap_or_else(|_| ".*".into());
  let pattern = regex::Regex::new(&pattern).unwrap_or_else(|_| {
    panic!("{}", format!("{} is not valid pattern", pattern))
  });

  for (vm, test) in tests {
    // filter the test
    let test = test.into_iter().filter(|(s, _)| pattern.is_match(s));

    for (name, hash) in test {
      let val = vm.clone().send_execute(hash, ()).unwrap();
      let fut = async move {
        let val = val.async_complete().await;
        match val {
          Ok(_) => {}
          Err(err) => {
            println!("error on {}: {}", name, err);
          }
        }
      };
      future.push(Box::pin(fut));
    }
  }

  let vec = futures::future::join_all(future.into_iter()).await;
  // let ok: Vec<_> = vec.iter().filter_map(|it| it.as_ref().ok()).collect();

  let err: Vec<_> = vec.iter().filter_map(|it| it.as_ref().err()).collect();

  if !err.is_empty() {
    for (name, err) in err {
      println!("error on {}: {}", name, err);
    }
    panic!("Error");
  }
}

async fn call_test(
  vm: runestick::Vm,
  name: String,
  hash: runestick::Hash,
) -> Result<(), (String, VmError)> {
  let last = name.split("::").last().unwrap();

  macro_rules! call {
    ($ex:ty) => {{
      to_name_error(name.clone(), async_call::<$ex>(vm, hash)).await?;
      println!("{} is ok", name);
      Ok(())
    }};
  }

  macro_rules! match_last {
    ($($name:pat => $ex:ty),+) => {
      match last {
        $($name => { call!($ex) }),+,
      }
    };
  }

  match_last! {
    "get_info" => OkDeserilizeValue<mado_core::MangaInfo>,
    _ => ()
  }
}

// call fut then add name if return fut error
async fn to_name_error<R>(
  name: String,
  fut: impl Future<Output = Result<R, VmError>>,
) -> Result<R, (String, VmError)> {
  let val = fut.await;

  val.map_err(|err| (name, err))
}

// async call vm then cast to T with FromValue
async fn async_call<T>(
  mut vm: runestick::Vm,
  hash: runestick::Hash,
) -> Result<T, VmError>
where
  T: runestick::FromValue,
{
  let val = vm.async_call(hash, ()).await?;

  T::from_value(val)
}
