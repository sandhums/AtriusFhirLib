use crate::element_definition::ElementDefinition;
use crate::format_helpers::{capitalize_first_letter, escape_doc_comment, format_binding, format_cardinality, format_constraints, format_doc_content, format_examples};
use crate::structure_definition::StructureDefinition;

/// Generates documentation comments for a FHIR struct/type from its StructureDefinition.
///
/// This function extracts type-level documentation from a StructureDefinition.
///
/// # Arguments
///
/// * `sd` - The StructureDefinition to document
///
/// # Returns
///
/// Returns a string containing formatted Rust doc comments for the type.
pub fn generate_struct_documentation(sd: &StructureDefinition) -> String {
    let mut output = String::new();

    // Type name
    output.push_str(&format!(
        "/// FHIR {} type\n",
        capitalize_first_letter(&sd.name)
    ));

    // Description
    if let Some(desc) = &sd.description {
        if !desc.is_empty() {
            output.push_str("/// \n");
            let escaped_desc = escape_doc_comment(desc);
            let formatted_lines = format_doc_content(&escaped_desc, false);

            // Split long descriptions into multiple lines
            for line in formatted_lines {
                if line.is_empty() {
                    output.push_str("/// \n");
                } else if line.len() <= 77 {
                    // Line fits, output as is
                    output.push_str("/// ");
                    output.push_str(&line);
                    output.push('\n');
                } else {
                    // Need to wrap - use word boundaries
                    let words = line.split_whitespace().collect::<Vec<_>>();
                    let mut current_line = String::new();

                    // Check if this line needs indentation
                    // Either it's already indented (continuation) or it's a list item
                    let trimmed_line = line.trim_start();
                    let is_list_item = trimmed_line.starts_with("* ")
                        || trimmed_line.starts_with("- ")
                        || trimmed_line.find(' ').is_some_and(|idx| {
                        let prefix = &trimmed_line[..idx];
                        (prefix.ends_with(')') || prefix.ends_with('.'))
                            && prefix.chars().next().is_some_and(|c| c.is_numeric())
                    });

                    // Determine if this is a numbered list that needs more indentation
                    let is_numbered_list = trimmed_line.find(' ').is_some_and(|idx| {
                        let prefix = &trimmed_line[..idx];
                        (prefix.ends_with(')') || prefix.ends_with('.'))
                            && prefix.chars().next().is_some_and(|c| c.is_numeric())
                    });

                    let indent = if line.starts_with("   ") {
                        "   " // Already has 3 spaces
                    } else if line.starts_with("  ") {
                        "  " // Already has 2 spaces
                    } else if is_numbered_list {
                        // For numbered lists, use 3 spaces for continuation lines
                        "   "
                    } else if is_list_item {
                        // For bullet/dash lists, use 2 spaces
                        "  "
                    } else {
                        ""
                    };

                    // For list items, we don't want to indent the first line
                    let first_line_indent = if is_list_item { "" } else { indent };

                    for word in words.iter() {
                        if current_line.is_empty() {
                            // First word - include indent if needed (but not for bullet points)
                            current_line = if !first_line_indent.is_empty() {
                                format!("{}{}", first_line_indent, word)
                            } else {
                                word.to_string()
                            };
                        } else if current_line.len() + 1 + word.len() <= 77 {
                            current_line.push(' ');
                            current_line.push_str(word);
                        } else {
                            // Output the current line
                            output.push_str("/// ");
                            output.push_str(&current_line);
                            output.push('\n');
                            // Start new line with this word, always use indent for continuations
                            current_line = if !indent.is_empty() {
                                format!("{}{}", indent, word)
                            } else {
                                word.to_string()
                            };
                        }
                    }

                    // Output any remaining content
                    if !current_line.is_empty() {
                        output.push_str("/// ");
                        output.push_str(&current_line);
                        output.push('\n');
                    }
                }
            }
        }
    }

    // Purpose
    if let Some(purpose) = &sd.purpose {
        if !purpose.is_empty() {
            output.push_str("/// \n");
            output.push_str("/// ## Purpose\n");
            let escaped_purpose = escape_doc_comment(purpose);
            let formatted_lines = format_doc_content(&escaped_purpose, false);

            for line in formatted_lines {
                if line.is_empty() {
                    output.push_str("/// \n");
                } else {
                    output.push_str(&format!("/// {}\n", line));
                }
            }
        }
    }

    // Kind and base
    output.push_str("/// \n");
    output.push_str(&format!(
        "/// ## Type: {} type\n",
        capitalize_first_letter(&sd.kind)
    ));

    if sd.r#abstract {
        output.push_str("/// Abstract type (cannot be instantiated directly)\n");
    }

    if let Some(base) = &sd.base_definition {
        output.push_str(&format!("/// Base type: {}\n", base));
    }

    // Status and version
    output.push_str("/// \n");
    output.push_str(&format!("/// ## Status: {}\n", sd.status));

    // FHIR version
    if let Some(version) = &sd.fhir_version {
        output.push_str(&format!("/// FHIR Version: {}\n", version));
    }

    // URL reference
    output.push_str("/// \n");
    output.push_str(&format!("/// See: [{}]({})\n", sd.name, sd.url));

    output
}

