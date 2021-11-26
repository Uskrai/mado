// use serde::{Deserialize, Serialize};
// use std::fmt::Display;
// use thiserror::Error;
//
// #[derive(Deserialize, Serialize, Debug, Clone)]
// pub struct HttpErrorInfo {
//   url: Option<String>,
//   status: Option<u16>,
//   message: String,
// }
//
// impl Display for HttpErrorInfo {
//   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//     f.write_str(&self.message)
//   }
// }
//
// #[derive(Deserialize, Serialize, Error, Debug, Clone)]
// #[serde(tag = "type", content = "content")]
// pub enum HttpError {
//   #[error("{0}")]
//   BuilderError(HttpErrorInfo),
//   #[error("{0}")]
//   TimeoutError(HttpErrorInfo),
//   #[error("{0}")]
//   RedirectError(HttpErrorInfo),
//   #[error("{0}")]
//   StatusError(HttpErrorInfo),
//   #[error("{0}")]
//   RequestError(HttpErrorInfo),
//   #[error("{0}")]
//   ConnectError(HttpErrorInfo),
//   #[error("{0}")]
//   BodyError(HttpErrorInfo),
//   #[error("{0}")]
//   DecodeError(HttpErrorInfo),
//   #[error("Not an error")]
//   Ok,
// }
//
// impl HttpError {
//   pub fn from_reqwest(error: reqwest::Error) -> Self {
//     if error.is_builder() {
//       Self::BuilderError(HttpErrorInfo::from_reqwest(error))
//     } else if error.is_timeout() {
//       Self::TimeoutError(HttpErrorInfo::from_reqwest(error))
//     } else if error.is_redirect() {
//       Self::RedirectError(HttpErrorInfo::from_reqwest(error))
//     } else if error.is_status() {
//       Self::StatusError(HttpErrorInfo::from_reqwest(error))
//     } else if error.is_request() {
//       Self::RequestError(HttpErrorInfo::from_reqwest(error))
//     } else if error.is_connect() {
//       Self::ConnectError(HttpErrorInfo::from_reqwest(error))
//     } else if error.is_body() {
//       Self::BodyError(HttpErrorInfo::from_reqwest(error))
//     } else if error.is_decode() {
//       Self::DecodeError(HttpErrorInfo::from_reqwest(error))
//     } else {
//       Self::Ok
//     }
//   }
// }
//
// impl HttpErrorInfo {
//   pub fn from_reqwest(error: reqwest::Error) -> Self {
//     Self {
//       url: error.url().map(|v| v.to_string()),
//       status: error.status().map(|v| v.as_u16()),
//       message: format!("{}", error),
//     }
//   }
// }
