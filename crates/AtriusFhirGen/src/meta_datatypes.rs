use serde::{Deserialize, Serialize};
use crate::base_types::Extension;
use crate::bundle::Bundle;
use crate::capability_statement::CapabilityStatement;
use crate::complex_datatypes::{Attachment, CodeableConcept, Coding, ContactPoint, Quantity, Range, Reference, Timing};
use crate::others::{CompartmentDefinition, OperationDefinition, SearchParameter};
use crate::structure_definition::StructureDefinition;

#[derive(Debug, Serialize, Deserialize)]
pub struct ContactDetail {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub name: Option<String>,
    pub telecom: Option<Vec<ContactPoint>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataRequirement {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    // TODO - more
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Expression {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub language: Option<String>,
    pub expression: Option<String>,
    pub reference: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ParameterDefinition {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub name: Option<String>,
    #[serde(rename = "use")]
    pub r#use: String,
    pub min: Option<i32>,
    pub max: Option<String>,
    pub documentation: Option<String>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub profile: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RelatedArtifact {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub classifier: Option<Vec<CodeableConcept>>,
    pub label: Option<String>,
    pub display: Option<String>,
    pub citation: Option<String>,
    pub document: Option<Attachment>,
    pub resource: Option<String>,
    #[serde(rename = "resourceReference")]
    pub resource_reference: Option<Reference>,
    #[serde(rename = "publicationStatus")]
    pub publication_status: Option<String>,
    #[serde(rename = "publicationDate")]
    pub publication_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TriggerDefinitionTiming {
    Timing(Timing),
    Reference(Reference),
    Date(String),
    String(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TriggerDefinition {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub name: Option<String>,
    pub code: Option<CodeableConcept>,
    #[serde(rename = "subscriptionTopic")]
    pub subscription_topic: Option<String>,
    pub timing: Option<TriggerDefinitionTiming>,
    pub data: Option<Vec<DataRequirement>>,
    pub condition: Option<Expression>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageContext {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub coding: Coding,
    pub value: UsageContextValue,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UsageContextValue {
    CodeableConcept(CodeableConcept),
    Quantity(Quantity),
    Range(Range),
    Reference(Reference),
}
// TODO Availability
// TODO ExtendedContactDetail
#[derive(Debug, Serialize, Deserialize)]
pub struct Dosage {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    // TODO - more
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Meta {
    #[serde(rename = "versionString")]
    pub version_id: Option<String>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<String>,
    pub source: Option<String>,
    pub profile: Option<Vec<String>>,
    pub security: Option<Vec<Coding>>,
    pub tag: Option<Vec<Coding>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Narrative {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub status: String,
    pub div: String,
}

/// Bootstrap representation of a FHIR Resource.
///
/// This enum represents the different types of FHIR resources that can appear
/// in specification Bundle files. It uses serde's tag-based deserialization
/// to automatically route JSON objects to the correct variant based on their
/// "resourceType" field.
///
/// ## Supported Resource Types
///
/// Only includes the resource types needed for parsing FHIR specification files:
/// - `StructureDefinition`: Core type definitions
/// - `SearchParameter`: Search parameter definitions
/// - `OperationDefinition`: FHIR operation definitions
/// - `CapabilityStatement`: Server capability declarations
/// - `CompartmentDefinition`: Compartment definitions
/// - `Bundle`: Nested bundle resources
///
/// This is a minimal set focused on code generation needs, not a complete
/// list of all FHIR resource types.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "resourceType")]
pub enum Resource {
    StructureDefinition(StructureDefinition),
    CapabilityStatement(CapabilityStatement),
    CompartmentDefinition(CompartmentDefinition),
    Bundle(Bundle),
    OperationDefinition(OperationDefinition),
    SearchParameter(SearchParameter),
}