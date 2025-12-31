
pub mod precise_decimal;
pub mod parameters;
pub mod fhir_version;
#[cfg(feature = "R4")]
pub mod r4;
#[cfg(feature = "R4B")]
pub mod r4b;
#[cfg(feature = "R5")]
pub mod r5;
#[cfg(feature = "R6")]
pub mod r6;
pub mod date_time;
mod element;


use serde::{Deserialize, Deserializer};
// Re-export commonly used types from parameters module
pub use parameters::{ParameterValueAccessor, VersionIndependentParameters};

/// Custom deserializer that is more forgiving of null values in JSON.
///
/// This creates a custom `Option<T>` deserializer that will return None for null values
/// but also for any deserialization errors. This makes it possible to skip over
/// malformed or unexpected values in FHIR JSON.
pub fn deserialize_forgiving_option<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    // Use the intermediate Value approach to check for null first
    let json_value = serde_json::Value::deserialize(deserializer)?;

    match json_value {
        serde_json::Value::Null => Ok(None),
        _ => {
            // Try to deserialize the value, but return None if it fails
            match T::deserialize(json_value) {
                Ok(value) => Ok(Some(value)),
                Err(_) => Ok(None), // Ignore errors and return None
            }
        }
    }
}


