use crate::unit_conversion::ConversionError;
use tracing::{error, warn};

/// Macro to replace unreachable!() with proper error handling and logging
macro_rules! handle_unknown_unit {
    ($unit:expr, $unit_type:expr) => {{
        error!(
            unit = $unit,
            unit_type = $unit_type,
            "Encountered unexpected unit in conversion function"
        );
        return Err(ConversionError::UnknownUnit($unit.to_string()));
    }};
}

/// Wrapper for unit conversion with error recovery
pub fn safe_convert_with_fallback<F>(
    conversion_fn: F,
    input: &str,
    output_unit: &str,
) -> Result<String, ConversionError>
where
    F: Fn(&str, &str) -> Result<String, ConversionError>,
{
    match conversion_fn(input, output_unit) {
        Ok(result) => Ok(result),
        Err(e) => {
            // Log the error with full context
            error!(
                input = input,
                output_unit = output_unit,
                error = %e,
                "Unit conversion failed"
            );
            
            // Try to provide helpful suggestions
            if let ConversionError::UnknownUnit(unit) = &e {
                warn!(
                    unknown_unit = unit,
                    "Consider adding support for this unit or checking for typos"
                );
            }
            
            Err(e)
        }
    }
}

/// Extension trait for Results in unit conversion context
pub trait ConversionResultExt<T> {
    /// Log conversion errors with context
    fn log_conversion_error(self, input: &str, output_unit: &str) -> Self;
}

impl<T> ConversionResultExt<T> for Result<T, ConversionError> {
    fn log_conversion_error(self, input: &str, output_unit: &str) -> Self {
        if let Err(ref e) = self {
            error!(
                input = input,
                output_unit = output_unit,
                error = %e,
                "Unit conversion failed"
            );
        }
        self
    }
}