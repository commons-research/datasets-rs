//! Download progress reporting for data sources.

use indicatif::{ProgressBar, ProgressStyle};

/// Displays progress for an artifact download
#[derive(Clone)]
pub struct DownloadProgress {
    progress_bar: ProgressBar,
}

impl DownloadProgress {
    /// Creates a progress display for download
    #[must_use]
    pub fn new(total_bytes: u64, message: String) -> Self {
        let progress_bar = if total_bytes > 0 {
            ProgressBar::new(total_bytes)
        } else {
            ProgressBar::new_spinner()
        };

        let style = if total_bytes > 0 {
            download_bar_style()
        } else {
            download_spinner_style()
        };

        progress_bar.set_style(style);
        progress_bar.set_message(message);

        Self { progress_bar }
    }

    /// Sets expected download size when available
    pub fn set_total_bytes(&self, total_bytes: u64) {
        if total_bytes == 0 {
            return;
        }
        if self.progress_bar.length().is_none() {
            self.progress_bar.set_style(download_bar_style());
        }
        self.progress_bar.set_length(total_bytes);
    }

    /// Sets the cumulative number of bytes downloaded
    pub fn set_position(&self, bytes_completed: u64) {
        self.progress_bar.set_position(bytes_completed);
    }
    
    /// Marks download as successful
    pub fn finish_with_message(&self, message: String) {
        self.progress_bar.finish_with_message(message);
    }

    /// Marks download unsuccessful 
    pub fn abandon_with_message(&self, message: String) {
        self.progress_bar.abandon_with_message(message);
    }
}

/// Creates progress-bar style used for download
fn download_bar_style() -> ProgressStyle {
    match ProgressStyle::with_template("[{elapsed_precise}] {bar:30.cyan/blue} {percent:>3}% \
         {binary_bytes}/{binary_total_bytes} {binary_bytes_per_sec} \
         ETA {eta} {msg}") {
            Ok(style) => style.progress_chars("=>-"),
            Err(_) => ProgressStyle::default_bar()
         }
}

/// Creates the spinner stlye used when total size not known
fn download_spinner_style() -> ProgressStyle {
    match ProgressStyle::with_template("[{elapsed_precise}] {spinner} {binary_bytes} {msg}") {
        Ok(style) => style,
        Err(_) => ProgressStyle::default_spinner()
    }
}