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
