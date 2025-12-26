use std::sync::Arc;
use rust_decimal::Decimal;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde::ser::SerializeStruct;
use atrius_fhirpath_support::evaluation_result::EvaluationResult;
use atrius_fhirpath_support::traits::IntoEvaluationResult;

/// High-precision decimal type that preserves original string representation.
///
/// FHIR requires that decimal values maintain their original precision and format
/// when serialized back to JSON. This type stores both the parsed `Decimal` value
/// for mathematical operations and the original string for serialization.
///
/// # FHIR Precision Requirements
///
/// FHIR decimal values must:
/// - Preserve trailing zeros (e.g., "12.340" vs "12.34")
/// - Maintain original precision during round-trip serialization
/// - Support high-precision arithmetic without floating-point errors
/// - Handle edge cases like very large or very small numbers
///
/// # Examples
///
/// ```rust
/// use helios_fhir::PreciseDecimal;
/// use rust_decimal::Decimal;
///
/// // Create from Decimal (derives string representation)
/// let precise = PreciseDecimal::from(Decimal::new(12340, 3)); // 12.340
/// assert_eq!(precise.original_string(), "12.340");
///
/// // Create with specific string format
/// let precise = PreciseDecimal::from_parts(
///     Some(Decimal::new(1000, 2)),
///     "10.00".to_string()
/// );
/// assert_eq!(precise.original_string(), "10.00");
/// ```
#[derive(Debug, Clone)]
pub struct PreciseDecimal {
    /// The parsed decimal value, `None` if parsing failed (e.g., out of range)
    value: Option<Decimal>,
    /// The original string representation preserving format and precision
    original_string: Arc<str>,
}
/// Implements equality comparison based on the parsed decimal value.
///
/// Two `PreciseDecimal` values are equal if their parsed `Decimal` values are equal,
/// regardless of their original string representations. This enables mathematical
/// equality while preserving string format for serialization.
///
/// # Examples
///
/// ```rust
/// use helios_fhir::PreciseDecimal;
/// use rust_decimal::Decimal;
///
/// let a = PreciseDecimal::from_parts(Some(Decimal::new(100, 1)), "10.0".to_string());
/// let b = PreciseDecimal::from_parts(Some(Decimal::new(1000, 2)), "10.00".to_string());
/// assert_eq!(a, b); // Same decimal value (10.0 == 10.00)
/// ```
impl PartialEq for PreciseDecimal {
    fn eq(&self, other: &Self) -> bool {
        // Compare parsed decimal values for mathematical equality
        self.value == other.value
    }
}

/// Marker trait implementation indicating total equality for `PreciseDecimal`.
impl Eq for PreciseDecimal {}

/// Implements partial ordering based on the parsed decimal value.
///
/// Ordering is based on the mathematical value of the decimal, not the string
/// representation. `None` values (unparseable decimals) are considered less than
/// any valid decimal value.
impl PartialOrd for PreciseDecimal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
/// Implements total ordering for `PreciseDecimal`.
///
/// Provides a consistent ordering for sorting operations. The ordering is based
/// on the mathematical value: `None` < `Some(smaller_decimal)` < `Some(larger_decimal)`.
impl Ord for PreciseDecimal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

// === PreciseDecimal Methods ===

