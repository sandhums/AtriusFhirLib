//! Terminology client for FHIR terminology server operations.
//!
//! This module provides a small HTTP client wrapper around standard FHIR terminology
//! operations.
//!
//! ## What this client is used for in Atrius
//!
//! Atrius performs validation in two tiers:
//! 1. **Local** validation (fast, zero I/O) for bindings that are fully enumerated
//!    in generated enums / ValueSet wrappers.
//! 2. **Remote terminology** validation (network I/O) as a fallback when a ValueSet
//!    contains rules or referenced CodeSystems that are not locally enumerated.
//!
//! In particular, FHIR element bindings are expressed in terms of **ValueSets**, so
//! binding validation calls **`/ValueSet/$validate-code`** (not `CodeSystem/$validate-code`).
//!
//! ## Sync vs async
//!
//! Most operations here are `async` and use `reqwest`.
//! However, the `FhirValidate` derive macro expands to synchronous Rust code and the
//! `FhirPathEngine` trait is intentionally runtime-agnostic. For that reason this
//! module also exposes `validate_vs_sync()` which:
//! - requires a Tokio runtime to be present
//! - uses `tokio::task::block_in_place` + `Handle::block_on` to avoid blocking reactor threads
//! - degrades to `None` if a runtime is not available
//!
//! Higher-level validation code decides how to treat `None` (unknown) results.

use reqwest::Client;
use serde_json::{Value, json};
use std::collections::HashMap;

use crate::error::{FhirPathError, FhirPathResult};
use atrius_fhir_lib::fhir_version::FhirVersion;
use crate::engine::TerminologyProvider;

/// HTTP client for interacting with a FHIR terminology server.
///
/// This client is intentionally lightweight:
/// - uses `reqwest::Client` for connection pooling
/// - keeps only a normalized `base_url` (no trailing slash)
/// - stores the configured `FhirVersion` for future request shaping (currently unused)
///
/// In Atrius, this client is typically wrapped behind a `TerminologyProvider` and injected
/// into the evaluation/validation engine.
#[derive(Clone)]
pub struct TerminologyClient {
    client: Client,
    base_url: String,
    #[allow(dead_code)]
    fhir_version: FhirVersion,
}

impl TerminologyClient {
    /// Extract the boolean `result` from a FHIR `Parameters` response.
    ///
    /// FHIR `$validate-code` returns a `Parameters` resource and commonly includes:
    ///
    /// ```json
    /// {"name":"result","valueBoolean":true}
    /// ```
    ///
    /// This helper returns:
    /// - `Some(true)` / `Some(false)` if the parameter is present
    /// - `None` if the response is not shaped as expected
    fn extract_validate_result_bool(json: &Value) -> Option<bool> {
        // FHIR Parameters:
        // { "resourceType": "Parameters", "parameter": [ {"name":"result","valueBoolean":true}, ... ] }
        let params = json.get("parameter")?.as_array()?;
        for p in params {
            if p.get("name")?.as_str()? == "result" {
                return p.get("valueBoolean").and_then(|v| v.as_bool());
            }
        }
        None
    }
    /// Synchronous wrapper for ValueSet membership validation.
    ///
    /// ### Why this exists
    ///
    /// Atrius generates validation code (via `#[derive(FhirValidate)]`) that is **synchronous**.
    /// We still want that generated code to be able to consult a terminology server when a
    /// ValueSet is not locally enumerable.
    ///
    /// Since this client is `async`, we provide a sync wrapper that can be called from
    /// synchronous validation paths.
    ///
    /// ### Runtime requirements
    ///
    /// This function requires a Tokio runtime to be present.
    /// - If no runtime is available, it returns `None` (unknown).
    /// - If a runtime is available, it runs the async request using `Handle::block_on`.
    ///
    /// To avoid blocking Tokio worker threads, the call is wrapped in `block_in_place`.
    ///
    /// ### Return semantics
    ///
    /// Returns:
    /// - `Some(true)`  => confirmed member of the ValueSet
    /// - `Some(false)` => confirmed NOT a member of the ValueSet
    /// - `None`        => unknown (no runtime, network failure, parse failure, etc.)
    pub fn validate_vs_sync(
        &self,
        value_set_url: &str,
        system: &str,
        code: &str,
    ) -> Option<bool> {
        // If we're not inside a Tokio runtime, we cannot safely drive the async client.
        // In that case, degrade to `None`.
        let handle = tokio::runtime::Handle::try_current().ok()?;

        // Avoid blocking Tokio worker threads.
        tokio::task::block_in_place(|| {
            handle
                .block_on(async {
                    match self
                        .validate_vs(value_set_url, Some(system), code, None, None)
                        .await
                    {
                        Ok(v) => Self::extract_validate_result_bool(&v),
                        Err(_) => None,
                    }
                })

        })
    }

