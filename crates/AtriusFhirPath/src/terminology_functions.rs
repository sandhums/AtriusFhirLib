//!
//! # Terminology Functions for FHIRPath `%terminologies` Object
//!
//! This module implements the `%terminologies` functions as defined in the [FHIRPath specification](https://hl7.org/fhirpath/N1/#terminology-functions),
//! providing a bridge between FHIRPath evaluation and external FHIR terminology servers.
//!
//! ## What are `%terminologies` Functions?
//! The `%terminologies` object in FHIRPath exposes functions for terminology operations such as:
//! - ValueSet expansion (`expand`)
//! - Code lookup (`lookup`)
//! - Code validation against a ValueSet or CodeSystem (`validateVS`, `validateCS`)
//! - Subsumption testing (`subsumes`)
//! - Concept mapping/translation (`translate`)
//! - Membership testing (`memberOf`)
//!
//! These functions enable FHIRPath expressions to interact with terminology content managed externally,
//! such as SNOMED CT, LOINC, or custom code systems and value sets.
//!
//! ## How Does This Module Work?
//! This module provides Rust implementations of the `%terminologies` functions, mapping each to the
//! corresponding FHIR RESTful operation (e.g., `ValueSet/$expand`, `CodeSystem/$validate-code`, etc.)
//! via an internal [`TerminologyClient`]. It handles extraction of arguments from [`EvaluationResult`]
//! objects, invocation of the appropriate FHIR operation, and conversion of the FHIR server's
//! Parameters/JSON response back to an `EvaluationResult`.
//!
//! ## Synchronous Wrappers for Async Operations
//! Terminology operations are network-bound and implemented as async functions in the client.
//! However, FHIRPath evaluation is often performed in a synchronous context (e.g., for CQL or
//! rule engines). To bridge this, the module provides a `block_on_async` helper that runs async
//! code in a blocking (sync) manner, using either the current Tokio runtime or a dedicated one.
//! This enables `%terminologies` functions to be used from both sync and async Rust code.
//!
//! ## Relation to Binding Validation (`memberOf`)
//! FHIRPath's `memberOf()` function, and more generally "binding validation", checks if a code
//! or Coding is a member of a ValueSet. This is implemented here by delegating to
//! `%terminologies.validateVS()` and extracting the boolean result from the FHIR Parameters
//! response, following the FHIRPath and FHIR validation semantics.
//!
//! ## Summary
//! In summary, this module enables FHIRPath terminology functions to:
//! - Call out to FHIR terminology servers for dynamic validation and expansion
//! - Work seamlessly in both sync and async Rust contexts
//! - Correctly marshal arguments and results between FHIRPath, Rust, and FHIR REST APIs
//! - Provide the basis for binding validation and terminology-aware logic in FHIRPath expressions

use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Handle;

use serde_json::Value;

use crate::evaluator::EvaluationContext;
use crate::terminology_client::TerminologyClient;
use atrius_fhirpath_support::evaluation_result::EvaluationError;
use atrius_fhirpath_support::evaluation_result::EvaluationResult;

lazy_static::lazy_static! {
    /// A lazily-initialized Tokio runtime used for running async terminology operations from sync contexts.
    ///
    /// # Safety and Blocking Semantics
    /// - This runtime is only used when not already inside a Tokio async context.
    /// - When running inside an async context, we use `block_in_place` to avoid blocking reactor threads.
    /// - This approach allows terminology functions to be called from both async and sync Rust code
    ///   without deadlocking or stalling async tasks.
    static ref RUNTIME: tokio::runtime::Runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
}

/// Executes an async future in a context that may be synchronous or asynchronous.
///
/// - If called from within a Tokio runtime, uses `block_in_place` to avoid blocking reactor threads,
///   and blocks on the current runtime's handle.
/// - If not inside an async runtime, uses the module-wide `RUNTIME` to block.
/// - This ensures terminology operations can be called from both sync and async FHIRPath evaluation.
///
/// # Why use `block_in_place`?
/// To avoid blocking core async reactor threads, which could deadlock async code.
fn block_on_async<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    if let Ok(handle) = Handle::try_current() {
        tokio::task::block_in_place(move || handle.block_on(future))
    } else {
        RUNTIME.block_on(future)
    }
}

