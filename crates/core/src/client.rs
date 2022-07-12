#[derive(Clone, Debug)]
pub enum Client {
    Http(crate::http::Client),
}

pub enum BodyStream {
    Http(crate::http::ResponseStream),
}

pub enum RequestBuilder {
    Http(crate::http::RequestBuilder),
}

impl From<crate::http::Client> for Client {
    fn from(v: crate::http::Client) -> Self {
        Self::Http(v)
    }
}

impl From<crate::http::RequestBuilder> for RequestBuilder {
    fn from(v: crate::http::RequestBuilder) -> Self {
        Self::Http(v)
    }
}

impl From<crate::http::ResponseStream> for BodyStream {
    fn from(v: crate::http::ResponseStream) -> Self {
        Self::Http(v)
    }
}