impl PreciseDecimal {
/// Creates a new `PreciseDecimal` from its constituent parts.
///
/// This constructor allows explicit control over both the parsed value and the
/// original string representation. Use this when you need to preserve a specific
/// string format or when parsing has already been attempted.
///
/// # Arguments
///
/// * `value` - The parsed decimal value, or `None` if parsing failed
/// * `original_string` - The original string representation to preserve
///
/// # Examples
///
/// ```rust
/// use helios_fhir::PreciseDecimal;
/// use rust_decimal::Decimal;
///
/// // Create with successful parsing
/// let precise = PreciseDecimal::from_parts(
///     Some(Decimal::new(12340, 3)),
///     "12.340".to_string()
/// );
///
/// // Create with failed parsing (preserves original string)
/// let invalid = PreciseDecimal::from_parts(
///     None,
///     "invalid_decimal".to_string()
/// );
/// ```
pub fn from_parts(value: Option<Decimal>, original_string: String) -> Self {
    Self {
        value,
        original_string: Arc::from(original_string.as_str()),
    }
}

/// Helper method to parse a decimal string with support for scientific notation.
///
/// This method handles the complexity of parsing decimal strings that may be in
/// scientific notation (with 'E' or 'e' exponents) or regular decimal format.
/// It normalizes 'E' to 'e' for consistent parsing while preserving the original
/// string representation for serialization.
///
/// # Arguments
///
/// * `s` - The string to parse as a decimal
///
/// # Returns
///
/// `Some(Decimal)` if parsing succeeds, `None` if the string is not a valid decimal.
///
/// # Examples
///
/// ```ignore
/// use helios_fhir::PreciseDecimal;
/// use rust_decimal::Decimal;
///
/// // Regular decimal format
/// assert!(PreciseDecimal::parse_decimal_string("123.45").is_some());
///
/// // Scientific notation with 'e'
/// assert!(PreciseDecimal::parse_decimal_string("1.23e2").is_some());
///
/// // Scientific notation with 'E' (normalized to 'e')
/// assert!(PreciseDecimal::parse_decimal_string("1.23E2").is_some());
///
/// // Invalid format
/// assert!(PreciseDecimal::parse_decimal_string("invalid").is_none());
/// ```
fn parse_decimal_string(s: &str) -> Option<Decimal> {
    // Normalize 'E' to 'e' for consistent parsing
    let normalized = s.replace('E', "e");

    if normalized.contains('e') {
        // Use scientific notation parsing
        Decimal::from_scientific(&normalized).ok()
    } else {
        // Use regular decimal parsing
        normalized.parse::<Decimal>().ok()
    }
}

/// Returns the parsed decimal value if parsing was successful.
///
/// This method provides access to the mathematical value for arithmetic
/// operations and comparisons. Returns `None` if the original string
/// could not be parsed as a valid decimal.
///
/// # Examples
///
/// ```rust
/// use helios_fhir::PreciseDecimal;
/// use rust_decimal::Decimal;
///
/// let precise = PreciseDecimal::from(Decimal::new(1234, 2)); // 12.34
/// assert_eq!(precise.value(), Some(Decimal::new(1234, 2)));
///
/// let invalid = PreciseDecimal::from_parts(None, "invalid".to_string());
/// assert_eq!(invalid.value(), None);
/// ```
pub fn value(&self) -> Option<Decimal> {
    self.value
}

    /// Returns the original string representation.
    ///
    /// This method provides access to the exact string format that was used
    /// to create this `PreciseDecimal`. This string is used during serialization
    /// to maintain FHIR's precision requirements.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use helios_fhir::PreciseDecimal;
    /// use rust_decimal::Decimal;
    ///
    /// let precise = PreciseDecimal::from_parts(
    ///     Some(Decimal::new(100, 2)),
    ///     "1.00".to_string()
    /// );
    /// assert_eq!(precise.original_string(), "1.00");
    /// ```
    pub fn original_string(&self) -> &str {
        &self.original_string
    }
}

/// Converts a `Decimal` to `PreciseDecimal` with derived string representation.
///
/// This implementation allows easy conversion from `rust_decimal::Decimal` values
/// by automatically generating the string representation using the decimal's
/// `Display` implementation.
///
/// # Examples
///
/// ```rust
/// use helios_fhir::PreciseDecimal;
/// use rust_decimal::Decimal;
///
/// let decimal = Decimal::new(12345, 3); // 12.345
/// let precise: PreciseDecimal = decimal.into();
/// assert_eq!(precise.value(), Some(decimal));
/// assert_eq!(precise.original_string(), "12.345");
/// ```
impl From<Decimal> for PreciseDecimal {
    fn from(value: Decimal) -> Self {
        // Generate string representation from the decimal value
        let original_string = Arc::from(value.to_string());
        Self {
            value: Some(value),
            original_string,
        }
    }
}

