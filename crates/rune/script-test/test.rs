use std::{cell::RefCell, rc::Rc};

use rune::CompileVisitor;
use runestick::{CompileMetaKind, Hash};

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

  futures::future::join_all(future.into_iter()).await;

  // for it in entry {
  //   let it = it.unwrap();
  //   if it.path().is_file() {
  //     let mut source = rune::Sources::new();
  //     source.insert(runestick::Source::from_path(&it.path()).unwrap());
  //     let unit = {
  //       let u =
  //         rune::load_sources(&context, &options, &mut source, &mut diagnostic);
  //
  //       if !diagnostic.is_empty() {
  //         let mut writter =
  //           StandardStream::stderr(rune::termcolor::ColorChoice::Always);
  //         diagnostic.emit_diagnostics(&mut writter, &source).unwrap();
  //       }
  //
  //       u.unwrap()
  //     };
  //
  //     let mut vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));
  //
  //     let res = vm.async_complete().await;
  //
  //     println!("{}", it.path().display());
  //     match res {
  //       Ok(_) => {}
  //       Err(e) => {
  //         println!("{}:{}", it.path().display(), e);
  //       }
  //     }
  //     // println!("{}", res);
  //   }
  // }

  // let mut source = runestick::Source::new(
  //   "tests",
  //   std::fs::read_to_string("script/tests.rn").unwrap(),
  // );
  // let path = source.path_mut();
  // *path = Some(PathBuf::new());
  // drop(path);
  //
  // if let Some(path) = source.path_mut() {
  //   path.push("tests.rn");
  // }
  //
  // let mut sources = rune::Sources::new();
  // sources.insert(source);
  // let context = create_context();
  //
  // let unit =
  //   rune::load_sources(&context, &options, &mut sources, &mut diagnostic);
  //
  // if !diagnostic.is_empty() {
  //   let mut writter =
  //     StandardStream::stderr(rune::termcolor::ColorChoice::Always);
  //   diagnostic.emit_diagnostics(&mut writter, &sources).unwrap();
  // }
  //
  // let unit = unit.unwrap();
  //
  // let mut vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));
  //
  // let mut vm = Vm::new(Arc::new(context.runtime()), Arc::new(unit));
  // let res = vm.async_call(["test_list"], ()).await;
  //
  // let res = match res {
  //   Ok(v) => v,
  //   Err(res) => {
  //     panic!("{}", res);
  //   }
  // };
  //
  // let res: Vec<(String, Function)> = res
  //   .into_vec()
  //   .unwrap()
  //   .take()
  //   .unwrap()
  //   .into_iter()
  //   .map(|v| {
  //     let obj = v.into_object().unwrap().take().unwrap();
  //     (
  //       match obj.get("path").unwrap().clone() {
  //         Value::StaticString(v) => v.to_string(),
  //         _ => panic!("path should be string, {:#?}", obj),
  //       },
  //       obj
  //         .get("location")
  //         .unwrap()
  //         .clone()
  //         .into_function()
  //         .unwrap()
  //         .take()
  //         .unwrap(),
  //     )
  //   })
  //   .collect();
  //
  //
  // for (name, fun) in res {
  //   let result = fun.async_send_call::<_, ()>(()).await;
  //
  //   if diagnostic.has_error() {
  //     let mut writter =
  //       StandardStream::stderr(rune::termcolor::ColorChoice::Always);
  //     diagnostic.emit_diagnostics(&mut writter, &sources).unwrap();
  //   }
  //
  //   print!("test {} ... ", name);
  //   match result {
  //     Ok(..) => println!("{}ok{}", "\x1b[32m", "\x1b[97m"),
  //     Err(err) => {
  //       println!("{}error{}", "\x1b[31m", "\x1b[97m");
  //       println!("{}", err);
  //     }
  //   }
  // }
}

// fn test_module() -> Result<Module, ContextError> {
//   let mut module = Module::with_crate("test");
//
//   macro_rules! register_type {
//     ($name:ident) => {
//       module.function(&[stringify!($name)], |v: DeserializeResult<$name>| {
//         match v.get() {
//           Ok(_) => Ok(()),
//           Err(v) => return Err(runestick::VmError::panic(v)),
//         }
//       })
//     };
//   }
//
//   register_type!(MangaInfo).unwrap();
//
//   Ok(module)
// }

// fn create_context() -> runestick::Context {
//   let mut context = mado_rune::create_context().unwrap();
//   use rune_modules::*;
//   context.install(&test::module(true).unwrap()).unwrap();
//   context.install(&fmt::module(true).unwrap()).unwrap();
//   context
//     .install(&mado_rune::testing::load_module().unwrap())
//     .unwrap();
//
//   context.install(&test_module().unwrap()).unwrap();
//
//   context
// }
