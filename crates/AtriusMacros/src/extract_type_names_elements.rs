use syn::{token, Lit, Meta, Ident};
use syn::punctuated::Punctuated;
use crate::type_helpers::get_option_inner_type;

/// Extracts namespace and name from type_info attributes.
pub(crate) fn extract_type_info_attributes(attrs: &[syn::Attribute], type_name: &Ident) -> (String, String) {
    for attr in attrs {
        if attr.path().is_ident("type_info") {
            if let Ok(list) =
                attr.parse_args_with(Punctuated::<Meta, token::Comma>::parse_terminated)
            {
                let mut namespace = None;
                let mut name = None;

                for meta in list {
                    if let Meta::NameValue(nv) = meta {
                        if nv.path.is_ident("namespace") {
                            if let syn::Expr::Lit(expr_lit) = nv.value {
                                if let Lit::Str(lit_str) = expr_lit.lit {
                                    namespace = Some(lit_str.value());
                                }
                            }
                        } else if nv.path.is_ident("name") {
                            if let syn::Expr::Lit(expr_lit) = nv.value {
                                if let Lit::Str(lit_str) = expr_lit.lit {
                                    name = Some(lit_str.value());
                                }
                            }
                        }
                    }
                }

                if let (Some(ns), Some(n)) = (namespace, name) {
                    return (format!("\"{}\"", ns), format!("\"{}\"", n));
                }
            }
        }
    }

    // Default: Assume FHIR namespace and use the type name
    ("\"FHIR\"".to_string(), format!("\"{}\"", type_name))
}

/// Extracts the FHIR type suffix from a choice element field name using pattern matching.
/// For example, "valueQuantity" -> Some(("value", "Quantity")), "valueString" -> Some(("value", "String"))
pub(crate) fn extract_type_suffix_from_field_name(field_name: &str) -> Option<(&str, &str)> {
    let chars: Vec<char> = field_name.chars().collect();

    // Look for the pattern: lowercase...Uppercase...
    // This indicates the transition from base name to type name
    let mut transition_index = None;

    for i in 1..chars.len() {
        if chars[i - 1].is_lowercase() && chars[i].is_uppercase() {
            transition_index = Some(i);
            break;
        }
    }

    if let Some(idx) = transition_index {
        let base_name = &field_name[..idx];
        let type_suffix = &field_name[idx..];

        // Validate that this looks like a valid FHIR type suffix:
        // - Starts with uppercase letter
        // - Has at least 2 characters (to avoid false positives like "valueA")
        // - Contains only alphanumeric characters (and potentially numbers at the end like Integer64)
        if type_suffix.len() >= 2
            && type_suffix.chars().next().is_some_and(|c| c.is_uppercase())
            && type_suffix.chars().all(|c| c.is_alphanumeric())
            && !base_name.is_empty()
        {
            return Some((base_name, type_suffix));
        }
    }

    None
}

/// Extracts the base name from fhir_choice_element attribute if present.
pub(crate) fn extract_choice_element_base_name(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("fhir_choice_element") {
            if let Ok(list) =
                attr.parse_args_with(Punctuated::<Meta, token::Comma>::parse_terminated)
            {
                for meta in list {
                    if let Meta::NameValue(nv) = meta {
                        if nv.path.is_ident("base_name") {
                            if let syn::Expr::Lit(expr_lit) = nv.value {
                                if let Lit::Str(lit_str) = expr_lit.lit {
                                    return Some(lit_str.value());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Extracts choice elements from fhir_resource attribute if present.
pub(crate) fn extract_resource_choice_elements(attrs: &[syn::Attribute]) -> Option<Vec<String>> {
    for attr in attrs {
        if attr.path().is_ident("fhir_resource") {
            if let Ok(list) =
                attr.parse_args_with(Punctuated::<Meta, token::Comma>::parse_terminated)
            {
                for meta in list {
                    if let Meta::NameValue(nv) = meta {
                        if nv.path.is_ident("choice_elements") {
                            if let syn::Expr::Lit(expr_lit) = nv.value {
                                if let Lit::Str(lit_str) = expr_lit.lit {
                                    // Split the comma-separated list of choice elements
                                    let elements: Vec<String> = lit_str
                                        .value()
                                        .split(',')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                    return Some(elements);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Extracts the FHIR type name from a type path for primitive FHIR types.
/// Returns None if the type is not a recognized FHIR primitive type.
pub(crate) fn extract_fhir_primitive_type_name(ty: &syn::Type) -> Option<&'static str> {
    // Get the inner type if this is an Option<T>
    let inner_type = if let Some(inner) = get_option_inner_type(ty) {
        inner
    } else {
        ty
    };

    // Check if this is a path type
    if let syn::Type::Path(type_path) = inner_type {
        if let Some(segment) = type_path.path.segments.last() {
            let type_name = segment.ident.to_string();

            // Map FHIR type aliases to their lowercase primitive names
            match type_name.as_str() {
                "Uri" => Some("uri"),
                "Code" => Some("code"),
                "Id" => Some("id"),
                "Oid" => Some("oid"),
                "Uuid" => Some("uuid"),
                "Canonical" => Some("canonical"),
                "Url" => Some("url"),
                "Markdown" => Some("markdown"),
                "Base64Binary" => Some("base64Binary"),
                "Instant" => Some("instant"),
                "Date" => Some("date"),
                "DateTime" => Some("dateTime"),
                "Time" => Some("time"),
                "String" => Some("string"),
                "Boolean" => Some("boolean"),
                "Integer" => Some("integer"),
                "Integer64" => Some("integer64"),
                "PositiveInt" => Some("positiveInt"),
                "UnsignedInt" => Some("unsignedInt"),
                "Decimal" => Some("decimal"),
                _ => None,
            }
        } else {
            None
        }
    } else {
        None
    }
}
