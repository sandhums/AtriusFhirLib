use serde::{Deserialize, Serialize};
use crate::base_types::Extension;
use crate::complex_datatypes::{CodeableConcept, Coding, Identifier};
use crate::element_definition::ElementDefinition;
use crate::meta_datatypes::{ContactDetail, Meta, Narrative, Resource, UsageContext};

/// Bootstrap representation of a FHIR StructureDefinition.
///
/// A StructureDefinition describes the structure, content, and constraints of a FHIR
/// data type or resource. This bootstrap version includes all fields necessary to
/// parse the official FHIR specification files and extract type information for
/// code generation.
///
/// ## Key Fields
///
/// - `name`: The name of the type being defined (e.g., "Patient", "string", "Observation")
/// - `kind`: The kind of definition ("resource", "complex-type", or "primitive-type")
/// - `abstract`: Whether this is an abstract base type (not directly instantiable)
/// - `snapshot`: Contains the complete element definitions for this type
/// - `differential`: Contains only the differences from the base definition
///
/// ## Usage in Code Generation
///
/// The code generator uses StructureDefinitions to:
/// 1. Identify which types to generate Rust code for
/// 2. Extract field definitions and their types
/// 3. Understand inheritance relationships
/// 4. Apply constraints and cardinality rules
#[derive(Debug, Serialize, Deserialize, Default)] // Added Default
pub struct StructureDefinition {
    pub id: Option<String>,
    pub meta: Option<Meta>,
    #[serde(rename = "implicitRules")]
    pub implicit_rules: Option<String>,
    pub language: Option<String>,
    pub text: Option<Narrative>,
    pub contained: Option<Vec<Resource>>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub url: String,
    pub identifier: Option<Vec<Identifier>>,
    pub version: Option<String>,
    #[serde(rename = "versionAlgorithmString")]
    pub version_algorithm_string: Option<String>,
    #[serde(rename = "versionAlgorithmCoding")]
    pub version_algorithm_coding: Option<Coding>,
    pub name: String,
    pub title: Option<String>,
    pub status: String,
    pub experimental: Option<bool>,
    pub date: Option<String>,
    pub publisher: Option<String>,
    pub contact: Option<Vec<ContactDetail>>,
    pub description: Option<String>,
    #[serde(rename = "useContext")]
    pub use_context: Option<Vec<UsageContext>>,
    pub jurisdiction: Option<Vec<CodeableConcept>>,
    pub purpose: Option<String>,
    pub copyright: Option<String>,
    #[serde(rename = "copyrightLabel")]
    pub copyright_label: Option<String>,
    pub keyword: Option<Vec<Coding>>,
    #[serde(rename = "fhirVersion")]
    pub fhir_version: Option<String>,
    pub mapping: Option<Vec<StructureDefinitionMapping>>,
    pub kind: String,
    #[serde(rename = "abstract")]
    pub r#abstract: bool,
    pub context: Option<Vec<StructureDefinitionContext>>,
    #[serde(rename = "contextInvariant")]
    pub context_invariant: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "baseDefinition")]
    pub base_definition: Option<String>,
    pub derivation: Option<String>,
    pub snapshot: Option<StructureDefinitionSnapshotOrDifferential>,
    pub differential: Option<StructureDefinitionSnapshotOrDifferential>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StructureDefinitionMapping {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub identity: String,
    pub uri: Option<String>,
    pub name: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StructureDefinitionContext {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub expression: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StructureDefinitionSnapshot {
    // Added this struct definition
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub element: Option<Vec<ElementDefinition>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StructureDefinitionSnapshotOrDifferential {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub element: Option<Vec<ElementDefinition>>,
}
