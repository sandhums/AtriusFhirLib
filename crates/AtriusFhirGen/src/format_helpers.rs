use crate::element_definition::{ElementDefinitionBinding, ElementDefinitionConstraint, ElementDefinitionExample};

/// Converts a FHIR field name to a valid Rust identifier.
///
/// This function transforms FHIR field names into valid Rust identifiers by:
/// - Converting camelCase to snake_case
/// - Escaping Rust keywords with the `r#` prefix
///
/// # Arguments
///
/// * `input` - The original FHIR field name
///
/// # Returns
///
/// Returns a string that is a valid Rust identifier.
///
/// # Examples
///
/// ```ignore
/// # use helios_fhir_gen::make_rust_safe;
/// assert_eq!(make_rust_safe("birthDate"), "birth_date");
/// assert_eq!(make_rust_safe("type"), "r#type");
/// assert_eq!(make_rust_safe("abstract"), "r#abstract");
/// ```
pub fn make_rust_safe(input: &str) -> String {
    let snake_case = input
        .chars()
        .enumerate()
        .fold(String::new(), |mut acc, (i, c)| {
            if i > 0 && c.is_uppercase() {
                acc.push('_');
            }
            acc.push(c.to_lowercase().next().unwrap());
            acc
        });

    match snake_case.as_str() {
        "type" | "use" | "abstract" | "for" | "ref" | "const" | "where" => {
            format!("r#{}", snake_case)
        }
        _ => snake_case,
    }
}

/// Capitalizes the first letter of a string.
///
/// This utility function is used to convert FHIR type names to proper Rust
/// type names that follow PascalCase conventions.
///
/// # Arguments
///
/// * `s` - The string to capitalize
///
/// # Returns
///
/// Returns a new string with the first character capitalized.
///
/// # Examples
///
/// ```ignore
/// # use helios_fhir_gen::capitalize_first_letter;
/// assert_eq!(capitalize_first_letter("patient"), "Patient");
/// assert_eq!(capitalize_first_letter("humanName"), "HumanName");
/// ```
pub fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Escapes markdown text for use in Rust doc comments.
///
/// This function escapes special characters that could interfere with
/// Rust's doc comment parsing.
///
/// # Arguments
///
/// * `text` - The markdown text to escape
///
/// # Returns
///
/// Returns the escaped text safe for use in doc comments.
pub fn escape_doc_comment(text: &str) -> String {
    // First, normalize all line endings to \n and remove bare CRs
    let normalized = text
        .replace("\r\n", "\n") // Convert Windows line endings
        .replace('\r', "\n"); // Convert bare CRs to newlines

    let mut result = String::new();
    let mut in_code_block = false;

    // Process each line
    for line in normalized.lines() {
        let trimmed_line = line.trim();

        // Check for code block markers
        if trimmed_line == "```" {
            if in_code_block {
                // This is a closing ```
                result.push_str("```\n");
                in_code_block = false;
            } else {
                // This is an opening ```
                result.push_str("```text\n");
                in_code_block = true;
            }
            continue;
        }

        // Apply standard replacements
        let processed = line
            .replace("*/", "*\\/")
            .replace("/*", "/\\*")
            // Fix common typos in FHIR spec
            .replace("(aka \"privacy tags\".", "(aka \"privacy tags\").")
            .replace("(aka \"tagged\")", "(aka 'tagged')")
            // Escape comparison operators that look like quote markers to clippy
            .replace(" <=", " \\<=")
            .replace(" >=", " \\>=")
            .replace("(<=", "(\\<=")
            .replace("(>=", "(\\>=");

        result.push_str(&processed);
        result.push('\n');
    }

    // Clean up excessive blank lines and trailing whitespace
    result = result.replace("\n\n\n", "\n\n");
    result.trim_end().to_string()
}

/// Formats text content for use in Rust doc comments, handling proper indentation.
///
/// This function ensures that multi-line content is properly formatted for Rust doc
/// comments, including handling bullet points and numbered lists that need continuation indentation.
///
/// # Arguments
///
/// * `text` - The text to format
/// * `in_list` - Whether we're currently in a list context
///
/// # Returns
///
/// Returns formatted lines ready for doc comment output.
pub fn format_doc_content(text: &str, in_list: bool) -> Vec<String> {
    let mut output = Vec::new();
    let mut in_list_item = false;

    for line in text.split('\n') {
        let trimmed = line.trim_start();

        // Check if this is a list item (bullet, numbered, or dash)
        let is_bullet = trimmed.starts_with("* ") && !in_list;
        let is_dash = trimmed.starts_with("- ") && !in_list;
        let is_numbered = !in_list && {
            // Match patterns like "1) ", "2. ", "10) ", etc.
            if let Some(first_space) = trimmed.find(' ') {
                let prefix = &trimmed[..first_space];
                // Check if it ends with ) or . and starts with a number
                (prefix.ends_with(')') || prefix.ends_with('.'))
                    && prefix.chars().next().is_some_and(|c| c.is_numeric())
            } else {
                false
            }
        };

        if is_bullet || is_numbered || is_dash {
            in_list_item = true;
            output.push(line.to_string());
        } else if in_list_item {
            // We're in a list item context
            if line.trim().is_empty() {
                // Empty line ends the list item
                output.push(String::new());
                in_list_item = false;
            } else if trimmed.starts_with("* ")
                || trimmed.starts_with("- ")
                || (trimmed.find(' ').is_some_and(|idx| {
                let prefix = &trimmed[..idx];
                (prefix.ends_with(')') || prefix.ends_with('.'))
                    && prefix.chars().next().is_some_and(|c| c.is_numeric())
            }))
            {
                // New list item
                output.push(line.to_string());
            } else {
                // Continuation line - needs to be indented
                let content = line.trim();
                if !content.is_empty() {
                    // For numbered lists like "1) text", indent to align with text
                    // For bullet/dash lists, use 2 spaces
                    let indent = if let Some(prev_line) = output.last() {
                        let prev_trimmed = prev_line.trim_start();
                        if let Some(space_pos) = prev_trimmed.find(' ') {
                            let prefix = &prev_trimmed[..space_pos];
                            if (prefix.ends_with(')') || prefix.ends_with('.'))
                                && prefix.chars().next().is_some_and(|c| c.is_numeric())
                            {
                                // It's a numbered list - use 3 spaces for safety
                                "   ".to_string()
                            } else {
                                "  ".to_string()
                            }
                        } else {
                            "  ".to_string()
                        }
                    } else {
                        "  ".to_string()
                    };
                    output.push(format!("{}{}", indent, content));
                }
            }
        } else {
            // Not in a list item - regular line
            output.push(line.to_string());
        }
    }

    output
}

