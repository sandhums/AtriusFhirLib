use quote::quote;
use syn::{token, Fields, Lit, Meta, Ident};
use syn::punctuated::Punctuated;
use crate::extract_type_names_elements::{extract_fhir_primitive_type_name, extract_resource_choice_elements};
use crate::field_helpers::is_flattened;
use crate::type_helpers::get_option_inner_type;

/// Determines the effective field name for FHIRPath object property access.
///
/// This function extracts the field name that should be used as a property key
/// in the generated `EvaluationResult::Object`, ensuring that FHIRPath expressions
/// can access fields using their FHIR specification names.
///
/// # Attribute Processing
///
/// - If `#[fhir_serde(rename = "customName")]` is present, uses the custom name
/// - Otherwise, uses the raw Rust field identifier without case conversion
///
/// # Difference from Serialization
///
/// Unlike `get_effective_field_name()` which converts to camelCase for JSON
/// serialization, this function preserves exact FHIR names for FHIRPath access.
/// This ensures FHIRPath expressions match the FHIR specification exactly.
///
/// # Arguments
///
/// * `field` - The field definition from the parsed struct
///
/// # Returns
///
/// The field name as it should appear in FHIRPath object property access.
///
/// # Examples
///
/// ```rust,ignore
/// // Field: pub implicit_rules: Option<Uri>
/// // Result: "implicit_rules" (raw identifier)
///
/// // Field: #[fhir_serde(rename = "implicitRules")]
/// //        pub implicit_rules: Option<Uri>
/// // Result: "implicitRules" (explicit rename for FHIR compliance)
/// ```
pub(crate) fn get_fhirpath_field_name(field: &syn::Field) -> String {
    for attr in &field.attrs {
        if attr.path().is_ident("fhir_serde") {
            if let Ok(list) =
                attr.parse_args_with(Punctuated::<Meta, token::Comma>::parse_terminated)
            {
                for meta in list {
                    if let Meta::NameValue(nv) = meta {
                        if nv.path.is_ident("rename") {
                            if let syn::Expr::Lit(expr_lit) = nv.value {
                                if let Lit::Str(lit_str) = expr_lit.lit {
                                    return lit_str.value();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // Default to the raw field identifier if no rename attribute found
    field.ident.as_ref().unwrap().to_string()
}

pub(crate) fn generate_fhirpath_struct_impl(
    name: &Ident,
    data: &syn::DataStruct,
    attrs: &[syn::Attribute],
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    let fields = match &data.fields {
        Fields::Named(fields) => &fields.named,
        _ => panic!("FhirPath derive macro only supports structs with named fields."),
    };

    let field_conversions = fields.iter().map(|field| {
        let field_name_ident = field.ident.as_ref().unwrap();
        let field_key_str = get_fhirpath_field_name(field); // Use the specific FHIRPath naming helper
        let field_ty = &field.ty; // Get the field type

        // Check if this field is flattened
        let is_field_flattened = is_flattened(field);

        // Check if this field is a FHIR primitive type that needs special handling
        let fhir_type_name = extract_fhir_primitive_type_name(field_ty);
        // Generate code to handle the field based on whether it's Option
        let is_option = get_option_inner_type(field_ty).is_some();

        // Special handling for flattened fields
        if is_field_flattened {
            // For flattened fields, we need to expand the inner object's fields into the parent map
            if is_option {
                quote! {
                    if let Some(inner_value) = &self.#field_name_ident {
                        let inner_result = inner_value.to_evaluation_result();
                        // If the inner result is an object, merge its fields into our map
                        if let atrius_fhirpath_support::evaluation_result::EvaluationResult::Object { map: inner_map, .. } = inner_result {
                            for (key, value) in inner_map {
                                map.insert(key, value);
                            }
                        }
                    }
                }
            } else {
                quote! {
                    let inner_result = self.#field_name_ident.to_evaluation_result();
                    // If the inner result is an object, merge its fields into our map
                    if let atrius_fhirpath_support::evaluation_result::EvaluationResult::Object { map: inner_map, .. } = inner_result {
                        for (key, value) in inner_map {
                            map.insert(key, value);
                        }
                    }
                }
            }
        } else if is_option {
            // For Option<T>, evaluate the inner value only if Some
            if let Some(type_name) = fhir_type_name {
                // Special handling for FHIR primitive types to preserve type information
                quote! {
                    if let Some(inner_value) = &self.#field_name_ident {
                        // Handle FHIR primitive types with proper type preservation
                        let mut field_result = inner_value.to_evaluation_result();
                        // Override type information for string-based FHIR primitive types
                        field_result = match field_result {
                            atrius_fhirpath_support::evaluation_result::EvaluationResult::String(s, _) => {
                                atrius_fhirpath_support::evaluation_result::EvaluationResult::fhir_string(s, #type_name)
                            },
                            atrius_fhirpath_support::evaluation_result::EvaluationResult::Boolean(b, _) => {
                                atrius_fhirpath_support::evaluation_result::EvaluationResult::fhir_boolean(b)
                            },
                            atrius_fhirpath_support::evaluation_result::EvaluationResult::Integer(i, _) => {
                                atrius_fhirpath_support::evaluation_result::EvaluationResult::fhir_integer(i)
                            },
                            atrius_fhirpath_support::evaluation_result::EvaluationResult::Decimal(d, _) => {
                                atrius_fhirpath_support::evaluation_result::EvaluationResult::fhir_decimal(d)
                            },
                            _ => field_result,
                        };
                        // Only insert if the inner evaluation is not Empty
                        if field_result != atrius_fhirpath_support::evaluation_result::EvaluationResult::Empty {
                            map.insert(#field_key_str.to_string(), field_result);
                        }
                    }
                    // If self.#field_name_ident is None, do nothing (don't insert Empty)
                }
            } else {
                quote! {
                    if let Some(inner_value) = &self.#field_name_ident {
                        let field_result = inner_value.to_evaluation_result();
                        // Only insert if the inner evaluation is not Empty
                        if field_result != atrius_fhirpath_support::evaluation_result::EvaluationResult::Empty {
                            map.insert(#field_key_str.to_string(), field_result);
                        }
                    }
                    // If self.#field_name_ident is None, do nothing (don't insert Empty)
                }
            }
        } else {
            // For non-Option<T>, evaluate directly
            if let Some(type_name) = fhir_type_name {
                // Special handling for FHIR primitive types to preserve type information
                quote! {
                    // Handle FHIR primitive types with proper type preservation
                    let mut field_result = self.#field_name_ident.to_evaluation_result();
                    // Override type information for FHIR primitive types
                    field_result = match field_result {
                        atrius_fhirpath_support::evaluation_result::EvaluationResult::String(s, _) => {
                            atrius_fhirpath_support::evaluation_result::EvaluationResult::fhir_string(s, #type_name)
                        },
                        atrius_fhirpath_support::evaluation_result::EvaluationResult::Boolean(b, _) => {
                            atrius_fhirpath_support::evaluation_result::EvaluationResult::fhir_boolean(b)
                        },
                        atrius_fhirpath_support::evaluation_result::EvaluationResult::Integer(i, _) => {
                            atrius_fhirpath_support::evaluation_result::EvaluationResult::fhir_integer(i)
                        },
                        atrius_fhirpath_support::evaluation_result::EvaluationResult::Decimal(d, _) => {
                            atrius_fhirpath_support::evaluation_result::EvaluationResult::fhir_decimal(d)
                        },
                        _ => field_result,
                    };
                    // Only insert if the evaluation is not Empty
                    if field_result != atrius_fhirpath_support::evaluation_result::EvaluationResult::Empty {
                        map.insert(#field_key_str.to_string(), field_result);
                    }
                }
            } else {
                quote! {
                    let field_result = self.#field_name_ident.to_evaluation_result();
                    // Only insert if the evaluation is not Empty
                    if field_result != atrius_fhirpath_support::evaluation_result::EvaluationResult::Empty {
                        map.insert(#field_key_str.to_string(), field_result);
                    }
                }
            }
        } // Return the generated code for this field
    });

    // Determine the type name to use for type info
    // For now, we'll use the struct name as the type name
    let type_name_str = name.to_string();

    let into_evaluation_result_impl = quote! {
        impl #impl_generics atrius_fhirpath_support::traits::IntoEvaluationResult for #name #ty_generics #where_clause {
            fn to_evaluation_result(&self) -> atrius_fhirpath_support::evaluation_result::EvaluationResult{
                // Use fully qualified path for HashMap
                let mut map = std::collections::HashMap::new();

                #(#field_conversions)* // Expand the field conversion logic

                // Return a typed object with FHIR type information
                atrius_fhirpath_support::evaluation_result::EvaluationResult::typed_object(
                    map,
                    "FHIR",
                    &#type_name_str
                )
            }
        }
    };

    // Check if this struct has the fhir_resource attribute with choice_elements
    if let Some(choice_elements) = extract_resource_choice_elements(attrs) {
        let choice_element_literals: Vec<_> = choice_elements
            .iter()
            .map(|elem| quote! { #elem })
            .collect();

        quote! {
            #into_evaluation_result_impl

            impl #impl_generics atrius_fhirpath_support::traits::FhirResourceMetadata for #name #ty_generics #where_clause {
                fn choice_elements() -> &'static [&'static str] {
                    &[#(#choice_element_literals),*]
                }
            }
        }
    } else {
        into_evaluation_result_impl
    }
}