    /// Create a new `TerminologyClient`.
    ///
    /// `base_url` is normalized by trimming any trailing `/`.
    ///
    /// Example base URLs:
    /// - `http://localhost:8080/fhir`
    /// - `https://tx.fhir.org/r4`
    pub fn new(base_url: String, fhir_version: FhirVersion) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            fhir_version,
        }
    }

    /// Creates a new terminology client with custom HTTP client
    ///
    /// # Arguments
    ///
    /// * `client` - Custom reqwest client (for authentication, timeouts, etc.)
    /// * `base_url` - The base URL of the terminology server
    /// * `fhir_version` - The FHIR version to use for requests
    #[allow(dead_code)]
    pub fn with_client(client: Client, base_url: String, fhir_version: FhirVersion) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            fhir_version,
        }
    }

    /// Expand a ValueSet using `GET /ValueSet/$expand`.
    ///
    /// This is typically used for authoring and tooling (e.g., generating pick-lists).
    /// For binding validation at runtime, prefer `$validate-code` via `validate_vs()`.
    pub async fn expand(
        &self,
        value_set_url: &str,
        params: Option<HashMap<String, String>>,
    ) -> FhirPathResult<Value> {
        let url = format!("{}/ValueSet/$expand", self.base_url);

        // Build query parameters
        let mut query_params = vec![("url".to_string(), value_set_url.to_string())];

        if let Some(params) = params {
            for (key, value) in params {
                query_params.push((key.clone(), value));
            }
        }

        let response = self
            .client
            .get(&url)
            .query(
                &query_params
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect::<Vec<_>>(),
            )
            .header("Accept", "application/fhir+json")
            .send()
            .await
            .map_err(|e| FhirPathError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| FhirPathError::ParseError(e.to_string()))
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(FhirPathError::TerminologyError(format!(
                "ValueSet expansion failed with status {}: {}",
                status, body
            )))
        }
    }

    /// Lookup details for a code using `POST /CodeSystem/$lookup`.
    ///
    /// This operation returns additional properties for a `system|code` such as display,
    /// designations, and other metadata depending on the terminology server.
    pub async fn lookup(
        &self,
        system: &str,
        code: &str,
        params: Option<HashMap<String, String>>,
    ) -> FhirPathResult<Value> {
        let url = format!("{}/CodeSystem/$lookup", self.base_url);

        let mut body = json!({
            "resourceType": "Parameters",
            "parameter": [
                {
                    "name": "system",
                    "valueUri": system
                },
                {
                    "name": "code",
                    "valueCode": code
                }
            ]
        });

        // Add additional parameters if provided
        if let Some(params) = params {
            if let Some(parameters) = body.get_mut("parameter").and_then(|p| p.as_array_mut()) {
                for (key, value) in params {
                    parameters.push(json!({
                        "name": key,
                        "valueString": value
                    }));
                }
            }
        }

        let response = self
            .client
            .post(&url)
            .json(&body)
            .header("Content-Type", "application/fhir+json")
            .header("Accept", "application/fhir+json")
            .send()
            .await
            .map_err(|e| FhirPathError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| FhirPathError::ParseError(e.to_string()))
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(FhirPathError::TerminologyError(format!(
                "Code lookup failed with status {}: {}",
                status, body
            )))
        }
    }

    /// Validate a `system|code` pair against a **ValueSet** using `POST /ValueSet/$validate-code`.
    ///
    /// ### Why ValueSet validation (and not CodeSystem validation)?
    ///
    /// FHIR element bindings are defined in terms of **ValueSets**.
    /// A ValueSet may include multiple CodeSystems, include subsets, apply filters,
    /// or contain include/exclude rules. Therefore binding enforcement must validate
    /// against the ValueSet, not merely check that the code exists in a CodeSystem.
    ///
    /// ### Parameters encoding
    ///
    /// This function sends a FHIR `Parameters` body. When `system` is provided,
    /// it uses a `coding` parameter:
    ///
    /// ```json
    /// {"name":"coding","valueCoding":{"system":"...","code":"..."}}
    /// ```
    ///
    /// If `system` is `None`, it falls back to sending `code` + `inferSystem=true`.
    /// Note: some terminology servers require an explicit system; Atrius binding
    /// validation typically provides `system`.
    ///
    /// ### Response
    ///
    /// Returns the raw JSON response (a FHIR `Parameters` resource). The boolean result
    /// is usually contained in `parameter[]` as `{ name: "result", valueBoolean: ... }`.
    pub async fn validate_vs(
        &self,
        value_set_url: &str,
        system: Option<&str>,
        code: &str,
        display: Option<&str>,
        params: Option<HashMap<String, String>>,
    ) -> FhirPathResult<Value> {
        let url = format!("{}/ValueSet/$validate-code", self.base_url);

        let mut parameters = vec![json!({
            "name": "url",
            "valueUri": value_set_url
        })];

        // If we have a system, use coding parameter, otherwise use code parameter
        if let Some(system) = system {
            let mut coding = json!({
                "system": system,
                "code": code
            });
            if let Some(display) = display {
                coding["display"] = json!(display);
            }
            parameters.push(json!({
                "name": "coding",
                "valueCoding": coding
            }));
        } else {
            // For codes without system, use the code parameter
            parameters.push(json!({
                "name": "code",
                "valueCode": code
            }));
            // Tell the server to infer the system from the ValueSet
            parameters.push(json!({
                "name": "inferSystem",
                "valueBoolean": true
            }));
            if let Some(display) = display {
                parameters.push(json!({
                    "name": "display",
                    "valueString": display
                }));
            }
        }

        // Add additional parameters if provided
        if let Some(params) = params {
            for (key, value) in params {
                parameters.push(json!({
                    "name": key,
                    "valueString": value
                }));
            }
        }

        let body = json!({
            "resourceType": "Parameters",
            "parameter": parameters
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .header("Content-Type", "application/fhir+json")
            .header("Accept", "application/fhir+json")
            .send()
            .await
            .map_err(|e| FhirPathError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            let result: Value = response
                .json()
                .await
                .map_err(|e| FhirPathError::ParseError(e.to_string()))?;

            Ok(result)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(FhirPathError::TerminologyError(format!(
                "ValueSet validation failed with status {}: {}",
                status, body
            )))
        }
    }

    /// Validate a code against a **CodeSystem** using `POST /CodeSystem/$validate-code`.
    ///
    /// This answers: “Is this code valid *in this CodeSystem*?”
    ///
    /// Note: this is **not sufficient** for FHIR element bindings, which are ValueSet-based.
    /// Binding enforcement should use `validate_vs()`.
    pub async fn validate_cs(
        &self,
        code_system_url: &str,
        code: &str,
        display: Option<&str>,
        params: Option<HashMap<String, String>>,
    ) -> FhirPathResult<Value> {
        let url = format!("{}/CodeSystem/$validate-code", self.base_url);

        let mut parameters = vec![
            json!({
                "name": "url",
                "valueUri": code_system_url
            }),
            json!({
                "name": "code",
                "valueCode": code
            }),
        ];

        if let Some(display) = display {
            parameters.push(json!({
                "name": "display",
                "valueString": display
            }));
        }

        // Add additional parameters if provided
        if let Some(params) = params {
            for (key, value) in params {
                parameters.push(json!({
                    "name": key,
                    "valueString": value
                }));
            }
        }

        let body = json!({
            "resourceType": "Parameters",
            "parameter": parameters
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .header("Content-Type", "application/fhir+json")
            .header("Accept", "application/fhir+json")
            .send()
            .await
            .map_err(|e| FhirPathError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| FhirPathError::ParseError(e.to_string()))
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(FhirPathError::TerminologyError(format!(
                "CodeSystem validation failed with status {}: {}",
                status, body
            )))
        }
    }

    /// Check subsumption using `POST /CodeSystem/$subsumes`.
    ///
    /// This answers: “Does `code_a` subsume `code_b` in this CodeSystem?”
    /// Commonly used with SNOMED CT hierarchies.
    pub async fn subsumes(
        &self,
        system: &str,
        code_a: &str,
        code_b: &str,
        params: Option<HashMap<String, String>>,
    ) -> FhirPathResult<Value> {
        let url = format!("{}/CodeSystem/$subsumes", self.base_url);

        let mut parameters = vec![
            json!({
                "name": "system",
                "valueUri": system
            }),
            json!({
                "name": "codeA",
                "valueCode": code_a
            }),
            json!({
                "name": "codeB",
                "valueCode": code_b
            }),
        ];

        // Add additional parameters if provided
        if let Some(params) = params {
            for (key, value) in params {
                parameters.push(json!({
                    "name": key,
                    "valueString": value
                }));
            }
        }

        let body = json!({
            "resourceType": "Parameters",
            "parameter": parameters
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .header("Content-Type", "application/fhir+json")
            .header("Accept", "application/fhir+json")
            .send()
            .await
            .map_err(|e| FhirPathError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            response
                .json()
                .await
                .map_err(|e| FhirPathError::ParseError(e.to_string()))
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(FhirPathError::TerminologyError(format!(
                "Subsumes check failed with status {}: {}",
                status, body
            )))
        }
    }

    /// Translate a code using `POST /ConceptMap/$translate`.
    ///
    /// This operation maps a source `system|code` to one or more target codes according
    /// to a `ConceptMap`.
    ///
    /// Note: the current implementation contains a small heuristic for inferring systems
    /// when `system` is empty. In Atrius production flows, callers should prefer providing
    /// an explicit system wherever possible.
    pub async fn translate(
        &self,
        concept_map_url: &str,
        system: &str,
        code: &str,
        target_system: Option<&str>,
        params: Option<HashMap<String, String>>,
    ) -> FhirPathResult<Value> {
        let url = format!("{}/ConceptMap/$translate", self.base_url);

        let mut parameters = vec![json!({
            "name": "url",
            "valueUri": concept_map_url
        })];

        // Create a coding parameter
        if !system.is_empty() {
            parameters.push(json!({
                "name": "coding",
                "valueCoding": {
                    "system": system,
                    "code": code
                }
            }));
        } else {
            // For codes without explicit system, we need to infer it from context
            // For FHIR ConceptMaps, certain codes have known systems
            let inferred_system = match code {
                "home" | "work" | "temp" | "old" | "billing" => "http://hl7.org/fhir/address-use",
                "male" | "female" | "other" | "unknown" => {
                    "http://hl7.org/fhir/administrative-gender"
                }
                _ => "",
            };

            if !inferred_system.is_empty() {
                parameters.push(json!({
                    "name": "coding",
                    "valueCoding": {
                        "system": inferred_system,
                        "code": code
                    }
                }));
            } else {
                // Fallback to just code
                parameters.push(json!({
                    "name": "code",
                    "valueCode": code
                }));
            }
        }

        if let Some(target) = target_system {
            parameters.push(json!({
                "name": "targetSystem",
                "valueUri": target
            }));
        }

        // Add additional parameters if provided
        if let Some(params) = params {
            for (key, value) in params {
                parameters.push(json!({
                    "name": key,
                    "valueString": value
                }));
            }
        }

        let body = json!({
            "resourceType": "Parameters",
            "parameter": parameters
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .header("Content-Type", "application/fhir+json")
            .header("Accept", "application/fhir+json")
            .send()
            .await
            .map_err(|e| FhirPathError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            let result: Value = response
                .json()
                .await
                .map_err(|e| FhirPathError::ParseError(e.to_string()))?;

            Ok(result)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(FhirPathError::TerminologyError(format!(
                "Translation failed with status {}: {}",
                status, body
            )))
        }
    }
}
/// Bridge `TerminologyClient` into the validation engine.
///
/// The engine calls `TerminologyProvider::validate_in_valueset()` when local ValueSet membership
/// checks cannot conclusively determine membership.
///
/// This implementation delegates to `validate_vs_sync()`, so it requires a Tokio runtime.
/// If no runtime is available, the result degrades to `None` (unknown).
impl TerminologyProvider for TerminologyClient {
    fn validate_in_valueset(&self, valueset_url: &str, system: &str, code: &str) -> Option<bool> {
        self.validate_vs_sync(valueset_url, system, code)
    }
}
#[cfg(test)]
mod tests {
    //! Unit tests for URL normalization and basic construction.
    use super::*;

    #[test]
    fn test_terminology_client_creation() {
        let client = TerminologyClient::new("https://tx.fhir.org/r4/".to_string(), FhirVersion::R4);
        assert_eq!(client.base_url, "https://tx.fhir.org/r4");
    }

    #[test]
    fn test_base_url_normalization() {
        let client = TerminologyClient::new("https://tx.fhir.org/r4/".to_string(), FhirVersion::R4);
        assert_eq!(client.base_url, "https://tx.fhir.org/r4");

        let client2 = TerminologyClient::new("https://tx.fhir.org/r4".to_string(), FhirVersion::R4);
        assert_eq!(client2.base_url, "https://tx.fhir.org/r4");
    }
}
