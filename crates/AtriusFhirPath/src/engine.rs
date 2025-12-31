use crate::parser::parser;
use crate::evaluator::{evaluate, EvaluationContext};
use chumsky::Parser;
use atrius_fhirpath_support::evaluation_error::EvaluationError;
use atrius_fhirpath_support::evaluation_result::EvaluationResult;
use atrius_fhirpath_support::FhirPathEngine;

#[derive(Default)]
pub struct AtriusFhirPathEngine;

impl AtriusFhirPathEngine {
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
}