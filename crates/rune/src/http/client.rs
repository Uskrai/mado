use super::*;
use mado_core::http::Client as HttpClient;
use rune::Any;

#[derive(Any, Clone, Debug)]
pub struct Client {
    inner: HttpClient,
}

macro_rules! impl_client {
    ($name:ident) => {
        pub fn $name(&self, url: Url) -> RequestBuilder {
            RequestBuilder {
                inner: self.inner.$name(url.into()),
            }
        }
    };
}

impl Client {
    pub fn default() -> Self {
        Self::new(HttpClient::default())
    }

    pub fn into_inner(self) -> HttpClient {
        self.inner
    }

    pub fn new(value: HttpClient) -> Self {
        Self { inner: value }
    }

    impl_client!(get);
    // impl_client!(post);
    // impl_client!(put);
    // impl_client!(delete);
    // impl_client!(head);
}
