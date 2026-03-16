use tracing_subscriber::EnvFilter;

/// Initialize the logging/tracing subsystem.
///
/// Log levels based on flags:
/// - Default (no flags): only errors
/// - `--verbose`: info + warn + error
/// - `--debug`: everything including trace and debug
///
/// The `ENVSAFE_LOG` environment variable can also be used to override
/// the filter (follows `tracing_subscriber::EnvFilter` syntax).
pub fn init(verbose: bool, debug_mode: bool) {
    // Check for env var override first
    let filter = if std::env::var("ENVSAFE_LOG").is_ok() {
        EnvFilter::from_env("ENVSAFE_LOG")
    } else if debug_mode {
        EnvFilter::new("envsafe=trace")
    } else if verbose {
        EnvFilter::new("envsafe=info")
    } else {
        EnvFilter::new("envsafe=error")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .init();

    tracing::debug!(
        "Logging initialized (verbose={}, debug={})",
        verbose,
        debug_mode
    );
}
