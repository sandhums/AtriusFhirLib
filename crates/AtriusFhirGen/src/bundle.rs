use serde::{Deserialize, Serialize};
use crate::base_types::Extension;
use crate::complex_datatypes::{Identifier, Signature};
use crate::meta_datatypes::{Meta, Resource};

/// Bootstrap representation of a FHIR Bundle.
///
/// A Bundle is a container for a collection of FHIR resources. In the context
/// of code generation, Bundles contain the StructureDefinitions and other
/// resources from the FHIR specification files.
///
/// ## Purpose in Code Generation
///
/// This Bundle type is used to parse the official FHIR specification JSON files,
/// which are provided as Bundles containing:
/// - StructureDefinitions for all FHIR types
/// - SearchParameters for resource search capabilities
/// - OperationDefinitions for FHIR operations
/// - Other metadata resources
///
/// ## Bundle Structure
///
/// - `type`: The type of bundle (typically "collection" for spec files)
/// - `entry`: Array of BundleEntry items, each containing a resource
/// - `total`: Total number of entries in the bundle
///
/// The code generator extracts StructureDefinitions from bundle entries
/// to drive the Rust code generation process.
#[derive(Debug, Serialize, Deserialize)]
pub struct Bundle {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub id: Option<String>,
    pub meta: Option<Meta>,
    #[serde(rename = "implicitRules")]
    pub implicit_rules: Option<String>,
    pub language: Option<String>,
    pub identifier: Option<Identifier>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub timestamp: Option<String>,
    pub total: Option<u32>,
    pub link: Option<Vec<BundleLink>>,
    pub entry: Option<Vec<BundleEntry>>,
    pub signature: Option<Signature>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BundleLink {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub relation: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BundleEntry {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub link: Option<Vec<BundleLink>>,
    #[serde(rename = "fullUrl")]
    pub full_url: Option<String>,
    pub resource: Option<Resource>,
    pub search: Option<BundleEntrySearch>,
    pub request: Option<BundleEntryRequest>,
    pub response: Option<BundleEntryResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BundleEntrySearch {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BundleEntryRequest {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub method: String,
    pub url: String,
    #[serde(rename = "ifNoneMatch")]
    pub if_none_match: Option<String>,
    #[serde(rename = "ifModifiedSince")]
    pub if_modified_since: Option<String>,
    #[serde(rename = "ifMatch")]
    pub if_match: Option<String>,
    #[serde(rename = "ifNoneExist")]
    pub if_none_exist: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BundleEntryResponse {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "modifierExtension")]
    pub modifier_extension: Option<Vec<Extension>>,
    pub status: String,
    pub location: Option<String>,
    pub etag: Option<String>,
    #[serde(rename = "lastModified")]
    pub last_modified: Option<String>,
    pub outcome: Option<Resource>,
}
