use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Initialise file-based logging. The returned `WorkerGuard` must be held for
/// the lifetime of the process — dropping it flushes and closes the log file.
pub fn init(log_level: &str) -> WorkerGuard {
    let log_dir = dirs::data_local_dir()
        .expect("cannot resolve local data directory")
        .join("shadow")
        .join("logs");

    std::fs::create_dir_all(&log_dir).expect("failed to create log directory");

    let file_appender = tracing_appender::rolling::daily(&log_dir, "shadow.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let level = log_level.parse::<tracing::Level>().unwrap_or(tracing::Level::INFO);
    // Only log our own crate at the configured level; silence all third-party crates
    // (AWS SDK, GCS client, etc. log credentials and internal state at INFO which is noise).
    // Log our crate at the configured level; only errors from third-party crates.
    let filter_str = format!("error,shadow_lib={level}");
    let filter = EnvFilter::new(&filter_str);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_span_events(FmtSpan::NONE)
        .with_filter(filter);

    #[cfg(debug_assertions)]
    {
        let stderr_filter = EnvFilter::new(&filter_str);
        let stderr_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_ansi(true)
            .with_span_events(FmtSpan::NONE)
            .with_filter(stderr_filter);

        tracing_subscriber::registry()
            .with(file_layer)
            .with(stderr_layer)
            .init();
    }

    #[cfg(not(debug_assertions))]
    {
        tracing_subscriber::registry().with(file_layer).init();
    }

    guard
}

/// Returns the log directory path as a string.
pub fn log_dir_path() -> String {
    dirs::data_local_dir()
        .expect("cannot resolve local data directory")
        .join("shadow")
        .join("logs")
        .to_string_lossy()
        .to_string()
}
