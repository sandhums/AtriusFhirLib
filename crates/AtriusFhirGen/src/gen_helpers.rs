use std::fs::File;
use std::io::{self, Write};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use serde_json::Result;
use crate::bundle::Bundle;
use crate::element_definition::ElementDefinition;
use crate::format_helpers::{capitalize_first_letter, escape_doc_comment};
use crate::gen_element_definitions::process_elements;
use crate::generate_resource_enum::{generate_resource_enum, generate_type_hierarchy_module};
use crate::generate_struct_element_doc::{generate_element_documentation, generate_struct_documentation};
use crate::meta_datatypes::Resource;
use crate::structure_definition::StructureDefinition;

/// Recursively visits directories to find relevant JSON specification files.
///
/// This function traverses the resource directory structure and collects all JSON files
/// that contain FHIR definitions, while filtering out files that aren't needed for
/// code generation (like concept maps and value sets).
///
/// # Arguments
///
/// * `dir` - Root directory to search for JSON files
///
/// # Returns
///
/// Returns a vector of `PathBuf`s pointing to relevant JSON specification files,
/// or an `io::Error` if directory traversal fails.
///
/// # Filtering Logic
///
/// Only includes JSON files that:
/// - Have a `.json` extension
/// - Do not contain "conceptmap" in the filename
/// - Do not contain "valueset" in the filename
///
/// This filtering focuses the code generation on structural definitions rather
/// than terminology content.
pub fn visit_dirs(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut json_files = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                json_files.extend(visit_dirs(&path)?);
            } else if let Some(ext) = path.extension() {
                if ext == "json" {
                    if let Some(filename) = path.file_name() {
                        let filename = filename.to_string_lossy();
                        if !filename.contains("conceptmap")
                            && !filename.contains("valueset")
                            && !filename.contains("bundle-entry")
                            && !filename.contains("download_metadata")
                        {
                            json_files.push(path);
                        }
                    }
                }
            }
        }
    }
    Ok(json_files)
}

/// Parses a JSON file containing FHIR StructureDefinitions into a Bundle.
///
/// This function reads a JSON file and deserializes it into a FHIR Bundle containing
/// StructureDefinitions and other FHIR resources used for code generation.
///
/// # Arguments
///
/// * `path` - Path to the JSON file to parse
///
/// # Returns
///
/// Returns a `Bundle` on success, or a `serde_json::Error` if parsing fails.
///
/// # File Format
///
/// Expects JSON files in the standard FHIR Bundle format with entries containing
/// StructureDefinition resources, as provided by the official FHIR specification.
pub fn parse_structure_definitions<P: AsRef<Path>>(path: P) -> Result<Bundle> {
    let file = File::open(path).map_err(serde_json::Error::io)?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader)
}

/// Determines if a StructureDefinition should be included in code generation.
///
/// This function filters StructureDefinitions to only include those that represent
/// concrete types that should have Rust code generated for them.
///
/// # Arguments
///
/// * `def` - The StructureDefinition to evaluate
///
/// # Returns
///
/// Returns `true` if the StructureDefinition should be processed for code generation.
///
/// # Criteria
///
/// A StructureDefinition is considered valid if:
/// - Kind is "complex-type", "primitive-type", or "resource"
/// - Derivation is "specialization" (concrete implementations)
/// - Abstract is `false` (not an abstract base type)
pub fn is_valid_structure_definition(def: &StructureDefinition) -> bool {
    (def.kind == "complex-type" || def.kind == "primitive-type" || def.kind == "resource")
        && def.derivation.as_deref() == Some("specialization")
        && !def.r#abstract
}

/// Checks if a StructureDefinition represents a FHIR primitive type.
///
/// Primitive types are handled differently in code generation, typically being
/// mapped to Rust primitive types or type aliases rather than full structs.
///
/// # Arguments
///
/// * `def` - The StructureDefinition to check
///
/// # Returns
///
/// Returns `true` if this is a primitive type definition.
pub fn is_primitive_type(def: &StructureDefinition) -> bool {
    def.kind == "primitive-type"
}

