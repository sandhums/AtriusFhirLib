use std::sync::Arc;
use atrius_fhir_lib::fhir_version::FhirResource;
use atrius_fhir_lib::r5::primitives::String as FhirString;
use atrius_fhir_lib::r5::{Address, Code, CodeableConcept, Coding, ContactPoint, ExtendedContactDetail, Extension, ExtensionValue, HumanName, Id, Identifier, Meta, Narrative, Organization, Patient, PatientContact, Resource, Uri};
use atrius_fhir_path::engine::{AtriusFhirPathEngine, HttpTerminologyProvider};
use atrius_fhir_path::{evaluate_expression, EvaluationContext};
use atrius_fhirpath_support::traits::IntoEvaluationResult;
use atrius_fhirpath_support::validate::FhirValidate;


fn main() {
    let engine = AtriusFhirPathEngine::new()
        .with_terminology_provider(Arc::new(HttpTerminologyProvider::new("http://localhost:8080/fhir")));
    let patient = Patient {
        id: Some("patient-123".to_string().into()),
        implicit_rules: Some("http://hospital.somerule.org".to_string().into()),
        text: Some(Narrative {
            status: "generated".to_string().into(),
            div: "<div xmlns=\"http://www.w3.org/1999/xhtml\"></div>".to_string().into(),
            ..Default::default()
        }),
        meta: Some(Meta {
            version_id: Some(Id {
                value: Some("1".to_string().into()),
                ..Default::default()
            }),
            security: Some(vec![
                Coding {
                    system: Some("http://terminology.hl7.org/CodeSystem/v3-Confidentiality".to_string().into()),
                    code: Some("N".to_string().into()),
                    display: Some("Normal".to_string().into()),
                    ..Default::default()
                }]),
            ..Default::default()
        }),
        identifier: Some(vec![
            Identifier {
                system: Some("http://hospital.smarthealthit.org".to_string().into()),
                value: Some("123".to_string().into()),
                ..Default::default()
            },
        ]),
        name: Some(vec![
            HumanName {
                family: Some("Doe".to_string().into()),
                given: Some(vec!["John".to_string().into()]),
                ..Default::default()
            }
        ]),
        gender: Some("female".to_string().into()),
        marital_status: Some( CodeableConcept {
            id: None,
            extension: None,
            coding: Some(vec![Coding {
                id: None,
                extension: None,
                system: Some("http://terminology.hl7.org/CodeSystem/v3-MaritalStatus".to_string().into()),
                version: None,
                code: Some("M".to_string().into()),
                display: Some("married".to_string().into()),
                user_selected: None,
            }]),
            text: None,
        }) ,
        contact: Some(vec![ PatientContact {
            ..Default::default()
        }]),
        ..Default::default()
    };
    let iden = Identifier {
        id: None,
        extension: Some(vec![Extension {
            id: None,
            extension: None,
            url: Default::default(),
            value: Option::from(ExtensionValue::String("data-absent-reason".to_string().into())),
        }]),
        r#use: None,
        r#type: None,
        system: Some("http://hospital.smarthealthit.org".to_string().into()),
        value: Some("None".to_string().into()),
        period: None,
        assigner: None,
    };
    let org = Organization {
        contained: Some(vec![Resource::Patient(patient.clone())]),
        contact: Some(vec![ExtendedContactDetail {
            telecom: Some(vec![ContactPoint {
                value: Some("some value".to_string().into()),
                r#use: Some("home".to_string().into()),
                ..Default::default()
            }]),
            address: Some(Address {
                r#use: Some("home".to_string().into()),
                ..Default::default()
            }),
            ..Default::default()
        }]),
        ..Default::default()
    };
    let resource = Resource::Patient(patient.clone());
    let json = serde_json::to_string(&resource).unwrap_or(String::new());
    let parsed: Patient = serde_json::from_str(&json).unwrap();
    let result = patient.to_evaluation_result();
    let resources = vec![FhirResource::R5(Box::new(resource))];
    let context = EvaluationContext::new(resources);
    let name_result = evaluate_expression("name.given", &context);
    let issues = patient.validate_with_engine(&engine);
    let issues2 = iden.validate_with_engine(&engine);
    let org_issues = org.validate_with_engine(&engine);
    // println!("{}", json);
    // println!("{:#?}", parsed);
    // println!("{:#?}", name_result);
    // println!("{:#?}", result);
    // println!("{:#?}", org_issues);
    println!("{:#?}", issues);
    println!("Hello, world!");
}