/// Implements serialization for `PreciseDecimal` preserving original format.
///
/// This implementation ensures that the exact original string representation
/// is preserved during JSON serialization, maintaining FHIR's precision
/// requirements including trailing zeros and specific formatting.
///
/// # FHIR Compliance
///
/// FHIR requires that decimal values maintain their original precision when
/// round-tripped through JSON. This implementation uses `serde_json::RawValue`
/// to serialize the original string directly as a JSON number.
///
/// # Examples
///
/// ```rust
/// use helios_fhir::PreciseDecimal;
/// use rust_decimal::Decimal;
/// use serde_json;
///
/// let precise = PreciseDecimal::from_parts(
///     Some(Decimal::new(1230, 2)),
///     "12.30".to_string()
/// );
///
/// let json = serde_json::to_string(&precise).unwrap();
/// assert_eq!(json, "12.30"); // Preserves trailing zero
/// ```
impl Serialize for PreciseDecimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Use RawValue to preserve exact string format in JSON
        match serde_json::value::RawValue::from_string(self.original_string.to_string()) {
            Ok(raw_value) => raw_value.serialize(serializer),
            Err(e) => Err(serde::ser::Error::custom(format!(
                "Failed to serialize PreciseDecimal '{}': {}",
                self.original_string, e
            ))),
        }
    }
}

/// Implements deserialization for `PreciseDecimal` preserving original format.
///
/// This implementation deserializes JSON numbers and strings into `PreciseDecimal`
/// while preserving the exact original string representation. It handles various
/// JSON formats including scientific notation and nested object structures.
///
/// # Supported Formats
///
/// - Direct numbers: `12.340`
/// - String numbers: `"12.340"`
/// - Scientific notation: `1.234e2` or `1.234E2`
/// - Nested objects: `{"value": 12.340}` (for macro-generated structures)
///
/// # Examples
///
/// ```rust
/// use helios_fhir::PreciseDecimal;
/// use serde_json;
///
/// // Deserialize from JSON number (trailing zeros are normalized)
/// let precise: PreciseDecimal = serde_json::from_str("12.340").unwrap();
/// assert_eq!(precise.original_string(), "12.340"); // JSON number format
///
/// // Deserialize from JSON string (preserves exact format)
/// let precise: PreciseDecimal = serde_json::from_str("\"12.340\"").unwrap();
/// assert_eq!(precise.original_string(), "12.340"); // Preserves string format
/// ```
impl<'de> Deserialize<'de> for PreciseDecimal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Use intermediate Value to capture exact string representation
        let json_value = serde_json::Value::deserialize(deserializer)?;

        match json_value {
            serde_json::Value::Number(n) => {
                // Extract string representation from JSON number
                let original_string = n.to_string();
                let parsed_value = Self::parse_decimal_string(&original_string);
                Ok(PreciseDecimal::from_parts(parsed_value, original_string))
            }
            serde_json::Value::String(s) => {
                // Use string value directly (preserves exact format)
                let parsed_value = Self::parse_decimal_string(&s);
                Ok(PreciseDecimal::from_parts(parsed_value, s))
            }
            // Handle nested object format (for macro-generated structures)
            serde_json::Value::Object(map) => match map.get("value") {
                Some(serde_json::Value::Number(n)) => {
                    let original_string = n.to_string();
                    let parsed_value = Self::parse_decimal_string(&original_string);
                    Ok(PreciseDecimal::from_parts(parsed_value, original_string))
                }
                Some(serde_json::Value::String(s)) => {
                    let original_string = s.clone();
                    let parsed_value = Self::parse_decimal_string(&original_string);
                    Ok(PreciseDecimal::from_parts(parsed_value, original_string))
                }
                Some(serde_json::Value::Null) => Err(de::Error::invalid_value(
                    de::Unexpected::Unit,
                    &"a number or string for decimal value",
                )),
                None => Err(de::Error::missing_field("value")),
                _ => Err(de::Error::invalid_type(
                    de::Unexpected::Map,
                    &"a map with a 'value' field containing a number or string",
                )),
            },
            // Handle remaining unexpected types
            other => Err(de::Error::invalid_type(
                match other {
                    serde_json::Value::Null => de::Unexpected::Unit, // Or Unexpected::Option if mapping null to None
                    serde_json::Value::Bool(b) => de::Unexpected::Bool(b),
                    serde_json::Value::Array(_) => de::Unexpected::Seq,
                    _ => de::Unexpected::Other("unexpected JSON type for PreciseDecimal"),
                },
                &"a number, string, or object with a 'value' field",
            )),
        }
    }
}

