/// Generates a Rust enum containing all FHIR resource types.
///
/// This function creates a single enum that can represent any FHIR resource,
/// using serde's tag-based deserialization to automatically route JSON to
/// the correct variant based on the "resourceType" field.
///
/// # Arguments
///
/// * `resources` - Vector of resource type names to include in the enum
///
/// # Returns
///
/// Returns a string containing the Rust enum definition.
///
/// # Generated Features
///
/// - Tagged enum with `#[serde(tag = "resourceType")]` for automatic routing
/// - All standard derives for functionality and compatibility
/// - Each variant contains the corresponding resource struct
pub fn generate_resource_enum(resources: Vec<String>) -> String {
    let mut output = String::new();
    // Remove Eq from derives to prevent MIR optimization cycle with Bundle
    output.push_str("#[derive(Debug, Serialize, Deserialize, Clone, FhirPath)]\n");
    output.push_str("#[serde(tag = \"resourceType\")]\n");
    output.push_str("pub enum Resource {\n");

    for resource in &resources {
        output.push_str(&format!("    {}({}),\n", resource, resource));
    }

    output.push_str("}\n\n");

    // Manual PartialEq implementation to break MIR optimization cycle with Bundle
    // Using #[inline(never)] prevents the compiler from inlining and creating cycles during optimization
    output.push_str(
        "// Manual PartialEq implementation to break MIR optimization cycle with Bundle\n",
    );
    output.push_str("impl PartialEq for Resource {\n");
    output.push_str("    #[inline(never)]\n");
    output.push_str("    fn eq(&self, other: &Self) -> bool {\n");
    output.push_str("        match (self, other) {\n");

    for resource in &resources {
        output.push_str(&format!(
            "            (Self::{}(a), Self::{}(b)) => a == b,\n",
            resource, resource
        ));
    }

    output.push_str("            _ => false,\n");
    output.push_str("        }\n");
    output.push_str("    }\n");
    output.push_str("}\n\n");

    output
}

/// Generates a module containing type hierarchy information extracted from FHIR specifications.
///
/// This function creates a module with functions to query type relationships at runtime,
/// allowing the code to understand FHIR type inheritance without hard-coding.
///
/// # Arguments
///
/// * `type_hierarchy` - HashMap mapping type names to their parent types
///
/// # Returns
///
/// Returns a string containing the type hierarchy module definition.
pub fn generate_type_hierarchy_module(
    type_hierarchy: &std::collections::HashMap<String, String>,
) -> String {
    let mut output = String::new();

    output.push_str("\n// --- Type Hierarchy Module ---\n");
    output.push_str("/// Type hierarchy information extracted from FHIR specifications\n");
    output.push_str("pub mod type_hierarchy {\n");
    output.push_str("    use std::collections::HashMap;\n");
    output.push_str("    use std::sync::OnceLock;\n\n");

    // Generate the static HashMap
    output.push_str("    /// Maps FHIR type names to their parent types\n");
    output.push_str("    static TYPE_PARENTS: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();\n\n");

    output
        .push_str("    fn get_type_parents() -> &'static HashMap<&'static str, &'static str> {\n");
    output.push_str("        TYPE_PARENTS.get_or_init(|| {\n");
    output.push_str("            let mut m = HashMap::new();\n");

    // Sort entries for consistent output
    let mut sorted_entries: Vec<_> = type_hierarchy.iter().collect();
    sorted_entries.sort_by_key(|(k, _)| k.as_str());

    for (child, parent) in sorted_entries {
        output.push_str(&format!(
            "            m.insert(\"{}\", \"{}\");\n",
            child, parent
        ));
    }

    output.push_str("            m\n");
    output.push_str("        })\n");
    output.push_str("    }\n\n");

    // Generate helper functions
    output.push_str("    /// Checks if a type is a subtype of another type\n");
    output.push_str("    pub fn is_subtype_of(child: &str, parent: &str) -> bool {\n");
    output.push_str("        // Direct match\n");
    output.push_str("        if child.eq_ignore_ascii_case(parent) {\n");
    output.push_str("            return true;\n");
    output.push_str("        }\n\n");
    output.push_str("        // Walk up the type hierarchy\n");
    output.push_str("        let mut current = child;\n");
    output.push_str("        while let Some(&parent_type) = get_type_parents().get(current) {\n");
    output.push_str("            if parent_type.eq_ignore_ascii_case(parent) {\n");
    output.push_str("                return true;\n");
    output.push_str("            }\n");
    output.push_str("            current = parent_type;\n");
    output.push_str("        }\n");
    output.push_str("        false\n");
    output.push_str("    }\n\n");

    output.push_str("    /// Gets the parent type of a given type\n");
    output.push_str("    pub fn get_parent_type(type_name: &str) -> Option<&'static str> {\n");
    output.push_str("        get_type_parents().get(type_name).copied()\n");
    output.push_str("    }\n\n");

    output.push_str("    /// Gets all subtypes of a given parent type\n");
    output.push_str("    pub fn get_subtypes(parent: &str) -> Vec<&'static str> {\n");
    output.push_str("        get_type_parents().iter()\n");
    output.push_str("            .filter_map(|(child, p)| {\n");
    output.push_str("                if p.eq_ignore_ascii_case(parent) {\n");
    output.push_str("                    Some(*child)\n");
    output.push_str("                } else {\n");
    output.push_str("                    None\n");
    output.push_str("                }\n");
    output.push_str("            })\n");
    output.push_str("            .collect()\n");
    output.push_str("    }\n");

    output.push_str("}\n\n");
    output
}