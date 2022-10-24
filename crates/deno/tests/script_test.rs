use std::{any::type_name, cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc};

use deno_core::{v8::{self, Local}, serde_v8};
use mado_deno::Runtime;
use serde::de::DeserializeOwned;
use tokio::task::LocalSet;

#[test]
pub fn script_test() -> Result<(), Box<dyn std::error::Error>> {
    let tokio = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let local_set = LocalSet::new();
    let _local_set_guard = local_set.enter();

    let test_set = LocalSet::new();
    let last_set = LocalSet::new();

    let runtime = Runtime::default();
    let inspector = runtime
        .js()
        .borrow_mut()
        .inspector()
        .borrow()
        .create_local_session();

    let mut coverage_collector =
        mado_deno_coverage::CoverageCollector::new(PathBuf::from("./coverage"), inspector);

    tokio.block_on(
        runtime
            .clone()
            .with_event_loop(coverage_collector.start_collecting()),
    )?;

    let mut runtime = mado_deno::ModuleLoader::from_runtime(runtime);

    let mut module_to_path = HashMap::new();

    for it in std::fs::read_dir("./dist/test/").unwrap() {
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

    let mut runtime = runtime.into_runtime();

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

        runtime.with_runtime_scope(|runtime, scope| {
            let namespace = namespace.open(scope);
            let names = namespace
                .get_property_names(scope, Default::default())
                .unwrap();
            let length = names.length();

            for j in 0..length {
                let name_v8 =
                    Local::<v8::String>::try_from(names.get_index(scope, j).unwrap()).unwrap();
                let name_str = name_v8.to_rust_string_lossy(scope);

                let filename = path.file_stem().unwrap().to_string_lossy().to_string();

                let split_to_name = || {
                    let mut name_str = name_str.splitn(3, "__").map(|it| it.to_string());

                    let filename = filename.to_string();
                    let testname = name_str.next().unwrap();
                    let expected = name_str.next().unwrap_or_else(|| "Any".to_string());
                    let unique_id = name_str.next();

                    Name {
                        filename,
                        testname,
                        expected,
                        unique_id,
                    }
                };

                let name = split_to_name();

                if pattern.is_match(&name.to_string()) {
                    let value = Local::<v8::Function>::try_from(
                        namespace.get(scope, name_v8.into()).unwrap(),
                    );

                    let value = match value {
                        Ok(value) => value,
                        Err(err) => {
                            dbg!(err);
                            continue;
                        }
                    };

                    let value = v8::Global::new(scope, value);
                    let is_last = name.testname.starts_with("close");

                    let spawning = || {
                        let runtime = runtime.clone();
                        let errors = errors.clone();
                        async move {
                            let result = test_function(runtime.clone(), name, value).await;

                            if let Err(err) = result {
                                errors.borrow_mut().push(err);
                            }
                        }
                    };

                    if is_last {
                        last_set.spawn_local(spawning());
                    } else {
                        test_set.spawn_local(spawning());
                    }
                }
            }
        });
    }

    local_set.block_on(&tokio, test_set);
    local_set.block_on(&tokio, last_set);
    local_set.block_on(
        &tokio,
        runtime
            .clone()
            .with_event_loop(coverage_collector.stop_collecting()),
    )?;

    if errors.borrow().is_empty() {
        return Ok(());
    }

    Err(Box::new(ErrorWrapper(Rc::try_unwrap(errors).unwrap().into_inner())))
}

pub struct ErrorWrapper(Vec<anyhow::Error>);

impl std::error::Error for ErrorWrapper {
    //
}

impl std::fmt::Display for ErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for it in self.0.iter() {
            writeln!(f, "{:?}", it)?;
        }

        Ok(())
    }
}