/// Provides FHIRPath `%terminologies` functions for terminology operations.
///
/// - This struct is the main entry point for FHIRPath terminology functions such as
///   `expand`, `lookup`, `validateVS`, `validateCS`, `subsumes`, and `translate`.
/// - Internally, it wraps a shared [`TerminologyClient`] (in an `Arc`) for efficient reuse.
/// - The client is typically reused from the [`EvaluationContext`], ensuring consistent
///   configuration (base URL, version, headers, etc.) and avoiding repeated connection setup.
/// - The base URL and FHIR version are determined from the context or provided explicitly,
///   following the configuration of the overall FHIRPath evaluation.
/// - The use of `Arc` allows safe sharing of the client across threads and evaluation contexts.
pub struct TerminologyFunctions {
    client: Arc<TerminologyClient>,
}

impl TerminologyFunctions {
    /// Creates a new `TerminologyFunctions` instance from an [`EvaluationContext`].
    ///
    /// - Reuses a shared [`TerminologyClient`] from the context if available,
    ///   avoiding duplicate configuration and connections.
    /// - If no client is present, creates a new one using the context's
    ///   terminology server URL and FHIR version.
    /// - The resulting client is wrapped in an `Arc` for thread-safe sharing.
    pub fn new(context: &EvaluationContext) -> Self {
        if let Some(shared) = context.get_terminology_client() {
            return Self { client: shared };
        }
        let server_url = context.get_terminology_server_url();
        let client = TerminologyClient::new(server_url, context.fhir_version);
        Self {
            client: Arc::new(client),
        }
    }

    /// Expands a ValueSet using the FHIR `ValueSet/$expand` operation.
    ///
    /// - **FHIRPath function:** `%terminologies.expand(valueSet, params)`
    /// - **Arguments:**
    ///   - `valueSet`: An `EvaluationResult::String` containing the ValueSet canonical URL.
    ///   - `params`: Optional object or Parameters resource, mapped to FHIR operation parameters.
    /// - **FHIR endpoint:** `ValueSet/$expand`
    /// - **Returns:** An `EvaluationResult` representing the expanded ValueSet (FHIR JSON).
    /// - **Error handling:** Returns `EvaluationError` if argument types are invalid or the operation fails.
    ///
    /// This function is typically used to enumerate all codes in a ValueSet, e.g. for displaying options.
    pub fn expand(
        &self,
        value_set: &EvaluationResult,
        params: Option<&EvaluationResult>,
    ) -> Result<EvaluationResult, EvaluationError> {
        // Extract ValueSet URL
        let value_set_url = match value_set {
            EvaluationResult::String(url, _) => url.clone(),
            _ => {
                return Err(EvaluationError::TypeError(
                    "expand() requires a ValueSet URL as string".to_string(),
                ));
            }
        };

        // Extract parameters if provided
        let params_map = extract_params_map(params)?;

        // Execute async operation in blocking context
        let client = self.client.clone();
        let result = block_on_async(async move { client.expand(&value_set_url, params_map).await });

        match result {
            Ok(value) => json_to_evaluation_result(value),
            Err(e) => Err(EvaluationError::InvalidOperation(format!(
                "ValueSet expansion failed: {}",
                e
            ))),
        }
    }

