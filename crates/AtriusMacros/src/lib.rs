//! # FHIR Macro - Procedural Macros for FHIR Implementation
//!
//! This crate provides procedural macros that enable automatic code generation for FHIR
//! (Fast Healthcare Interoperability Resources) implementations in Rust. It contains the
//! core macro functionality that powers serialization, deserialization, and FHIRPath
//! evaluation across the entire FHIR ecosystem.

//!
//! ## Overview
//!
//! The `fhir_macro` crate implements two essential derive macros:
//!
//! - **`#[derive(FhirSerde)]`** - Custom serialization/deserialization handling FHIR's
//!   JSON representation including its extension pattern
//! - **`#[derive(FhirPath)]`** - Automatic conversion to FHIRPath evaluation results for
//!   resource traversal
//!
//! These macros are automatically applied to thousands of generated FHIR types, eliminating
//! the need for hand-written serialization code while ensuring compliance with FHIR's
//! complex serialization requirements.
//!
//! ## FHIR Serialization Challenges
//!
//! FHIR has several unique serialization patterns that require special handling:
//!
//! ### Extension Pattern
//!
//! FHIR primitives can have associated metadata stored in a parallel `_fieldName` object:
//!
//! ```json
//! {
//!   "status": "active",
//!   "_status": {
//!     "id": "status-1",
//!     "extension": [...]
//!   }
//! }
//! ```
//!
//! ### Array Serialization
//!
//! Arrays of primitives are split into separate primitive and extension arrays:
//!
//! ```json
//! {
//!   "given": ["John", "Michael", null],
//!   "_given": [null, {"id": "name-2"}, {}]
//! }
//! ```
//!
//! ### Choice Types
//!
//! FHIR's `[x]` fields are serialized as single key-value pairs with type suffixes:
//!
//! ```json
//! { "valueQuantity": {...} }  // for Quantity type
//! { "valueString": "text" }   // for String type
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use fhir_macro::{FhirSerde, FhirPath};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, FhirSerde, FhirPath, Default)]
//! pub struct Patient {
//!     pub id: Option<String>,
//!     pub extension: Option<Vec<Extension>>,
//!     #[fhir_serde(rename = "implicitRules")]
//!     pub implicit_rules: Option<Uri>,
//!     pub active: Option<Boolean>,  // Element<bool, Extension>
//!     pub name: Option<Vec<HumanName>>,
//! }
//! ```

extern crate proc_macro;


use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};
use crate::deserialize_impl::generate_deserialize_impl;
use crate::extract_type_names_elements::extract_type_info_attributes;
use crate::fhir_path_enum_impl::generate_fhirpath_enum_impl;
use crate::fhir_path_field_struct_impl::generate_fhirpath_struct_impl;
use crate::serialize_is_empty_impl::{generate_is_empty_impl, generate_serialize_impl};

pub(crate) mod field_helpers;
pub(crate) mod type_helpers;
pub(crate) mod serialize_is_empty_impl;
pub(crate) mod deserialize_impl;
pub(crate) mod fhir_path_field_struct_impl;
pub(crate) mod extract_type_names_elements;
pub(crate) mod fhir_path_enum_impl;
pub(crate) mod fhir_validate;

