use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use crate::evaluation_result::EvaluationResult;

/// Trait for FHIR choice element types.
///
/// This trait is implemented by generated enum types that represent FHIR choice elements
/// (fields with [x] in the FHIR specification). It provides metadata about the choice
/// element that enables proper polymorphic access in FHIRPath expressions.
///
/// # Example
///
/// For a FHIR field like `Observation.value[x]`, the generated enum would implement:
/// ```rust,ignore
/// impl ChoiceElement for ObservationValue {
///     fn base_name() -> &'static str {
///         "value"
///     }
///     
///     fn possible_field_names() -> Vec<&'static str> {
///         vec!["valueQuantity", "valueCodeableConcept", "valueString", ...]
///     }
/// }
/// ```
pub trait ChoiceElement {
    /// Returns the base name of the choice element without the [x] suffix.
    ///
    /// For example, for `value[x]`, this returns "value".
    fn base_name() -> &'static str;

    /// Returns all possible field names that this choice element can manifest as.
    ///
    /// For example, for `value[x]`, this might return:
    /// ["valueQuantity", "valueCodeableConcept", "valueString", ...]
    fn possible_field_names() -> Vec<&'static str>;
}

/// Trait for FHIR resource metadata.
///
/// This trait is implemented by generated FHIR resource structs to provide
/// metadata about the resource's structure, particularly which fields are
/// choice elements. This enables accurate polymorphic field access in FHIRPath.
///
/// # Example
///
/// ```rust,ignore
/// impl FhirResourceMetadata for Observation {
///     fn choice_elements() -> &'static [&'static str] {
///         &["value", "effective", "component.value"]
///     }
/// }
/// ```
pub trait FhirResourceMetadata {
    /// Returns the names of all choice element fields in this resource.
    ///
    /// The returned slice contains the base names (without [x]) of fields
    /// that are choice elements in the FHIR specification.
    fn choice_elements() -> &'static [&'static str];
}

/// Universal conversion trait for transforming values into FHIRPath evaluation results.
///
/// This trait provides the bridge between FHIR data types and the FHIRPath evaluation
/// system. It allows any type to be converted into an `EvaluationResult` that can be
/// processed by FHIRPath expressions.
///
/// # Implementation Guidelines
///
/// When implementing this trait:
/// - Return `EvaluationResult::Empty` for `None` or missing values
/// - Use appropriate variant types (Boolean, String, Integer, etc.)
/// - For complex types, use `EvaluationResult::Object` with field mappings
/// - For arrays/collections, use `EvaluationResult::Collection`
///
/// # Examples
///
/// ```rust
/// use helios_fhirpath_support::{EvaluationResult, IntoEvaluationResult};
///
/// struct CustomType {
///     value: String,
///     active: bool,
/// }
///
/// impl IntoEvaluationResult for CustomType {
///     fn to_evaluation_result(&self) -> EvaluationResult {
///         let mut map = std::collections::HashMap::new();
///         map.insert("value".to_string(), self.value.to_evaluation_result());
///         map.insert("active".to_string(), self.active.to_evaluation_result());
///         EvaluationResult::Object { map, type_info: None }
///     }
/// }
/// ```
pub trait IntoEvaluationResult {
    /// Converts this value into a FHIRPath evaluation result.
    ///
    /// This method should transform the implementing type into the most appropriate
    /// `EvaluationResult` variant that represents the value's semantics in FHIRPath.
    fn to_evaluation_result(&self) -> EvaluationResult;
}
// === IntoEvaluationResult Implementations ===
//
// The following implementations provide conversions from standard Rust types
// and common patterns into EvaluationResult variants. These enable seamless
// integration between Rust code and the FHIRPath evaluation system.

/// Converts a `String` to `EvaluationResult::String`.
///
/// This is the most direct conversion for text values in the FHIRPath system.
impl IntoEvaluationResult for String {
    fn to_evaluation_result(&self) -> EvaluationResult {
        EvaluationResult::string(self.clone())
    }
}

/// Converts a `bool` to `EvaluationResult::Boolean`.
///
/// Enables direct use of Rust boolean values in FHIRPath expressions.
impl IntoEvaluationResult for bool {
    fn to_evaluation_result(&self) -> EvaluationResult {
        EvaluationResult::boolean(*self)
    }
}

/// Converts an `i32` to `EvaluationResult::Integer`.
///
/// Automatically promotes to `i64` for consistent integer handling.
impl IntoEvaluationResult for i32 {
    fn to_evaluation_result(&self) -> EvaluationResult {
        EvaluationResult::integer(*self as i64)
    }
}

/// Converts an `i64` to `EvaluationResult::Integer`.
///
/// This is the primary integer type used in FHIRPath evaluation.
impl IntoEvaluationResult for i64 {
    fn to_evaluation_result(&self) -> EvaluationResult {
        EvaluationResult::integer64(*self)
    }
}

