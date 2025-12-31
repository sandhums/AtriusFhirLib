//! # FHIRPath Support Types
//!
//! This crate provides the foundational types and traits that serve as a bridge between
//! the FHIRPath evaluator and the broader FHIR ecosystem. It defines the common data
//! structures and conversion interfaces that enable seamless integration across all
//! components of the FHIRPath implementation.
//!
//! ## Overview
//!
//! The fhirpath_support crate acts as the universal communication layer that allows:
//! - FHIRPath evaluator to work with unified result types
//! - FHIR data structures to convert into FHIRPath-compatible formats
//! - Code generation macros to produce FHIRPath-aware implementations
//! - Type conversion system to handle data transformation
//!
//! ## Core Types
//!
//! - [`EvaluationResult`] - Universal result type for FHIRPath expression evaluation
//! - [`EvaluationError`] - Comprehensive error handling for evaluation failures
//! - [`IntoEvaluationResult`] - Trait for converting types to evaluation results
//!
//! ## Usage Example
//!
//! ```rust
//! use helios_fhirpath_support::{EvaluationResult, IntoEvaluationResult};
//!
//! // Convert a string to an evaluation result
//! let text = "Hello, FHIR!".to_string();
//! let result = text.to_evaluation_result();
//! assert_eq!(result, EvaluationResult::String("Hello, FHIR!".to_string(), None));
//!
//! // Work with collections
//! let numbers = vec![1, 2, 3];
//! let collection = numbers.to_evaluation_result();
//! assert_eq!(collection.count(), 3);
//! ```

pub mod type_info;
pub mod evaluation_result;
pub mod evaluation_error;
pub mod traits;
pub mod validate;

pub use validate::*;