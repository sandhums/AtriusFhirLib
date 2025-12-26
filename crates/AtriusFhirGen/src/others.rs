use serde::{Deserialize, Serialize};
use crate::base_types::Extension;
use crate::complex_datatypes::{CodeableConcept, Coding, Identifier};
use crate::meta_datatypes::{ContactDetail, Meta, Narrative, Resource, UsageContext};

#[derive(Debug, Serialize, Deserialize)]
pub struct CompartmentDefinition {
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
    pub purpose: Option<String>,
    pub code: String,
    pub search: bool,
    pub resource: Option<Vec<CompartmentDefinitionResource>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompartmentDefinitionResource {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub code: String,
    pub param: Option<Vec<String>>,
    pub documentation: Option<String>,
    #[serde(rename = "startParam")]
    pub start_param: Option<String>,
    #[serde(rename = "endParam")]
    pub end_param: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationDefinition {
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
    pub name: String,
    pub title: Option<String>,
    pub status: String,
    pub kind: String,
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
    #[serde(rename = "affectsState")]
    pub affects_state: Option<bool>,
    pub code: String,
    pub comment: Option<String>,
    pub base: Option<String>,
    pub resource: Option<Vec<String>>,
    pub system: bool,
    #[serde(rename = "type")]
    pub r#type: bool,
    pub instance: bool,
    #[serde(rename = "inputProfile")]
    pub input_profile: Option<String>,
    #[serde(rename = "outputProfile")]
    pub output_profile: Option<String>,
    pub parameter: Option<Vec<OperationDefinitionParameter>>,
    pub overload: Option<Vec<OperationDefinitionOverload>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationDefinitionParameter {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub name: String,
    #[serde(rename = "use")]
    pub r#use_: String,
    pub scope: Option<Vec<String>>,
    pub min: i32,
    pub max: String,
    pub documentation: Option<String>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    #[serde(rename = "allowedType")]
    pub allowed_type: Option<Vec<String>>,
    #[serde(rename = "targetProfile")]
    pub target_profile: Option<Vec<String>>,
    #[serde(rename = "searchType")]
    pub search_type: Option<String>,
    pub binding: Option<OperationDefinitionParameterBinding>,
    #[serde(rename = "referencedFrom")]
    pub referenced_from: Option<Vec<OperationDefinitionParameterReferencedFrom>>,
    pub part: Option<Vec<OperationDefinitionParameter>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationDefinitionParameterBinding {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub strength: String,
    #[serde(rename = "valueSet")]
    pub value_set: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationDefinitionParameterReferencedFrom {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub source: String,
    #[serde(rename = "sourceId")]
    pub source_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationDefinitionOverload {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    #[serde(rename = "parameterName")]
    pub parameter_name: Option<Vec<String>>,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchParameter {
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
    #[serde(rename = "derivedFrom")]
    pub derived_from: Option<String>,
    pub status: String,
    pub experimental: Option<bool>,
    pub date: Option<String>,
    pub publisher: Option<String>,
    pub contact: Option<Vec<ContactDetail>>,
    pub description: String,
    #[serde(rename = "useContext")]
    pub use_context: Option<Vec<UsageContext>>,
    pub jurisdiction: Option<Vec<CodeableConcept>>,
    pub purpose: Option<String>,
    pub copyright: Option<String>,
    #[serde(rename = "copyrightLabel")]
    pub copyright_label: Option<String>,
    pub code: String,
    pub base: Vec<String>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub expression: Option<String>,
    #[serde(rename = "processingMode")]
    pub processing_mode: Option<String>,
    pub constraint: Option<String>,
    pub target: Option<Vec<String>>,
    #[serde(rename = "multipleOr")]
    pub multiple_or: Option<bool>,
    #[serde(rename = "multipleAnd")]
    pub multiple_and: Option<bool>,
    pub comparator: Option<Vec<String>>,
    pub modifier: Option<Vec<String>>,
    pub chain: Option<Vec<String>>,
    pub component: Option<Vec<SearchParameterComponent>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchParameterComponent {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub definition: String,
    pub expression: String,
}