/// Converts an `f64` to `EvaluationResult::Decimal` with error handling.
///
/// Uses high-precision `Decimal` type to avoid floating-point errors.
/// Returns `Empty` for invalid values like NaN or Infinity.
impl IntoEvaluationResult for f64 {
    fn to_evaluation_result(&self) -> EvaluationResult {
        Decimal::from_f64(*self)
            .map(EvaluationResult::decimal)
            .unwrap_or(EvaluationResult::Empty)
    }
}

/// Converts a `rust_decimal::Decimal` to `EvaluationResult::Decimal`.
///
/// This is the preferred conversion for precise decimal values in FHIR.
impl IntoEvaluationResult for Decimal {
    fn to_evaluation_result(&self) -> EvaluationResult {
        EvaluationResult::decimal(*self)
    }
}

// === Generic Container Implementations ===
//
// These implementations handle common Rust container types, enabling
// seamless conversion of complex data structures to FHIRPath results.

/// Converts `Option<T>` to either the inner value's result or `Empty`.
///
/// This is fundamental for handling FHIR's optional fields and nullable values.
/// `Some(value)` converts the inner value, `None` becomes `Empty`.
impl<T> IntoEvaluationResult for Option<T>
where
    T: IntoEvaluationResult,
{
    fn to_evaluation_result(&self) -> EvaluationResult {
        match self {
            Some(value) => value.to_evaluation_result(),
            None => EvaluationResult::Empty,
        }
    }
}

/// Converts `Vec<T>` to `EvaluationResult::Collection`.
///
/// Each item in the vector is converted to an `EvaluationResult`. The resulting
/// collection is marked as having defined order (FHIRPath collections maintain order).
impl<T> IntoEvaluationResult for Vec<T>
where
    T: IntoEvaluationResult,
{
    fn to_evaluation_result(&self) -> EvaluationResult {
        let collection: Vec<EvaluationResult> = self
            .iter()
            .map(|item| item.to_evaluation_result())
            .collect();
        EvaluationResult::Collection {
            items: collection,
            has_undefined_order: false,
            type_info: None,
        }
    }
}

/// Converts `Box<T>` to the result of the boxed value.
///
/// This enables use of boxed values (often used to break circular references
/// in FHIR data structures) directly in FHIRPath evaluation.
impl<T> IntoEvaluationResult for Box<T>
where
    T: IntoEvaluationResult + ?Sized,
{
    fn to_evaluation_result(&self) -> EvaluationResult {
        (**self).to_evaluation_result()
    }
}

// New
impl<T> IntoEvaluationResult for &T
where
    T: IntoEvaluationResult + ?Sized,
{
    fn to_evaluation_result(&self) -> EvaluationResult {
        (*self).to_evaluation_result()
    }
}
/// Convenience function for converting values to evaluation results.
///
/// This function provides a unified interface for conversion that can be used
/// by the evaluator and macro systems. It's particularly useful when working
/// with trait objects or in generic contexts.
///
/// # Arguments
///
/// * `value` - Any value implementing `IntoEvaluationResult`
///
/// # Returns
///
/// The `EvaluationResult` representation of the input value.
///
/// # Examples
///
/// ```rust
/// use helios_fhirpath_support::{convert_value_to_evaluation_result, EvaluationResult};
///
/// let result = convert_value_to_evaluation_result(&"hello".to_string());
/// assert_eq!(result, EvaluationResult::String("hello".to_string(), None));
///
/// let numbers = vec![1, 2, 3];
/// let collection_result = convert_value_to_evaluation_result(&numbers);
/// assert_eq!(collection_result.count(), 3);
/// ```
pub fn convert_value_to_evaluation_result<T>(value: &T) -> EvaluationResult
where
    T: IntoEvaluationResult + ?Sized,
{
    value.to_evaluation_result()
}

/// Formats a unit for display in toString() output
pub(crate) fn format_unit_for_display(unit: &str) -> String {
    // FHIRPath spec formatting for units in toString():
    // - Calendar word units (week, day, etc.): displayed without quotes
    // - UCUM code units ('wk', 'mg', etc.): displayed with quotes

    // Calendar word units that don't need quotes
    const CALENDAR_WORDS: &[&str] = &[
        "year",
        "years",
        "month",
        "months",
        "week",
        "weeks",
        "day",
        "days",
        "hour",
        "hours",
        "minute",
        "minutes",
        "second",
        "seconds",
        "millisecond",
        "milliseconds",
    ];

    if CALENDAR_WORDS.contains(&unit) {
        // Calendar word units: display without quotes (R5 behavior, likely correct)
        unit.to_string()
    } else {
        // UCUM code units: display with quotes
        format!("'{}'", unit)
    }
}