/// Generates comprehensive documentation comments for a FHIR element.
///
/// This function extracts all available documentation from an ElementDefinition
/// and formats it into structured Rust doc comments.
///
/// # Arguments
///
/// * `element` - The ElementDefinition to document
///
/// # Returns
///
/// Returns a string containing formatted Rust doc comments.
/// IMPORTANT: Every line in the returned string MUST start with "///"
pub fn generate_element_documentation(element: &ElementDefinition) -> String {
    let mut output = String::new();

    // Short description (primary doc comment)
    if let Some(short) = &element.short {
        output.push_str(&format!("/// {}\n", escape_doc_comment(short)));
    }

    // Full definition
    if let Some(definition) = &element.definition {
        if !definition.is_empty() {
            output.push_str("/// \n");
            let escaped_definition = escape_doc_comment(definition);
            let formatted_lines = format_doc_content(&escaped_definition, false);

            // Process each formatted line
            for line in formatted_lines {
                if line.is_empty() {
                    output.push_str("/// \n");
                } else if line.len() <= 77 {
                    // Line fits, output as is
                    output.push_str("/// ");
                    output.push_str(&line);
                    output.push('\n');
                } else {
                    // Need to wrap - use word boundaries
                    let words = line.split_whitespace().collect::<Vec<_>>();
                    let mut current_line = String::new();

                    // Check if this line needs indentation
                    // Either it's already indented (continuation) or it's a list item
                    let trimmed_line = line.trim_start();
                    let is_list_item = trimmed_line.starts_with("* ")
                        || trimmed_line.starts_with("- ")
                        || trimmed_line.find(' ').is_some_and(|idx| {
                        let prefix = &trimmed_line[..idx];
                        (prefix.ends_with(')') || prefix.ends_with('.'))
                            && prefix.chars().next().is_some_and(|c| c.is_numeric())
                    });

                    // Determine if this is a numbered list that needs more indentation
                    let is_numbered_list = trimmed_line.find(' ').is_some_and(|idx| {
                        let prefix = &trimmed_line[..idx];
                        (prefix.ends_with(')') || prefix.ends_with('.'))
                            && prefix.chars().next().is_some_and(|c| c.is_numeric())
                    });

                    let indent = if line.starts_with("   ") {
                        "   " // Already has 3 spaces
                    } else if line.starts_with("  ") {
                        "  " // Already has 2 spaces
                    } else if is_numbered_list {
                        // For numbered lists, use 3 spaces for continuation lines
                        "   "
                    } else if is_list_item {
                        // For bullet/dash lists, use 2 spaces
                        "  "
                    } else {
                        ""
                    };

                    // For list items, we don't want to indent the first line
                    let first_line_indent = if is_list_item { "" } else { indent };

                    for word in words.iter() {
                        if current_line.is_empty() {
                            // First word - include indent if needed (but not for bullet points)
                            current_line = if !first_line_indent.is_empty() {
                                format!("{}{}", first_line_indent, word)
                            } else {
                                word.to_string()
                            };
                        } else if current_line.len() + 1 + word.len() <= 77 {
                            current_line.push(' ');
                            current_line.push_str(word);
                        } else {
                            // Output the current line
                            output.push_str("/// ");
                            output.push_str(&current_line);
                            output.push('\n');
                            // Start new line with this word, always use indent for continuations
                            current_line = if !indent.is_empty() {
                                format!("{}{}", indent, word)
                            } else {
                                word.to_string()
                            };
                        }
                    }

                    // Output any remaining content
                    if !current_line.is_empty() {
                        output.push_str("/// ");
                        output.push_str(&current_line);
                        output.push('\n');
                    }
                }
            }
        }
    }

    // Requirements
    if let Some(requirements) = &element.requirements {
        if !requirements.is_empty() {
            output.push_str("/// \n");
            output.push_str("/// ## Requirements\n");
            let escaped_requirements = escape_doc_comment(requirements);
            let formatted_lines = format_doc_content(&escaped_requirements, false);

            for line in formatted_lines {
                if line.is_empty() {
                    output.push_str("/// \n");
                } else if line.len() <= 77 {
                    // Line fits, output as is
                    output.push_str("/// ");
                    output.push_str(&line);
                    output.push('\n');
                } else {
                    // Need to wrap - use word boundaries
                    let words = line.split_whitespace().collect::<Vec<_>>();
                    let mut current_line = String::new();

                    // Check if this line needs indentation
                    // Either it's already indented (continuation) or it's a list item
                    let trimmed_line = line.trim_start();
                    let is_list_item = trimmed_line.starts_with("* ")
                        || trimmed_line.starts_with("- ")
                        || trimmed_line.find(' ').is_some_and(|idx| {
                        let prefix = &trimmed_line[..idx];
                        (prefix.ends_with(')') || prefix.ends_with('.'))
                            && prefix.chars().next().is_some_and(|c| c.is_numeric())
                    });

                    // Determine if this is a numbered list that needs more indentation
                    let is_numbered_list = trimmed_line.find(' ').is_some_and(|idx| {
                        let prefix = &trimmed_line[..idx];
                        (prefix.ends_with(')') || prefix.ends_with('.'))
                            && prefix.chars().next().is_some_and(|c| c.is_numeric())
                    });

                    let indent = if line.starts_with("   ") {
                        "   " // Already has 3 spaces
                    } else if line.starts_with("  ") {
                        "  " // Already has 2 spaces
                    } else if is_numbered_list {
                        // For numbered lists, use 3 spaces for continuation lines
                        "   "
                    } else if is_list_item {
                        // For bullet/dash lists, use 2 spaces
                        "  "
                    } else {
                        ""
                    };

                    // For list items, we don't want to indent the first line
                    let first_line_indent = if is_list_item { "" } else { indent };

                    for word in words.iter() {
                        if current_line.is_empty() {
                            // First word - include indent if needed (but not for bullet points)
                            current_line = if !first_line_indent.is_empty() {
                                format!("{}{}", first_line_indent, word)
                            } else {
                                word.to_string()
                            };
                        } else if current_line.len() + 1 + word.len() <= 77 {
                            current_line.push(' ');
                            current_line.push_str(word);
                        } else {
                            // Output the current line
                            output.push_str("/// ");
                            output.push_str(&current_line);
                            output.push('\n');
                            // Start new line with this word, always use indent for continuations
                            current_line = if !indent.is_empty() {
                                format!("{}{}", indent, word)
                            } else {
                                word.to_string()
                            };
                        }
                    }

                    // Output any remaining content
                    if !current_line.is_empty() {
                        output.push_str("/// ");
                        output.push_str(&current_line);
                        output.push('\n');
                    }
                }
            }
        }
    }

    // Implementation comments
    if let Some(comment) = &element.comment {
        if !comment.is_empty() {
            output.push_str("/// \n");
            output.push_str("/// ## Implementation Notes\n");
            let escaped_comment = escape_doc_comment(comment);
            let formatted_lines = format_doc_content(&escaped_comment, false);

            for line in formatted_lines {
                if line.is_empty() {
                    output.push_str("/// \n");
                } else if line.len() <= 77 {
                    // Line fits, output as is
                    output.push_str("/// ");
                    output.push_str(&line);
                    output.push('\n');
                } else {
                    // Need to wrap - use word boundaries
                    let words = line.split_whitespace().collect::<Vec<_>>();
                    let mut current_line = String::new();

                    // Check if this line needs indentation
                    // Either it's already indented (continuation) or it's a list item
                    let trimmed_line = line.trim_start();
                    let is_list_item = trimmed_line.starts_with("* ")
                        || trimmed_line.starts_with("- ")
                        || trimmed_line.find(' ').is_some_and(|idx| {
                        let prefix = &trimmed_line[..idx];
                        (prefix.ends_with(')') || prefix.ends_with('.'))
                            && prefix.chars().next().is_some_and(|c| c.is_numeric())
                    });

                    // Determine if this is a numbered list that needs more indentation
                    let is_numbered_list = trimmed_line.find(' ').is_some_and(|idx| {
                        let prefix = &trimmed_line[..idx];
                        (prefix.ends_with(')') || prefix.ends_with('.'))
                            && prefix.chars().next().is_some_and(|c| c.is_numeric())
                    });

                    let indent = if line.starts_with("   ") {
                        "   " // Already has 3 spaces
                    } else if line.starts_with("  ") {
                        "  " // Already has 2 spaces
                    } else if is_numbered_list {
                        // For numbered lists, use 3 spaces for continuation lines
                        "   "
                    } else if is_list_item {
                        // For bullet/dash lists, use 2 spaces
                        "  "
                    } else {
                        ""
                    };

                    // For list items, we don't want to indent the first line
                    let first_line_indent = if is_list_item { "" } else { indent };

                    for word in words.iter() {
                        if current_line.is_empty() {
                            // First word - include indent if needed (but not for bullet points)
                            current_line = if !first_line_indent.is_empty() {
                                format!("{}{}", first_line_indent, word)
                            } else {
                                word.to_string()
                            };
                        } else if current_line.len() + 1 + word.len() <= 77 {
                            current_line.push(' ');
                            current_line.push_str(word);
                        } else {
                            // Output the current line
                            output.push_str("/// ");
                            output.push_str(&current_line);
                            output.push('\n');
                            // Start new line with this word, always use indent for continuations
                            current_line = if !indent.is_empty() {
                                format!("{}{}", indent, word)
                            } else {
                                word.to_string()
                            };
                        }
                    }

                    // Output any remaining content
                    if !current_line.is_empty() {
                        output.push_str("/// ");
                        output.push_str(&current_line);
                        output.push('\n');
                    }
                }
            }
        }
    }

    // Cardinality
    let cardinality = format_cardinality(element.min, element.max.as_deref());
    output.push_str("/// \n");
    output.push_str(&format!("/// ## Cardinality: {}\n", cardinality));

    // Special semantics
    let mut special_semantics = Vec::new();

    if element.is_modifier == Some(true) {
        let mut modifier_text = "Modifier element".to_string();
        if let Some(reason) = &element.is_modifier_reason {
            modifier_text.push_str(&format!(" - {}", escape_doc_comment(reason)));
        }
        special_semantics.push(modifier_text);
    }

    if element.is_summary == Some(true) {
        special_semantics.push("Included in summary".to_string());
    }

    if element.must_support == Some(true) {
        special_semantics.push("Must be supported".to_string());
    }

    if let Some(meaning) = &element.meaning_when_missing {
        special_semantics.push(format!("When missing: {}", escape_doc_comment(meaning)));
    }

    if let Some(order) = &element.order_meaning {
        special_semantics.push(format!("Order meaning: {}", escape_doc_comment(order)));
    }

    if !special_semantics.is_empty() {
        output.push_str("/// \n");
        output.push_str("/// ## Special Semantics\n");
        for semantic in special_semantics {
            output.push_str(&format!("/// - {}\n", semantic));
        }
    }

    // Constraints
    if let Some(constraints) = &element.constraint {
        let constraint_doc = format_constraints(constraints);
        if !constraint_doc.is_empty() {
            output.push_str("/// \n");
            output.push_str(&constraint_doc);
        }
    }

    // Examples
    if let Some(examples) = &element.example {
        let example_doc = format_examples(examples);
        if !example_doc.is_empty() {
            output.push_str("/// \n");
            output.push_str(&example_doc);
        }
    }

    // Binding
    let binding_doc = format_binding(element.binding.as_ref());
    if !binding_doc.is_empty() {
        output.push_str("/// \n");
        output.push_str(&binding_doc);
    }

    // Aliases
    if let Some(aliases) = &element.alias {
        if !aliases.is_empty() {
            output.push_str("/// \n");
            output.push_str("/// ## Aliases\n");

            // Handle aliases that might contain newlines
            let all_aliases = aliases.join(", ");
            let escaped_aliases = escape_doc_comment(&all_aliases);

            // Split on newlines and ensure each line has the /// prefix
            for line in escaped_aliases.split('\n') {
                if line.trim().is_empty() {
                    output.push_str("/// \n");
                } else {
                    output.push_str(&format!("/// {}\n", line));
                }
            }
        }
    }

    // Conditions
    if let Some(conditions) = &element.condition {
        if !conditions.is_empty() {
            output.push_str("/// \n");
            output.push_str("/// ## Conditions\n");
            output.push_str(&format!("/// Used when: {}\n", conditions.join(", ")));
        }
    }

    // Validate that all non-empty lines have the /// prefix
    let validated_output = output.lines()
        .enumerate()
        .map(|(i, line)| {
            if line.trim().is_empty() {
                "/// ".to_string()
            } else if line.starts_with("///") {
                line.to_string()
            } else {
                // This should never happen, but if it does, add the prefix
                eprintln!("ERROR in generate_element_documentation for {}: Line {} missing /// prefix: {}",
                          &element.path, i, line);
                format!("/// {}", line)
            }
        })
        .collect::<Vec<String>>()
        .join("\n");

    if !validated_output.is_empty() && !validated_output.ends_with('\n') {
        format!("{}\n", validated_output)
    } else {
        validated_output
    }
}
