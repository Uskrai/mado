pub mod error;
pub mod http;
mod module;
mod runtime;
pub mod task;

pub use module::*;
pub use runtime::*;

pub enum Resource {
    Json(serde_json::Value),
    Error(self::error::Error),
}

impl deno_core::Resource for Resource {}

#[deno_core::op]
async fn op_tokio_sleep(mili: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(mili)).await;
}

// #[deno_core::op]
// async fn op_resource_clone(state: &mut deno_core::OpState, rid: u32) -> Result<u32, anyhow::Error> {
//     let it = state.resource_table.get_any(rid)?;
//
//     Ok(state.resource_table.add_rc(it))
// }

pub fn extensions() -> Vec<deno_core::Extension> {
    vec![
        deno_console::init(),
        crate::http::init(),
        crate::error::init(),
        deno_core::ExtensionBuilder::default()
            .ops(vec![op_tokio_sleep::decl()])
            .build(),
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
    // using this give better information about path failing
    // #[cfg(debug_assertions)]
    let deserializer = serde_v8::from_v8::<serde_json::Value>(scope, value).unwrap();

    // #[cfg(not(debug_assertions))]
    // let deserializer = &mut serde_v8::Deserializer::new(scope, it, None);

    let mut track = serde_path_to_error::Track::new();

    let deserializer = serde_path_to_error::Deserializer::new(deserializer, &mut track);
    T::deserialize(deserializer)
        .with_context(|| anyhow::anyhow!("cannot deserialize at {:?}", track.path()))
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(tag = "type", content = "content")]
pub enum ResultJson<T> {
    Ok(T),
    Err(self::error::ErrorJson),
}

impl<T> ResultJson<T> {
    pub fn to_result(self, state: &mut deno_core::OpState) -> Result<T, crate::error::Error> {
        match self {
            Self::Ok(v) => Ok(v),
            Self::Err(v) => Err(v.take(state)),
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
