// crates/fhirpath-support/src/validation.rs
use crate::evaluation_result::{EvaluationResult};
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

/// Something that can evaluate a boolean FHIRPath expression over a focus node.
pub trait FhirPathEngine {
    fn eval_bool(&self, focus: &EvaluationResult, expr: &str) -> Result<bool, EvaluationError>;
}

/// Types that can validate themselves using generated invariants.
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