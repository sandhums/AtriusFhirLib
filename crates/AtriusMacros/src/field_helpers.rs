use heck::ToLowerCamelCase;
use syn::{token, Lit, Meta};
use syn::punctuated::Punctuated;

/// Determines the effective field name for FHIR serialization.
///
/// This function extracts the field name that should be used during JSON serialization,
/// respecting FHIR naming conventions and custom rename attributes.
///
/// # Attribute Processing
///
/// - If `#[fhir_serde(rename = "customName")]` is present, uses the custom name
/// - Otherwise, converts the Rust field name from `snake_case` to `camelCase`
///
/// # Arguments
///
/// * `field` - The field definition from the parsed struct
///
/// # Returns
///
/// The field name as it should appear in the serialized JSON.
///
/// # Examples
///
/// ```rust,ignore
/// // Field: pub implicit_rules: Option<Uri>
/// // Result: "implicitRules" (camelCase conversion)
///
/// // Field: #[fhir_serde(rename = "modifierExtension")]
/// //        pub modifier_extension: Option<Vec<Extension>>
/// // Result: "modifierExtension" (explicit rename)
/// ```
pub(crate) fn get_effective_field_name(field: &syn::Field) -> String {
    for attr in &field.attrs {
        if attr.path().is_ident("fhir_serde")
            && let Ok(list) =
            attr.parse_args_with(Punctuated::<Meta, token::Comma>::parse_terminated)
        {
            for meta in list {
                if let Meta::NameValue(nv) = meta
                    && nv.path.is_ident("rename")
                    && let syn::Expr::Lit(expr_lit) = nv.value
                    && let Lit::Str(lit_str) = expr_lit.lit
                {
                    return lit_str.value();
                }
            }
        }
    }
    // Default to camelCase if no rename attribute found
    field
        .ident
        .as_ref()
        .unwrap()
        .to_string()
        .to_lower_camel_case()
}

/// Checks if a field should be flattened during serialization.
///
/// This function determines whether a field has the `#[fhir_serde(flatten)]` attribute,
/// which indicates that the field's contents should be serialized directly into the
/// parent object rather than as a nested object.
///
/// # FHIR Usage
///
/// Flattening is commonly used for:
/// - **Choice types**: FHIR `[x]` fields that can be one of several types
/// - **Inheritance**: Base class fields that should appear at the same level
/// - **Resource polymorphism**: Fields that contain different resource types
///
/// # Arguments
///
/// * `field` - The field definition to check for the flatten attribute
///
/// # Returns
///
/// `true` if the field has `#[fhir_serde(flatten)]`, `false` otherwise.
///
/// # Examples
///
/// ```rust,ignore
/// // Regular field (not flattened)
/// pub name: Option<String>,  // false
///
/// // Flattened choice type field
/// #[fhir_serde(flatten)]
/// pub subject: Option<ActivityDefinitionSubject>,  // true
/// ```
pub(crate) fn is_flattened(field: &syn::Field) -> bool {
    for attr in &field.attrs {
        if attr.path().is_ident("fhir_serde")
            && let Ok(list) =
            attr.parse_args_with(Punctuated::<Meta, token::Comma>::parse_terminated)
        {
            for meta in list {
                if let Meta::Path(path) = meta
                    && path.is_ident("flatten")
                {
                    return true;
                }
            }
        }
    }
    false
}