// --- End PreciseDecimal ---
/// Specialized element container for FHIR decimal values with precision preservation.
///
/// This type combines the generic `Element` pattern with `PreciseDecimal` to provide
/// a complete solution for FHIR decimal elements that require both extension support
/// and precision preservation during serialization round-trips.
///
/// # Type Parameters
///
/// * `E` - The extension type (typically the generated `Extension` struct)
///
/// # FHIR Decimal Requirements
///
/// FHIR decimal elements must:
/// - Preserve original string precision (e.g., "12.30" vs "12.3")
/// - Support mathematical operations using `Decimal` arithmetic
/// - Handle extension metadata through `id` and `extension` fields
/// - Serialize back to the exact original format when possible
///
/// # Examples
///
/// ```rust
/// use helios_fhir::{DecimalElement, PreciseDecimal, r4::Extension};
/// use rust_decimal::Decimal;
///
/// // Create from a Decimal value
/// let decimal_elem = DecimalElement::<Extension>::new(Decimal::new(1234, 2)); // 12.34
///
/// // Create with extensions
/// let extended_decimal: DecimalElement<Extension> = DecimalElement {
///     value: Some(PreciseDecimal::from_parts(
///         Some(Decimal::new(12300, 3)),
///         "12.300".to_string()
///     )),
///     id: Some("precision-example".to_string()),
///     extension: Some(vec![/* extensions */]),
/// };
///
/// // Access the mathematical value
/// if let Some(precise) = &extended_decimal.value {
///     if let Some(decimal_val) = precise.value() {
///         println!("Mathematical value: {}", decimal_val);
///     }
///     println!("Original format: {}", precise.original_string());
/// }
/// ```
///
/// # Serialization Behavior
///
/// - **Value only**: Serializes as a JSON number preserving original precision
/// - **With extensions**: Serializes as an object with `value`, `id`, and `extension` fields
/// - **No value**: Serializes as an object with just the extension fields, or `null` if empty
///
/// # Integration with FHIRPath
///
/// When used with FHIRPath evaluation, `DecimalElement` returns:
/// - The `Decimal` value for mathematical operations
/// - An object representation when extension metadata is accessed
/// - Empty collection when the element has no value or extensions
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct DecimalElement<E> {
    /// Optional element identifier for referencing within the resource
    pub id: Option<String>,
    /// Optional extensions providing additional metadata
    pub extension: Option<Vec<E>>,
    /// The decimal value with precision preservation
    pub value: Option<PreciseDecimal>,
}
impl<E> DecimalElement<E> {
/// Creates a new `DecimalElement` with the specified decimal value.
///
/// This constructor creates a simple decimal element with no extensions or ID,
/// containing only the decimal value. The original string representation is
/// automatically derived from the `Decimal` value's `Display` implementation.
///
/// # Arguments
///
/// * `value` - The `Decimal` value to store
///
/// # Returns
///
/// A new `DecimalElement` with the value set and `id`/`extension` as `None`.
///
/// # Examples
///
/// ```rust
/// use helios_fhir::{DecimalElement, r4::Extension};
/// use rust_decimal::Decimal;
///
/// // Create a simple decimal element
/// let element = DecimalElement::<Extension>::new(Decimal::new(12345, 3)); // 12.345
///
/// // Verify the structure
/// assert!(element.id.is_none());
/// assert!(element.extension.is_none());
/// assert!(element.value.is_some());
///
/// // Access the decimal value
/// if let Some(precise_decimal) = &element.value {
///     assert_eq!(precise_decimal.value(), Some(Decimal::new(12345, 3)));
///     assert_eq!(precise_decimal.original_string(), "12.345");
/// }
/// ```
///
/// # Usage in FHIR Resources
///
/// This method is typically used when creating FHIR elements programmatically:
///
/// ```rust
/// use helios_fhir::{DecimalElement, r4::{Extension, Observation}};
/// use rust_decimal::Decimal;
///
/// let temperature = DecimalElement::<Extension>::new(Decimal::new(3672, 2)); // 36.72
///
/// // Would be used in an Observation like:
/// // observation.value_quantity.value = Some(temperature);
/// ```
pub fn new(value: Decimal) -> Self {
    // Convert the Decimal to PreciseDecimal, which automatically handles
    // storing the original string representation via the From trait
    let precise_value = PreciseDecimal::from(value);
    Self {
        id: None,
        extension: None,
        value: Some(precise_value),
    }
}

