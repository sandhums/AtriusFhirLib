/// Comprehensive error type for FHIRPath evaluation failures.
///
/// This enum covers all categories of errors that can occur during FHIRPath expression
/// evaluation, from type mismatches to semantic violations. Each variant provides
/// specific context about the failure to aid in debugging and error reporting.
///
/// # Error Categories
///
/// - **Type Errors**: Mismatched types in operations or function calls
/// - **Argument Errors**: Invalid arguments passed to functions
/// - **Runtime Errors**: Errors during expression evaluation (division by zero, etc.)
/// - **Semantic Errors**: Violations of FHIRPath semantic rules
/// - **System Errors**: Internal errors and edge cases
///
/// # Error Handling
///
/// All variants implement `std::error::Error` and `Display` for standard Rust
/// error handling patterns. The error messages are designed to be user-friendly
/// and provide actionable information for debugging.
///
/// # Examples
///
/// ```rust
/// use helios_fhirpath_support::EvaluationError;
///
/// // Type error example
/// let error = EvaluationError::TypeError(
///     "Cannot add String and Integer".to_string()
/// );
///
/// // Display the error
/// println!("{}", error); // "Type Error: Cannot add String and Integer"
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationError {
    /// Type mismatch or incompatible type operation.
    ///
    /// Occurs when operations are attempted on incompatible types or when
    /// functions receive arguments of unexpected types.
    ///
    /// Example: "Expected Boolean, found Integer"
    TypeError(String),
    /// Invalid argument provided to a function.
    ///
    /// Occurs when function arguments don't meet the required constraints
    /// or format expectations.
    ///
    /// Example: "Invalid argument for function 'where'"
    InvalidArgument(String),
    /// Reference to an undefined variable.
    ///
    /// Occurs when expressions reference variables that haven't been defined
    /// in the current evaluation context.
    ///
    /// Example: "Variable '%undefinedVar' not found"
    UndefinedVariable(String),
    /// Invalid operation for the given operand types.
    ///
    /// Occurs when operators are used with incompatible operand types or
    /// when operations are not supported for the given types.
    ///
    /// Example: "Cannot add String and Integer"
    InvalidOperation(String),
    /// Incorrect number of arguments provided to a function.
    ///
    /// Occurs when functions are called with too many or too few arguments
    /// compared to their specification.
    ///
    /// Example: "Function 'substring' expects 1 or 2 arguments, got 3"
    InvalidArity(String),
    /// Invalid array or collection index.
    ///
    /// Occurs when collection indexing operations use invalid indices
    /// (negative numbers, non-integers, out of bounds).
    ///
    /// Example: "Index must be a non-negative integer"
    InvalidIndex(String),
    /// Attempted division by zero.
    ///
    /// Occurs during mathematical operations when the divisor is zero.
    /// This is a specific case of arithmetic error with clear semantics.
    DivisionByZero,
    /// Arithmetic operation resulted in overflow.
    ///
    /// Occurs when mathematical operations produce results that exceed
    /// the representable range of the target numeric type.
    ArithmeticOverflow,
    /// Invalid regular expression pattern.
    ///
    /// Occurs when regex-based functions receive malformed regex patterns
    /// that cannot be compiled.
    ///
    /// Example: "Invalid regex pattern: unclosed parenthesis"
    InvalidRegex(String),
    /// Invalid type specifier in type operations.
    ///
    /// Occurs when type checking operations (is, as, ofType) receive
    /// invalid or unrecognized type specifiers.
    ///
    /// Example: "Unknown type 'InvalidType'"
    InvalidTypeSpecifier(String),
    /// Collection cardinality error for singleton operations.
    ///
    /// Occurs when operations expecting a single value receive collections
    /// with zero or multiple items.
    ///
    /// Example: "Expected singleton, found collection with 3 items"
    SingletonEvaluationError(String),
    /// Semantic rule violation.
    ///
    /// Occurs when expressions violate FHIRPath semantic rules, such as
    /// accessing non-existent properties in strict mode or violating
    /// contextual constraints.
    ///
    /// Example: "Property 'invalidField' does not exist on type 'Patient'"
    SemanticError(String),
    /// Unsupported function called.
    ///
    /// Occurs when a FHIRPath function is recognized but not yet implemented
    /// in this evaluation engine.
    ///
    /// Example: "Function 'conformsTo' is not implemented"
    UnsupportedFunction(String),
    /// Generic error for cases not covered by specific variants.
    ///
    /// Used for internal errors, edge cases, or temporary error conditions
    /// that don't fit into the specific error categories.
    ///
    /// Example: "Internal evaluation error"
    Other(String),
}

// === Standard Error Trait Implementations ===

/// Implements the standard `Error` trait for `EvaluationError`.
///
/// This allows `EvaluationError` to be used with Rust's standard error handling
/// mechanisms, including `?` operator, `Result` combinators, and error chains.
impl std::error::Error for EvaluationError {}

/// Implements the `Display` trait for user-friendly error messages.
///
/// Provides formatted, human-readable error messages that include error category
/// prefixes for easy identification of error types.
impl std::fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvaluationError::TypeError(msg) => write!(f, "Type Error: {}", msg),
            EvaluationError::InvalidArgument(msg) => write!(f, "Invalid Argument: {}", msg),
            EvaluationError::UndefinedVariable(name) => write!(f, "Undefined Variable: {}", name),
            EvaluationError::InvalidOperation(msg) => write!(f, "Invalid Operation: {}", msg),
            EvaluationError::InvalidArity(msg) => write!(f, "Invalid Arity: {}", msg),
            EvaluationError::InvalidIndex(msg) => write!(f, "Invalid Index: {}", msg),
            EvaluationError::DivisionByZero => write!(f, "Division by zero"),
            EvaluationError::ArithmeticOverflow => write!(f, "Arithmetic overflow"),
            EvaluationError::InvalidRegex(msg) => write!(f, "Invalid Regex: {}", msg),
            EvaluationError::InvalidTypeSpecifier(msg) => {
                write!(f, "Invalid Type Specifier: {}", msg)
            }
            EvaluationError::SingletonEvaluationError(msg) => {
                write!(f, "Singleton Evaluation Error: {}", msg)
            }
            EvaluationError::SemanticError(msg) => write!(f, "Semantic Error: {}", msg),
            EvaluationError::UnsupportedFunction(msg) => write!(f, "Unsupported Function: {}", msg),
            EvaluationError::Other(msg) => write!(f, "Evaluation Error: {}", msg),
        }
    }
}
