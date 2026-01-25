// crates/fhirpath-support/src/validation.rs
use crate::evaluation_result::EvaluationResult;
use crate::evaluation_error::EvaluationError;
use crate::traits::IntoEvaluationResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub key: &'static str,
    pub severity: ValidationSeverity,
    pub path: &'static str,        // declared FHIR path (e.g. "Parameters.parameter")
    pub instance_path: String,     // concrete instance path (e.g. "Parameters.parameter[0]")
    pub expression: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone)]
pub struct Invariant {
    pub key: &'static str,
    pub severity: ValidationSeverity,
    pub human: &'static str,
    pub expr: &'static str,
    pub path: &'static str,
}

/// Engine interface used by generated validation code.
///
/// This trait has two responsibilities:
/// 1) Evaluate FHIRPath boolean expressions (invariants)
/// 2) Optionally validate ValueSet membership via an external terminology service
///
/// The derive macro `#[derive(FhirValidate)]` generates validation code that:
/// - always checks invariants via [`eval_bool`]
/// - checks ValueSet bindings via generated membership functions first
/// - may fall back to [`validate_code_in_valueset`] when a ValueSet is not fully enumerable locally
///
/// Implementations can be pure/in-memory (no terminology) or can wire a remote server
/// (e.g., HAPI/Snowstorm) to answer `$validate-code`.
pub trait FhirPathEngine {
    fn eval_bool(&self, focus: &EvaluationResult, expr: &str) -> Result<bool, EvaluationError>;
    /// Validate `system|code` membership in a ValueSet using an external terminology service.
    ///
    /// This is used as a fallback when local (generated) membership checks are not enough.
    ///
    /// ## Parameters
    /// - `valueset_url`: canonical URL of the ValueSet (e.g. `http://hl7.org/fhir/ValueSet/marital-status`)
    /// - `system`: code system canonical URL (e.g. `http://terminology.hl7.org/CodeSystem/v3-MaritalStatus`)
    /// - `code`: the code value (e.g. `M`)
    ///
    /// ## Return semantics (tri-state)
    /// - `Some(true)` => confirmed member of the ValueSet
    /// - `Some(false)` => confirmed NOT a member
    /// - `None` => unknown (no terminology configured, network error, server error, etc.)
    ///
    /// The 'derive' macro decides how to degrade on `None` (typically a Warning:
    /// "could not be verified (terminology unavailable)").
    fn validate_code_in_valueset(&self, valueset_url: &str, system: &str, code: &str) -> Option<bool> {
        let _ = (valueset_url, system, code);
        None
    }
}

/// Types that can validate themselves using generated invariants (and, via macro expansion, bindings).
///
/// Notes:
/// - The default implementation of [`validate_with_engine`] only checks *type-level* invariants.
/// - Field-level invariants and ValueSet bindings are injected by the derive macro
///   `#[derive(FhirValidate)]` in `atrius-macros`.
///
/// In other words: if you are looking for binding logic, it is in the macro output,
/// not in this default method body.
pub trait FhirValidate: IntoEvaluationResult {
    fn invariants() -> &'static [Invariant];

    fn validate_with_engine(&self, engine: &dyn FhirPathEngine) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let focus = self.to_evaluation_result();

        for inv in Self::invariants() {
            match engine.eval_bool(&focus, inv.expr) {
                Ok(true) => {}
                Ok(false) => issues.push(ValidationIssue {
                    key: inv.key,
                    severity: inv.severity,
                    path: inv.path,
                    instance_path: inv.path.to_string(),
                    expression: inv.expr,
                    message: inv.human,
                }),
                Err(_) => issues.push(ValidationIssue {
                    key: inv.key,
                    severity: inv.severity,
                    path: inv.path,
                    instance_path: inv.path.to_string(),
                    expression: inv.expr,
                    message: inv.human, // you can add diagnostics later
                }),
            }
        }

        issues
    }
}
/// Convenience impl so macro-generated validation can recurse into borrowed values (`&T`).
///
/// The 'derive' macro traverses fields using `.as_ref()`, `.iter()`, etc., which naturally yields
/// references (`&T`). Without this impl, recursive calls like:
/// `FhirValidate::validate_with_engine(value, engine)`
/// would fail when `value` is a reference.
///
/// This impl simply delegates to the underlying `T`.
impl<T> FhirValidate for &T
where
    T: FhirValidate + ?Sized,
{
    fn invariants() -> &'static [Invariant] {
        T::invariants()
    }

    fn validate_with_engine(&self, engine: &dyn FhirPathEngine) -> Vec<ValidationIssue> {
        (*self).validate_with_engine(engine)
    }
}