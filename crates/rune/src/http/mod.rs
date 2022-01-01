use mado_core::http::{RequestBuilder as HttpRequestBuilder, Response as HttpResponse};
// use reqwest::header::SET_COOKIE;
use rune::{Any, ContextError, Module};

mod client;
mod url;

pub use self::url::Url;
pub use client::Client;

#[derive(Any, Debug)]
pub struct RequestBuilder {
    inner: HttpRequestBuilder,
}

impl RequestBuilder {
    // pub fn query(self, name: String, value: String) -> Self {
    //     RequestBuilder {
    //         inner: self.inner.query(&[(name, value)]),
    //     }
    // }

    // wrapper_fun!(basic_auth, username: String, password: Option<String>);
    // wrapper_fun!(bearer_auth, token: String);
    // wrapper_fun!(timeout, timeout: Duration);

    pub fn header(mut self, name: String, value: String) -> Self {
        self.inner = self.inner.header(name, value);
        self
    }

    pub async fn send(self) -> Result<Response, crate::Error> {
        self.inner
            .send()
            .await
            .map(|inner| Response { inner })
            .map_err(Into::into)
    }

    pub fn into_inner(self) -> HttpRequestBuilder {
        self.inner
    }
}

#[derive(Any, Debug)]
pub struct Response {
    inner: HttpResponse,
}

#[derive(Any, Debug)]
pub struct StatusCode(mado_core::http::StatusCode);

impl StatusCode {
    pub fn as_string(&self) -> String {
        self.0.as_str().to_string()
    }

    pub fn as_u16(&self) -> u16 {
        self.0.as_u16()
    }

    pub fn is_client_error(&self) -> bool {
        self.0.is_client_error()
    }

    pub fn is_server_error(&self) -> bool {
        self.0.is_server_error()
    }

    pub fn is_informational(&self) -> bool {
        self.0.is_informational()
    }

    pub fn is_success(&self) -> bool {
        self.0.is_success()
    }

    pub fn is_redirection(&self) -> bool {
        self.0.is_redirection()
    }
}

impl Response {
    pub fn url(&self) -> String {
        self.inner.url().to_string()
    }

    pub fn status(&self) -> StatusCode {
        StatusCode(self.inner.status())
    }
    pub async fn text(self) -> String {
        self.inner.text().await.unwrap()
    }

    pub async fn json(self) -> Result<super::Json, crate::Error> {
        Ok(self.inner.json::<serde_json::Value>().await?.into())
    }

    pub fn bytes_stream(self) -> BytesStream {
        BytesStream(self.inner.stream())
    }
}

#[derive(Any)]
pub struct BytesStream(mado_core::http::ResponseStream);

impl BytesStream {
    pub fn into_inner(self) -> mado_core::http::ResponseStream {
        self.0
    }
}

// impl futures_core::stream::Stream for BytesStream {
//     type Item = Result<bytes::Bytes, mado_core::Error>;
//     fn poll_next(
//         mut self: std::pin::Pin<&mut Self>,
//         cx: &mut std::task::Context<'_>,
//     ) -> std::task::Poll<Option<Self::Item>> {
//         self.0
//             .as_mut()
//             .poll_next(cx)
//             .map_err(|err| mado_core::Error::ExternalError(err.into()))
//     }
// }

pub fn load_module() -> Result<Module, ContextError> {
    let module = Module::with_crate_item("mado", &["http"]);

    mado_rune_macros::register_module! {
        (Client) => {
            inst => {
                get, clone
            }
            // inst => {
            //   get, post, put, delete, head
            // }
            associated => {
                default: default_
            }
        },
        (RequestBuilder) => {
            inst => {
                header
            }
            // inst => {
            //   query, cookie, header
            // },
            async_inst => {
                send
            }
        },
        (Response) => {
            inst => {
                url, status, bytes_stream
            },
            async_inst => {
                text, json
            }
        },
        (Url) => {
            associated => {
                parse
            }
            inst => {
                to_string, query, clone, path, extension
            }
            protocol => {
                to_string_debug: STRING_DEBUG
            }
        },
        (StatusCode) => {
            inst => {
                as_string, as_u16, is_success, is_redirection,
                is_client_error, is_server_error
            }
        }
    }

    load_module_with(module)
}
