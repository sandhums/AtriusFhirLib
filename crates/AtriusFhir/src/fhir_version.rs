use atrius_fhirpath_support::evaluation_result::EvaluationResult;
use atrius_fhirpath_support::traits::IntoEvaluationResult;
#[cfg(feature = "R4")]
use crate::r4;
#[cfg(feature = "R4B")]
use crate::r4b;
#[cfg(feature = "R5")]
use crate::r5;
#[cfg(feature = "R6")]
use crate::r6;

/// Multi-version FHIR resource container supporting version-agnostic operations.
///
/// This enum provides a unified interface for working with FHIR resources across
/// different specification versions. It enables applications to handle multiple
/// FHIR versions simultaneously while maintaining type safety and version-specific
/// behavior where needed.
///
/// # Supported Versions
///
/// - **R4**: FHIR 4.0.1 (normative)
/// - **R4B**: FHIR 4.3.0 (ballot)
/// - **R5**: FHIR 5.0.0 (ballot)
/// - **R6**: FHIR 6.0.0 (draft)
///
/// # Feature Flags
///
/// Each FHIR version is controlled by a corresponding Cargo feature flag.
/// Only enabled versions will be available in the enum variants.
///
/// # Examples
///
/// ```rust
/// use helios_fhir::{FhirResource, FhirVersion};
/// # #[cfg(feature = "R4")]
/// use helios_fhir::r4::{Patient, HumanName};
///
/// # #[cfg(feature = "R4")]
/// {
///     // Create an R4 patient
///     let patient = Patient {
///         name: Some(vec![HumanName {
///             family: Some("Doe".to_string().into()),
///             given: Some(vec!["John".to_string().into()]),
///             ..Default::default()
///         }]),
///         ..Default::default()
///     };
///
///     // Wrap in version-agnostic container
///     let resource = FhirResource::R4(Box::new(helios_fhir::r4::Resource::Patient(patient)));
///     assert_eq!(resource.version(), FhirVersion::R4);
/// }
/// ```
///
/// # Version Detection
///
/// Use the `version()` method to determine which FHIR version a resource uses:
///
/// ```rust
/// # use helios_fhir::{FhirResource, FhirVersion};
/// # #[cfg(feature = "R4")]
/// # {
/// # let resource = FhirResource::R4(Box::new(helios_fhir::r4::Resource::Patient(Default::default())));
/// match resource.version() {
///     #[cfg(feature = "R4")]
///     FhirVersion::R4 => println!("This is an R4 resource"),
///     #[cfg(feature = "R4B")]
///     FhirVersion::R4B => println!("This is an R4B resource"),
///     #[cfg(feature = "R5")]
///     FhirVersion::R5 => println!("This is an R5 resource"),
///     #[cfg(feature = "R6")]
///     FhirVersion::R6 => println!("This is an R6 resource"),
/// }
/// # }
/// ```
#[derive(Debug)]
pub enum FhirResource {
    /// FHIR 4.0.1 (normative) resource
    #[cfg(feature = "R4")]
    R4(Box<r4::Resource>),
    /// FHIR 4.3.0 (ballot) resource
    #[cfg(feature = "R4B")]
    R4B(Box<r4b::Resource>),
    /// FHIR 5.0.0 (ballot) resource
    #[cfg(feature = "R5")]
    R5(Box<r5::Resource>),
    /// FHIR 6.0.0 (draft) resource
    #[cfg(feature = "R6")]
    R6(Box<r6::Resource>),
}