/// Derives `serde::Serialize` and `serde::Deserialize` implementations for FHIR types.
///
/// This procedural macro automatically generates serialization and deserialization code
/// that handles FHIR's complex JSON representation patterns, including the extension
/// pattern, choice types, and array serialization.
///
/// # Supported Attributes
///
/// - `#[fhir_serde(rename = "name")]` - Renames a field for serialization
/// - `#[fhir_serde(flatten)]` - Flattens a field into the parent object
///
/// # Generated Implementations
///
/// The macro generates both `Serialize` and `Deserialize` implementations that:
///
/// ## For Structs:
/// - Handle FHIR extension pattern (`field` and `_field` pairs)
/// - Support `Element<T, Extension>` and `DecimalElement<Extension>` types
/// - Serialize arrays with split primitive/extension arrays
/// - Apply field renaming and flattening as specified
///
/// ## For Enums:
/// - Serialize as single key-value pairs for choice types
/// - Handle extension patterns for element-containing variants
/// - Support resource type enums with proper discriminators
///
/// # FHIR Extension Pattern
///
/// For fields containing Element types, the macro automatically handles the FHIR
/// extension pattern where primitives and their metadata are stored separately:
///
/// ```json
/// {
///   "status": "active",        // Primitive value
///   "_status": {               // Extension metadata
///     "id": "status-1",
///     "extension": [...]
///   }
/// }
/// ```
///
/// # Examples
///
/// ```rust,ignore
/// use fhir_macro::FhirSerde;
///
/// #[derive(FhirSerde)]
/// pub struct Patient {
///     pub id: Option<String>,
///     #[fhir_serde(rename = "implicitRules")]
///     pub implicit_rules: Option<Uri>,
///     pub active: Option<Boolean>,  // Element<bool, Extension>
/// }
///
/// #[derive(FhirSerde)]
/// pub enum ObservationValue {
///     #[fhir_serde(rename = "valueQuantity")]
///     Quantity(Quantity),
///     #[fhir_serde(rename = "valueString")]
///     String(String),
/// }
/// ```
///
/// # Error Handling
///
/// The generated deserialization code includes comprehensive error handling:
/// - Field-specific error messages for debugging
/// - Graceful handling of missing or malformed extension data
/// - Type validation for choice types and element containers
///
/// # Performance
///
/// The generated code is optimized for:
/// - Minimal allocations during serialization/deserialization
/// - Efficient field access using direct struct field access
/// - Lazy evaluation of extension objects (only when present)
/// - Vector pre-allocation for known array sizes
#[proc_macro_derive(FhirSerde, attributes(fhir_serde))]
pub fn fhir_serde_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let generics = input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let serialize_impl = generate_serialize_impl(&input.data, &name);

    // Pass all generic parts to deserialize generator
    let deserialize_impl = generate_deserialize_impl(&input.data, &name);
    let is_empty_impl = generate_is_empty_impl(
        &input.data,
        &name,
        &impl_generics,
        &ty_generics,
        where_clause,
    )
        .unwrap_or_default();

    let expanded = quote! {
        // --- Serialize Implementation ---
        impl #impl_generics serde::Serialize for #name #ty_generics #where_clause {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                #serialize_impl
            }
        }

        // --- Deserialize Implementation ---
        impl<'de> #impl_generics serde::Deserialize<'de> for #name #ty_generics #where_clause
        where
            // Add bounds for generic types used in fields if necessary
            // Example: T: serde::Deserialize<'de>,
        {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                #deserialize_impl
            }
        }

        #is_empty_impl
    };

    TokenStream::from(expanded)
}

//=============================================================================
// FHIRPath Derive Macro and Implementation Functions
//=============================================================================