    /// Returns `true` if the element has no value, id, or extensions.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.value.is_none() && self.id.is_none() && self.extension.is_none()
    }
}
// Custom Deserialize for DecimalElement<E> using intermediate Value
impl<'de, E> Deserialize<'de> for DecimalElement<E>
where
    E: Deserialize<'de> + Default,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize into an intermediate serde_json::Value first
        let json_value = serde_json::Value::deserialize(deserializer)?;

        match json_value {
            // Handle primitive JSON Number
            serde_json::Value::Number(n) => {
                // Directly parse the number string to create PreciseDecimal
                let s = n.to_string(); // Note: n.to_string() might normalize exponent case (e.g., 'E' -> 'e')
                // Replace 'E' with 'e' for parsing
                let s_for_parsing = s.replace('E', "e");
                // Use from_scientific if 'e' is present, otherwise parse
                let parsed_value = if s_for_parsing.contains('e') {
                    Decimal::from_scientific(&s_for_parsing).ok()
                } else {
                    s_for_parsing.parse::<Decimal>().ok()
                };
                // Store the ORIGINAL string `s` (as returned by n.to_string()).
                let pd = PreciseDecimal::from_parts(parsed_value, s);
                Ok(DecimalElement {
                    id: None,
                    extension: None,
                    value: Some(pd),
                })
            }
            // Handle primitive JSON String
            serde_json::Value::String(s) => {
                // Directly parse the string to create PreciseDecimal
                // Replace 'E' with 'e' for parsing
                let s_for_parsing = s.replace('E', "e");
                // Use from_scientific if 'e' is present, otherwise parse
                let parsed_value = if s_for_parsing.contains('e') {
                    Decimal::from_scientific(&s_for_parsing).ok()
                } else {
                    s_for_parsing.parse::<Decimal>().ok()
                };
                // Store the ORIGINAL string `s`.
                let pd = PreciseDecimal::from_parts(parsed_value, s); // s is owned, no clone needed
                Ok(DecimalElement {
                    id: None,
                    extension: None,
                    value: Some(pd),
                })
            }
            // Handle JSON object: deserialize fields individually
            serde_json::Value::Object(map) => {
                let mut id: Option<String> = None;
                let mut extension: Option<Vec<E>> = None;
                let mut value: Option<PreciseDecimal> = None;

                for (k, v) in map {
                    match k.as_str() {
                        "id" => {
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            // Deserialize id directly from its Value
                            id = Deserialize::deserialize(v).map_err(de::Error::custom)?;
                        }
                        "extension" => {
                            if extension.is_some() {
                                return Err(de::Error::duplicate_field("extension"));
                            }
                            // Deserialize extension directly from its Value
                            extension = Deserialize::deserialize(v).map_err(de::Error::custom)?;
                        }
                        "value" => {
                            if value.is_some() {
                                return Err(de::Error::duplicate_field("value"));
                            }
                            // Deserialize value using PreciseDecimal::deserialize from its Value
                            // Handle null explicitly within the value field
                            if v.is_null() {
                                value = None;
                            } else {
                                value = Some(
                                    PreciseDecimal::deserialize(v).map_err(de::Error::custom)?,
                                );
                            }
                        }
                        // Ignore any unknown fields encountered
                        _ => {} // Simply ignore unknown fields
                    }
                }
                Ok(DecimalElement {
                    id,
                    extension,
                    value,
                })
            }
            // Handle JSON Null for the whole element
            serde_json::Value::Null => Ok(DecimalElement::default()), // Default has value: None
            // Handle other unexpected types
            other => Err(de::Error::invalid_type(
                match other {
                    serde_json::Value::Bool(b) => de::Unexpected::Bool(b),
                    serde_json::Value::Array(_) => de::Unexpected::Seq,
                    _ => de::Unexpected::Other("unexpected JSON type for DecimalElement"),
                },
                &"a decimal number, string, object, or null",
            )),
        }
    }
}

