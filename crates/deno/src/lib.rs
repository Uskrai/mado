mod chromium;
pub mod error;
pub mod http;
mod module;
mod runtime;
pub mod task;

pub use error::{Error, ErrorJson};
pub use module::*;
pub use runtime::*;

pub enum Resource {
    Json(serde_json::Value),
    Error(self::error::Error),
}

impl deno_core::Resource for Resource {}

pub fn spawn_local<F>(future: F)
where
    F: std::future::Future + 'static,
{
    tokio::task::spawn_local(future);
}

#[deno_core::op2(async)]
async fn op_tokio_sleep(mili: u32) {
    tokio::time::sleep(std::time::Duration::from_millis(mili.into())).await;
}

#[deno_core::op2]
#[serde]
fn op_url_join(#[string] url: &str, #[string] end: &str) -> ResultJson<String> {
    url::Url::parse(url)
        .and_then(|it| it.join(end))
        .map(|it| it.to_string())
        .map_err(ErrorJson::custom)
        .into()
}
// #[deno_core::op]
// async fn op_resource_clone(state: &mut deno_core::OpState, rid: u32) -> Result<u32, anyhow::Error> {
//     let it = state.resource_table.get_any(rid)?;
//
//     Ok(state.resource_table.add_rc(it))
// }

deno_core::extension!(
    libs,
    ops = [op_tokio_sleep, op_url_join],
    js = [
        "script/bootstrap/00_bootstrap.js",
        "script/bootstrap/01_console.js",
    ]
);

pub fn extensions() -> Vec<deno_core::Extension> {
    // let bootstrap = deno_core::include_js_files!(
    //     bootstrap
    //     "script/bootstrap/00_bootstrap.js",
    //     "script/bootstrap/01_console.js",
    // );
    vec![
        // deno_console::deno_console::init_ops_and_esm(),
        crate::http::init(),
        crate::error::init(),
        crate::module::init(),
        crate::task::init(),
        self::libs::init_ops_and_esm(),
        // deno_core::ExtensionBuilder::default()
        //     .ops(vec![op_tokio_sleep::decl()])
        //     .build(),
        // deno_core::Extension::builder()
        //     .js(deno_core::include_js_files!(
        //         prefix "bootstrap",
        //         "../script/bootstrap/00_bootstrap.js",
        //         "../script/bootstrap/01_console.js",
        //     ))
        //     .build(),
        crate::chromium::init(),
    ]
}

pub fn from_v8<T>(
    scope: &mut deno_core::v8::HandleScope,
    value: deno_core::v8::Local<deno_core::v8::Value>,
) -> Result<T, anyhow::Error>
where
    T: serde::de::DeserializeOwned,
{
    use anyhow::Context;
    use deno_core::serde_v8;
    use tap::TapFallible;
    // using this give better information about path failing
    // #[cfg(debug_assertions)]
    let deserializer = serde_v8::from_v8::<serde_json::Value>(scope, value).unwrap();

    // #[cfg(not(debug_assertions))]
    // let deserializer = &mut serde_v8::Deserializer::new(scope, it, None);

    let mut track = serde_path_to_error::Track::new();

    let deserializer = serde_path_to_error::Deserializer::new(deserializer, &mut track);
    T::deserialize(deserializer)
        .with_context(|| format!("cannot deserialize at {:?}", track.path()))
        .tap_err(|e| tracing::error!("{:?}", e))
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(tag = "type", content = "content")]
pub enum ResultJson<T> {
    Ok(T),
    Err(self::error::ErrorJson),
}

// impl<T> From<Result<T, ErrorJson>> for ResultJson<T> {
//     fn from(value: Result<T, ErrorJson>) -> Self {
//         match value {
//             Result::Ok(v) => Self::Ok(v),
//             Result::Err(err) => Self::Err(err),
//         }
//     }
//     // pub fn from_result(result: Result<T, ErrorJson>) -> Self {
//     // }
// }
//
#[macro_export]
macro_rules! try_json {
    ($expr:expr $(,)?) => {
        match $expr {
            $crate::ResultJson::Ok(val) => val,
            $crate::ResultJson::Err(err) => {
                return $crate::ResultJson::Err(std::convert::From::from(err));
            }
        }
    };
}

impl<T> ResultJson<T> {
    pub fn to_result(self, state: &mut deno_core::OpState) -> Result<T, crate::error::Error> {
        match self {
            Self::Ok(v) => Ok(v),
            Self::Err(v) => Err(v.take(state)),
        }
    }
}

pub trait ToResultJson<T>: Sized {
    fn to_result_json(self, state: &mut deno_core::OpState) -> ResultJson<T>;

    fn to_result_json_borrow(
        self,
        state: std::rc::Rc<std::cell::RefCell<deno_core::OpState>>,
    ) -> ResultJson<T> {
        self.to_result_json(&mut state.borrow_mut())
    }
}

impl<T> ToResultJson<T> for Result<T, self::error::Error> {
    fn to_result_json(self, state: &mut deno_core::OpState) -> ResultJson<T> {
        match self {
            Ok(it) => ResultJson::Ok(it),
            Err(err) => ResultJson::Err(ErrorJson::from_error(state, err)),
        }
    }
}

impl<T> From<Result<T, self::error::ErrorJson>> for ResultJson<T> {
    fn from(v: Result<T, self::error::ErrorJson>) -> Self {
        match v {
            Ok(v) => Self::Ok(v),
            Err(v) => Self::Err(v),
        }
    }
}

impl<T> From<ResultJson<T>> for Result<T, self::error::ErrorJson> {
    fn from(v: ResultJson<T>) -> Self {
        match v {
            ResultJson::Ok(v) => Ok(v),
            ResultJson::Err(v) => Err(v),
        }
    }
}
