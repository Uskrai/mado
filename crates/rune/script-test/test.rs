use futures::Future;
use mado_rune::{DeserializeResult, MockChapterTask, Rune, VmError};
use rune::{
    compile::{CompileVisitor, Meta, MetaKind},
    runtime::{FromValue, VmError as RuneVmError, VmErrorKind},
    Any, Hash,
};

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
    fn from_value(value: rune::runtime::Value) -> Result<Self, RuneVmError> {
        let deser = DeserializeResult::<T>::from_value(value)?;

        let inner = deser.get().map_err(|err| VmErrorKind::Panic {
            reason: rune::runtime::Panic::custom(err),
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
    fn from_value(value: rune::runtime::Value) -> Result<Self, RuneVmError> {
        let result = value
            .into_result()?
            .take()
            .unwrap()
            .map_err(mado_rune::Error::from_value);

        let inner = match result {
            Ok(inner) => T::from_value(inner)?,
            Err(result) => return Err(RuneVmError::panic(result?)),
        };

        Ok(Self { inner })
    }
}

#[derive(Default, Clone, Debug)]
struct TestVisitor {
    function: Vec<(String, Hash)>,
}

impl CompileVisitor for TestVisitor {
    fn register_meta(&mut self, meta: &Meta) {
        let item = meta.kind.clone();

        // push to Self::function if is_test
        if let MetaKind::Function {
            type_hash,
            is_test: true,
            ..
        } = item
        {
            // get root name. e.g: test::get_info, test::get_info_404
            let name = meta.item.item.to_string();
            let name = name.trim_start_matches("test::");
            let source = meta.source.clone().unwrap();

            // get path
            let path = source.path.unwrap();
            let path = path.file_stem().unwrap().to_string_lossy();

            self.function
                .push((format!("{}::{}", path, name), type_hash));
        }
    }
}

impl TestVisitor {
    /// Clone function, clear internal, then return function
    pub fn into_function(self) -> Vec<(String, Hash)> {
        self.function
    }
}

#[tokio::main]
async fn main() {
    let mut options = rune::Options::default();
    options.test(true);

    let mut tests = Vec::new();

    let entry = std::fs::read_dir("script").unwrap();
    for it in entry {
        let it = it.unwrap();
        if it.path().is_file() {
            let mut visitor = TestVisitor::default();

            let vm = mado_rune::Build::default()
                .visitor(&mut visitor)
                .options(&options)
                .with_path(&it.path())
                .unwrap()
                .build();

            match vm {
                Ok(vm) => {
                    let vm_function = visitor.into_function();
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
    let pattern = regex::Regex::new(&pattern)
        .unwrap_or_else(|_| panic!("{}", format!("{} is not valid pattern", pattern)));

    for (vm, test) in tests {
        // filter the test
        let test = test.into_iter().filter(|(s, _)| pattern.is_match(s));

        for (name, hash) in test {
            let fut = call_test(vm.clone(), name, hash);
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

async fn call_test(rune: Rune, name: String, hash: rune::Hash) -> Result<(), (String, VmError)> {
    let last = name.split("::").last().unwrap();

    macro_rules! call {
        ($ex:ty) => {{
            to_name_error(name.clone(), async_call::<$ex>(rune, hash)).await?;

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
      "get_info" => OkDeserilizeValue<mado_core::MangaAndChaptersInfo>,
      "get_chapter_images" => OkAnyValue<MockChapterTask>,
      "download_image" => mado_rune::http::RequestBuilder,
      _ => rune::Value
    }
}

// call fut then add name if return fut error
async fn to_name_error<R, F>(name: String, fut: F) -> Result<R, (String, VmError)>
where
    F: Future<Output = Result<R, VmError>>,
{
    let val = fut.await;

    val.map_err(|err| (name, err))
}

// async call vm then cast to T with FromValue
async fn async_call<T>(rune: Rune, hash: rune::Hash) -> Result<T, VmError>
where
    T: rune::runtime::FromValue,
{
    rune.async_call(hash, ()).await
}
