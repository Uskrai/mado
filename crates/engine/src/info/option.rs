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
    convert_to_unicode: bool,
    replacement: String,
}

impl Default for SanitizeOptions {
    fn default() -> Self {
        Self {
            windows: true,
            truncate: false,
            convert_to_unicode: true,
            replacement: "_".to_string(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct DownloadOption(Arc<Inner>);

pub struct PatternReplace<'a>(&'a str, &'a str);

fn replace_all(haystack: &str, p: &[PatternReplace]) -> String {
    let pattern = p.iter().map(|it| it.0);

    let ac = aho_corasick::AhoCorasick::new(pattern);

    let mut result = String::new();

    ac.replace_all_with(haystack, &mut result, |mat, _, dst| {
        let replace = &p[mat.pattern()];
        dst.push_str(replace.1);
        true
    });

    result
}

impl DownloadOption {
    pub fn sanitize_filename(&self, name: &str) -> String {
        let option = &self.0.sanitize_option.lock();

        let name = if option.convert_to_unicode {
            replace_all(
                name,
                [
                    PatternReplace("?", "？"),
                    PatternReplace("/", "∕"),
                    PatternReplace("\\", "⧵"),
                    PatternReplace("<", "＜"),
                    PatternReplace(">", "＞"),
                    PatternReplace(":", "꞉"),
                    PatternReplace("*", "⁎"),
                ]
                .as_slice(),
            )
        } else {
            name.to_string()
        };

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
