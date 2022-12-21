use std::sync::Arc;

use parking_lot::Mutex;

#[derive(Debug)]
struct Inner {
    sanitize_option: SanitizeOptions,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            sanitize_option: Default::default(),
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
pub struct DownloadOption(Arc<Mutex<Inner>>);

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
        self.0.lock().sanitize_option.replacement = replacement;
    }
}