    /// Performs a FHIR code lookup using the `CodeSystem/$lookup` operation.
    ///
    /// - **FHIRPath function:** `%terminologies.lookup(coded, params)`
    /// - **Arguments:**
    ///   - `coded`: An `EvaluationResult` representing a Coding or code string; must provide `system` and `code`.
    ///   - `params`: Optional key-value parameters for FHIR lookup.
    /// - **FHIR endpoint:** `CodeSystem/$lookup`
    /// - **Returns:** An `EvaluationResult` corresponding to the FHIR Parameters response with code details.
    /// - **Error handling:** Returns `EvaluationError` if extraction or network fails.
    ///
    /// Use this to retrieve properties, display, or other metadata for a code in a system.
    pub fn lookup(
        &self,
        coded: &EvaluationResult,
        params: Option<&EvaluationResult>,
    ) -> Result<EvaluationResult, EvaluationError> {
        // Extract system and code from Coding
        let (system, code) = extract_coding(coded)?;

        // Extract parameters if provided
        let params_map = extract_params_map(params)?;

        // Execute async operation
        let client = self.client.clone();
        let result = block_on_async(async move { client.lookup(&system, &code, params_map).await });

        match result {
            Ok(value) => json_to_evaluation_result(value),
            Err(e) => Err(EvaluationError::InvalidOperation(format!(
                "Code lookup failed: {}",
                e
            ))),
        }
    }

    /// Validates a code or Coding against a ValueSet using `ValueSet/$validate-code`.
    ///
    /// - **FHIRPath function:** `%terminologies.validateVS(valueSet, coded, params)`
    /// - **Arguments:**
    ///   - `valueSet`: An `EvaluationResult::String` with the ValueSet canonical URL.
    ///   - `coded`: An `EvaluationResult` for a code string or Coding object (should contain `system`, `code`, optional `display`).
    ///   - `params`: Optional parameter object or Parameters resource; merged with system/code/display.
    /// - **FHIR endpoint:** `ValueSet/$validate-code`
    /// - **Subtle points:** Most FHIR servers (e.g., Snowstorm, HAPI) require a `system` when a `code` is provided. If `system` is empty, it is omitted.
    ///   The `system_opt` is derived from the Coding or code string; if empty, it is not sent.
    /// - **Returns:** An `EvaluationResult` representing the FHIR Parameters response, typically with `parameter[name=result].valueBoolean` indicating membership.
    /// - **Error handling:** Returns `EvaluationError` if argument types are invalid or server returns error.
    ///
    /// This function is the canonical way to check ValueSet membership or binding validity.
    pub fn validate_vs(
        &self,
        value_set: &EvaluationResult,
        coded: &EvaluationResult,
        params: Option<&EvaluationResult>,
    ) -> Result<EvaluationResult, EvaluationError> {
        // Extract ValueSet URL
        let value_set_url = match value_set {
            EvaluationResult::String(url, _) => url.clone(),
            _ => {
                return Err(EvaluationError::TypeError(
                    "validateVS() requires a ValueSet URL as string".to_string(),
                ));
            }
        };

        // Extract coding information (system, code, display)
        let (system, code, display) = extract_coding_with_display(coded)?;

        // Extract parameters
        let params_map = extract_params_map(params)?;

        // Execute async operation
        let client = self.client.clone();
        let system_opt = if system.is_empty() {
            None
        } else {
            Some(system.clone())
        };

        let result = block_on_async(async move {
            let system_ref = system_opt.as_deref();
            let display_ref = display.as_deref();
            // Use ValueSet/$validate-code for ValueSet validation.
            // This is the canonical endpoint for checking if a code is in a ValueSet.
            client
                .validate_vs(&value_set_url, system_ref, &code, display_ref, params_map)
                .await
        });

        match result {
            Ok(value) => json_to_evaluation_result(value),
            Err(e) => Err(EvaluationError::InvalidOperation(format!(
                "ValueSet validation failed: {}",
                e
            ))),
        }
    }

