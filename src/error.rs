use color_eyre::eyre::{self, WrapErr};
use tracing::{error, warn};

/// Initialize error handling
pub fn init() -> eyre::Result<()> {
    // Install color-eyre for better error reporting
    color_eyre::install()?;
    Ok(())
}

/// Extension trait for Results to add logging capabilities
pub trait ResultExt<T> {
    /// Log an error and return it
    fn log_error(self) -> Self;

    /// Log a warning if error and convert to Option
    fn log_warn(self) -> Option<T>;

    /// Log error with additional context
    fn log_error_with_context(self, context: &str) -> Self;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn log_error(self) -> Self {
        if let Err(ref e) = self {
            error!(error = %e, "Operation failed");
        }
        self
    }

    fn log_warn(self) -> Option<T> {
        match self {
            Ok(val) => Some(val),
            Err(e) => {
                warn!(error = %e, "Non-critical error occurred");
                None
            }
        }
    }

    fn log_error_with_context(self, context: &str) -> Self {
        if let Err(ref e) = self {
            error!(error = %e, context = context, "Operation failed with context");
        }
        self
    }
}

/// Helper function to get environment variable with proper error handling
pub fn get_env_var(name: &str) -> eyre::Result<String> {
    std::env::var(name)
        .wrap_err_with(|| format!("Failed to read environment variable '{name}'"))
}

/// Helper function to get optional environment variable
pub fn get_optional_env_var(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(val) => Some(val),
        Err(std::env::VarError::NotPresent) => None,
        Err(e) => {
            warn!(
                var_name = name,
                error = %e,
                "Failed to read environment variable"
            );
            None
        }
    }
}