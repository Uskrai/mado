use std::fmt::Display;

use mado_engine::{DownloadProgressStatus, DownloadResumedStatus};
use rusqlite::{
    types::{FromSql, ToSqlOutput, Value},
    ToSql,
};

#[derive(Clone, Debug, PartialEq)]
pub enum DownloadStatus {
    Resumed,
    Paused,
    Error(String),
    Finished,
}

impl DownloadStatus {
    pub fn paused() -> Self {
        Self::Paused
    }

    pub fn resumed() -> Self {
        Self::Resumed
    }

    pub fn error<S: Into<String>>(error: S) -> Self {
        Self::Error(error.into())
    }

    pub fn finished() -> Self {
        Self::Finished
    }

    pub fn error_parse<S: std::fmt::Display>(error: S) -> Self {
        Self::Error(format!("cannot parse status: {}", error))
    }
}

impl From<&str> for DownloadStatus {
    fn from(s: &str) -> Self {
        let string = s;

        if string == "Finished" {
            Self::Finished
        } else if string == "Paused" {
            Self::Paused
        } else if string == "Resumed" {
            Self::Resumed
        } else {
            let fun = || {
                let (first, last) = string.split_once('(')?;
                let last = last.strip_suffix(')')?;

                if first == "Error" {
                    Some(DownloadProgressStatus::Error(last.to_string()))
                } else {
                    None
                }
            };

            match fun() {
                Some(v) => Self::from(mado_engine::DownloadStatus::InProgress(v)),
                None => Self::error_parse(s),
            }
        }
    }
}

impl From<String> for DownloadStatus {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl From<&mado_engine::DownloadStatus> for DownloadStatus {
    fn from(v: &mado_engine::DownloadStatus) -> Self {
        match v {
            mado_engine::DownloadStatus::InProgress(v) => match v {
                DownloadProgressStatus::Resumed(_) => Self::Resumed,
                DownloadProgressStatus::Paused => Self::Paused,
                DownloadProgressStatus::Error(v) => Self::Error(v.to_string()),
            },
            mado_engine::DownloadStatus::Finished => Self::Finished,
        }
    }
}

impl From<mado_engine::DownloadStatus> for DownloadStatus {
    fn from(v: mado_engine::DownloadStatus) -> Self {
        Self::from(&v)
    }
}

impl From<DownloadStatus> for mado_engine::DownloadStatus {
    fn from(v: DownloadStatus) -> Self {
        match v {
            DownloadStatus::Resumed => {
                mado_engine::DownloadStatus::resumed(DownloadResumedStatus::default())
            }
            DownloadStatus::Paused => mado_engine::DownloadStatus::paused(),
            DownloadStatus::Error(v) => mado_engine::DownloadStatus::error(v),
            DownloadStatus::Finished => mado_engine::DownloadStatus::finished(),
        }
    }
}

impl Display for DownloadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadStatus::Resumed => write!(f, "Resumed"),
            DownloadStatus::Paused => write!(f, "Paused"),
            DownloadStatus::Error(v) => write!(f, "Error({})", v),
            DownloadStatus::Finished => write!(f, "Finished"),
        }
    }
}

impl ToSql for DownloadStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Text(self.to_string())))
    }
}

impl FromSql for DownloadStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let string = String::column_result(value)?;

        Ok(Self::from(string))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!(DownloadStatus::from("Resumed"), DownloadStatus::resumed());

        assert_eq!(DownloadStatus::from("Paused"), DownloadStatus::Paused);
        assert_eq!(
            DownloadStatus::from("Error(Foo)"),
            DownloadStatus::error("Foo")
        );

        assert_eq!(DownloadStatus::from("Finished"), DownloadStatus::Finished);

        assert_ne!(DownloadStatus::from("Resumed("), DownloadStatus::resumed());

        assert_eq!(
            DownloadStatus::from("Resumed(Waiting"),
            DownloadStatus::error_parse("Resumed(Waiting")
        );

        assert_eq!(
            DownloadStatus::from("Resumed(Error)"),
            DownloadStatus::error_parse("Resumed(Error)")
        );

        assert_ne!(
            DownloadStatus::from("Error(Foo"),
            DownloadStatus::error("Foo")
        );

        assert_eq!(
            DownloadStatus::from("ResumedWaiting)"),
            DownloadStatus::error_parse("ResumedWaiting)")
        );

        assert_eq!(
            DownloadStatus::from("Finished()"),
            DownloadStatus::error_parse("Finished()")
        );

        assert_eq!(
            DownloadStatus::from("Error(Foo"),
            DownloadStatus::error_parse("Error(Foo")
        );
    }

    #[test]
    fn to_str() {
        assert_eq!(DownloadStatus::from("Error(Foo)").to_string(), "Error(Foo)");

        assert_eq!(DownloadStatus::from("Resumed").to_string(), "Resumed");

        assert_eq!(DownloadStatus::paused().to_string(), "Paused");

        assert_eq!(DownloadStatus::finished().to_string(), "Finished");
    }

    #[test]
    fn from_status() {
        assert_eq!(
            DownloadStatus::Finished,
            mado_engine::DownloadStatus::finished().into(),
        );

        assert_eq!(
            DownloadStatus::Paused,
            mado_engine::DownloadStatus::paused().into(),
        );

        assert_eq!(
            DownloadStatus::error("error"),
            mado_engine::DownloadStatus::error("error").into()
        );

        assert_eq!(
            DownloadStatus::Resumed,
            mado_engine::DownloadStatus::resumed(DownloadResumedStatus::Waiting).into()
        );
    }

    #[test]
    fn to_status() {
        assert_eq!(
            mado_engine::DownloadStatus::finished(),
            DownloadStatus::Finished.into(),
        );

        assert_eq!(
            mado_engine::DownloadStatus::paused(),
            DownloadStatus::Paused.into(),
        );

        assert_eq!(
            mado_engine::DownloadStatus::error("error"),
            DownloadStatus::error("error").into(),
        );

        assert_eq!(
            mado_engine::DownloadStatus::resumed(Default::default()),
            DownloadStatus::Resumed.into(),
        );
        //
    }
}