    /// Validates a code or Coding against a CodeSystem using `CodeSystem/$validate-code`.
    ///
    /// - **FHIRPath function:** `%terminologies.validateCS(codeSystem, coded, params)`
    /// - **Arguments:**
    ///   - `codeSystem`: An `EvaluationResult::String` with the CodeSystem canonical URL.
    ///   - `coded`: An `EvaluationResult` for a code string or Coding object (should contain `code` and optional `display`).
    ///   - `params`: Optional parameter object or Parameters resource.
    /// - **FHIR endpoint:** `CodeSystem/$validate-code`
    /// - **Subtle points:** This function must use the CodeSystem endpoint, not ValueSet. Historically, some implementations incorrectly pointed to ValueSet/$validate-code.
    ///   Verify that the endpoint matches the FHIR spec for CodeSystem validation.
    /// - **Returns:** An `EvaluationResult` representing the FHIR Parameters response, typically with `parameter[name=result].valueBoolean`.
    /// - **Error handling:** Returns `EvaluationError` on extraction or network failure.
    ///
    /// Use this to validate that a code exists within a CodeSystem, not just in a ValueSet.
    pub fn validate_cs(
        &self,
        code_system: &EvaluationResult,
        coded: &EvaluationResult,
        params: Option<&EvaluationResult>,
    ) -> Result<EvaluationResult, EvaluationError> {
        // Extract CodeSystem URL
        let code_system_url = match code_system {
            EvaluationResult::String(url, _) => url.clone(),
            _ => {
                return Err(EvaluationError::TypeError(
                    "validateCS() requires a CodeSystem URL as string".to_string(),
                ));
            }
        };

        // Extract code and display from Coding or code string
        let (_system, code, display) = extract_coding_with_display(coded)?;

        // Extract parameters
        let params_map = extract_params_map(params)?;

        // Execute async operation
        let client = self.client.clone();

        let result = block_on_async(async move {
            let display_ref = display.as_deref();
            // Use CodeSystem/$validate-code for CodeSystem validation.
            // This is distinct from ValueSet/$validate-code and is required by the FHIR spec.
            // Historically, some code incorrectly used ValueSet for this operation.
            client
                .validate_cs(&code_system_url, &code, display_ref, params_map)
                .await
        });

        match result {
            Ok(value) => json_to_evaluation_result(value),
            Err(e) => Err(EvaluationError::InvalidOperation(format!(
                "CodeSystem validation failed: {}",
                e
            ))),
        }
    }

    /// Checks if one code subsumes another using `CodeSystem/$subsumes`.
    ///
    /// - **FHIRPath function:** `%terminologies.subsumes(system, codedA, codedB, params)`
    /// - **Arguments:**
    ///   - `system`: An `EvaluationResult::String` with the CodeSystem URL.
    ///   - `coded1`, `coded2`: `EvaluationResult`s for Coding or code strings.
    ///   - `params`: Optional parameters.
    /// - **FHIR endpoint:** `CodeSystem/$subsumes`
    /// - **Returns:** An `EvaluationResult::String` containing the FHIR outcome code ("equivalent", "subsumes", etc.).
    /// - **Error handling:** Returns `EvaluationError` if arguments are invalid or operation fails.
    ///
    /// Use this to determine hierarchy/subsumption relationships between codes.
    pub fn subsumes(
        &self,
        system: &EvaluationResult,
        coded1: &EvaluationResult,
        coded2: &EvaluationResult,
        params: Option<&EvaluationResult>,
    ) -> Result<EvaluationResult, EvaluationError> {
        // Extract system URL
        let system_url = match system {
            EvaluationResult::String(url, _) => url.clone(),
            _ => {
                return Err(EvaluationError::TypeError(
                    "subsumes() requires a system URL as string".to_string(),
                ));
            }
        };

        // Extract codes
        let (_sys1, code1) = extract_coding(coded1)?;
        let (_sys2, code2) = extract_coding(coded2)?;

        // Extract parameters
        let params_map = extract_params_map(params)?;

        // Execute async operation
        let client = self.client.clone();
        let result = block_on_async(async move {
            client
                .subsumes(&system_url, &code1, &code2, params_map)
                .await
        });

        match result {
            Ok(value) => {
                // Extract the 'outcome' parameter value
                if let Some(parameters) = value.get("parameter").and_then(|p| p.as_array()) {
                    for param in parameters {
                        if param.get("name").and_then(|n| n.as_str()) == Some("outcome") {
                            if let Some(code) = param.get("valueCode").and_then(|c| c.as_str()) {
                                return Ok(EvaluationResult::string(code.to_string()));
                            }
                        }
                    }
                }
                Err(EvaluationError::InvalidOperation(
                    "subsumes() result missing outcome parameter".to_string(),
                ))
            }
            Err(e) => Err(EvaluationError::InvalidOperation(format!(
                "Subsumes check failed: {}",
                e
            ))),
        }
    }

