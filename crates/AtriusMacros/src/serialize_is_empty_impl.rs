//=============================================================================
// FhirSerde Implementation Generator Functions
//=============================================================================

use quote::{format_ident, quote};
use syn::{token, Data, Fields, Ident, Lit, Meta};
use syn::punctuated::Punctuated;

use crate::field_helpers::{get_effective_field_name, is_flattened};
use crate::type_helpers::get_element_info;

/// Generates the `serde::Serialize` implementation for FHIR types.
///
/// This function is the core of FHIR serialization code generation, producing
/// implementations that handle all the complex FHIR serialization patterns including
/// the extension pattern, choice types, and array serialization.
///
/// # Generated Code Patterns
///
/// ## For Structs:
/// - **Extension Pattern**: Separates primitive values and extension metadata
/// - **Array Handling**: Splits arrays into primitive and extension arrays
/// - **Field Counting**: Dynamically calculates field count for serializer
/// - **Conditional Serialization**: Only serializes non-empty fields
/// - **Flattening Support**: Handles `#[fhir_serde(flatten)]` attributes
///
/// ## For Enums:
/// - **Choice Type Serialization**: Single key-value pair output
/// - **Extension Support**: Handles element-containing enum variants
/// - **Variant Renaming**: Applies `#[fhir_serde(rename)]` attributes
///
/// # FHIR-Specific Serialization
///
/// The generated code handles several FHIR-specific patterns:
///
/// 1. **Element Extension Pattern**:
///    ```json
///    { "field": "value", "_field": {"id": "...", "extension": []} }
///    ```
///
/// 2. **Array Split Pattern**:
///    ```json
///    { "items": ["a", null, "c"], "_items": [null, {"id": "b"}, null] }
///    ```
///
/// 3. **Choice Type Pattern**:
///    ```json
///    { "valueString": "text" }  // not { "value": {"String": "text"} }
///    ```
///
/// # Arguments
///
/// * `data` - The parsed data structure (struct or enum)
/// * `name` - The type name being generated for
///
/// # Returns
///
/// TokenStream containing the complete `serialize` method implementation.
pub(crate) fn generate_serialize_impl(data: &Data, name: &Ident) -> proc_macro2::TokenStream {
    match *data {
        Data::Enum(ref data) => {
            // Handle enum serialization
            let mut match_arms = Vec::new();

            for variant in &data.variants {
                let variant_name = &variant.ident;

                // Get the rename attribute if present
                let mut rename = None;
                for attr in &variant.attrs {
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
                                rename = Some(lit_str.value());
                            }
                        }
                    }
                }

                // Use the rename value or the variant name as a string
                let variant_key = rename.unwrap_or_else(|| variant_name.to_string());

                // Handle different variant field types
                match &variant.fields {
                    Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                        // Newtype variant (e.g., String(String))
                        let field = fields.unnamed.first().unwrap();
                        let field_ty = &field.ty;

                        // Check if this is a primitive type that might have extensions
                        let (is_element, is_decimal_element, _, _) = get_element_info(field_ty);

                        if is_element || is_decimal_element {
                            // For Element types, we need special handling for the _fieldName pattern
                            let underscore_variant_key = format!("_{}", variant_key);

                            match_arms.push(quote! {
                                // Removed 'ref' from pattern
                                Self::#variant_name(value) => {
                                    // Check if the element has id or extension that needs to be serialized
                                    let has_extension = value.id.is_some() || value.extension.is_some();
                                    // Serialize the primitive value
                                    if value.value.is_some() {
                                        // Use serialize_entry for SerializeMap
                                        state.serialize_entry(#variant_key, &value.value)?;
                                    }
                                    // Serialize the extension part if present
                                    if has_extension {
                                        #[derive(serde::Serialize)]
                                        struct IdAndExtensionHelper<'a> {
                                            #[serde(skip_serializing_if = "Option::is_none")]
                                            id: &'a Option<std::string::String>,
                                            #[serde(skip_serializing_if = "Option::is_none")]
                                            extension: &'a Option<Vec<Extension>>,
                                        }
                                        let extension_part = IdAndExtensionHelper {
                                            id: &value.id,
                                            extension: &value.extension,
                                        };
                                        // Use serialize_entry for SerializeMap
                                        state.serialize_entry(#underscore_variant_key, &extension_part)?;
                                    }
                                    // Don't return Result here, just continue
                                }
                            });
                        } else {
                            // Regular newtype variant
                            match_arms.push(quote! {
                                // Removed 'ref' from pattern
                                Self::#variant_name(value) => {
                                    state.serialize_entry(#variant_key, value)?;
                                }
                            });
                        }
                    }
                    Fields::Unnamed(_) => {
                        // Tuple variant with multiple fields
                        match_arms.push(quote! {
                            Self::#variant_name(ref value) => {
                                state.serialize_entry(#variant_key, value)?;
                            }
                        });
                    }
                    Fields::Named(_fields) => {
                        // Struct variant
                        match_arms.push(quote! {
                            Self::#variant_name { .. } => {
                                state.serialize_entry(#variant_key, self)?;
                            }
                        });
                    }
                    Fields::Unit => {
                        // Unit variant
                        match_arms.push(quote! {
                            Self::#variant_name => {
                                state.serialize_entry(#variant_key, &())?;
                            }
                        });
                    }
                }
            }

            // Generate the enum serialization implementation
            quote! {
                // Count the number of fields to serialize (always 1 for an enum variant)
                let count = 1;

                // Import SerializeMap trait to access serialize_entry method
                use serde::ser::SerializeMap;

                // Create a serialization state
                let mut state = serializer.serialize_map(Some(count))?;

                // Match on self to determine which variant to serialize
                match self {
                    #(#match_arms)*
                }

                // End the map serialization
                state.end()
            }
        }
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    // Check if any fields have the flatten attribute - define this at the top level
                    let has_flattened_fields = fields.named.iter().any(is_flattened);

                    // Import SerializeMap trait if we have flattened fields
                    let import_serialize_map = if has_flattened_fields {
                        quote! { use serde::ser::SerializeMap; }
                    } else {
                        quote! { use serde::ser::SerializeStruct; }
                    };

                    let mut field_serializers = Vec::new();
                    let mut field_counts = Vec::new();
                    for field in fields.named.iter() {
                        let field_name_ident = field.ident.as_ref().unwrap(); // Keep original ident for access
                        let field_ty = &field.ty;
                        let effective_field_name_str = get_effective_field_name(field);
                        let underscore_field_name_str = format!("_{}", effective_field_name_str);

                        // Destructure the 4 return values from get_element_info
                        // We need is_element, is_decimal_element, is_option, is_vec here
                        let (is_element, is_decimal_element, is_option, is_vec) =
                            get_element_info(field_ty);

                        // Determine if it's an FHIR element type we need to handle specially
                        let is_fhir_element = is_element || is_decimal_element;

                        // Use field_name_ident for accessing the struct field
                        let field_access = quote! { self.#field_name_ident };

                        let extension_field_ident =
                            format_ident!("is_{}_extension", field_name_ident);

                        // Check if field has flatten attribute
                        let field_is_flattened = is_flattened(field);

                        let field_counting_code = if field_is_flattened {
                            // For flattened fields, we don't increment the count
                            // as they will be flattened into the parent object
                            quote! {
                                // No count increment for flattened fields
                                #[allow(unused_variables)]
                                let mut #extension_field_ident = false;
                            }
                        } else if is_option && !is_vec && is_fhir_element {
                            quote! {
                                let mut #extension_field_ident = false;
                                if let Some(field) = &#field_access {
                                    if field.value.is_some() {
                                        count += 1;
                                    }
                                    if field.id.is_some() || field.extension.is_some() {
                                        count += 1;
                                        #extension_field_ident = true;
                                    }
                                }
                            }
                        } else if is_vec && is_fhir_element {
                            // Handle Vec<Element> counting - count both primitive and extension arrays if present
                            let vec_access = if is_option {
                                quote! { #field_access.as_ref() }
                            } else {
                                quote! { Some(&#field_access) }
                            };
                            quote! {
                                if let Some(vec_value) = #vec_access {
                                    if !vec_value.is_empty() {
                                        // Count primitive array
                                        count += 1;
                                        // Count extension array if any elements have extensions
                                        if vec_value.iter().any(|element| element.id.is_some() || element.extension.is_some()) {
                                            count += 1;
                                        }
                                    }
                                }
                            }
                        } else if !is_vec && is_fhir_element {
                            quote! {
                                let mut #extension_field_ident = false;
                                if #field_access.value.is_some() {
                                    count += 1;
                                }
                                if #field_access.id.is_some() || #field_access.extension.is_some() {
                                    count += 1;
                                    #extension_field_ident = true;
                                }
                            }
                        } else {
                            // Only count non-Option fields or Some Option fields
                            if is_option {
                                quote! {
                                    if #field_access.is_some() {
                                        count += 1;
                                    }
                                }
                            } else {
                                quote! {
                                    count += 1;
                                }
                            }
                        };

                        // Check if field has flatten attribute
                        let field_is_flattened = is_flattened(field);

                        let field_serializing_code = if field_is_flattened {
                            // For flattened fields, use FlatMapSerializer
                            quote! {
                                // Use serde::Serialize::serialize with FlatMapSerializer
                                serde::Serialize::serialize(
                                    &#field_access,
                                    serde::__private::ser::FlatMapSerializer(&mut state)
                                )?;
                            }
                        } else if is_vec && is_fhir_element {
                            // Handles Vec<Element> or Option<Vec<Element>>
                            // Determine how to access the vector based on whether it's wrapped in Option
                            let vec_access = if is_option {
                                quote! { #field_access.as_ref() } // Access Option<Vec<T>> as Option<&Vec<T>>
                            } else {
                                quote! { Some(&#field_access) } // Treat Vec<T> as Some(&Vec<T>) for consistent handling
                            };

                            // Determine which serialization method to call (map vs struct)
                            let serialize_call = if has_flattened_fields {
                                quote! { state.serialize_entry }
                            } else {
                                quote! { state.serialize_field }
                            };

                            quote! {
                                // Handle Vec<Element> by splitting into primitive and extension arrays
                                if let Some(vec_value) = #vec_access { // Use the adjusted access logic
                                    if !vec_value.is_empty() {
                                        // Create primitive array
                                        let mut primitive_array = Vec::with_capacity(vec_value.len());
                                        // Create extension array
                                        let mut extension_array = Vec::with_capacity(vec_value.len());
                                        // Track if we need to include _fieldName
                                        let mut has_extensions = false;

                                        // Process each element
                                        for element in vec_value.iter() {
                                            // Add primitive value or null
                                            match &element.value {
                                                Some(value) => {
                                                    match serde_json::to_value(value) {
                                                        Ok(json_val) => primitive_array.push(json_val),
                                                        Err(e) => return Err(serde::ser::Error::custom(format!("Failed to serialize primitive value: {}", e))),
                                                    }
                                                },
                                                None => primitive_array.push(serde_json::Value::Null),
                                            }

                                            // Check if this element has id or extension
                                            if element.id.is_some() || element.extension.is_some() {
                                                has_extensions = true;
                                                // Use helper struct for consistent serialization of id/extension
                                                #[derive(serde::Serialize)]
                                                struct IdAndExtensionHelper<'a> {
                                                    #[serde(skip_serializing_if = "Option::is_none")]
                                                    id: &'a Option<std::string::String>,
                                                    #[serde(skip_serializing_if = "Option::is_none")]
                                                    extension: &'a Option<Vec<Extension>>,
                                                }
                                                let extension_part = IdAndExtensionHelper {
                                                    id: &element.id,
                                                    extension: &element.extension,
                                                };
                                                // Serialize the helper and push null if it serializes to null (e.g., both fields are None)
                                                match serde_json::to_value(&extension_part) {
                                                    Ok(json_val) if !json_val.is_null() => extension_array.push(json_val),
                                                    Ok(_) => extension_array.push(serde_json::Value::Null), // Push null if helper serialized to null
                                                    Err(e) => return Err(serde::ser::Error::custom(format!("Failed to serialize extension part: {}", e))),
                                                }
                                            } else {
                                                // No id or extension
                                                extension_array.push(serde_json::Value::Null);
                                            }
                                        }

                                        // Check if the primitive array contains any non-null values
                                        let should_serialize_primitive_array = primitive_array.iter().any(|v| !v.is_null());

                                        // Serialize primitive array only if it has non-null values
                                        if should_serialize_primitive_array {
                                            #serialize_call(&#effective_field_name_str, &primitive_array)?;
                                        }

                                        // Serialize extension array if needed, using the correct method
                                        if has_extensions {
                                            // Use the existing underscore_field_name_str variable which lives longer
                                            #serialize_call(&#underscore_field_name_str, &extension_array)?;
                                        }
                                    }
                                }
                            }
                        } else if is_option && !is_vec && is_fhir_element {
                            // Handles Option<Element> (but not Vec)
                            if has_flattened_fields {
                                // For SerializeMap
                                quote! {
                                    if let Some(field) = &#field_access {
                                        if let Some(value) = field.value.as_ref() {
                                            // Use serialize_entry for SerializeMap
                                            state.serialize_entry(&#effective_field_name_str, value)?;
                                        }
                                        if #extension_field_ident {
                                            #[derive(serde::Serialize)]
                                            struct IdAndExtensionHelper<'a> {
                                                #[serde(skip_serializing_if = "Option::is_none")]
                                                id: &'a Option<std::string::String>,
                                                #[serde(skip_serializing_if = "Option::is_none")]
                                                extension: &'a Option<Vec<Extension>>,
                                            }
                                            let extension_part = IdAndExtensionHelper {
                                                id: &field.id,
                                                extension: &field.extension,
                                            };
                                            // Use serialize_entry for SerializeMap
                                            // No format! here, #underscore_field_name_str is already a string literal
                                            state.serialize_entry(&#underscore_field_name_str, &extension_part)?;
                                        }
                                    }
                                }
                            } else {
                                // For SerializeStruct
                                quote! {
                                    if let Some(field) = &#field_access {
                                        if let Some(value) = field.value.as_ref() {
                                            // Use serialize_field for SerializeStruct
                                            state.serialize_field(&#effective_field_name_str, value)?;
                                        }
                                        if #extension_field_ident {
                                            #[derive(serde::Serialize)]
                                            struct IdAndExtensionHelper<'a> {
                                                #[serde(skip_serializing_if = "Option::is_none")]
                                                id: &'a Option<std::string::String>,
                                                #[serde(skip_serializing_if = "Option::is_none")]
                                                extension: &'a Option<Vec<Extension>>,
                                            }
                                            let extension_part = IdAndExtensionHelper {
                                                id: &field.id,
                                                extension: &field.extension,
                                            };
                                            // Use serialize_field for SerializeStruct
                                            // No format! here, #underscore_field_name_str is already a string literal
                                            state.serialize_field(&#underscore_field_name_str, &extension_part)?;
                                        }
                                    }
                                }
                            }
                        } else if !is_vec && is_fhir_element {
                            if has_flattened_fields {
                                // For SerializeMap
                                quote! {
                                    if let Some(value) = #field_access.value.as_ref() {
                                        // Use serialize_entry for SerializeMap
                                        state.serialize_entry(&#effective_field_name_str, value)?;
                                    }
                                    if #extension_field_ident {
                                        #[derive(serde::Serialize)]
                                        struct IdAndExtensionHelper<'a> {
                                            #[serde(skip_serializing_if = "Option::is_none")]
                                            id: &'a Option<std::string::String>,
                                            #[serde(skip_serializing_if = "Option::is_none")]
                                            extension: &'a Option<Vec<Extension>>,
                                        }
                                        let extension_part = IdAndExtensionHelper {
                                            id: &#field_access.id,
                                            extension: &#field_access.extension,
                                        };
                                        // Use serialize_entry for SerializeMap
                                        state.serialize_entry(#underscore_field_name_str, &extension_part)?;
                                    }
                                }
                            } else {
                                // For SerializeStruct
                                quote! {
                                    if let Some(value) = #field_access.value.as_ref() {
                                        // Use serialize_field for SerializeStruct
                                        state.serialize_field(&#effective_field_name_str, value)?;
                                    }
                                    if #extension_field_ident {
                                        #[derive(serde::Serialize)]
                                        struct IdAndExtensionHelper<'a> {
                                            #[serde(skip_serializing_if = "Option::is_none")]
                                            id: &'a Option<std::string::String>,
                                            #[serde(skip_serializing_if = "Option::is_none")]
                                            extension: &'a Option<Vec<Extension>>,
                                        }
                                        let extension_part = IdAndExtensionHelper {
                                            id: &#field_access.id,
                                            extension: &#field_access.extension,
                                        };
                                        // Use serialize_field for SerializeStruct
                                        // No format! here, #underscore_field_name_str is already a string literal
                                        state.serialize_field(&#underscore_field_name_str, &extension_part)?;
                                    }
                                }
                            }
                        } else if is_option {
                            // Skip serializing if the Option is None
                            if has_flattened_fields {
                                // For SerializeMap
                                quote! {
                                    if let Some(value) = &#field_access {
                                        // Use serialize_entry for SerializeMap
                                        state.serialize_entry(&#effective_field_name_str, value)?;
                                    }
                                }
                            } else {
                                // For SerializeStruct
                                quote! {
                                    if let Some(value) = &#field_access {
                                        // Use serialize_field for SerializeStruct
                                        state.serialize_field(&#effective_field_name_str, value)?;
                                    }
                                }
                            }
                        } else if is_vec {
                            // Regular Vec handling (not Element)
                            if has_flattened_fields {
                                // For SerializeMap
                                quote! {
                                    if !#field_access.is_empty() {
                                        // Use serialize_entry for SerializeMap
                                        state.serialize_entry(&#effective_field_name_str, &#field_access)?;
                                    }
                                }
                            } else {
                                // For SerializeStruct
                                quote! {
                                    if !#field_access.is_empty() {
                                        // Use serialize_field for SerializeStruct
                                        state.serialize_field(&#effective_field_name_str, &#field_access)?;
                                    }
                                }
                            }
                        } else {
                            // For non-Option types, check if it's a struct with all None/null fields
                            if has_flattened_fields {
                                // For SerializeMap
                                quote! {
                                    if !#field_access.is_empty() {
                                        // Use serialize_entry for SerializeMap
                                        state.serialize_entry(&#effective_field_name_str, &#field_access)?;
                                    }
                                }
                            } else {
                                // For SerializeStruct
                                quote! {
                                    if !#field_access.is_empty() {
                                        // Use serialize_field for SerializeStruct
                                        state.serialize_field(&#effective_field_name_str, &#field_access)?;
                                    }
                                }
                            }
                        };

                        field_counts.push(field_counting_code);
                        field_serializers.push(field_serializing_code);
                    }
                    // Use the has_flattened_fields variable defined at the top of the function
                    if has_flattened_fields {
                        // If we have flattened fields, use serialize_map instead of serialize_struct
                        quote! {
                            let mut count = 0;
                            #(#field_counts)*
                            #import_serialize_map
                            let mut state = serializer.serialize_map(Some(count))?;
                            #(#field_serializers)*
                            state.end()
                        }
                    } else {
                        // If no flattened fields, use serialize_struct as before
                        quote! {
                            let mut count = 0;
                            #(#field_counts)*
                            #import_serialize_map
                            let mut state = serializer.serialize_struct(stringify!(#name), count)?;
                            #(#field_serializers)*
                            state.end()
                        }
                    }
                }
                Fields::Unnamed(_) => panic!("Tuple structs not supported by FhirSerde"),
                Fields::Unit => panic!("Unit structs not supported by FhirSerde"),
            }
        }
        Data::Union(_) => panic!("Enums and Unions not supported by FhirSerde"),
    }
}

