use chrono::Local;
use std::fs::{self, File};
use std::path::Path;
use std::sync::Once;
use tracing::Level;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, time::ChronoUtc},
    prelude::*,
};

static INIT: Once = Once::new();

/// Initialize JSON logging to a file
pub fn init_json_logging() {
    INIT.call_once(|| {
        // Create logs directory if it doesn't exist
        let logs_dir = Path::new("logs");
        if !logs_dir.exists() {
            fs::create_dir_all(logs_dir).expect("Failed to create logs directory");
        }

        // Create log file with timestamp in the name
        let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
        let log_file_path = logs_dir.join(format!("garden_cli_{}.json", timestamp));
        let file = File::create(&log_file_path).expect("Failed to create log file");

        // Create a JSON layer that writes to the file
        let json_layer = fmt::layer()
            .with_writer(file)
            .json()
            .with_timer(ChronoUtc::rfc_3339());

        // Create a console layer for standard output
        let console_layer = fmt::layer()
            .with_writer(std::io::stdout)
            .with_ansi(true)
            .with_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()));

        // Register both layers
        tracing_subscriber::registry()
            .with(json_layer)
            .with(console_layer)
            .init();

        tracing::info!("JSON logging initialized to file: {:?}", log_file_path);
    });
}
