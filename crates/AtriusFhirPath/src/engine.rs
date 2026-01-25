//! Atrius FHIRPath evaluation engine.
//!
//! This crate provides the concrete `AtriusFhirPathEngine` which implements the
//! runtime-agnostic `atrius_fhirpath_support::FhirPathEngine` trait.
//!
//! ## What this engine does
//! - Parses a FHIRPath expression (typically an invariant expression).
//! - Evaluates it against a focus node (`EvaluationResult`).
//! - Coerces the output to a singleton boolean (`true`/`false`) as required by
//!   FHIR invariants.
//!
//! ## Remote terminology hook (ValueSet bindings)
//! Invariant evaluation (`eval_bool`) is purely local.
//! ValueSet binding validation is separate and can optionally call out to an
//! external terminology server via `FhirPathEngine::validate_code_in_valueset`.
//!
//! The HTTP-backed implementation (`HttpTerminologyProvider`) is gated behind
//! the `terminology-http` feature so downstream crates can opt out of HTTP
//! dependencies (or provide their own provider).

use std::sync::Arc;
use crate::parser::parser;
use crate::evaluator::{evaluate, EvaluationContext};
use chumsky::Parser;
use atrius_fhirpath_support::evaluation_error::EvaluationError;
use atrius_fhirpath_support::evaluation_result::EvaluationResult;
use atrius_fhirpath_support::FhirPathEngine;

/// Pluggable terminology adapter used for *remote* ValueSet/CodeSystem validation.
///
/// The FHIRPath engine itself is intentionally runtime-agnostic. To keep
/// `AtriusFhirPathEngine` usable in libraries and across different executors,
/// this interface is synchronous.
///
/// Implementations may:
/// - use a blocking HTTP client (simple and predictable),
/// - internally run async code by creating/using a runtime,
/// - or talk to an embedded/local terminology store.
///
/// Returning `None` means “unknown / cannot be verified” (misconfigured,
/// unavailable, network error, server error, etc.). The binding validator
/// decides how to degrade (warn vs error).
pub trait TerminologyProvider: Send + Sync {
    /// Validate whether `system|code` is in the ValueSet identified by `valueset_url`.
    ///
    /// Returns:
    /// - Some(true)  => confirmed member
    /// - Some(false) => confirmed NOT a member
    /// - None        => cannot decide (misconfigured, unavailable, etc.)
    fn validate_in_valueset(&self, valueset_url: &str, system: &str, code: &str) -> Option<bool>;
}

/// Configuration for `AtriusFhirPathEngine`.
///
/// Today this is mostly the optional terminology provider, but the struct keeps
/// room for future knobs (FHIR version selection, evaluation options, etc.).
#[derive(Clone, Default)]
pub struct EngineConfig {
    pub terminology: Option<Arc<dyn TerminologyProvider>>,
}

/// Concrete FHIRPath engine used by Atrius.
///
/// Implements:
/// - `FhirPathEngine::eval_bool` for invariant evaluation
/// - `FhirPathEngine::validate_code_in_valueset` for optional terminology lookups
///
/// The engine is cheap to clone (it holds shared `Arc` references).
#[derive(Clone, Default)]
pub struct AtriusFhirPathEngine {
    config: EngineConfig,
}

impl AtriusFhirPathEngine {
    /// Create a new engine with default configuration (no terminology provider).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an engine with an explicit configuration.
    pub fn with_config(config: EngineConfig) -> Self {
        Self { config }
    }

    /// Attach a terminology provider.
    ///
    /// When present, binding validation can call out to the terminology server via
    /// `FhirPathEngine::validate_code_in_valueset`.
    pub fn with_terminology_provider(mut self, provider: Arc<dyn TerminologyProvider>) -> Self {
        self.config.terminology = Some(provider);
        self
    }

    /// Convenience wrapper used by callers that want to invoke terminology lookup
    /// directly on this concrete engine.
    ///
    /// Note: the binding validator should normally call the trait method
    /// (`FhirPathEngine::validate_code_in_valueset`) so it works with any engine.
    pub fn validate_code_in_valueset(
        &self,
        valueset_url: &str,
        system: &str,
        code: &str,
    ) -> Option<bool> {
        self.config
            .terminology
            .as_ref()
            .and_then(|p| p.validate_in_valueset(valueset_url, system, code))
    }

    /// Coerce an evaluated FHIRPath result into a *singleton* boolean.
    ///
    /// FHIR invariants must evaluate to a boolean. The evaluator may produce:
    /// - `Empty` or empty collection (treated as `false`)
    /// - `Boolean` (returned as-is)
    /// - singleton collections (recursively unwrapped)
    /// - multi-item collections or non-boolean values (semantic error)
    fn coerce_to_bool(
        &self,
        expr: &str,
        result: EvaluationResult,
    ) -> Result<bool, EvaluationError> {
        match result {
            EvaluationResult::Empty => Ok(false),

            EvaluationResult::Boolean(b, _) => Ok(b),

            EvaluationResult::Collection { items, .. } => match items.len() {
                0 => Ok(false),
                1 => self.coerce_to_bool(expr, items.into_iter().next().unwrap()),
                n => Err(EvaluationError::SemanticError(format!(
                    "FHIR invariant '{}' must evaluate to a singleton boolean, got {} items",
                    expr, n
                ))),
            },

            other => Err(EvaluationError::SemanticError(format!(
                "FHIR invariant '{}' must evaluate to boolean, got {}",
                expr,
                other.type_name()
            ))),
        }
    }
}

