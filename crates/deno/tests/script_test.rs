use std::{cell::RefCell, collections::HashMap, rc::Rc};

use deno_core::v8::{self, Local};
use mado_deno::Runtime;
use serde::de::DeserializeOwned;
use tokio::task::LocalSet;

#[test]
pub fn script_test() -> Result<(), ErrorWrapper> {
    let tokio = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let options = deno_core::RuntimeOptions {
        module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
        extensions: mado_deno::extensions(),
        ..Default::default()
    };

    let local_set = LocalSet::new();

    // local_set.block_on(&tokio, async {
    let mut runtime = mado_deno::ModuleLoader::new(options);

    let mut module_to_path = HashMap::new();

    for it in std::fs::read_dir("./script/test/").unwrap() {
        let it = it.unwrap();
        let path = it.path();

        tokio.block_on(async {
            if path.extension() == Some(std::ffi::OsStr::new("js")) {
                match runtime.load_file(&path).await {
                    Ok(i) => {
                        module_to_path.insert(i, it.path());
                    }
                    Err(err) => {
                        println!("cannot load {}: {:?}", path.to_string_lossy(), err);
                    }
                };
            }
        });
    }

    let runtime = runtime.into_runtime();
    // let shared_runtime = runtime.into_js_runtime();

    let pattern = std::env::var("MADO_DENO_TEST").unwrap_or_else(|_| ".*".into());
    let pattern = regex::Regex::new(&pattern)
        .unwrap_or_else(|_| panic!("{}", format!("{} is not valid pattern", pattern)));

    let errors = Rc::new(RefCell::new(vec![]));

    for (index, path) in module_to_path.iter() {
        let namespace = runtime
            .js()
            .borrow_mut()
            .get_module_namespace(*index)
            .unwrap();

        runtime.with_scope(|scope| {
            let namespace = namespace.open(scope);
            let names = namespace.get_property_names(scope).unwrap();
            let length = names.length();

            for j in 0..length {
                let name =
                    Local::<v8::String>::try_from(names.get_index(scope, j).unwrap()).unwrap();
                let name_str = name.to_rust_string_lossy(scope);

                let filename = path.file_stem().unwrap().to_string_lossy().to_string();

                let mut name_str = name_str.splitn(3, "__").map(|it| it.to_string());

                let filename = filename.to_string();
                let testname = name_str.next().unwrap();
                let expected = name_str.next().unwrap();
                let unique_id = name_str.next();

                let name_str = Name {
                    filename,
                    testname,
                    expected,
                    unique_id,
                };

                if pattern.is_match(&name_str.to_string()) {
                    let value = Local::<v8::Function>::try_from(
                        namespace.get(scope, name.clone().into()).unwrap(),
                    );

                    let value = match value {
                        Ok(value) => value,
                        Err(err) => {
                            dbg!(err);
                            continue;
                        }
                    };

                    let value = v8::Global::new(scope, value);

                    local_set.spawn_local({
                        let runtime = runtime.clone();
                        let errors = errors.clone();
                        async move {
                            let result = test_function(runtime.clone(), name_str, value).await;

                            if let Err(err) = result {
                                errors.borrow_mut().push(err);
                            }
                        }
                    });
                }
            }
        });
    }

    tokio.block_on(local_set);

    if errors.borrow().is_empty() {
        return Ok(());
    }

    return Err(ErrorWrapper(errors));
}

pub struct ErrorWrapper(Rc<RefCell<Vec<anyhow::Error>>>);

impl std::fmt::Debug for ErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for it in self.0.borrow().iter() {
            writeln!(f, "{}", it)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Name {
    filename: String,
    testname: String,
    expected: String,
    unique_id: Option<String>,
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}__{}__{}", self.filename, self.testname, self.expected)?;

        if let Some(unique_id) = &self.unique_id {
            write!(f, "__{}", unique_id)?;
        }

        Ok(())
    }
}

async fn test_function(
    runtime: Runtime,
    name: Name,
    function: v8::Global<v8::Function>,
) -> Result<bool, anyhow::Error> {
    let promise = {
        runtime.with_scope(|scope| {
            let nul = v8::null(scope);

            let function = function.open(scope);
            function
                .call(scope, nul.into(), &[])
                .map(|it| v8::Global::new(scope, it))
        })
    };

    let promise = match promise {
        Some(promise) => promise,
        None => {
            return Ok(false);
        }
    };

    let value = runtime.resolve_value(promise).await;

    let value = match value {
        Ok(value) => value,
        Err(err) => {
            println!("error resolving {} value {:?}", name, err);
            return Ok(false);
        }
    };

    macro_rules! match_first {
        ($($name:pat => $ex:ty)*) => {
            match (name.testname.as_str()) {
                $($name => { check_type::<$ex>(runtime, &name, value) })*
            }
        };
    }

    match_first! {
        "getInfo" => mado_core::MangaAndChaptersInfo
        "getChapterImage" => Vec<mado_core::ChapterImageInfo>
        "downloadImage" => mado_deno::http::RequestBuilder
        _ => serde_json::Value
    }
}

fn check_type<T>(
    mut runtime: Runtime,
    name: &Name,
    // filename: &str,
    // part: Vec<&str>,
    value: v8::Global<v8::Value>,
) -> Result<bool, anyhow::Error>
where
    T: DeserializeOwned,
{
    runtime.with_scope_ops(|scope, state| {
        // let name = format!("{}__{}", filename, part.join("__"));
        let real_value = v8::Local::new(scope, value);

        let expected = &name.expected;

        let split_expected = expected.split('_').collect::<Vec<_>>();

        let value = mado_deno::from_v8::<mado_deno::ResultJson<T>>(scope, real_value.clone())
            .map(|it| it.to_result(state));

        let mut print_error = |error: Option<anyhow::Error>| {
            use std::fmt::Write;
            let mut string = String::new();
            writeln!(string, "expected: {} at {}", expected, name).unwrap();
            // let string = serde_v8::to_utf8(
            let actual: serde_json::Value = serde_v8::from_v8(scope, real_value).unwrap();

            // match actual {
            //     Ok(actual) => {
            writeln!(
                string,
                "actual: {}",
                serde_json::to_string_pretty(&actual).unwrap()
            )
            .unwrap();
            //     }
            //     Err(err) => {
            //         writeln!(string, "actual: {}", real_value.to_detail_string(scope)).unwrap()
            //     }
            // }

            if let Some(err) = error {
                writeln!(string, "{}", err).unwrap();
            }

            return Err(anyhow::anyhow!(string));
        };

        match (split_expected[0], value) {
            ("Ok", Ok(Ok(_))) => {}
            ("Err", Ok(Err(error))) => {
                if split_expected.get(1) != Some(&error.to_string_variant().as_str()) {
                    return print_error(Some(error.into()));
                }
            }
            (_, err) => {
                return print_error(err.and_then(|it| it.map_err(Into::into)).err());
            }
        }

        println!("{} success", name);

        Ok(true)
    })
}

// pub trait Debugging {
//     fn to_string(&self) -> String;
// }
//
// impl<T> Debugging for T
// where
//     T: serde::Serialize,
// {
//     fn to_string(&self) -> String {
//         serde_json::to_string_pretty(&self).unwrap()
//     }
// }
//
// impl<T> Debugging for T
// where T