/// Derives the `helios_fhirpath_support::IntoEvaluationResult` trait for FHIRPath evaluation.
///
/// This procedural macro automatically generates implementations that convert FHIR
/// types into `EvaluationResult` objects that can be used in FHIRPath expressions.
/// This enables seamless integration between FHIR resources and the FHIRPath evaluator.
///
/// # Generated Implementations
///
/// ## For Structs:
/// - Converts struct fields to an `EvaluationResult::Object` with a HashMap
/// - Uses FHIR field names (respecting `#[fhir_serde(rename)]` attributes)
/// - Filters out empty/None fields to produce clean object representations
/// - Handles nested objects recursively through the trait
///
/// ## For Enums:
/// - **Choice types**: Delegates to the contained value's implementation
/// - **Resource enum**: Adds `resourceType` field automatically for resource variants
/// - **Unit variants**: Returns the variant name as a string (for status codes, etc.)
///
/// # FHIRPath Integration
///
/// The generated implementations enable FHIR resources to be used directly in
/// FHIRPath expressions such as:
/// - `Patient.name.family` - Access nested object properties
/// - `Observation.value.unit` - Access choice type properties  
/// - `Bundle.entry.resource.resourceType` - Access resource type discriminators
///
/// # Field Name Handling
///
/// Field names in the resulting object follow FHIR naming conventions:
/// - Uses `#[fhir_serde(rename = "name")]` if present
/// - Otherwise uses the raw Rust field identifier (not converted to camelCase)
/// - This ensures FHIRPath expressions match FHIR specification naming
///
/// # Examples
///
/// ```rust,ignore
/// use fhir_macro::FhirPath;
/// use helios_fhirpath_support::{IntoEvaluationResult, EvaluationResult};
///
/// #[derive(FhirPath)]
/// pub struct Patient {
///     pub id: Option<String>,
///     #[fhir_serde(rename = "implicitRules")]
///     pub implicit_rules: Option<Uri>,
///     pub active: Option<Boolean>,
/// }
///
/// // Usage in FHIRPath evaluation
/// let patient = Patient {
///     id: Some("123".to_string()),
///     active: Some(Boolean::from(true)),
///     implicit_rules: None,  // Filtered out
/// };
///
/// let result = patient.into_evaluation_result();
/// // Results in EvaluationResult::Object with:
/// // - "id" → "123"
/// // - "active" → true  
/// // - "implicitRules" field omitted (was None)
/// ```
///
/// # Resource Enum Special Handling
///
/// For the top-level `Resource` enum, the macro automatically adds the `resourceType`
/// field to enable proper FHIRPath resource type discrimination:
///
/// ```rust,ignore
/// #[derive(FhirPath)]
/// pub enum Resource {
///     Patient(Patient),
///     Observation(Observation),
/// }
///
/// // Resource::Patient(patient_data) becomes:
/// // {
/// //   "resourceType": "Patient",
/// //   ...patient_data fields...
/// // }
/// ```
///
/// # Empty Field Filtering
///
/// The generated implementation automatically filters out fields that evaluate to
/// `EvaluationResult::Empty`, ensuring clean object representations for FHIRPath
/// traversal. This includes:
/// - `None` values in `Option<T>` fields
/// - Empty collections
/// - Objects with no meaningful content
#[proc_macro_derive(FhirPath, attributes(fhir_serde, fhir_choice_element, fhir_resource))]
pub fn fhir_path_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let trait_impl = match &input.data {
        Data::Struct(data) => generate_fhirpath_struct_impl(
            name,
            data,
            &input.attrs,
            &impl_generics,
            &ty_generics,
            where_clause,
        ),
        Data::Enum(data) => generate_fhirpath_enum_impl(
            name,
            data,
            &input.attrs,
            &impl_generics,
            &ty_generics,
            where_clause,
        ),
        Data::Union(_) => panic!("FhirPath derive macro does not support unions."),
    };

    TokenStream::from(trait_impl)
}

// Derive macro for TypeInfo trait.
///
/// This macro generates implementations of the TypeInfo trait for FHIR types,
/// providing type namespace and name information needed by the FHIRPath type() function.
///
/// # Attributes
///
/// - `#[type_info(namespace = "FHIR", name = "boolean")]` - Specifies custom namespace and name
/// - If not specified, defaults are inferred from the type name
///
/// # Examples
///
/// ```rust,ignore
/// #[derive(TypeInfo)]
/// #[type_info(namespace = "FHIR", name = "boolean")]
/// pub struct Boolean(pub Element<bool, Extension>);
///
/// #[derive(TypeInfo)]
/// pub struct Patient {
///     // fields...
/// }
/// ```
#[proc_macro_derive(TypeInfo, attributes(type_info))]
pub fn type_info_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Extract type_info attributes if present
    let (namespace, type_name) = extract_type_info_attributes(&input.attrs, name);

    let expanded = quote! {
        impl #impl_generics helios_fhirpath_support::TypeInfo for #name #ty_generics #where_clause {
            fn type_namespace() -> &'static str {
                #namespace
            }

            fn type_name() -> &'static str {
                #type_name
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(FhirValidate, attributes(fhir_invariant, fhir_binding))]
pub fn derive_fhir_validate(input: TokenStream) -> TokenStream {
    fhir_validate::derive(input)
}