impl FhirPathEngine for AtriusFhirPathEngine {
    /// Parse + evaluate a FHIRPath expression and coerce it to a boolean.
    ///
    /// This is used by generated invariant checks.
    ///
    /// Errors:
    /// - `InvalidArgument` for parse errors
    /// - `SemanticError` for non-boolean / non-singleton results
    /// - evaluation errors from the evaluator
    fn eval_bool(
        &self,
        focus: &EvaluationResult,
        expr: &str,
    ) -> Result<bool, EvaluationError> {
        // 1. Parse
        let parsed = parser()
            .parse(expr)
            .into_result()
            .map_err(|errs| {
                let msg = errs
                    .into_iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<_>>()
                    .join("; ");
                EvaluationError::InvalidArgument(format!("Invalid FHIRPath: {expr}. {msg}"))
            })?;
        // 2. Build context
        let context = EvaluationContext::new_empty_with_default_version();
        let result = evaluate(&parsed, &context, Some(focus))?;

        // 3. Boolean coercion
        self.coerce_to_bool(expr, result)
    }
    /// Delegate ValueSet membership validation to the configured terminology provider.
    ///
    /// Returning `None` means “unknown / unavailable”. The caller (binding validator)
    /// chooses the fallback behavior.
    fn validate_code_in_valueset(
        &self,
        valueset_url: &str,
        system: &str,
        code: &str,
    ) -> Option<bool> {
        // If a terminology provider is configured, delegate to it.
        // If not configured (or fails), return None = unknown/unavailable.
        self.config.terminology
            .as_ref()
            .and_then(|p| p.validate_in_valueset(valueset_url, system, code))
    }
}

/// Blocking HTTP terminology provider.
///
/// This implementation calls a FHIR terminology server endpoint:
/// `GET {base}/ValueSet/$validate-code?url=<vs>&system=<system>&code=<code>`
///
/// Feature-gated behind `terminology-http` to keep the core engine free of HTTP
/// dependencies unless explicitly enabled.
///
/// Note: This uses `reqwest::blocking` for simplicity and runtime-agnostic behavior.
/// If you need async I/O, implement `TerminologyProvider` using an async client
/// and an internal runtime adapter.
#[cfg(feature = "terminology-http")]
pub struct HttpTerminologyProvider {
    pub base_url: String,
    pub client: reqwest::blocking::Client,
}

#[cfg(feature = "terminology-http")]
impl HttpTerminologyProvider {
    /// Create a new HTTP terminology provider using the given base URL.
    ///
    /// `base_url` should point to the FHIR server root (e.g. `http://localhost:8080/fhir`).
    /// Trailing slashes are tolerated.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Normalize the base URL by trimming a trailing `/`.
    ///
    /// This prevents double slashes when constructing endpoint URLs.
    fn normalize_base(&self) -> String {
        self.base_url.trim_end_matches('/').to_string()
    }
}

#[cfg(feature = "terminology-http")]
impl TerminologyProvider for HttpTerminologyProvider {
    /// Call `$validate-code` on the remote FHIR terminology server.
    ///
    /// Expected response shape (FHIR `Parameters`):
    /// - `parameter[name == "result"].valueBoolean` indicates membership.
    ///
    /// Returns `None` if:
    /// - the HTTP request fails,
    /// - a non-2xx response is returned,
    /// - or the body cannot be parsed / does not contain a boolean `result`.
    fn validate_in_valueset(&self, valueset_url: &str, system: &str, code: &str) -> Option<bool> {
        #[derive(serde::Deserialize)]
        struct Parameters {
            #[serde(default)]
            parameter: Vec<Parameter>,
        }

        #[derive(serde::Deserialize)]
        struct Parameter {
            name: String,
            #[serde(rename = "valueBoolean")]
            value_boolean: Option<bool>,
        }

        let url = format!("{}/ValueSet/$validate-code", self.normalize_base());
        #[cfg(debug_assertions)]
        println!("{:#?}", url);
        let resp = self
            .client
            .get(url)
            .query(&[
                ("url", valueset_url),
                ("system", system),
                ("code", code),
            ])
            .send()
            .ok()?;
        #[cfg(debug_assertions)]
        println!("{:#?}", resp);
        if !resp.status().is_success() {
            return None;
        }

        let params: Parameters = resp.json().ok()?;
        params
            .parameter
            .into_iter()
            .find(|p| p.name == "result")
            .and_then(|p| p.value_boolean)
    }
}