pub(crate) fn generate_is_empty_impl(
    data: &Data,
    name: &Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> Option<proc_macro2::TokenStream> {
    match data {
        Data::Struct(data_struct) => {
            let fields = match &data_struct.fields {
                Fields::Named(named) => &named.named,
                _ => return None,
            };

            let mut field_checks = Vec::new();

            for field in fields {
                let field_name_ident = field.ident.as_ref().unwrap();
                let (is_element, is_decimal_element, is_option, is_vec) =
                    get_element_info(&field.ty);
                let is_fhir_element = is_element || is_decimal_element;
                let field_is_flattened = is_flattened(field);

                let field_check = if field_is_flattened {
                    if is_option {
                        let tmp = format_ident!("__fhir_flatten_opt_{}", field_name_ident);
                        quote! {
                            self.#field_name_ident
                                .as_ref()
                                .map_or(true, |#tmp| #tmp.is_empty())
                        }
                    } else if is_vec {
                        let tmp = format_ident!("__fhir_flatten_vec_{}", field_name_ident);
                        quote! {
                            self.#field_name_ident.iter().all(|#tmp| #tmp.is_empty())
                        }
                    } else {
                        quote! { self.#field_name_ident.is_empty() }
                    }
                } else if is_option && !is_vec && is_fhir_element {
                    let tmp = format_ident!("__fhir_element_opt_{}", field_name_ident);
                    quote! {
                        self.#field_name_ident
                            .as_ref()
                            .map_or(true, |#tmp| {
                                #tmp.value.is_none()
                                    && #tmp.id.is_none()
                                    && #tmp.extension.is_none()
                            })
                    }
                } else if is_vec && is_fhir_element {
                    let vec_ident = format_ident!("__fhir_vec_ref_{}", field_name_ident);
                    let element_ident = format_ident!("__fhir_vec_elem_{}", field_name_ident);
                    let vec_access = if is_option {
                        quote! { self.#field_name_ident.as_ref() }
                    } else {
                        quote! { Some(&self.#field_name_ident) }
                    };
                    quote! {
                        #vec_access.map_or(true, |#vec_ident| {
                            #vec_ident.iter().all(|#element_ident| {
                                #element_ident.value.is_none()
                                    && #element_ident.id.is_none()
                                    && #element_ident.extension.is_none()
                            })
                        })
                    }
                } else if !is_vec && is_fhir_element {
                    quote! {
                        self.#field_name_ident.value.is_none()
                            && self.#field_name_ident.id.is_none()
                            && self.#field_name_ident.extension.is_none()
                    }
                } else if is_option {
                    quote! { self.#field_name_ident.is_none() }
                } else {
                    quote! { self.#field_name_ident.is_empty() }
                };

                field_checks.push(field_check);
            }

            let body = if field_checks.is_empty() {
                quote! { true }
            } else {
                quote! {
                    true #(&& #field_checks)*
                }
            };

            Some(quote! {
                impl #impl_generics #name #ty_generics #where_clause {
                    #[doc(hidden)]
                    pub fn is_empty(&self) -> bool {
                        #body
                    }
                }
            })
        }
        Data::Enum(_) => Some(quote! {
            impl #impl_generics #name #ty_generics #where_clause {
                #[doc(hidden)]
                pub fn is_empty(&self) -> bool {
                    false
                }
            }
        }),
        Data::Union(_) => None,
    }
}
