use futures_lite::io::AsyncReadExt;
use isahc::AsyncReadResponseExt;
use serde::de::DeserializeOwned;

#[derive(Debug, Clone)]
pub struct Client {
    client: isahc::HttpClient,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            client: isahc::HttpClientBuilder::new().build().unwrap(),
        }
    }
}

impl Client {
    pub fn get(&self, url: crate::Url) -> RequestBuilder {
        let builder = http::request::Request::get(url.as_str());
        self.builder(url, builder)
    }

    pub fn builder(&self, url: crate::Url, request: http::request::Builder) -> RequestBuilder {
        RequestBuilder {
            request,
            client: self.client.clone(),
            url,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    HttpError(#[from] http::Error),
    #[error(transparent)]
    IsahcError(#[from] isahc::Error),
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    ExternalError(#[from] anyhow::Error),
}

#[derive(Debug)]
pub struct RequestBuilder {
    request: http::request::Builder,
    client: isahc::HttpClient,
    url: crate::Url,
}

impl RequestBuilder {
    pub fn header(mut self, key: String, value: String) -> Self {
        self.request = self.request.header(key, value);
        self
    }
    pub async fn send(self) -> Result<Response, Error> {
        let response = self
            .client
            .send_async(self.request.body(isahc::AsyncBody::empty())?)
            .await?;

        Ok(Response {
            response,
            url: self.url,
        })
    }
}

#[derive(Debug)]
pub struct Response {
    response: isahc::Response<isahc::AsyncBody>,
    url: crate::Url,
}

impl Response {
    pub fn status(&self) -> http::StatusCode {
        self.response.status()
    }

    pub fn url(&self) -> &crate::Url {
        &self.url
    }

    pub fn header_str(&self, name: &str) -> Option<&str> {
        self.response
            .headers()
            .get(name)
            .and_then(|it| it.to_str().ok())
    }

    pub async fn text(mut self) -> Result<String, Error> {
        self.response.text().await.map_err(Into::into)
    }

    pub async fn json<D: DeserializeOwned + Unpin>(mut self) -> Result<D, Error> {
        self.response.json().await.map_err(Into::into)
    }

    pub fn stream(self) -> ResponseStream {
        ResponseStream {
            body: self.response.into_body(),
        }
    }
}

pub struct ResponseStream {
    body: isahc::AsyncBody,
}

impl ResponseStream {
    pub fn body_mut(&mut self) -> &mut isahc::AsyncBody {
        &mut self.body
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        self.body.read(buf).await
    }
}

pub use http::StatusCode;