    /// Translates a code using a ConceptMap and the FHIR `ConceptMap/$translate` operation.
    ///
    /// - **FHIRPath function:** `%terminologies.translate(conceptMap, code, params)`
    /// - **Arguments:**
    ///   - `conceptMap`: An `EvaluationResult::String` with the ConceptMap canonical URL.
    ///   - `code`: An `EvaluationResult` for a Coding or code string.
    ///   - `params`: Optional parameters; `targetSystem` may be extracted and sent as a distinct argument.
    /// - **FHIR endpoint:** `ConceptMap/$translate`
    /// - **Returns:** An `EvaluationResult` representing the FHIR Parameters response (typically with `match` parameters).
    /// - **Error handling:** Returns `EvaluationError` on failure.
    ///
    /// Use this to map codes between code systems (e.g., SNOMED -> ICD-10) using a ConceptMap.
    pub fn translate(
        &self,
        concept_map: &EvaluationResult,
        code: &EvaluationResult,
        params: Option<&EvaluationResult>,
    ) -> Result<EvaluationResult, EvaluationError> {
        // Extract ConceptMap URL
        let concept_map_url = match concept_map {
            EvaluationResult::String(url, _) => url.clone(),
            _ => {
                return Err(EvaluationError::TypeError(
                    "translate() requires a ConceptMap URL as string".to_string(),
                ));
            }
        };

        // Extract coding
        let (system, code_str) = extract_coding(code)?;

        // Extract target system from params if provided
        let mut params_map = extract_params_map(params)?;
        let target_system = params_map.as_mut().and_then(|m| m.remove("targetSystem"));

        // Execute async operation
        let client = self.client.clone();

        let result = block_on_async(async move {
            let target_system_ref = target_system.as_deref();
            client
                .translate(
                    &concept_map_url,
                    &system,
                    &code_str,
                    target_system_ref,
                    params_map,
                )
                .await
        });

        match result {
            Ok(value) => json_to_evaluation_result(value),
            Err(e) => Err(EvaluationError::InvalidOperation(format!(
                "Translation failed: {}",
                e
            ))),
        }
    }
}

