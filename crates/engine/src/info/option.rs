use std::sync::Arc;

use parking_lot::Mutex;

use crate::TaskSchedulerOption;

#[derive(Debug)]
struct Inner {
    sanitize_option: Mutex<SanitizeOptions>,
    scheduler: Arc<TaskSchedulerOption>,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            sanitize_option: Default::default(),
            scheduler: Default::default(),
        }
    }
}

#[derive(Debug)]
struct SanitizeOptions {
    windows: bool,
    truncate: bool,
    replacement: String,
}

impl Default for SanitizeOptions {
    fn default() -> Self {
        Self {
            windows: true,
            truncate: false,
            replacement: "_".to_string(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct DownloadOption(Arc<Inner>);

impl DownloadOption {
    pub fn sanitize_filename(&self, name: &str) -> String {
        let this = self.0.lock();

        let option = &this.sanitize_option;
        let option = sanitize_filename::Options {
            truncate: option.truncate,
            windows: option.windows,
            replacement: &option.replacement,
        };

        return sanitize_filename::sanitize_with_options(name, option);
    }

    pub fn set_sanitize_replacement(&self, replacement: String) {
        self.0.sanitize_option.lock().replacement = replacement;
    }

    pub fn scheduler(&self) -> Arc<TaskSchedulerOption> {
        self.0.scheduler.clone()
    }

    pub fn set_download_limit(&self, value: usize) {
        self.0.scheduler.set_download_limit(value);
    }
}
