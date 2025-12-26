use serde::{Deserialize, Serialize};
use crate::base_types::Extension;

#[derive(Debug, Serialize, Deserialize)]
pub struct Address {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "use")]
    pub r#use: Option<String>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub text: Option<String>,
    pub line: Option<Vec<String>>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "postalString")]
    pub postal_code: Option<String>,
    pub county: Option<String>,
    pub period: Option<Period>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Age {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub value: Option<String>,
    pub comparator: Option<String>,
    pub unit: Option<String>,
    pub system: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AnnotationAuthor {
    #[serde(rename = "authorReference")]
    AuthorReference(Reference),
    #[serde(rename = "authorString")]
    AuthorString(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Annotation {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub author: Option<AnnotationAuthor>,
    pub time: Option<String>,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub data: Option<String>,
    pub url: Option<String>,
    pub size: Option<i64>,
    pub hash: Option<String>,
    pub title: Option<String>,
    pub creation: Option<String>,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub frames: Option<u32>,
    pub duration: Option<String>,
    pub pages: Option<u32>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct CodeableConcept {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub coding: Option<Vec<Coding>>,
    pub text: Option<String>,
}
// TODO - CodeableReference
/// Bootstrap representation of a FHIR Coding.
///
/// A Coding is a representation of a defined concept using a symbol from
/// a defined "code system". This bootstrap version provides the essential
/// fields needed for parsing StructureDefinitions.
#[derive(Debug, Serialize, Deserialize)]
pub struct Coding {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub system: Option<String>,
    pub version: Option<String>,
    pub code: Option<String>,
    pub display: Option<String>,
    #[serde(rename = "userSelected")]
    pub user_selected: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContactPoint {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub system: Option<String>,
    pub value: Option<String>,
    #[serde(rename = "use")]
    pub r#use: Option<String>,
    pub rank: Option<u32>,
    pub period: Option<Period>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Count {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub value: Option<String>,
    pub comparator: Option<String>,
    pub unit: Option<String>,
    pub system: Option<String>,
    pub code: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Distance {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub value: Option<String>,
    pub comparator: Option<String>,
    pub unit: Option<String>,
    pub system: Option<String>,
    pub code: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Duration {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub value: Option<String>,
    pub comparator: Option<String>,
    pub unit: Option<String>,
    pub system: Option<String>,
    pub code: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct HumanName {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "use")]
    pub r#use: Option<String>,
    pub text: Option<String>,
    pub family: Option<String>,
    pub given: Option<Vec<String>>,
    pub prefix: Option<Vec<String>>,
    pub suffix: Option<Vec<String>>,
    pub period: Option<Period>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Identifier {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "use")]
    pub r#use: Option<String>,
    #[serde(rename = "type")]
    pub r#type: Option<CodeableConcept>,
    pub system: Option<String>,
    pub value: Option<String>,
    pub period: Option<Period>,
    pub assigner: Option<Box<Reference>>, // Use of Box here for recursive type
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Money {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub value: Option<String>,
    pub currency: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Period {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub start: Option<String>,
    pub end: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Quantity {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub value: Option<String>,
    pub comparator: Option<String>,
    pub unit: Option<String>,
    pub system: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleQuantity {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub value: Option<String>,
    pub unit: Option<String>,
    pub system: Option<String>,
    pub code: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Range {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub low: Option<SimpleQuantity>,
    pub high: Option<SimpleQuantity>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Ratio {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub numerator: Option<Quantity>,
    pub denominator: Option<SimpleQuantity>,
}
// TODO RatioRange
#[derive(Debug, Serialize, Deserialize)]
pub struct Reference {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub reference: Option<String>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub identifier: Option<Identifier>,
    pub display: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct SampledData {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub origin: SimpleQuantity,
    pub interval: Option<String>,
    #[serde(rename = "intervalUnit")]
    pub interval_unit: String,
    pub factor: Option<String>,
    #[serde(rename = "lowerLimit")]
    pub lower_limit: Option<String>,
    #[serde(rename = "upperLimit")]
    pub upper_limit: Option<String>,
    pub dimensions: u32,
    #[serde(rename = "codeMap")]
    pub code_map: Option<String>,
    pub offsets: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Signature {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    #[serde(rename = "type")]
    pub r#type: Option<Vec<Coding>>,
    pub when: Option<String>,
    pub who: Option<Reference>,
    #[serde(rename = "onBehalfOf")]
    pub on_behalf_of: Option<Reference>,
    #[serde(rename = "targetFormat")]
    pub target_format: Option<String>,
    #[serde(rename = "sigFormat")]
    pub sig_format: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TimingRepeatBounds {
    Duration(Duration),
    Range(Range),
    Period(Period),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimingRepeat {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub bounds: Option<TimingRepeatBounds>,
    pub count: Option<u32>,
    #[serde(rename = "countMax")]
    pub count_max: Option<u32>,
    pub duration: Option<String>,
    #[serde(rename = "durationMax")]
    pub duration_max: Option<u32>,
    #[serde(rename = "durationUnit")]
    pub duration_unit: Option<String>,
    pub frequency: Option<u32>,
    #[serde(rename = "frequencyMax")]
    pub frequency_max: Option<u32>,
    pub period: Option<String>,
    #[serde(rename = "periodMax")]
    pub period_max: Option<String>,
    #[serde(rename = "periodUnit")]
    pub period_unit: Option<String>,
    #[serde(rename = "dayOfWeek")]
    pub day_of_week: Option<Vec<String>>,
    #[serde(rename = "timeOfDay")]
    pub time_of_day: Option<Vec<String>>,
    pub when: Option<Vec<String>>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Timing {
    pub id: Option<String>,
    pub extension: Option<Vec<Extension>>,
    pub event: Option<Vec<String>>,
    pub repeat: Option<TimingRepeat>,
    pub code: Option<CodeableConcept>,
}