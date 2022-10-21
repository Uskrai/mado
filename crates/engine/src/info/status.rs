#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DownloadResumedStatus {
    Waiting,
    Downloading,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum DownloadProgressStatus {
    Resumed(DownloadResumedStatus),
    Paused,
    // we don't need StdError here because this is only used to shows to user
    Error(String),
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

    pub fn is_paused(&self) -> bool {
        matches!(self, Self::InProgress(DownloadProgressStatus::Paused))
    }

    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Finished)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::InProgress(DownloadProgressStatus::Error(..)))
    }

    pub fn resumed(status: DownloadResumedStatus) -> Self {
        Self::InProgress(DownloadProgressStatus::Resumed(status))
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
}