type BundleInfo = (
    std::collections::HashMap<String, String>,
    Vec<String>,
    Vec<String>,
);

/// Extracts type hierarchy and resource information from a bundle
pub fn extract_bundle_info(bundle: &Bundle) -> Option<BundleInfo> {
    let mut type_hierarchy = std::collections::HashMap::new();
    let mut resources = Vec::new();
    let mut complex_types = Vec::new();

    if let Some(entries) = bundle.entry.as_ref() {
        for entry in entries {
            if let Some(resource) = &entry.resource {
                if let Resource::StructureDefinition(def) = resource {
                    if is_valid_structure_definition(def) {
                        // Extract type hierarchy from baseDefinition
                        if let Some(base_def) = &def.base_definition {
                            if let Some(parent) = base_def.split('/').next_back() {
                                type_hierarchy.insert(def.name.clone(), parent.to_string());
                            }
                        }

                        if def.kind == "resource" && !def.r#abstract {
                            resources.push(def.name.clone());
                        } else if def.kind == "complex-type" && !def.r#abstract {
                            complex_types.push(def.name.clone());
                        }
                    }
                }
            }
        }
    }

    Some((type_hierarchy, resources, complex_types))
}

/// Generates global constructs (Resource enum, type hierarchy, etc.) once at the end
pub fn generate_global_constructs(
    output_path: impl AsRef<Path>,
    type_hierarchy: &std::collections::HashMap<String, String>,
    all_resources: &[String],
    all_complex_types: &[String],
) -> io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_path.as_ref())?;

    // Generate the Resource enum
    if !all_resources.is_empty() {
        let resource_enum = generate_resource_enum(all_resources.to_vec());
        write!(file, "{}", resource_enum)?;

        // Add From<T> implementations for base types
        writeln!(
            file,
            "// --- From<T> Implementations for Element<T, Extension> ---"
        )?;
        writeln!(file, "impl From<bool> for Element<bool, Extension> {{")?;
        writeln!(file, "    fn from(value: bool) -> Self {{")?;
        writeln!(file, "        Self {{")?;
        writeln!(file, "            value: Some(value),")?;
        writeln!(file, "            ..Default::default()")?;
        writeln!(file, "        }}")?;
        writeln!(file, "    }}")?;
        writeln!(file, "}}")?;

        writeln!(
            file,
            "impl From<std::primitive::i32> for Element<std::primitive::i32, Extension> {{"
        )?;
        writeln!(file, "    fn from(value: std::primitive::i32) -> Self {{")?;
        writeln!(file, "        Self {{")?;
        writeln!(file, "            value: Some(value),")?;
        writeln!(file, "            ..Default::default()")?;
        writeln!(file, "        }}")?;
        writeln!(file, "    }}")?;
        writeln!(file, "}}")?;

        writeln!(
            file,
            "impl From<std::string::String> for Element<std::string::String, Extension> {{"
        )?;
        writeln!(file, "    fn from(value: std::string::String) -> Self {{")?;
        writeln!(file, "        Self {{")?;
        writeln!(file, "            value: Some(value),")?;
        writeln!(file, "            ..Default::default()")?;
        writeln!(file, "        }}")?;
        writeln!(file, "    }}")?;
        writeln!(file, "}}")?;
        writeln!(file, "// --- End From<T> Implementations ---")?;
    }

    // Generate type hierarchy module
    if !type_hierarchy.is_empty() {
        let type_hierarchy_module = generate_type_hierarchy_module(type_hierarchy);
        write!(file, "{}", type_hierarchy_module)?;
    }

    // Generate ComplexTypes struct and FhirComplexTypeProvider implementation
    if !all_complex_types.is_empty() {
        writeln!(file, "\n// --- Complex Types Provider ---")?;
        writeln!(file, "/// Marker struct for complex type information")?;
        writeln!(file, "pub struct ComplexTypes;")?;
        writeln!(
            file,
            "\nimpl crate::fhir_version::FhirComplexTypeProvider for ComplexTypes {{"
        )?;
        writeln!(
            file,
            "    fn get_complex_type_names() -> Vec<&'static str> {{"
        )?;
        writeln!(file, "        vec![")?;
        for complex_type in all_complex_types {
            writeln!(file, "            \"{}\",", complex_type)?;
        }
        writeln!(file, "        ]")?;
        writeln!(file, "    }}")?;
        writeln!(file, "}}")?;
    }

    Ok(())
}
/// Converts a FHIR StructureDefinition to Rust code.
///
/// This function is the main entry point for converting a single StructureDefinition
/// into its corresponding Rust representation, handling both primitive and complex types.
///
/// # Arguments
///
/// * `sd` - The StructureDefinition to convert
/// * `cycles` - Set of detected circular dependencies that need special handling
///
/// # Returns
///
/// Returns a string containing the generated Rust code for this structure.
///
/// # Type Handling
///
/// - **Primitive types**: Generates type aliases using `Element<T, Extension>`
/// - **Complex types**: Generates full struct definitions with all fields
/// - **Resources**: Generates structs that can be included in the Resource enum
pub fn structure_definition_to_rust(
    sd: &StructureDefinition,
    cycles: &std::collections::HashSet<(String, String)>,
) -> String {
    let mut output = String::new();

    // Handle primitive types differently
    if is_primitive_type(sd) {
        return generate_primitive_type(sd);
    }

    // Generate struct documentation for the main type
    let struct_doc = generate_struct_documentation(sd);

    // Process elements for complex types and resources
    if let Some(snapshot) = &sd.snapshot {
        if let Some(elements) = &snapshot.element {
            let mut processed_types = std::collections::HashSet::new();
            // Find the root element to get its documentation
            let root_element_doc = elements
                .iter()
                .find(|e| e.path == sd.name)
                .map(generate_element_documentation)
                .unwrap_or_default();

            process_elements(
                elements,
                &mut output,
                &mut processed_types,
                cycles,
                &sd.name,
                if !struct_doc.is_empty() {
                    Some(&struct_doc)
                } else if !root_element_doc.is_empty() {
                    Some(&root_element_doc)
                } else {
                    None
                },
            );
        }
    }
    output
}