impl std::fmt::Debug for ErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for it in self.0.iter() {
            writeln!(f, "{:?}", it)?;
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
    mut runtime: Runtime,
    name: Name,
    function: v8::Global<v8::Function>,
) -> Result<bool, anyhow::Error> {
    let promise = {
        runtime.with_scope(|scope| {
            let scope = &mut v8::TryCatch::new(scope);
            let nul = v8::null(scope);

            let function = function.open(scope);
            let val = function
                .call(scope, nul.into(), &[])
                .map(|it| v8::Global::new(scope, it));

            if let Some(ex) = scope.exception() {
                println!("{:?}", ex);
            }

            val
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

    fn result<T>(it: Result<ResultTest<T>, anyhow::Error>) -> Result<bool, anyhow::Error> {
        it.and_then(|it| match it {
            ResultTest::Error(err) => Err(err),
            _ => Ok(true),
        })
    }

    fn test_state<T: deno_core::Resource + 'static>(
        state: &mut deno_core::OpState,
        rid: u32,
    ) -> Result<bool, anyhow::Error> {
        let it = state.resource_table.get_any(rid)?;

        if it.downcast_rc::<T>().is_none() {
            let ty = type_name::<T>();

            Err(anyhow::anyhow!(
                "Bad resource type {}, expected: {:?}",
                rid,
                ty
            ))
        } else {
            Ok(true)
        }
    }

    macro_rules! match_first1 {
        ($variable:ident, $type:ty, $name:pat,  $expr:expr) => {
            {
                let $variable = check_type::<$type>(runtime.clone(), &name, value);
                $expr
            }
        };
        ($variable:ident, $type:ty, $name:pat) => {
            {
                let $variable = check_type::<$type>(runtime, &name, value);
                result($variable)
            }
        };
        ($variable:ident, $($name:pat = $type:ty $(=> $expr:expr)?)*) => {
            match (name.testname.as_str()) {
                $(
                    $name => {
                        match_first1!($variable, $type, $name $(, $expr)?)
                    }
                )*
            }
        };
    }

    match_first1! {
        it,
        "getInfo" = mado_core::MangaAndChaptersInfo
        "getChapterImage" = Vec<mado_core::ChapterImageInfo>
        "downloadImage" = u32 => {
            match it {
                Ok(ResultTest::Expected(Ok(it))) => {
                    runtime.with_state(|state| test_state::<mado_deno::MadoCoreRequestBuilderResource>(state, it))
                }
                _ => result(it)
            }
        }
        _ = serde_json::Value => {
            result(it)
        }
    }
}

pub enum ResultTest<T> {
    Expected(Result<T, mado_deno::error::Error>),
    Error(anyhow::Error),
    Any,
}

fn check_type<T>(
    mut runtime: Runtime,
    name: &Name,
    value: v8::Global<v8::Value>,
) -> Result<ResultTest<T>, anyhow::Error>
where
    T: DeserializeOwned,
{
    runtime.with_scope_state(|scope, state| {
        // let name = format!("{}__{}", filename, part.join("__"));
        let real_value = v8::Local::new(scope, value);

        let expected = &name.expected;

        let mut split_expected = expected.splitn(2, '_');

        let value = mado_deno::from_v8::<mado_deno::ResultJson<T>>(scope, real_value)
            .map(|it| it.to_result(state));

        let mut print_error = |error: Option<anyhow::Error>| {
            use std::fmt::Write;
            let mut string = String::new();
            writeln!(string, "expected: {} at {}", expected, name).unwrap();
            let actual: serde_json::Value = serde_v8::from_v8(scope, real_value).unwrap();

            writeln!(
                string,
                "actual: {}",
                serde_json::to_string_pretty(&actual).unwrap()
            )
            .unwrap();

            if let Some(err) = error {
                writeln!(string, "{}", err).unwrap();
            }

            anyhow::anyhow!(string)
        };

        let it = match (split_expected.next(), value) {
            (Some("Ok"), Ok(Ok(ok))) => ResultTest::Expected(Ok(ok)),
            (Some("Err"), Ok(Err(error))) => {
                let it = expected_error(&error, split_expected.next());

                if it {
                    ResultTest::Expected(Err(error))
                } else {
                    ResultTest::Error(print_error(Some(error.into())))
                }
            }
            (Some("Any"), _) => ResultTest::Any,
            (_, err) => {
                ResultTest::Error(print_error(err.and_then(|it| it.map_err(Into::into)).err()))
            }
        };

        if !matches!(it, ResultTest::Error(_)) {
            println!("{} success", name);
        }

        Ok(it)
    })
}

pub fn expected_error(error: &mado_deno::error::Error, expected: Option<&str>) -> bool {
    use mado_core::Error as MadoError;
    use mado_deno::error::Error;

    let mut expected = expected.map(|it| it.splitn(2, '_')).into_iter().flatten();

    match expected.next() {
        Some("MadoError") => {
            let mut expected = expected
                .next()
                .map(|it| it.splitn(2, '_'))
                .into_iter()
                .flatten();

            match (error, expected.next()) {
                (Error::MadoError(error), Some(variant)) => {
                    if error.to_string_variant() == variant {
                        let mut expected = expected
                            .next()
                            .map(|it| it.splitn(2, '_'))
                            .into_iter()
                            .flatten();

                        match (error, expected.next()) {
                            (MadoError::ExternalError(error), Some(variant)) => {
                                variant == "Error" && error.is::<Error>()
                            }
                            _ => true,
                        }
                    } else {
                        false
                    }
                }
                (Error::MadoError(_), None) => true,
                // Not MadoError
                _ => false,
            }
        }
        Some(expected) => expected == error.to_string_variant(),
        None => true,
    }
}