/// Formats cardinality information into human-readable text.
///
/// # Arguments
///
/// * `min` - Minimum cardinality (0 or 1)
/// * `max` - Maximum cardinality ("1", "*", or a specific number)
///
/// # Returns
///
/// Returns a formatted string describing the cardinality.
pub fn format_cardinality(min: Option<u32>, max: Option<&str>) -> String {
    let min_val = min.unwrap_or(0);
    let max_val = max.unwrap_or("1");

    match (min_val, max_val) {
        (0, "1") => "Optional (0..1)".to_string(),
        (1, "1") => "Required (1..1)".to_string(),
        (0, "*") => "Optional, Multiple (0..*)".to_string(),
        (1, "*") => "Required, Multiple (1..*)".to_string(),
        (min, max) => format!("{min}..{max}"),
    }
}

/// Formats constraint information for documentation.
///
/// # Arguments
///
/// * `constraints` - Vector of ElementDefinitionConstraint
///
/// # Returns
///
/// Returns formatted constraint documentation.
pub fn format_constraints(constraints: &[ElementDefinitionConstraint]) -> String {
    if constraints.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    output.push_str("/// ## Constraints\n");

    for constraint in constraints {
        let escaped_human = escape_doc_comment(&constraint.human);

        // Handle multi-line constraint descriptions
        let human_lines: Vec<&str> = escaped_human.split('\n').collect();

        if human_lines.len() == 1 {
            // Single line - output as before
            output.push_str(&format!(
                "/// - **{}**: {} ({})\n",
                constraint.key, escaped_human, constraint.severity
            ));
        } else {
            // Multi-line - format the first line with key and severity
            output.push_str(&format!(
                "/// - **{}**: {} ({})\n",
                constraint.key, human_lines[0], constraint.severity
            ));

            // Add subsequent lines with proper indentation
            for line in &human_lines[1..] {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    output.push_str(&format!("///   {}\n", trimmed));
                }
            }
        }

        if let Some(expr) = &constraint.expression {
            output.push_str(&format!(
                "///   Expression: `{}`\n",
                escape_doc_comment(expr)
            ));
        }
    }

    output
}

/// Formats example values for documentation.
///
/// # Arguments
///
/// * `examples` - Vector of ElementDefinitionExample
///
/// # Returns
///
/// Returns formatted example documentation.
pub fn format_examples(examples: &[ElementDefinitionExample]) -> String {
    if examples.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    output.push_str("/// ## Examples\n");

    for example in examples {
        output.push_str(&format!(
            "/// - {}: {:?}\n",
            escape_doc_comment(&example.label),
            example.value
        ));
    }

    output
}

/// Formats binding information for documentation.
///
/// # Arguments
///
/// * `binding` - Optional ElementDefinitionBinding
///
/// # Returns
///
/// Returns formatted binding documentation.
pub fn format_binding(binding: Option<&ElementDefinitionBinding>) -> String {
    if let Some(b) = binding {
        let mut output = String::new();
        output.push_str("/// ## Binding\n");

        output.push_str(&format!("/// - **Strength**: {}\n", b.strength));

        if let Some(desc) = &b.description {
            let escaped_desc = escape_doc_comment(desc);
            let desc_lines: Vec<&str> = escaped_desc.split('\n').collect();

            if desc_lines.len() == 1 {
                // Single line - output as before
                output.push_str(&format!("/// - **Description**: {}\n", escaped_desc));
            } else {
                // Multi-line - format the first line with "Description:"
                output.push_str(&format!("/// - **Description**: {}\n", desc_lines[0]));

                // Add subsequent lines with proper indentation
                for line in &desc_lines[1..] {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        output.push_str(&format!("///   {}\n", trimmed));
                    }
                }
            }
        }

        if let Some(vs) = &b.value_set {
            output.push_str(&format!("/// - **ValueSet**: {}\n", vs));
        }

        output
    } else {
        String::new()
    }
}

