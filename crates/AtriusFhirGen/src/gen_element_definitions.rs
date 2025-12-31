use crate::constraint_helpers::format_constraint_attributes;
use crate::element_definition::ElementDefinition;
use crate::format_helpers::{capitalize_first_letter, make_rust_safe};
use crate::gen_helpers::{extract_content_reference_id, generate_type_name};
use crate::generate_struct_element_doc::generate_element_documentation;

/// Processes ElementDefinitions to generate Rust struct and enum definitions.
///
/// This function groups related ElementDefinitions by their parent path and generates
/// the corresponding Rust types, including handling of choice types (polymorphic elements).
///
/// # Arguments
///
/// * `elements` - Slice of ElementDefinitions to process
/// * `output` - Mutable string to append generated code to
/// * `processed_types` - Set tracking which types have already been generated
/// * `cycles` - Set of detected circular dependencies requiring Box<T> handling
/// * `root_type_name` - The name of the root type (e.g., "Patient")
/// * `root_doc` - Optional documentation for the root type
///
/// # Process Overview
///
/// 1. **Grouping**: Groups elements by their parent path (e.g., "Patient.name")
/// 2. **Choice Types**: Generates enums for choice elements ending in "\[x\]"
/// 3. **Structs**: Generates struct definitions with all fields
/// 4. **Deduplication**: Ensures each type is only generated once
///
/// # Generated Code Features
///
/// - Derives for Debug, Clone, PartialEq, Eq, FhirSerde, FhirPath, Default
/// - Choice type enums with proper serde renaming
/// - Cycle-breaking with Box<T> where needed
/// - Optional wrapping for elements with min=0
pub fn process_elements(
    elements: &[ElementDefinition],
    output: &mut String,
    processed_types: &mut std::collections::HashSet<String>,
    cycles: &std::collections::HashSet<(String, String)>,
    root_type_name: &str,
    root_doc: Option<&str>,
) {
    // Group elements by their parent path
    let mut element_groups: std::collections::HashMap<String, Vec<&ElementDefinition>> =
        std::collections::HashMap::new();

    // First pass - collect all type names that will be generated
    for element in elements {
        let path_parts: Vec<&str> = element.path.split('.').collect();
        if path_parts.len() > 1 {
            let parent_path = path_parts[..path_parts.len() - 1].join(".");
            element_groups.entry(parent_path).or_default().push(element);
        }
    }

    // Process each group in sorted order for deterministic output
    let mut sorted_groups: Vec<_> = element_groups.into_iter().collect();
    sorted_groups.sort_by(|a, b| a.0.cmp(&b.0));

    for (path, group) in sorted_groups {
        let type_name = generate_type_name(&path);

        // Skip if we've already processed this type
        if processed_types.contains(&type_name) {
            continue;
        }

        processed_types.insert(type_name.clone());

        // Process choice types first
        let choice_fields: Vec<_> = group.iter().filter(|e| e.path.ends_with("[x]")).collect();
        for choice in choice_fields {
            let base_name = choice
                .path
                .rsplit('.')
                .next()
                .unwrap()
                .trim_end_matches("[x]");

            let enum_name = format!(
                "{}{}",
                capitalize_first_letter(&type_name),
                capitalize_first_letter(base_name)
            );

            // Skip if we've already processed this enum
            if processed_types.contains(&enum_name) {
                continue;
            }
            processed_types.insert(enum_name.clone());

            // Add documentation comment for the enum
            output.push_str(&format!(
                "/// Choice of types for the {}\\[x\\] field in {}\n",
                base_name,
                capitalize_first_letter(&type_name)
            ));

            // Generate enum derives - Remove Eq to prevent MIR optimization cycles
            let enum_derives = ["Debug", "Clone", "PartialEq", "FhirSerde", "FhirPath"];
            output.push_str(&format!("#[derive({})]\n", enum_derives.join(", ")));

            // Add choice element attribute to mark this as a choice type
            output.push_str(&format!(
                "#[fhir_choice_element(base_name = \"{}\")]\n",
                base_name
            ));

            // Add other serde attributes and enum definition
            output.push_str(&format!("pub enum {} {{\n", enum_name));

            if let Some(types) = &choice.r#type {
                for ty in types {
                    let type_code = capitalize_first_letter(&ty.code);
                    let rename_value = format!("{}{}", base_name, type_code);

                    // Add documentation for each variant
                    output.push_str(&format!(
                        "    /// Variant accepting the {} type.\n",
                        type_code
                    ));
                    output.push_str(&format!(
                        "    #[fhir_serde(rename = \"{}\")]\n",
                        rename_value
                    ));
                    output.push_str(&format!("    {}({}),\n", type_code, type_code));
                }
            }
            output.push_str("}\n\n");
        }

        // Collect all choice element fields for this struct
        let choice_element_fields: Vec<String> = group
            .iter()
            .filter(|e| e.path.ends_with("[x]"))
            .filter_map(|e| e.path.rsplit('.').next())
            .map(|name| name.trim_end_matches("[x]").to_string())
            .collect();

        // Add struct documentation
        if path == *root_type_name {
            // This is the root type, use the provided documentation
            if let Some(doc) = root_doc {
                output.push_str(doc);
            }
        } else {
            // For nested types, try to find the documentation from the element
            if let Some(type_element) = elements.iter().find(|e| e.path == path) {
                let doc = generate_element_documentation(type_element);
                if !doc.is_empty() {
                    output.push_str(&doc);
                }
            } else {
                // Generate a basic doc comment
                output.push_str(&format!(
                    "/// {} sub-type\n",
                    capitalize_first_letter(&type_name)
                ));
            }
        }

        // Collect (but do not emit yet) struct-level executable constraint attributes for the
        // element that defines this struct itself (e.g., `Attachment`). We'll emit these
        // after `#[derive(...)]` for prettier output ordering.
        let struct_invariant_attrs: String = if let Some(type_element) = elements.iter().find(|e| e.path == path) {
            if let Some(constraints) = &type_element.constraint {
                format_constraint_attributes(constraints, &type_element.path)
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        // Generate struct derives - Remove Eq to prevent MIR optimization cycles
        let derives = [
            "Debug",
            "Clone",
            "PartialEq",
            "FhirSerde",
            "FhirPath",
            "Default",
            "FhirValidate",
        ];
        output.push_str(&format!("#[derive({})]\n", derives.join(", ")));

        // Add fhir_resource attribute if there are choice elements
        if !choice_element_fields.is_empty() {
            let choice_elements_str = choice_element_fields.join(",");
            output.push_str(&format!(
                "#[fhir_resource(choice_elements = \"{}\")]\n",
                choice_elements_str
            ));
        }

        // Emit struct-level invariants AFTER derive (and after fhir_resource if present)
        if !struct_invariant_attrs.trim().is_empty() {
            for line in struct_invariant_attrs.lines() {
                if !line.trim().is_empty() {
                    output.push_str(line);
                    output.push('\n');
                }
            }
        }

        // Add other serde attributes and struct definition
        output.push_str(&format!(
            "pub struct {} {{\n",
            capitalize_first_letter(&type_name)
        ));

        for element in &group {
            if let Some(field_name) = element.path.rsplit('.').next() {
                if !field_name.contains("[x]") {
                    generate_element_definition(element, &type_name, output, cycles, elements);
                } else {
                    // For choice types, we've already created an enum, so we just need to add the field
                    // that uses that enum type. We don't need to expand each choice type into separate fields.
                    generate_element_definition(element, &type_name, output, cycles, elements);
                }
            }
        }
        output.push_str("}\n\n");
    }
}

/// Generates a Rust field definition from a FHIR ElementDefinition.
///
/// This function converts a single FHIR element into a Rust struct field,
/// handling type mapping, cardinality, choice types, and circular references.
///
/// # Arguments
///
/// * `element` - The ElementDefinition to convert
/// * `type_name` - Name of the parent type containing this element
/// * `output` - Mutable string to append the field definition to
/// * `cycles` - Set of circular dependencies requiring Box<T> handling
/// * `elements` - All elements (used for resolving content references)
///
/// # Field Generation Features
///
/// - **Type Mapping**: Maps FHIR types to appropriate Rust types
/// - **Cardinality**: Wraps in `Option<T>` for min=0, `Vec<T>` for max="*"
/// - **Choice Types**: Uses generated enum types for polymorphic elements
/// - **Cycle Breaking**: Adds `Box<T>` for circular references
/// - **Serde Attributes**: Adds rename and flatten attributes as needed
/// - **Content References**: Resolves `#id` references to other elements
pub fn generate_element_definition(
    element: &ElementDefinition,
    type_name: &str,
    output: &mut String,
    cycles: &std::collections::HashSet<(String, String)>,
    elements: &[ElementDefinition],
) {
    if let Some(field_name) = element.path.rsplit('.').next() {
        let rust_field_name = make_rust_safe(field_name);

        let mut serde_attrs = Vec::new();
        // Handle field renaming, ensuring we don't add duplicate rename attributes
        if field_name != rust_field_name {
            // For choice fields, use the name without [x]
            if field_name.ends_with("[x]") {
                serde_attrs.push(format!(
                    "rename = \"{}\"",
                    field_name.trim_end_matches("[x]")
                ));
            } else {
                serde_attrs.push(format!("rename = \"{}\"", field_name));
            }
        }

        let ty = match element.r#type.as_ref().and_then(|t| t.first()) {
            Some(ty) => ty,
            None => {
                if let Some(content_ref) = &element.content_reference {
                    let ref_id = extract_content_reference_id(content_ref);
                    if let Some(referenced_element) = elements
                        .iter()
                        .find(|e| e.id.as_ref().is_some_and(|id| id == ref_id))
                    {
                        if let Some(ref_ty) =
                            referenced_element.r#type.as_ref().and_then(|t| t.first())
                        {
                            ref_ty
                        } else {
                            return;
                        }
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            }
        };
        let is_array = element.max.as_deref() == Some("*");
        let base_type = match ty.code.as_str() {
            // https://build.fhir.org/fhirpath.html#types
            "http://hl7.org/fhirpath/System.Boolean" => "bool",
            "http://hl7.org/fhirpath/System.String" => "String",
            "http://hl7.org/fhirpath/System.Integer" => "std::primitive::i32",
            "http://hl7.org/fhirpath/System.Long" => "std::primitive::i64",
            "http://hl7.org/fhirpath/System.Decimal" => "std::primitive::f64",
            "http://hl7.org/fhirpath/System.Date" => "std::string::String",
            "http://hl7.org/fhirpath/System.DateTime" => "std::string::String",
            "http://hl7.org/fhirpath/System.Time" => "std::string::String",
            "http://hl7.org/fhirpath/System.Quantity" => "std::string::String",
            "Element" | "BackboneElement" => &generate_type_name(&element.path),
            // Fix for R6 TestPlan: replace Base with BackboneElement
            // See https://github.com/HeliosSoftware/hfs/issues/11
            "Base" if element.path.contains("TestPlan") => &generate_type_name(&element.path),
            _ => &capitalize_first_letter(&ty.code),
        };

        let base_type = if let Some(content_ref) = &element.content_reference {
            let ref_id = extract_content_reference_id(content_ref);
            if !ref_id.is_empty() {
                generate_type_name(ref_id)
            } else {
                base_type.to_string()
            }
        } else {
            base_type.to_string()
        };

        let mut type_str = if field_name.ends_with("[x]") {
            let base_name = field_name.trim_end_matches("[x]");
            let enum_name = format!(
                "{}{}",
                capitalize_first_letter(type_name),
                capitalize_first_letter(base_name)
            );
            // For choice fields, we use flatten instead of rename
            serde_attrs.clear(); // Clear any previous attributes
            serde_attrs.push("flatten".to_string());
            format!("Option<{}>", enum_name)
        } else if is_array {
            format!("Option<Vec<{}>>", base_type)
        } else if element.min.unwrap_or(0) == 0 {
            format!("Option<{}>", base_type)
        } else {
            base_type.to_string()
        };

        // Add Box<> to break cycles (only to the "to" type in the cycle)
        if let Some(field_type) = element.r#type.as_ref().and_then(|t| t.first()) {
            let from_type = element.path.split('.').next().unwrap_or("");
            if !from_type.is_empty() {
                for (cycle_from, cycle_to) in cycles.iter() {
                    if cycle_from == from_type && cycle_to == &field_type.code {
                        // Add Box<> around the type, preserving Option if present
                        if type_str.starts_with("Option<") {
                            type_str = format!("Option<Box<{}>>", &type_str[7..type_str.len() - 1]);
                        } else {
                            type_str = format!("Box<{}>", type_str);
                        }
                        break;
                    }
                }
            }
        }

        // Generate documentation for this field
        let doc_comment = generate_element_documentation(element);
        if !doc_comment.is_empty() {
            // Debug: Check for any issues
            if doc_comment
                .lines()
                .any(|line| !line.trim().is_empty() && !line.starts_with("//"))
            {
                eprintln!("\n=== WARNING: Found doc comment with lines missing /// prefix ===");
                eprintln!("Field: {}", element.path);
                eprintln!("Doc comment has {} lines", doc_comment.lines().count());
                for (i, line) in doc_comment.lines().enumerate() {
                    if !line.trim().is_empty() && !line.starts_with("//") {
                        eprintln!("  Line {}: Missing prefix: {:?}", i, line);
                    }
                }
                eprintln!("==================================================\n");
            }

            // Indent all doc comments with 4 spaces
            for line in doc_comment.lines() {
                // Ensure every line is a proper doc comment
                if line.trim().is_empty() {
                    output.push_str("    /// \n");
                } else if line.starts_with("///") {
                    output.push_str(&format!("    {}\n", line));
                } else {
                    // This line doesn't have a doc comment prefix - this is a bug!
                    eprintln!("WARNING: Doc comment line without /// prefix: {}", line);
                    output.push_str(&format!("    /// {}\n", line));
                }
            }
        }
        // New - Emit executable constraint attributes (in addition to doc-only constraints).
        if let Some(constraints) = &element.constraint {
            let attrs = format_constraint_attributes(constraints, &element.path);
            if !attrs.is_empty() {
                for line in attrs.lines() {
                    if !line.trim().is_empty() {
                        output.push_str("    ");
                        output.push_str(line);
                        output.push('\n');
                    }
                }
            }
        }

        // Output consolidated serde attributes if any exist
        if !serde_attrs.is_empty() {
            output.push_str(&format!("    #[fhir_serde({})]\n", serde_attrs.join(", ")));
        }

        // For choice fields, strip the [x] from the field name
        let clean_field_name = if rust_field_name.ends_with("[x]") {
            rust_field_name.trim_end_matches("[x]").to_string()
        } else {
            rust_field_name
        };

        // Check if the line would be too long (rustfmt's default max line width is 100)
        // Account for "    pub " (8 chars) + ": " (2 chars) + "," (1 char) = 11 extra chars
        let line_length = 8 + clean_field_name.len() + 2 + type_str.len() + 1;

        if line_length > 100 {
            // For Option<Vec<...>>, rustfmt prefers a specific format
            if type_str.starts_with("Option<Vec<") && type_str.ends_with(">>") {
                // Extract the inner type
                let inner_type = &type_str[11..type_str.len() - 2];
                output.push_str(&format!(
                    "    pub {}: Option<\n        Vec<{}>,\n    >,\n",
                    clean_field_name, inner_type
                ));
            } else if type_str.starts_with("Option<") && type_str.ends_with(">") {
                // For other Option<...> types that are too long
                let inner_type = &type_str[7..type_str.len() - 1];
                output.push_str(&format!(
                    "    pub {}:\n        Option<{}>,\n",
                    clean_field_name, inner_type
                ));
            } else {
                // Break other long type declarations across multiple lines
                output.push_str(&format!(
                    "    pub {}:\n        {},\n",
                    clean_field_name, type_str
                ));
            }
        } else {
            output.push_str(&format!("    pub {}: {},\n", clean_field_name, type_str));
        }
    }
}
