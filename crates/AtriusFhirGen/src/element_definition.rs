use serde::{Deserialize, Serialize};
use crate::base_types::Extension;
use crate::complex_datatypes::{Address, Age, Annotation, Attachment, CodeableConcept, Coding, ContactPoint, Count, Distance, Duration, HumanName, Identifier, Money, Period, Quantity, Range, Ratio, Reference, SampledData, Signature, Timing};
use crate::meta_datatypes::{ContactDetail, DataRequirement, Dosage, Expression, Meta, ParameterDefinition, RelatedArtifact, TriggerDefinition, UsageContext};

/// Bootstrap representation of a FHIR ElementDefinition.
///
/// An ElementDefinition describes a single element (field) within a FHIR type,
/// including its data type, cardinality, constraints, and other metadata.
/// This is the core building block used by the code generator to create
/// Rust struct fields.
///
/// ## Key Fields
///
/// - `path`: The full path to this element (e.g., "Patient.name.given")
/// - `type`: The data type(s) this element can contain
/// - `min`/`max`: Cardinality constraints (0..1, 1..1, 0..*, etc.)
/// - `content_reference`: Reference to another element definition
///
/// ## Usage in Code Generation
///
/// The code generator uses ElementDefinitions to:
/// 1. Generate Rust struct fields with appropriate types
/// 2. Determine `Option<T>` vs `T` based on cardinality
/// 3. Create `Vec<T>` for arrays (max="*")
/// 4. Handle choice types (elements ending in "\[x\]")
/// 5. Resolve type references and detect cycles
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ElementDefinition {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub path: String,
    pub representation: Option<Vec<String>>,
    #[serde(rename = "sliceName")]
    pub slice_name: Option<String>,
    #[serde(rename = "sliceIsConstraining")]
    pub slice_is_constraining: Option<bool>,
    pub label: Option<String>,
    pub code: Option<Vec<Coding>>,
    pub slicing: Option<ElementDefinitionSlicing>,
    pub short: Option<String>,
    pub definition: Option<String>,
    pub comment: Option<String>,
    pub requirements: Option<String>,
    pub alias: Option<Vec<String>>,
    pub min: Option<u32>,
    pub max: Option<String>,
    pub base: Option<ElementDefinitionBase>,
    #[serde(rename = "contentReference")]
    pub content_reference: Option<String>,
    #[serde(rename = "type")]
    pub r#type: Option<Vec<ElementDefinitionType>>,
    #[serde(rename = "defaultValue")]
    pub default_value: Option<ElementDefinitionDefaultValue>,
    #[serde(rename = "meaningWhenMissing")]
    pub meaning_when_missing: Option<String>,
    #[serde(rename = "orderMeaning")]
    pub order_meaning: Option<String>,
    pub fixed: Option<ElementDefinitionDefaultValue>,
    pub pattern: Option<ElementDefinitionDefaultValue>,
    pub example: Option<Vec<ElementDefinitionExample>>,
    #[serde(rename = "minValue")]
    pub min_value: Option<ElementDefinitionMinMaxValue>,
    #[serde(rename = "maxValue")]
    pub max_value: Option<ElementDefinitionMinMaxValue>,
    #[serde(rename = "maxLength")]
    pub max_length: Option<i32>,
    pub condition: Option<Vec<String>>,
    pub constraint: Option<Vec<ElementDefinitionConstraint>>,
    #[serde(rename = "mustHaveValue")]
    pub must_have_value: Option<bool>,
    #[serde(rename = "valueAlternatives")]
    pub value_alternatives: Option<Vec<String>>,
    #[serde(rename = "mustSupport")]
    pub must_support: Option<bool>,
    #[serde(rename = "isModifier")]
    pub is_modifier: Option<bool>,
    #[serde(rename = "isModifierReason")]
    pub is_modifier_reason: Option<String>,
    #[serde(rename = "isSummary")]
    pub is_summary: Option<bool>,
    pub binding: Option<ElementDefinitionBinding>,
    pub mapping: Option<Vec<ElementDefinitionMapping>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElementDefinitionMapping {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub identity: String,
    pub language: Option<String>,
    pub map: String,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElementDefinitionBinding {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub strength: String,
    pub description: Option<String>,
    #[serde(rename = "valueSet")]
    pub value_set: Option<String>,
    pub additional: Option<Vec<ElementDefinitionBindingAdditional>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElementDefinitionBindingAdditional {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub purpose: String,
    #[serde(rename = "valueSet")]
    pub value_set: String,
    pub documentation: Option<String>,
    #[serde(rename = "shortDoco")]
    pub short_doco: Option<String>,
    pub usage: Option<Vec<UsageContext>>,
    pub any: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElementDefinitionExample {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub label: String,
    pub value: Option<ElementDefinitionDefaultValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElementDefinitionConstraint {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub key: String,
    pub requirements: Option<String>,
    pub severity: String,
    pub suppress: Option<bool>,
    pub human: String,
    pub expression: Option<String>,
    pub source: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub enum ElementDefinitionDefaultValue {
    Base64Binary(String),
    Boolean(bool),
    Canonical(String),
    Code(String),
    Date(String),
    DateTime(String),
    Decimal(String),
    Id(String),
    Instant(String),
    Integer(i32),
    Markdown(String),
    Oid(String),
    PositiveInt(u32),
    String(String),
    Time(String),
    UnsignedInt(u32),
    Uri(String),
    Url(String),
    Uuid(String),
    Address(Address),
    Age(Age),
    Annotation(Annotation),
    Attachment(Attachment),
    CodeableConcept(CodeableConcept),
    Coding(Coding),
    ContactPoint(ContactPoint),
    Count(Count),
    Distance(Distance),
    Duration(Duration),
    HumanName(HumanName),
    Identifier(Identifier),
    Money(Money),
    Period(Period),
    Quantity(Quantity),
    Range(Range),
    Ratio(Ratio),
    Reference(Reference),
    SampledData(SampledData),
    Signature(Signature),
    Timing(Timing),
    ContactDetail(ContactDetail),
    DataRequirement(DataRequirement),
    Expression(Expression),
    ParameterDefinition(ParameterDefinition),
    RelatedArtifact(RelatedArtifact),
    TriggerDefinition(TriggerDefinition),
    UsageContext(UsageContext),
    Dosage(Dosage),
    Meta(Meta),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ElementDefinitionMinMaxValue {
    Date(String),
    DateTime(String),
    Instant(String),
    Time(String),
    Decimal(String),
    Integer(i32),
    Integer64(i64),
    PositiveInt(u32),
    UnsignedInt(u32),
    Quantity(Quantity),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElementDefinitionSlicingDescriminator {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElementDefinitionSlicing {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub descriminator: Option<Vec<ElementDefinitionSlicingDescriminator>>,
    pub description: Option<String>,
    pub ordered: Option<bool>,
    pub rules: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElementDefinitionBase {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub path: String,
    pub min: u32,
    pub max: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElementDefinitionType {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub code: String,
    pub profile: Option<Vec<String>>,
    #[serde(rename = "targetProfile")]
    pub target_profile: Option<Vec<String>>,
    pub aggregation: Option<Vec<String>>,
    pub versioning: Option<String>,
}

impl ElementDefinitionType {
    /// Creates a new ElementDefinitionType with just a code.
    ///
    /// This is a convenience constructor for creating simple type references
    /// during testing or when building ElementDefinitions programmatically.
    ///
    /// # Arguments
    ///
    /// * `code` - The FHIR type code (e.g., "string", "Patient", "Reference")
    ///
    /// # Returns
    ///
    /// Returns a new ElementDefinitionType with the specified code and all
    /// other fields set to None.
    pub fn new(code: String) -> ElementDefinitionType {
        ElementDefinitionType {
            id: None,
            extension: None,
            code,
            profile: None,
            target_profile: None,
            aggregation: None,
            versioning: None,
        }
    }
}