/// Generates Rust type aliases for FHIR primitive types.
///
/// FHIR primitive types are mapped to appropriate Rust types and wrapped in
/// the `Element<T, Extension>` container to handle FHIR's extension mechanism.
///
/// # Arguments
///
/// * `sd` - The StructureDefinition for the primitive type
///
/// # Returns
///
/// Returns a string containing the type alias definition.
///
/// # Type Mappings
///
/// - `boolean` → `Element<bool, Extension>`
/// - `integer` → `Element<i32, Extension>`
/// - `decimal` → `DecimalElement<Extension>` (special handling for precision)
/// - `string`/`code`/`uri` → `Element<String, Extension>`
/// - Date/time types → `Element<PrecisionDate/DateTime/Time, Extension>` (precision-aware types)
///
/// # Note
///
/// This function must be kept in sync with `extract_inner_element_type` in
/// `fhir_macro/src/lib.rs` to ensure consistent type handling.
pub fn generate_primitive_type(sd: &StructureDefinition) -> String {
    let type_name = &sd.name;
    let mut output = String::new();

    // Determine the value type based on the primitive type
    let value_type = match type_name.as_str() {
        "boolean" => "bool",
        "integer" | "positiveInt" | "unsignedInt" => "std::primitive::i32",
        "decimal" => "std::primitive::f64",
        "integer64" => "std::primitive::i64",
        "string" => "std::string::String",
        "code" => "std::string::String",
        "base64Binary" => "std::string::String",
        "canonical" => "std::string::String",
        "id" => "std::string::String",
        "oid" => "std::string::String",
        "uri" => "std::string::String",
        "url" => "std::string::String",
        "uuid" => "std::string::String",
        "markdown" => "std::string::String",
        "xhtml" => "std::string::String",
        "date" => "crate::date_time::PrecisionDate",
        "dateTime" => "crate::date_time::PrecisionDateTime",
        "instant" => "crate::date_time::PrecisionInstant",
        "time" => "crate::date_time::PrecisionTime",
        _ => "std::string::String",
    };

    // Add type-specific documentation
    match type_name.as_str() {
        "boolean" => {
            output.push_str("/// FHIR primitive type for boolean values (true/false)\n");
        }
        "integer" => {
            output.push_str("/// FHIR primitive type for whole number values\n");
        }
        "positiveInt" => {
            output.push_str("/// FHIR primitive type for positive whole number values (> 0)\n");
        }
        "unsignedInt" => {
            output
                .push_str("/// FHIR primitive type for non-negative whole number values (>= 0)\n");
        }
        "decimal" => {
            output
                .push_str("/// FHIR primitive type for decimal numbers with arbitrary precision\n");
        }
        "string" => {
            output.push_str("/// FHIR primitive type for character sequences\n");
        }
        "code" => {
            output.push_str("/// FHIR primitive type for coded values drawn from a defined set\n");
        }
        "uri" => {
            output
                .push_str("/// FHIR primitive type for Uniform Resource Identifiers (RFC 3986)\n");
        }
        "url" => {
            output.push_str("/// FHIR primitive type for Uniform Resource Locators\n");
        }
        "canonical" => {
            output.push_str(
                "/// FHIR primitive type for canonical URLs that reference FHIR resources\n",
            );
        }
        "base64Binary" => {
            output.push_str("/// FHIR primitive type for base64-encoded binary data\n");
        }
        "date" => {
            output.push_str("/// FHIR primitive type for date values (year, month, day)\n");
        }
        "dateTime" => {
            output.push_str("/// FHIR primitive type for date and time values\n");
        }
        "instant" => {
            output.push_str(
                "/// FHIR primitive type for instant in time values (to millisecond precision)\n",
            );
        }
        "time" => {
            output.push_str("/// FHIR primitive type for time of day values\n");
        }
        "id" => {
            output.push_str("/// FHIR primitive type for logical IDs within FHIR resources\n");
        }
        "oid" => {
            output.push_str("/// FHIR primitive type for Object Identifiers (OIDs)\n");
        }
        "uuid" => {
            output.push_str("/// FHIR primitive type for Universally Unique Identifiers (UUIDs)\n");
        }
        "markdown" => {
            output.push_str("/// FHIR primitive type for markdown-formatted text\n");
        }
        "xhtml" => {
            output
                .push_str("/// FHIR primitive type for XHTML-formatted text with limited subset\n");
        }
        _ => {
            output.push_str(&format!(
                "/// FHIR primitive type {}\n",
                capitalize_first_letter(type_name)
            ));
        }
    }

    // Add description if available
    if let Some(desc) = &sd.description {
        if !desc.is_empty() {
            output.push_str("/// \n");
            output.push_str(&format!("/// {}\n", escape_doc_comment(desc)));
        }
    }

    // Add reference to the spec
    output.push_str(&format!("/// \n/// See: [{}]({})\n", sd.name, sd.url));

    // Generate a type alias using Element<T, Extension> or DecimalElement<Extension> for decimal type
    if type_name == "decimal" {
        output.push_str("pub type Decimal = DecimalElement<Extension>;\n\n");
    } else {
        output.push_str(&format!(
            "pub type {} = Element<{}, Extension>;\n\n",
            capitalize_first_letter(type_name),
            value_type
        ));
        // REMOVED From<T> generation from here to avoid conflicts
    }

    output
}

