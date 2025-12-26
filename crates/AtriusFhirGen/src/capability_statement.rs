use serde::{Deserialize, Serialize};
use crate::base_types::Extension;
use crate::complex_datatypes::{CodeableConcept, Coding, Identifier, Reference};
use crate::meta_datatypes::{ContactDetail, Meta, Narrative, Resource, UsageContext};

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilityStatement {
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
    pub url: Option<String>,
    pub identifier: Option<Vec<Identifier>>,
    pub version: Option<String>,
    #[serde(rename = "versionAlgorithmString")]
    pub version_algorithm_string: Option<String>,
    #[serde(rename = "versionAlgorithmCoding")]
    pub version_algorithm_coding: Option<Coding>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub status: String,
    pub experimental: Option<bool>,
    pub date: String,
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
    pub kind: String,
    pub instantiates: Option<Vec<String>>,
    pub imports: Option<Vec<String>>,
    pub software: Option<CapabilityStatementSoftware>,
    pub implementation: Option<CapabilityStatementImplementation>,
    #[serde(rename = "fhirVersion")]
    pub fhir_version: String,
    pub format: Vec<String>,
    #[serde(rename = "patchFormat")]
    pub patch_format: Option<Vec<String>>,
    #[serde(rename = "acceptLanguage")]
    pub accept_language: Option<Vec<String>>,
    #[serde(rename = "implementationGuide")]
    pub implementation_guide: Option<Vec<String>>,
    pub rest: Option<Vec<CapabilityStatementRest>>,
    pub messaging: Option<Vec<CapabilityStatementMessaging>>,
    pub document: Option<Vec<CapabilityStatementDocument>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilityStatementSoftware {
    pub name: String,
    pub version: Option<String>,
    #[serde(rename = "releaseDate")]
    pub release_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilityStatementImplementation {
    pub description: String,
    pub url: Option<String>,
    pub custodian: Option<Reference>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilityStatementRest {
    pub mode: String,
    pub documentation: Option<String>,
    pub security: Option<CapabilityStatementSecurity>,
    pub resource: Option<Vec<CapabilityStatementResource>>,
    pub interaction: Option<Vec<CapabilityStatementInteraction>>,
    #[serde(rename = "searchParam")]
    pub search_param: Option<Vec<CapabilityStatementSearchParam>>,
    pub operation: Option<Vec<CapabilityStatementOperation>>,
    pub compartment: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilityStatementSecurity {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub cors: Option<bool>,
    pub service: Option<Vec<CodeableConcept>>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilityStatementResource {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub profile: Option<String>,
    #[serde(rename = "supportedProfile")]
    pub supported_profile: Option<Vec<String>>,
    pub documentation: Option<String>,
    pub interaction: Option<Vec<CapabilityStatementInteraction>>,
    pub versioning: Option<String>,
    #[serde(rename = "readHistory")]
    pub read_history: Option<bool>,
    #[serde(rename = "updateCreate")]
    pub update_create: Option<bool>,
    #[serde(rename = "conditionalCreate")]
    pub conditional_create: Option<bool>,
    #[serde(rename = "conditionalRead")]
    pub conditional_read: Option<String>,
    #[serde(rename = "conditionalUpdate")]
    pub conditional_update: Option<bool>,
    #[serde(rename = "conditionalPatch")]
    pub conditional_patch: Option<bool>,
    #[serde(rename = "conditionalDelete")]
    pub conditional_delete: Option<String>,
    #[serde(rename = "referencePolicy")]
    pub reference_policy: Option<Vec<String>>,
    #[serde(rename = "searchInclude")]
    pub search_include: Option<Vec<String>>,
    #[serde(rename = "searchRevInclude")]
    pub search_rev_include: Option<Vec<String>>,
    #[serde(rename = "searchParam")]
    pub search_param: Option<Vec<CapabilityStatementSearchParam>>,
    pub operation: Option<Vec<CapabilityStatementOperation>>,
    pub compartment: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilityStatementInteraction {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub code: String,
    pub documentation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilityStatementSearchParam {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub name: String,
    pub definition: Option<String>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub documentation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilityStatementOperation {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub name: String,
    pub definition: String,
    pub documentation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilityStatementMessaging {
    pub endpoint: Option<Vec<CapabilityStatementEndpoint>>,
    #[serde(rename = "reliableCache")]
    pub reliable_cache: Option<u32>,
    pub documentation: Option<String>,
    #[serde(rename = "supportedMessage")]
    pub supported_message: Option<Vec<CapabilityStatementSupportedMessage>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilityStatementEndpoint {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub protocol: Coding,
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilityStatementSupportedMessage {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub mode: String,
    pub definition: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapabilityStatementDocument {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub mode: String,
    pub documentation: Option<String>,
    pub profile: String,
}