impl FhirResource {
/// Returns the FHIR specification version of this resource.
///
/// This method provides version detection for multi-version applications,
/// enabling version-specific processing logic and compatibility checks.
///
/// # Returns
///
/// The `FhirVersion` enum variant corresponding to this resource's specification.
///
/// # Examples
///
/// ```rust
/// use helios_fhir::{FhirResource, FhirVersion};
///
/// # #[cfg(feature = "R5")]
/// # {
/// # let resource = FhirResource::R5(Box::new(helios_fhir::r5::Resource::Patient(Default::default())));
/// let version = resource.version();
/// assert_eq!(version, FhirVersion::R5);
///
/// // Use version for conditional logic
/// match version {
///     FhirVersion::R5 => {
///         println!("Processing R5 resource with latest features");
///     },
///     FhirVersion::R4 => {
///         println!("Processing R4 resource with normative features");
///     },
///     _ => {
///         println!("Processing other FHIR version");
///     }
/// }
/// # }
/// ```
pub fn version(&self) -> FhirVersion {
    match self {
        #[cfg(feature = "R4")]
        FhirResource::R4(_) => FhirVersion::R4,
        #[cfg(feature = "R4B")]
        FhirResource::R4B(_) => FhirVersion::R4B,
        #[cfg(feature = "R5")]
        FhirResource::R5(_) => FhirVersion::R5,
        #[cfg(feature = "R6")]
        FhirResource::R6(_) => FhirVersion::R6,
    }
}
}
///Implement the trait for the top-level enum
impl IntoEvaluationResult for FhirResource {
    fn to_evaluation_result(&self) -> EvaluationResult {
        match self {
            #[cfg(feature = "R4")]
            FhirResource::R4(r) => (*r).to_evaluation_result(), // Call impl on inner Box<r4::Resource>
            #[cfg(feature = "R4B")]
            FhirResource::R4B(r) => (*r).to_evaluation_result(), // Call impl on inner Box<r4b::Resource>
            #[cfg(feature = "R5")]
            FhirResource::R5(r) => (*r).to_evaluation_result(), // Call impl on inner Box<r5::Resource>
            #[cfg(feature = "R6")]
            FhirResource::R6(r) => (*r).to_evaluation_result(), // Call impl on inner Box<r6::Resource>
            // Note: If no features are enabled, this match might be empty or non-exhaustive.
            // This is generally okay as the enum itself wouldn't be usable.
        }
    }
}

/// Enumeration of supported FHIR specification versions.
///
/// This enum represents the different versions of the FHIR (Fast Healthcare
/// Interoperability Resources) specification that this library supports.
/// Each version represents a specific release of the FHIR standard with
/// its own set of features, resources, and compatibility requirements.
///
/// # Version Status
///
/// - **R4** (4.0.1): Normative version, widely adopted in production
/// - **R4B** (4.3.0): Ballot version with additional features
/// - **R5** (5.0.0): Ballot version with significant enhancements
/// - **R6** (6.0.0): Draft version under active development
///
/// # Feature Flags
///
/// Each version is controlled by a corresponding Cargo feature flag:
/// - `R4`: Enables FHIR R4 support
/// - `R4B`: Enables FHIR R4B support
/// - `R5`: Enables FHIR R5 support
/// - `R6`: Enables FHIR R6 support
///
/// # Examples
///
/// ```rust
/// use helios_fhir::FhirVersion;
///
/// // Version comparison
/// # #[cfg(all(feature = "R4", feature = "R5"))]
/// # {
/// assert_ne!(FhirVersion::R4, FhirVersion::R5);
/// # }
///
/// // String representation
/// # #[cfg(feature = "R4")]
/// # {
/// let version = FhirVersion::R4;
/// assert_eq!(version.as_str(), "R4");
/// assert_eq!(version.to_string(), "R4");
/// # }
/// ```
///
/// # CLI Integration
///
/// This enum implements `clap::ValueEnum` for command-line argument parsing:
///
/// ```rust,no_run
/// use clap::Parser;
/// use helios_fhir::FhirVersion;
///
/// #[derive(Parser)]
/// struct Args {
///     #[arg(value_enum)]
///     version: FhirVersion,
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FhirVersion {
    /// FHIR 4.0.1 (normative) - The current normative version
    #[cfg(feature = "R4")]
    R4,
    /// FHIR 4.3.0 (ballot) - Intermediate version with additional features
    #[cfg(feature = "R4B")]
    R4B,
    /// FHIR 5.0.0 (ballot) - Next major version with significant changes
    #[cfg(feature = "R5")]
    R5,
    /// FHIR 6.0.0 (draft) - Future version under development
    #[cfg(feature = "R6")]
    R6,
}

impl FhirVersion {
/// Returns the string representation of the FHIR version.
///
/// This method provides the standard version identifier as used in
/// FHIR documentation, URLs, and configuration files.
///
/// # Returns
///
/// A static string slice representing the version (e.g., "R4", "R5").
///
/// # Examples
///
/// ```rust
/// use helios_fhir::FhirVersion;
///
/// # #[cfg(feature = "R4")]
/// assert_eq!(FhirVersion::R4.as_str(), "R4");
/// # #[cfg(feature = "R5")]
/// assert_eq!(FhirVersion::R5.as_str(), "R5");
/// ```
///
/// # Usage
///
/// This method is commonly used for:
/// - Logging and debugging output
/// - Configuration file parsing
/// - API endpoint construction
/// - Version-specific resource loading
pub fn as_str(&self) -> &'static str {
    match self {
        #[cfg(feature = "R4")]
        FhirVersion::R4 => "R4",
        #[cfg(feature = "R4B")]
        FhirVersion::R4B => "R4B",
        #[cfg(feature = "R5")]
        FhirVersion::R5 => "R5",
        #[cfg(feature = "R6")]
        FhirVersion::R6 => "R6",
    }
}
}

