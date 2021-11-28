use super::*;
use rune::Any;

#[derive(Any, Clone, Debug)]
pub struct Client {
    inner: reqwest::Client,
}

macro_rules! impl_client {
    ($name:ident) => {
        pub fn $name(&self, url: Url) -> RequestBuilder {
            RequestBuilder {
                inner: self.inner.$name::<reqwest::Url>(url.into()),
            }
        }
    };
}

impl Client {
    pub fn default() -> Self {
        Self::new(reqwest::Client::new())
    }

    pub fn new(value: reqwest::Client) -> Self {
        Self { inner: value }
    }

    impl_client!(get);
    impl_client!(post);
    impl_client!(put);
    impl_client!(delete);
    impl_client!(head);
}
