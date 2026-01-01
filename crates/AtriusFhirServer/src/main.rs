use atrius_fhir_lib::fhir_version::FhirResource;
use atrius_fhir_lib::r5::{HumanName, Identifier, Patient, Resource, Uri};
 use atrius_fhir_lib::r5::primitives::String as FhirString;
 use atrius_fhir_path::{evaluate_expression, EvaluationContext};
 use atrius_fhirpath_support::traits::IntoEvaluationResult;




fn main() {
    let id: FhirString = "patient-123".to_string().into();
    let uri: Uri = "http://hospital.smarthealthit.org".to_string().into();
    let value: FhirString = "123".to_string().into();
    let family: FhirString = "Doe".to_string().into();
    let given: FhirString = "John".to_string().into();
    let patient = Patient {
        id: Some(id.clone()),
        identifier: Some(vec![
            Identifier {
                system: Some(uri.clone()),
                value: Some(value.clone()),
                ..Default::default()
            }
        ]),
        name: Some(vec![
            HumanName {
                family: Some(family.clone()),
                given: Some(vec![given.clone()]),
                ..Default::default()
            }
        ]),
        ..Default::default()
    };
    let resource = Resource::Patient(patient.clone());
    let json = serde_json::to_string(&resource).unwrap_or(String::new());
    let parsed: Resource = serde_json::from_str(&json).unwrap();
    let result = patient.to_evaluation_result();
    let resources = vec![FhirResource::R5(Box::new(resource))];
    let context = EvaluationContext::new(resources);
    let name_result = evaluate_expression("name.given", &context);
    println!("{}", json);
    println!("{:#?}", parsed);
    println!("{:#?}", name_result);
    println!("{:#?}", result);
    println!("Hello, world!");
}