/// Implements `Display` trait for user-friendly output formatting.
///
/// This enables `FhirVersion` to be used in string formatting operations
/// and provides consistent output across different contexts.
///
/// # Examples
///
/// ```rust
/// use helios_fhir::FhirVersion;
///
/// # #[cfg(feature = "R5")]
/// # {
/// let version = FhirVersion::R5;
/// println!("Using FHIR version: {}", version); // Prints: "Using FHIR version: R5"
///
/// let formatted = format!("fhir-{}.json", version);
/// assert_eq!(formatted, "fhir-R5.json");
/// # }
/// ```
impl std::fmt::Display for FhirVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Provides a default FHIR version when R4 feature is enabled.
///
/// R4 is chosen as the default because it is the current normative version
/// of the FHIR specification and is widely adopted in production systems.
///
/// # Examples
///
/// ```rust
/// use helios_fhir::FhirVersion;
///
/// # #[cfg(feature = "R4")]
/// # {
/// let default_version = FhirVersion::default();
/// assert_eq!(default_version, FhirVersion::R4);
/// # }
/// ```
#[cfg(feature = "R4")]
impl Default for FhirVersion {
    fn default() -> Self {
        FhirVersion::R4
    }
}
/// Implements `clap::ValueEnum` for command-line argument parsing.
    ///
    /// This implementation enables `FhirVersion` to be used directly as a command-line
    /// argument type with clap, providing automatic parsing, validation, and help text
    /// generation.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use clap::Parser;
    /// use helios_fhir::FhirVersion;
    ///
    /// #[derive(Parser)]
    /// struct Args {
    ///     /// FHIR specification version to use
    ///     #[arg(value_enum, default_value_t = FhirVersion::default())]
    ///     version: FhirVersion,
    /// }
    ///
    /// // Command line: my-app --version R5
    /// let args = Args::parse();
    /// println!("Using FHIR version: {}", args.version);
    /// ```
    ///
    /// # Generated Help Text
    ///
    /// When using this enum with clap, the help text will automatically include
    /// all available FHIR versions based on enabled feature flags.
    impl clap::ValueEnum for FhirVersion {
        fn value_variants<'a>() -> &'a [Self] {
            &[
                #[cfg(feature = "R4")]
                FhirVersion::R4,
                #[cfg(feature = "R4B")]
                FhirVersion::R4B,
                #[cfg(feature = "R5")]
                FhirVersion::R5,
                #[cfg(feature = "R6")]
                FhirVersion::R6,
            ]
        }

        fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
            Some(clap::builder::PossibleValue::new(self.as_str()))
        }
    }

/// Trait for providing FHIR resource type information
///
/// This trait allows querying which resource types are available in a specific
/// FHIR version without hardcoding resource type lists in multiple places.
pub trait FhirResourceTypeProvider {
    /// Returns a vector of all resource type names supported in this FHIR version
    fn get_resource_type_names() -> Vec<&'static str>;

    /// Checks if a given type name is a resource type in this FHIR version
    fn is_resource_type(type_name: &str) -> bool {
        Self::get_resource_type_names()
            .iter()
            .any(|&resource_type| resource_type.eq_ignore_ascii_case(type_name))
    }
}

/// Trait for providing FHIR complex type information
///
/// This trait allows querying which complex data types are available in a specific
/// FHIR version without hardcoding complex type lists in multiple places.
pub trait FhirComplexTypeProvider {
    /// Returns a vector of all complex type names supported in this FHIR version
    fn get_complex_type_names() -> Vec<&'static str>;

    /// Checks if a given type name is a complex type in this FHIR version
    fn is_complex_type(type_name: &str) -> bool {
        Self::get_complex_type_names()
            .iter()
            .any(|&complex_type| complex_type.eq_ignore_ascii_case(type_name))
    }
}