/// Extracts system and code from a Coding or code string.
///
/// - **Supported shapes:**
///   - `EvaluationResult::String`: treated as a code with empty system.
///   - `EvaluationResult::Object` with `system` and `code` fields.
/// - **Failure modes:** Returns `TypeError` if `code` is missing or input is not a string/object.
/// - Used by most terminology operations to extract arguments.
fn extract_coding(coded: &EvaluationResult) -> Result<(String, String), EvaluationError> {
    match coded {
        // Direct code string
        EvaluationResult::String(code, _) => Ok((String::new(), code.clone())),

        // Coding object
        EvaluationResult::Object { map, .. } => {
            let system = map
                .get("system")
                .and_then(|v| match v {
                    EvaluationResult::String(s, _) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();

            let code = map
                .get("code")
                .and_then(|v| match v {
                    EvaluationResult::String(c, _) => Some(c.clone()),
                    _ => None,
                })
                .ok_or_else(|| {
                    EvaluationError::TypeError("Coding must have a 'code' element".to_string())
                })?;

            Ok((system, code))
        }

        _ => Err(EvaluationError::TypeError(
            "Expected string code or Coding object".to_string(),
        )),
    }
}

/// Extracts system, code, and display from a Coding or code string.
///
/// - **Supported shapes:**
///   - `EvaluationResult::String`: code with empty system and no display.
///   - `EvaluationResult::Object` with `system`, `code`, and optional `display`.
/// - **Failure modes:** Returns `TypeError` if `code` is missing or input is not a string/object.
fn extract_coding_with_display(
    coded: &EvaluationResult,
) -> Result<(String, String, Option<String>), EvaluationError> {
    match coded {
        // Direct code string
        EvaluationResult::String(code, _) => Ok((String::new(), code.clone(), None)),

        // Coding object
        EvaluationResult::Object { map, .. } => {
            let system = map
                .get("system")
                .and_then(|v| match v {
                    EvaluationResult::String(s, _) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_default();

            let code = map
                .get("code")
                .and_then(|v| match v {
                    EvaluationResult::String(c, _) => Some(c.clone()),
                    _ => None,
                })
                .ok_or_else(|| {
                    EvaluationError::TypeError("Coding must have a 'code' element".to_string())
                })?;

            let display = map.get("display").and_then(|v| match v {
                EvaluationResult::String(d, _) => Some(d.clone()),
                _ => None,
            });

            Ok((system, code, display))
        }

        _ => Err(EvaluationError::TypeError(
            "Expected string code or Coding object".to_string(),
        )),
    }
}

/// Extracts a parameter map from an `EvaluationResult` representing a Parameters resource or object.
///
/// - **Supported shapes:**
///   - `EvaluationResult::Object` with a `parameter` field (FHIR Parameters resource).
///   - `EvaluationResult::Object` with direct key-value pairs (simple map).
/// - **Failure modes:** Returns `TypeError` if not an object or Parameters resource.
/// - Only string, boolean, integer, or decimal parameter values are supported.
fn extract_params_map(
    params: Option<&EvaluationResult>,
) -> Result<Option<HashMap<String, String>>, EvaluationError> {
    match params {
        None => Ok(None),
        Some(EvaluationResult::Object { map, .. }) => {
            let mut params_map = HashMap::new();

            // Check if it's a Parameters resource
            if let Some(EvaluationResult::Collection { items, .. }) = map.get("parameter") {
                // Extract parameters from Parameters resource format
                for item in items {
                    if let EvaluationResult::Object { map: param_map, .. } = item {
                        if let (Some(name), Some(value)) = (
                            param_map.get("name").and_then(|n| match n {
                                EvaluationResult::String(s, _) => Some(s),
                                _ => None,
                            }),
                            extract_parameter_value(param_map),
                        ) {
                            params_map.insert(name.clone(), value);
                        }
                    }
                }
            } else {
                // Treat as simple key-value map
                for (key, value) in map {
                    if let EvaluationResult::String(v, _) = value {
                        params_map.insert(key.clone(), v.clone());
                    }
                }
            }

            Ok(Some(params_map))
        }
        Some(_) => Err(EvaluationError::TypeError(
            "Parameters must be an object or Parameters resource".to_string(),
        )),
    }
}

/// Extracts the value from a parameter element map.
///
/// - **Supported:** Finds the first `value[x]` key in the parameter map and returns its stringified value.
/// - **Failure:** Returns `None` if no value[x] present or not a supported type.
fn extract_parameter_value(param_map: &HashMap<String, EvaluationResult>) -> Option<String> {
    // Check for various value[x] types
    for (key, value) in param_map {
        if key.starts_with("value") {
            match value {
                EvaluationResult::String(s, _) => return Some(s.clone()),
                EvaluationResult::Boolean(b, _) => return Some(b.to_string()),
                EvaluationResult::Integer(i, _) => return Some(i.to_string()),
                EvaluationResult::Decimal(d, _) => return Some(d.to_string()),
                _ => {}
            }
        }
    }
    None
}

/// Converts a `serde_json::Value` (FHIR JSON) to an `EvaluationResult`.
///
/// - **Supported:** All JSON types, including objects and arrays, are recursively converted.
/// - **Failure:** Returns `EvaluationError` only if a nested value cannot be converted.
/// - Used to map FHIR server responses to FHIRPath results.
fn json_to_evaluation_result(value: Value) -> Result<EvaluationResult, EvaluationError> {
    match value {
        Value::Null => Ok(EvaluationResult::Empty),
        Value::Bool(b) => Ok(EvaluationResult::boolean(b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(EvaluationResult::integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(EvaluationResult::decimal(
                    rust_decimal::Decimal::from_f64_retain(f)
                        .unwrap_or(rust_decimal::Decimal::ZERO),
                ))
            } else {
                Ok(EvaluationResult::string(n.to_string()))
            }
        }
        Value::String(s) => Ok(EvaluationResult::string(s)),
        Value::Array(arr) => {
            let items: Result<Vec<_>, _> = arr.into_iter().map(json_to_evaluation_result).collect();
            Ok(EvaluationResult::Collection {
                items: items?,
                has_undefined_order: false,
                type_info: None,
            })
        }
        Value::Object(obj) => {
            let mut map = HashMap::new();
            for (key, val) in obj {
                map.insert(key, json_to_evaluation_result(val)?);
            }
            Ok(EvaluationResult::Object {
                map,
                type_info: None,
            })
        }
    }
}

/// Implements the FHIRPath `memberOf()` function for Coding/CodeableConcept.
///
/// - **FHIRPath function:** `coding.memberOf(valueSetUrl)`
/// - **How it works:** Delegates to `%terminologies.validateVS` and extracts the `parameter[name=result].valueBoolean`
///   from the FHIR Parameters response.
/// - **Default:** If the result is missing or not boolean, returns `false`.
/// - **Arguments:**
///   - `coding`: The code or Coding to check.
///   - `value_set_url`: The ValueSet canonical URL.
///   - `context`: The FHIRPath evaluation context.
/// - **Returns:** `EvaluationResult::Boolean` indicating membership.
/// - **Relation:** This is the standard FHIRPath binding membership check.
pub fn member_of(
    coding: &EvaluationResult,
    value_set_url: &str,
    context: &EvaluationContext,
) -> Result<EvaluationResult, EvaluationError> {
    let terminology = TerminologyFunctions::new(context);

    // Call validateVS and extract the result
    let validation_result = terminology.validate_vs(
        &EvaluationResult::string(value_set_url.to_string()),
        coding,
        None,
    )?;

    // Extract the 'result' parameter from the Parameters response
    if let EvaluationResult::Object { map, .. } = validation_result {
        if let Some(EvaluationResult::Collection { items, .. }) = map.get("parameter") {
            for item in items {
                if let EvaluationResult::Object { map: param_map, .. } = item {
                    if param_map.get("name").and_then(|n| match n {
                        EvaluationResult::String(s, _) => Some(s.as_str()),
                        _ => None,
                    }) == Some("result")
                    {
                        // Return the boolean value
                        if let Some(EvaluationResult::Boolean(result, type_info)) =
                            param_map.get("valueBoolean")
                        {
                            return Ok(EvaluationResult::Boolean(*result, type_info.clone()));
                        }
                    }
                }
            }
        }
    }

    // If we couldn't extract the result, return false
    Ok(EvaluationResult::boolean(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_coding_from_string() {
        let code = EvaluationResult::string("12345".to_string());
        let (system, code_str) = extract_coding(&code).unwrap();
        assert_eq!(system, "");
        assert_eq!(code_str, "12345");
    }

    #[test]
    fn test_extract_coding_from_object() {
        let mut map = HashMap::new();
        map.insert(
            "system".to_string(),
            EvaluationResult::string("http://loinc.org".to_string()),
        );
        map.insert(
            "code".to_string(),
            EvaluationResult::string("1234-5".to_string()),
        );
        map.insert(
            "display".to_string(),
            EvaluationResult::string("Test Code".to_string()),
        );

        let coding = EvaluationResult::Object {
            map,
            type_info: None,
        };

        let (system, code, display) = extract_coding_with_display(&coding).unwrap();
        assert_eq!(system, "http://loinc.org");
        assert_eq!(code, "1234-5");
        assert_eq!(display, Some("Test Code".to_string()));
    }
}
