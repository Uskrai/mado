use std::sync::Arc;

#[derive(PartialEq, Eq, Debug)]
pub struct QueueItem(pub by_address::ByAddress<Arc<crate::DownloadInfo>>);

impl std::ops::Deref for QueueItem {
    type Target = Arc<crate::DownloadInfo>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.order().partial_cmp(&other.0.order())
    }
}

impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.order().cmp(&other.0.order())
    }
}

impl QueueItem {
    pub fn new(info: Arc<crate::DownloadInfo>) -> Self {
        Self(by_address::ByAddress(info))
    }
}
