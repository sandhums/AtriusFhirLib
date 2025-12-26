use quote::quote;
use syn::{Fields, Ident};
use crate::extract_type_names_elements::{extract_choice_element_base_name, extract_type_suffix_from_field_name};

pub(crate) fn generate_fhirpath_enum_impl(
    name: &Ident,
    data: &syn::DataEnum,
    attrs: &[syn::Attribute],
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    // Handle empty enums (like initial R6 Resource enum)
    if data.variants.is_empty() {
        let is_resource_enum = name == "Resource";

        let additional_impl = if is_resource_enum {
            quote! {
                impl #impl_generics crate::fhir_version::FhirResourceTypeProvider for #name #ty_generics #where_clause {
                    fn get_resource_type_names() -> Vec<&'static str> {
                        vec![] // Empty enum has no resource types
                    }
                }
            }
        } else {
            quote! {}
        };

        return quote! {
            impl #impl_generics atrius_fhirpath_support::traits::IntoEvaluationResult for #name #ty_generics #where_clause {
                fn to_evaluation_result(&self) -> atrius_fhirpath_support::evaluation_result::EvaluationResult {
                    // This should never be called for an empty enum
                    unreachable!("Empty enum should not be instantiated")
                }
            }

            #additional_impl
        };
    }

    // Check if the enum being derived is the top-level Resource enum
    let is_resource_enum = name == "Resource";

    // If this is a Resource enum, collect all variant names for the FhirResourceTypeProvider trait
    let resource_type_names: Vec<String> = if is_resource_enum {
        data.variants
            .iter()
            .map(|variant| variant.ident.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let match_arms = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_name_str = variant_name.to_string();

        match &variant.fields {
            Fields::Unit => {
                // For unit variants, return the variant name as a string (like a code)
                // This is likely for status codes etc., not the Resource enum
                quote! {
                    Self::#variant_name => atrius_fhirpath_support::evaluation_result::EvaluationResult ::string(#variant_name_str.to_string()),
                }
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                // Newtype variant
                if is_resource_enum {
                    // Special handling for the Resource enum: add resourceType
                    quote! {
                        Self::#variant_name(value) => {
                            let mut result = value.to_evaluation_result(); // Call on inner Box<ResourceStruct>
                            if let atrius_fhirpath_support::evaluation_result::EvaluationResult ::Object { ref mut map, .. } = result {
                                // Insert the resourceType field using the variant name
                                map.insert(
                                    "resourceType".to_string(),
                                    atrius_fhirpath_support::evaluation_result::EvaluationResult ::string(#variant_name_str.to_string())
                                );
                            }
                            // Return the (potentially modified) result
                            result
                        }
                    }
                } else {
                    // For other enums (like choice types), preserve type information from the variant
                    // Extract type information from the variant name or rename attribute
                    let variant_name_str = variant_name.to_string();
                    // Check for fhir_serde rename attribute to get the FHIR field name
                    let mut fhir_field_name = variant_name_str.clone();
                    for attr in &variant.attrs {
                        if attr.path().is_ident("fhir_serde") {
                            if let Ok(list) = attr.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::token::Comma>::parse_terminated) {
                                for meta in list {
                                    if let syn::Meta::NameValue(nv) = meta {
                                        if nv.path.is_ident("rename") {
                                            if let syn::Expr::Lit(expr_lit) = nv.value {
                                                if let syn::Lit::Str(lit_str) = expr_lit.lit {
                                                    fhir_field_name = lit_str.value();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Extract FHIR type from choice element field name (e.g., "valueCode" -> "code")
                    let fhir_type = if fhir_field_name.starts_with("value") && fhir_field_name.len() > 5 {
                        // Convert first character to lowercase for FHIR primitive types
                        let type_part = &fhir_field_name[5..]; // Remove "value" prefix
                        let mut chars = type_part.chars();
                        match chars.next() {
                            None => variant_name_str.clone(),
                            Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
                        }
                    } else if fhir_field_name.ends_with("Boolean") {
                        // Special case for FHIR boolean primitives - use lowercase
                        "boolean".to_string()
                    } else if fhir_field_name.ends_with("Integer") {
                        // Special case for FHIR integer primitives - use lowercase  
                        "integer".to_string()
                    } else if fhir_field_name.ends_with("Decimal") {
                        // Special case for FHIR decimal primitives - use lowercase
                        "decimal".to_string()
                    } else if fhir_field_name.ends_with("String") {
                        // Special case for FHIR string primitives - use lowercase
                        "string".to_string()
                    } else if fhir_field_name.ends_with("Instant") {
                        // Special case for FHIR instant primitives - use lowercase
                        "instant".to_string()
                    } else if fhir_field_name.ends_with("DateTime") {
                        // Special case for FHIR dateTime primitives - use lowercase
                        "dateTime".to_string()
                    } else if fhir_field_name.ends_with("Date") {
                        // Special case for FHIR date primitives - use lowercase
                        "date".to_string()
                    } else if fhir_field_name.ends_with("Time") {
                        // Special case for FHIR time primitives - use lowercase
                        "time".to_string()
                    } else {
                        // Fallback to variant name if it doesn't match known patterns
                        // Convert first character to lowercase for consistency with FHIR primitive naming
                        let mut chars = variant_name_str.chars();
                        match chars.next() {
                            None => variant_name_str.clone(),
                            Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
                        }
                    };
                    // For choice type enums that will be flattened, we need to return an object
                    // with the polymorphic field name as the key
                    // A choice type enum is one where variants have rename attributes with type suffixes
                    // e.g., "deceasedBoolean", "valueString", etc.
                    let is_choice_type_enum = fhir_field_name != variant_name_str &&
                        extract_type_suffix_from_field_name(&fhir_field_name).is_some();

                    if is_choice_type_enum {
                        quote! {
                            Self::#variant_name(value) => {
                                // Get the base evaluation result from the inner value
                                let mut result = value.to_evaluation_result();
                                // Add FHIR type information to preserve type for .ofType() operations
                                // For choice type enums, always use the type determined from the field name
                                result = match result {
                                    atrius_fhirpath_support::evaluation_result::EvaluationResult::String(s, _existing_type_info) => {
                                        // Always use the determined type from the field name for choice types
                                        let type_info = atrius_fhirpath_support::type_info::TypeInfoResult::new("FHIR", &#fhir_type);
                                        atrius_fhirpath_support::evaluation_result::EvaluationResult::String(s, Some(type_info))
                                    },
                                    atrius_fhirpath_support::evaluation_result::EvaluationResult::Integer(i, existing_type_info) => {
                                        let type_info = existing_type_info.unwrap_or_else(|| atrius_fhirpath_support::type_info::TypeInfoResult::new("FHIR", &#fhir_type));
                                        atrius_fhirpath_support::evaluation_result::EvaluationResult::Integer(i, Some(type_info))
                                    },
                                    atrius_fhirpath_support::evaluation_result::EvaluationResult::Decimal(d, existing_type_info) => {
                                        let type_info = existing_type_info.unwrap_or_else(|| atrius_fhirpath_support::type_info::TypeInfoResult::new("FHIR", &#fhir_type));
                                        atrius_fhirpath_support::evaluation_result::EvaluationResult::Decimal(d, Some(type_info))
                                    },
                                    atrius_fhirpath_support::evaluation_result::EvaluationResult::Boolean(b, existing_type_info) => {
                                        let type_info = existing_type_info.unwrap_or_else(|| atrius_fhirpath_support::type_info::TypeInfoResult::new("FHIR", &#fhir_type));
                                        atrius_fhirpath_support::evaluation_result::EvaluationResult::Boolean(b, Some(type_info))
                                    },
                                    atrius_fhirpath_support::evaluation_result::EvaluationResult::Object { map, type_info: existing_type_info } => {
                                        let type_info = existing_type_info.unwrap_or_else(|| atrius_fhirpath_support::type_info::TypeInfoResult::new("FHIR", &#fhir_type));
                                        atrius_fhirpath_support::evaluation_result::EvaluationResult::Object {
                                            map,
                                            type_info: Some(type_info),
                                        }
                                    },
                                    _ => result, // For other types, return as-is
                                };

                                // Wrap the result in an object with the field name as the key
                                let mut map = std::collections::HashMap::new();
                                map.insert(#fhir_field_name.to_string(), result);
                                atrius_fhirpath_support::evaluation_result::EvaluationResult::Object {
                                    map,
                                    type_info: None, // No type info for the wrapper object
                                }
                            }
                        }
                    } else {
                        quote! {
                            Self::#variant_name(value) => {
                                // Get the base evaluation result from the inner value
                                let mut result = value.to_evaluation_result();
                                // Add FHIR type information to preserve type for .ofType() operations
                                // For choice type enums, always use the type determined from the field name
                                result = match result {
                                    atrius_fhirpath_support::evaluation_result::EvaluationResult::String(s, _existing_type_info) => {
                                        // Always use the determined type from the field name for choice types
                                        let type_info = atrius_fhirpath_support::type_info::TypeInfoResult::new("FHIR", &#fhir_type);
                                        atrius_fhirpath_support::evaluation_result::EvaluationResult::String(s, Some(type_info))
                                    },
                                    atrius_fhirpath_support::evaluation_result::EvaluationResult::Integer(i, existing_type_info) => {
                                        let type_info = existing_type_info.unwrap_or_else(|| atrius_fhirpath_support::type_info::TypeInfoResult::new("FHIR", &#fhir_type));
                                        atrius_fhirpath_support::evaluation_result::EvaluationResult::Integer(i, Some(type_info))
                                    },
                                    atrius_fhirpath_support::evaluation_result::EvaluationResult::Decimal(d, existing_type_info) => {
                                        let type_info = existing_type_info.unwrap_or_else(|| atrius_fhirpath_support::type_info::TypeInfoResult::new("FHIR", &#fhir_type));
                                        atrius_fhirpath_support::evaluation_result::EvaluationResult::Decimal(d, Some(type_info))
                                    },
                                    atrius_fhirpath_support::evaluation_result::EvaluationResult::Boolean(b, existing_type_info) => {
                                        let type_info = existing_type_info.unwrap_or_else(|| atrius_fhirpath_support::type_info::TypeInfoResult::new("FHIR", &#fhir_type));
                                        atrius_fhirpath_support::evaluation_result::EvaluationResult::Boolean(b, Some(type_info))
                                    },
                                    atrius_fhirpath_support::evaluation_result::EvaluationResult::Object { map, type_info: existing_type_info } => {
                                        let type_info = existing_type_info.unwrap_or_else(|| atrius_fhirpath_support::type_info::TypeInfoResult::new("FHIR", &#fhir_type));
                                        atrius_fhirpath_support::evaluation_result::EvaluationResult::Object {
                                            map,
                                            type_info: Some(type_info),
                                        }
                                    },
                                    _ => result, // For other types, return as-is
                                };
                                result
                            }
                        }
                    }
                }
            }
            // For tuple or struct variants (uncommon in FHIR choice types or Resource enum),
            // the direct FHIRPath evaluation is less clear.
            // Returning Empty seems like a reasonable default.
            Fields::Unnamed(_) | Fields::Named(_) => {
                quote! {
                     // Match all fields but ignore them for now
                     Self::#variant_name { .. } => atrius_fhirpath_support::evaluation_result::EvaluationResult::Empty,
                 }
            }
        }
    });

    // Handle the case where the enum has no variants
    let body = if data.variants.is_empty() {
        // An empty enum cannot be instantiated, so this method is technically unreachable.
        // Return Empty as a safe default.
        quote! { atrius_fhirpath_support::evaluation_result::EvaluationResult::Empty }
    } else {
        // Generate the match statement for enums with variants
        quote! {
            match self {
                #(#match_arms)*
            }
        }
    };

    let into_evaluation_result_impl = quote! {
        impl #impl_generics atrius_fhirpath_support::traits::IntoEvaluationResult for #name #ty_generics #where_clause {
            fn to_evaluation_result(&self) -> atrius_fhirpath_support::evaluation_result::EvaluationResult{
                 #body // Use the generated body (either Empty or the match statement)
            }
        }
    };

    // Generate additional FhirResourceTypeProvider implementation for Resource enums
    if is_resource_enum {
        let resource_type_literals: Vec<_> = resource_type_names
            .iter()
            .map(|name| {
                quote! { #name }
            })
            .collect();

        // Generate resource_name method for Resource enum
        let resource_name_arms = data.variants.iter().map(|variant| {
            let variant_name = &variant.ident;
            let variant_name_str = variant_name.to_string();

            match &variant.fields {
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    // Newtype variant (expected for Resource enum)
                    quote! {
                        Self::#variant_name(_) => #variant_name_str,
                    }
                }
                _ => {
                    // For other field types, still return the variant name
                    quote! {
                        Self::#variant_name { .. } => #variant_name_str,
                    }
                }
            }
        });

        // Generate get_last_updated method for Resource enum
        let get_last_updated_arms = data.variants.iter().map(|variant| {
            let variant_name = &variant.ident;

            match &variant.fields {
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    quote! {
                        Self::#variant_name(resource) => {
                            resource.meta.as_ref()
                                .and_then(|m| m.last_updated.as_ref())
                                .and_then(|lu| {
                                    // Handle Element<PrecisionDateTime> - get the value and convert to chrono
                                    lu.value.as_ref().map(|precision_dt| {
                                        // PrecisionDateTime has a to_chrono_datetime() method
                                        precision_dt.to_chrono_datetime()
                                    })
                                })
                        }
                    }
                }
                _ => {
                    quote! {
                        Self::#variant_name { .. } => None,
                    }
                }
            }
        });

        quote! {
            #into_evaluation_result_impl

            impl #impl_generics #name #ty_generics #where_clause {
                /// Returns the resource type name as a string.
                /// This is equivalent to the resourceType field in FHIR JSON.
                pub fn resource_name(&self) -> &'static str {
                    match self {
                        #(#resource_name_arms)*
                    }
                }

                /// Returns the lastUpdated timestamp from the resource's metadata if available.
                pub fn get_last_updated(&self) -> Option<::chrono::DateTime<::chrono::Utc>> {
                    match self {
                        #(#get_last_updated_arms)*
                    }
                }
            }

            impl #impl_generics crate::fhir_version::FhirResourceTypeProvider for #name #ty_generics #where_clause {
                fn get_resource_type_names() -> Vec<&'static str> {
                    vec![#(#resource_type_literals),*]
                }
            }
        }
    } else {
        // Check if this is a choice element enum
        if let Some(base_name) = extract_choice_element_base_name(attrs) {
            // Extract possible field names from the enum variants
            let field_names: Vec<String> = data.variants.iter().filter_map(|variant| {
                // Look for the fhir_serde(rename = "...") attribute
                for attr in &variant.attrs {
                    if attr.path().is_ident("fhir_serde") {
                        if let Ok(list) = attr.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::token::Comma>::parse_terminated) {
                            for meta in list {
                                if let syn::Meta::NameValue(nv) = meta {
                                    if nv.path.is_ident("rename") {
                                        if let syn::Expr::Lit(expr_lit) = nv.value {
                                            if let syn::Lit::Str(lit_str) = expr_lit.lit {
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
            }).collect();

            let field_name_literals: Vec<_> =
                field_names.iter().map(|name| quote! { #name }).collect();

            quote! {
                #into_evaluation_result_impl

                impl #impl_generics atrius_fhirpath_support::traits::ChoiceElement for #name #ty_generics #where_clause {
                    fn base_name() -> &'static str {
                        #base_name
                    }

                    fn possible_field_names() -> Vec<&'static str> {
                        vec![#(#field_name_literals),*]
                    }
                }
            }
        } else {
            into_evaluation_result_impl
        }
    }
}