// Reinstate custom Serialize implementation for DecimalElement
// Remove PartialEq bound for E
impl<E> Serialize for DecimalElement<E>
where
    E: Serialize, // Removed PartialEq bound for E
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // If we only have a value and no other fields, serialize just the value
        if self.id.is_none() && self.extension.is_none() {
            if let Some(value) = &self.value {
                // Serialize the PreciseDecimal directly, invoking its custom Serialize impl
                return value.serialize(serializer);
            } else {
                // If value is also None, serialize as null
                // based on updated test_serialize_decimal_with_no_fields
                return serializer.serialize_none();
            }
        }

        // Otherwise, serialize as a struct with all present fields
        // Calculate the number of fields that are NOT None
        let mut len = 0;
        if self.id.is_some() {
            len += 1;
        }
        if self.extension.is_some() {
            len += 1;
        }
        if self.value.is_some() {
            len += 1;
        }

        // Start serializing a struct with the calculated length
        let mut state = serializer.serialize_struct("DecimalElement", len)?;

        // Serialize 'id' field if it's Some
        if let Some(id) = &self.id {
            state.serialize_field("id", id)?;
        }

        // Serialize 'extension' field if it's Some
        if let Some(extension) = &self.extension {
            state.serialize_field("extension", extension)?;
        }

        // Serialize 'value' field if it's Some
        if let Some(value) = &self.value {
            // Serialize the PreciseDecimal directly, invoking its custom Serialize impl
            state.serialize_field("value", value)?;
        }

        // End the struct serialization
        state.end()
    }
}
// For DecimalElement<E> - Returns Decimal value if present, otherwise handles id/extension
impl<E> IntoEvaluationResult for DecimalElement<E>
where
    E: IntoEvaluationResult + Clone,
{
    fn to_evaluation_result(&self) -> EvaluationResult {
        // Prioritize returning the primitive decimal value if it exists
        if let Some(precise_decimal) = &self.value {
            if let Some(decimal_val) = precise_decimal.value() {
                // Return FHIR decimal
                return EvaluationResult::fhir_decimal(decimal_val);
            }
            // If PreciseDecimal holds None for value, fall through to check id/extension
        }

        // If value is None, but id or extension exist, return an Object with those
        if self.id.is_some() || self.extension.is_some() {
            let mut map = std::collections::HashMap::new();
            if let Some(id) = &self.id {
                map.insert("id".to_string(), EvaluationResult::string(id.clone()));
            }
            if let Some(ext) = &self.extension {
                let ext_collection: Vec<EvaluationResult> =
                    ext.iter().map(|e| e.to_evaluation_result()).collect();
                if !ext_collection.is_empty() {
                    map.insert(
                        "extension".to_string(),
                        EvaluationResult::collection(ext_collection),
                    );
                }
            }
            // Only return Object if map is not empty
            if !map.is_empty() {
                return EvaluationResult::typed_object(map, "FHIR", "decimal");
            }
        }
        // If value, id, and extension are all None, return Empty
        EvaluationResult::Empty
    }
}