#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
pub enum DownloadResumedStatus {
    #[default]
    Queue,
    Waiting,
    Downloading,
}

impl DownloadResumedStatus {
    /// Returns `true` if the download resumed status is [`Queue`].
    ///
    /// [`Queue`]: DownloadResumedStatus::Queue
    #[must_use]
    pub fn is_queue(&self) -> bool {
        matches!(self, Self::Queue)
    }

    /// Returns `true` if the download resumed status is [`Waiting`].
    ///
    /// [`Waiting`]: DownloadResumedStatus::Waiting
    #[must_use]
    pub fn is_waiting(&self) -> bool {
        matches!(self, Self::Waiting)
    }

    /// Returns `true` if the download resumed status is [`Downloading`].
    ///
    /// [`Downloading`]: DownloadResumedStatus::Downloading
    #[must_use]
    pub fn is_downloading(&self) -> bool {
        matches!(self, Self::Downloading)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DownloadProgressStatus {
    Resumed(DownloadResumedStatus),
    Paused,
    // we don't need StdError here because this is only used to shows to user
    Error(String),
}

impl DownloadProgressStatus {
    pub fn as_resumed(&self) -> Option<&DownloadResumedStatus> {
        if let Self::Resumed(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DownloadStatus {
    InProgress(DownloadProgressStatus),
    Finished,
}

impl DownloadStatus {
    pub fn is_resumed(&self) -> bool {
        matches!(self, Self::InProgress(DownloadProgressStatus::Resumed(..)))
    }

    pub fn is_downloading(&self) -> bool {
        self.as_resumed()
            .map(|it| it.is_downloading())
            .unwrap_or(false)
    }

    pub fn is_queue(&self) -> bool {
        self.as_resumed().map(|it| it.is_queue()).unwrap_or(false)
    }

    pub fn is_paused(&self) -> bool {
        matches!(self, Self::InProgress(DownloadProgressStatus::Paused))
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Finished)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::InProgress(DownloadProgressStatus::Error(..)))
    }

    pub fn resumed(status: DownloadResumedStatus) -> Self {
        Self::InProgress(DownloadProgressStatus::Resumed(status))
    }

    pub fn queued() -> Self {
        Self::resumed(DownloadResumedStatus::Queue)
    }

    pub fn waiting() -> Self {
        Self::resumed(DownloadResumedStatus::Waiting)
    }

    pub fn downloading() -> Self {
        Self::resumed(DownloadResumedStatus::Downloading)
    }

    pub fn paused() -> Self {
        Self::InProgress(DownloadProgressStatus::Paused)
    }

    pub fn error<S: std::fmt::Display>(error: S) -> Self {
        Self::InProgress(DownloadProgressStatus::Error(error.to_string()))
    }

    pub fn finished() -> Self {
        Self::Finished
    }

    pub fn to_human_variant(&self) -> &'static str {
        match self {
            DownloadStatus::InProgress(status) => match status {
                DownloadProgressStatus::Resumed(status) => match status {
                    DownloadResumedStatus::Queue => "Queue",
                    DownloadResumedStatus::Waiting => "Waiting",
                    DownloadResumedStatus::Downloading => "Downloading",
                },
                DownloadProgressStatus::Paused => "Paused",
                DownloadProgressStatus::Error(_) => "Error",
            },
            DownloadStatus::Finished => "Finished",
        }
    }

    pub fn message(&self) -> Option<&str> {
        match self {
            DownloadStatus::InProgress(DownloadProgressStatus::Error(err)) => Some(err.as_str()),
            _ => None,
        }
    }

    pub fn to_human_string(&self) -> String {
        match self.message() {
            Some(str) => format!("{}: {}", self.to_human_variant(), str),
            None => self.to_human_variant().to_string(),
        }
    }

    pub fn as_resumed(&self) -> Option<&DownloadResumedStatus> {
        self.as_in_progress().and_then(|it| it.as_resumed())
    }

    pub fn as_in_progress(&self) -> Option<&DownloadProgressStatus> {
        if let Self::InProgress(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn message() {
        macro_rules! assert {
            ($name:ident  $(($nameparam:expr))?, $right: expr) => {
                assert_eq!(DownloadStatus::$name( $($nameparam)? ).message(), $right);
            };
        }

        assert!(waiting, None);
        assert!(finished, None);
        assert!(queued, None);
        assert!(downloading, None);
        assert!(paused, None);
        assert!(error("Err"), Some("Err"));
    }

    #[test]
    pub fn to_human_variant() {
        macro_rules! assert {
            ($name:ident  $(($nameparam:expr))?, $right: expr) => {
                assert_eq!(DownloadStatus::$name( $($nameparam)? ).to_human_variant(), $right);
            };
        }

        assert!(waiting, "Waiting");
        assert!(finished, "Finished");
        assert!(queued, "Queue");
        assert!(downloading, "Downloading");
        assert!(paused, "Paused");
        assert!(error("Err"), "Error");
    }

    #[test]
    pub fn to_human_string() {
        macro_rules! assert {
            ($name:ident  $(($nameparam:expr))?, $right: expr) => {
                assert_eq!(DownloadStatus::$name( $($nameparam)? ).to_human_string(), $right);
            };
        }

        assert!(waiting, "Waiting");
        assert!(finished, "Finished");
        assert!(queued, "Queue");
        assert!(downloading, "Downloading");
        assert!(paused, "Paused");
        assert!(error("Err"), "Error: Err");
    }
}