/// Detects circular dependencies between FHIR types.
///
/// This function analyzes ElementDefinitions to find circular references between
/// types where both directions have a cardinality of 1 (max="1"). Such cycles
/// would cause infinite-sized structs in Rust, so they need to be broken with
/// `Box<T>` pointers.
///
/// # Arguments
///
/// * `elements` - All ElementDefinitions to analyze for cycles
///
/// # Returns
///
/// Returns a set of tuples representing detected cycles. Each tuple contains
/// the two type names that form a cycle.
///
/// # Cycle Detection Logic
///
/// 1. Builds a dependency graph of type relationships with max="1"
/// 2. Finds bidirectional dependencies (A → B and B → A)
/// 3. Adds special cases like Bundle → Resource for known problematic cycles
///
/// # Example
///
/// If `Identifier` has a field of type `Reference` and `Reference` has a field
/// of type `Identifier`, both with max="1", this creates a cycle that must be
/// broken by boxing one of the references.
pub fn detect_struct_cycles(
    elements: &Vec<&ElementDefinition>,
) -> std::collections::HashSet<(String, String)> {
    let mut cycles = std::collections::HashSet::new();
    let mut graph: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    // Build direct dependencies where max=1
    for element in elements {
        if let Some(types) = &element.r#type {
            let path_parts: Vec<&str> = element.path.split('.').collect();
            if path_parts.len() > 1 {
                let from_type = path_parts[0].to_string();
                if !from_type.is_empty() && element.max.as_deref() == Some("1") {
                    for ty in types {
                        if !ty.code.contains('.') && from_type != ty.code {
                            graph
                                .entry(from_type.clone())
                                .or_default()
                                .push(ty.code.clone());
                        }
                    }
                }
            }
        }
    }

    // Find cycles between exactly two structs
    for (from_type, deps) in &graph {
        for to_type in deps {
            if let Some(back_deps) = graph.get(to_type) {
                if back_deps.contains(from_type) {
                    // We found a cycle between exactly two structs
                    cycles.insert((from_type.clone(), to_type.clone()));
                }
            }
        }
    }

    // Add cycle from Bundle to Resource since Bundle.issues contains Resources (an specially generated enum) beginning in R5
    if elements
        .iter()
        .any(|e| e.id.as_ref().is_some_and(|id| id == "Bundle.issues"))
    {
        cycles.insert(("Bundle".to_string(), "Resource".to_string()));
    }

    cycles
}
/// Extracts the element ID from a contentReference value.
///
/// This function handles both local contentReferences (starting with #) and
/// URL-based contentReferences that include a fragment after #.
///
/// # Arguments
///
/// * `content_ref` - The contentReference value from a FHIR ElementDefinition
///
/// # Returns
///
/// Returns the element ID portion of the contentReference.
///
/// # Examples
///
/// - "#Patient.name" → "Patient.name"
/// - "https://sql-on-fhir.org/ig/StructureDefinition/ViewDefinition#ViewDefinition.select" → "ViewDefinition.select"
/// - "invalid-ref" → ""
pub fn extract_content_reference_id(content_ref: &str) -> &str {
    if let Some(fragment_start) = content_ref.find('#') {
        let fragment = &content_ref[fragment_start + 1..];
        if !fragment.is_empty() { fragment } else { "" }
    } else {
        ""
    }
}

/// Generates a Rust type name from a FHIR element path.
///
/// This function converts dotted FHIR paths into appropriate Rust type names
/// using PascalCase conventions.
///
/// # Arguments
///
/// * `path` - The FHIR element path (e.g., "Patient.name.given")
///
/// # Returns
///
/// Returns a PascalCase type name suitable for Rust.
///
/// # Examples
///
/// - "Patient" → "Patient"
/// - "Patient.name" → "PatientName"
/// - "Observation.value.quantity" → "ObservationValueQuantity"
///
/// # Note
///
/// The first path segment becomes the base name, and subsequent segments
/// are capitalized and concatenated to create a compound type name.
pub fn generate_type_name(path: &str) -> String {
    let parts: Vec<&str> = path.split('.').collect();
    if !parts.is_empty() {
        let mut result = String::from(parts[0]);
        for part in &parts[1..] {
            result.push_str(
                &part
                    .chars()
                    .next()
                    .unwrap()
                    .to_uppercase()
                    .chain(part.chars().skip(1))
                    .collect::<String>(),
            );
        }
        result
    } else {
        String::from("Empty path provided to generate_type_name")
    }
}


