

// --- Internal Visitor for Element Object Deserialization ---

use std::marker::PhantomData;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeStruct;
use atrius_fhirpath_support::evaluation_result::EvaluationResult;
use atrius_fhirpath_support::traits::IntoEvaluationResult;
use atrius_fhirpath_support::type_info::TypeInfoResult;
use crate::date_time::PrecisionInstant;

/// Internal visitor struct for deserializing Element objects from JSON maps.
///
/// This visitor handles the complex deserialization logic for Element<V, E> when
/// the JSON input is an object containing id, extension, and value fields.
struct ElementObjectVisitor<V, E>(PhantomData<(V, E)>);

impl<'de, V, E> Visitor<'de> for ElementObjectVisitor<V, E>
where
    V: Deserialize<'de>,
    E: Deserialize<'de>,
{
    type Value = Element<V, E>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an Element object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut id: Option<String> = None;
        let mut extension: Option<Vec<E>> = None;
        let mut value: Option<V> = None;

        // Manually deserialize fields from the map
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "id" => {
                    if id.is_some() {
                        return Err(de::Error::duplicate_field("id"));
                    }
                    id = Some(map.next_value()?);
                }
                "extension" => {
                    if extension.is_some() {
                        return Err(de::Error::duplicate_field("extension"));
                    }
                    extension = Some(map.next_value()?);
                }
                "value" => {
                    if value.is_some() {
                        return Err(de::Error::duplicate_field("value"));
                    }
                    // Deserialize directly into Option<V>
                    value = Some(map.next_value()?);
                }
                // Ignore any unknown fields encountered
                _ => {
                    let _ = map.next_value::<de::IgnoredAny>()?;
                }
            }
        }

        Ok(Element {
            id,
            extension,
            value,
        })
    }
}
// Generic element container supporting FHIR's extension mechanism.
///
/// In FHIR, most primitive elements can be extended with additional metadata
/// through the `id` and `extension` fields. This container type provides
/// the infrastructure to support this pattern across all FHIR data types.
///
/// # Type Parameters
///
/// * `V` - The value type (e.g., `String`, `i32`, `PreciseDecimal`)
/// * `E` - The extension type (typically the generated `Extension` struct)
///
/// # FHIR Element Structure
///
/// FHIR elements can appear in three forms:
/// 1. **Primitive value**: Just the value itself (e.g., `"text"`, `42`)
/// 2. **Extended primitive**: An object with `value`, `id`, and/or `extension` fields
/// 3. **Extension-only**: An object with just `id` and/or `extension` (no value)
///
/// # Examples
///
/// ```rust
/// use helios_fhir::{Element, r4::Extension};
///
/// // Simple primitive value
/// let simple: Element<String, Extension> = Element {
///     value: Some("Hello World".to_string()),
///     id: None,
///     extension: None,
/// };
///
/// // Extended primitive with ID
/// let with_id: Element<String, Extension> = Element {
///     value: Some("Hello World".to_string()),
///     id: Some("text-element-1".to_string()),
///     extension: None,
/// };
///
/// // Extension-only element (no value)
/// let extension_only: Element<String, Extension> = Element {
///     value: None,
///     id: Some("disabled-element".to_string()),
///     extension: Some(vec![/* extensions */]),
/// };
/// ```
///
/// # Serialization Behavior
///
/// - If only `value` is present: serializes as the primitive value directly
/// - If `id` or `extension` are present: serializes as an object with all fields
/// - If everything is `None`: serializes as `null`
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Element<V, E> {
    /// Optional element identifier for referencing within the resource
    pub id: Option<String>,
    /// Optional extensions providing additional metadata
    pub extension: Option<Vec<E>>,
    /// The actual primitive value
    pub value: Option<V>,
}

impl<V, E> Element<V, E> {
    /// Returns `true` if no value, id, or extensions are present.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.value.is_none() && self.id.is_none() && self.extension.is_none()
    }
}
// Custom Deserialize for Element<V, E>
// Remove PartialEq/Eq bounds for V and E as they are not needed for deserialization itself
impl<'de, V, E> Deserialize<'de> for Element<V, E>
where
    V: Deserialize<'de> + 'static, // Added 'static for TypeId comparisons
    E: Deserialize<'de>,           // Removed PartialEq
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Use the AnyValueVisitor approach to handle different JSON input types
        struct AnyValueVisitor<V, E>(PhantomData<(V, E)>);

        impl<'de, V, E> Visitor<'de> for AnyValueVisitor<V, E>
        where
            V: Deserialize<'de> + 'static,
            E: Deserialize<'de>,
        {
            type Value = Element<V, E>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter
                    .write_str("a primitive value (string, number, boolean), an object, or null")
            }

            // Handle primitive types by attempting to deserialize V and wrapping it
            fn visit_bool<Er>(self, v: bool) -> Result<Self::Value, Er>
            where
                Er: de::Error,
            {
                V::deserialize(de::value::BoolDeserializer::new(v)).map(|value| Element {
                    id: None,
                    extension: None,
                    value: Some(value),
                })
            }
            fn visit_i64<Er>(self, v: i64) -> Result<Self::Value, Er>
            where
                Er: de::Error,
            {
                V::deserialize(de::value::I64Deserializer::new(v)).map(|value| Element {
                    id: None,
                    extension: None,
                    value: Some(value),
                })
            }
            fn visit_u64<Er>(self, v: u64) -> Result<Self::Value, Er>
            where
                Er: de::Error,
            {
                V::deserialize(de::value::U64Deserializer::new(v)).map(|value| Element {
                    id: None,
                    extension: None,
                    value: Some(value),
                })
            }
            fn visit_f64<Er>(self, v: f64) -> Result<Self::Value, Er>
            where
                Er: de::Error,
            {
                V::deserialize(de::value::F64Deserializer::new(v)).map(|value| Element {
                    id: None,
                    extension: None,
                    value: Some(value),
                })
            }
            fn visit_str<Er>(self, v: &str) -> Result<Self::Value, Er>
            where
                Er: de::Error,
            {
                use std::any::TypeId;

                // Try to handle numeric strings for integer types
                if TypeId::of::<V>() == TypeId::of::<i64>() {
                    if let Ok(int_val) = v.parse::<i64>() {
                        return V::deserialize(de::value::I64Deserializer::new(int_val)).map(
                            |value| Element {
                                id: None,
                                extension: None,
                                value: Some(value),
                            },
                        );
                    }
                } else if TypeId::of::<V>() == TypeId::of::<i32>() {
                    if let Ok(int_val) = v.parse::<i32>() {
                        return V::deserialize(de::value::I32Deserializer::new(int_val)).map(
                            |value| Element {
                                id: None,
                                extension: None,
                                value: Some(value),
                            },
                        );
                    }
                } else if TypeId::of::<V>() == TypeId::of::<u64>() {
                    if let Ok(int_val) = v.parse::<u64>() {
                        return V::deserialize(de::value::U64Deserializer::new(int_val)).map(
                            |value| Element {
                                id: None,
                                extension: None,
                                value: Some(value),
                            },
                        );
                    }
                } else if TypeId::of::<V>() == TypeId::of::<u32>() {
                    if let Ok(int_val) = v.parse::<u32>() {
                        return V::deserialize(de::value::U32Deserializer::new(int_val)).map(
                            |value| Element {
                                id: None,
                                extension: None,
                                value: Some(value),
                            },
                        );
                    }
                }

                // Fall back to normal string deserialization
                V::deserialize(de::value::StrDeserializer::new(v)).map(|value| Element {
                    id: None,
                    extension: None,
                    value: Some(value),
                })
            }
            fn visit_string<Er>(self, v: String) -> Result<Self::Value, Er>
            where
                Er: de::Error,
            {
                use std::any::TypeId;

                // Try to handle numeric strings for integer types
                if TypeId::of::<V>() == TypeId::of::<i64>() {
                    if let Ok(int_val) = v.parse::<i64>() {
                        return V::deserialize(de::value::I64Deserializer::new(int_val)).map(
                            |value| Element {
                                id: None,
                                extension: None,
                                value: Some(value),
                            },
                        );
                    }
                } else if TypeId::of::<V>() == TypeId::of::<i32>() {
                    if let Ok(int_val) = v.parse::<i32>() {
                        return V::deserialize(de::value::I32Deserializer::new(int_val)).map(
                            |value| Element {
                                id: None,
                                extension: None,
                                value: Some(value),
                            },
                        );
                    }
                } else if TypeId::of::<V>() == TypeId::of::<u64>() {
                    if let Ok(int_val) = v.parse::<u64>() {
                        return V::deserialize(de::value::U64Deserializer::new(int_val)).map(
                            |value| Element {
                                id: None,
                                extension: None,
                                value: Some(value),
                            },
                        );
                    }
                } else if TypeId::of::<V>() == TypeId::of::<u32>() {
                    if let Ok(int_val) = v.parse::<u32>() {
                        return V::deserialize(de::value::U32Deserializer::new(int_val)).map(
                            |value| Element {
                                id: None,
                                extension: None,
                                value: Some(value),
                            },
                        );
                    }
                }

                // Fall back to normal string deserialization
                V::deserialize(de::value::StringDeserializer::new(v.clone())).map(|value| Element {
                    // Clone v for error message
                    id: None,
                    extension: None,
                    value: Some(value),
                })
            }
            fn visit_borrowed_str<Er>(self, v: &'de str) -> Result<Self::Value, Er>
            where
                Er: de::Error,
            {
                use std::any::TypeId;

                // Try to handle numeric strings for integer types
                if TypeId::of::<V>() == TypeId::of::<i64>() {
                    if let Ok(int_val) = v.parse::<i64>() {
                        return V::deserialize(de::value::I64Deserializer::new(int_val)).map(
                            |value| Element {
                                id: None,
                                extension: None,
                                value: Some(value),
                            },
                        );
                    }
                } else if TypeId::of::<V>() == TypeId::of::<i32>() {
                    if let Ok(int_val) = v.parse::<i32>() {
                        return V::deserialize(de::value::I32Deserializer::new(int_val)).map(
                            |value| Element {
                                id: None,
                                extension: None,
                                value: Some(value),
                            },
                        );
                    }
                } else if TypeId::of::<V>() == TypeId::of::<u64>() {
                    if let Ok(int_val) = v.parse::<u64>() {
                        return V::deserialize(de::value::U64Deserializer::new(int_val)).map(
                            |value| Element {
                                id: None,
                                extension: None,
                                value: Some(value),
                            },
                        );
                    }
                } else if TypeId::of::<V>() == TypeId::of::<u32>() {
                    if let Ok(int_val) = v.parse::<u32>() {
                        return V::deserialize(de::value::U32Deserializer::new(int_val)).map(
                            |value| Element {
                                id: None,
                                extension: None,
                                value: Some(value),
                            },
                        );
                    }
                }
                // Fall back to normal string deserialization
                V::deserialize(de::value::BorrowedStrDeserializer::new(v)).map(|value| Element {
                    id: None,
                    extension: None,
                    value: Some(value),
                })
            }
            fn visit_bytes<Er>(self, v: &[u8]) -> Result<Self::Value, Er>
            where
                Er: de::Error,
            {
                V::deserialize(de::value::BytesDeserializer::new(v)).map(|value| Element {
                    id: None,
                    extension: None,
                    value: Some(value),
                })
            }
            fn visit_byte_buf<Er>(self, v: Vec<u8>) -> Result<Self::Value, Er>
            where
                Er: de::Error,
            {
                // Use BytesDeserializer with a slice reference &v
                V::deserialize(de::value::BytesDeserializer::new(&v)).map(|value| Element {
                    id: None,
                    extension: None,
                    value: Some(value),
                })
            }
            // Handle null
            fn visit_none<Er>(self) -> Result<Self::Value, Er>
            where
                Er: de::Error,
            {
                Ok(Element {
                    id: None,
                    extension: None,
                    value: None,
                })
            }
            fn visit_unit<Er>(self) -> Result<Self::Value, Er>
            where
                Er: de::Error,
            {
                Ok(Element {
                    id: None,
                    extension: None,
                    value: None,
                })
            }

            // Handle Option<T> by visiting Some
            fn visit_some<De>(self, deserializer: De) -> Result<Self::Value, De::Error>
            where
                De: Deserializer<'de>,
            {
                // Re-dispatch to deserialize_any to handle the inner type correctly
                deserializer.deserialize_any(self)
            }
            // Handle object
            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                // Deserialize the map using ElementObjectVisitor
                // Need to create a deserializer from the map access
                let map_deserializer = de::value::MapAccessDeserializer::new(map);
                map_deserializer.deserialize_map(ElementObjectVisitor(PhantomData))
            }

            // We don't expect sequences for a single Element
            fn visit_seq<A>(self, _seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                Err(de::Error::invalid_type(de::Unexpected::Seq, &self))
            }
        }

        // Start deserialization using the visitor
        deserializer.deserialize_any(AnyValueVisitor(PhantomData))
    }
}
// Custom Serialize for Element<V, E>
// Remove PartialEq/Eq bounds for V and E as they are not needed for serialization itself
impl<V, E> Serialize for Element<V, E>
where
    V: Serialize, // Removed PartialEq + Eq
    E: Serialize, // Removed PartialEq
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // If id and extension are None, serialize value directly (or null)
        if self.id.is_none() && self.extension.is_none() {
            match &self.value {
                Some(val) => val.serialize(serializer),
                None => serializer.serialize_none(),
            }
        } else {
            // Otherwise, serialize as an object containing id, extension, value if present
            let mut len = 0;
            if self.id.is_some() {
                len += 1;
            }
            if self.extension.is_some() {
                len += 1;
            }
            if self.value.is_some() {
                len += 1;
            }

            let mut state = serializer.serialize_struct("Element", len)?;
            if let Some(id) = &self.id {
                state.serialize_field("id", id)?;
            }
            if let Some(extension) = &self.extension {
                state.serialize_field("extension", extension)?;
            }
        // Restore value serialization for direct Element serialization
                    if let Some(value) = &self.value {
                        state.serialize_field("value", value)?;
                    }
                    state.end()
                }
            }
        }
// For Element<V, E> - Returns Object with id, extension, value if present
impl<V, E> IntoEvaluationResult for Element<V, E>
where
    V: IntoEvaluationResult + Clone + 'static,
    E: IntoEvaluationResult + Clone,
{
    fn to_evaluation_result(&self) -> EvaluationResult {
        use std::any::TypeId;

        // Prioritize returning the primitive value if it exists
        if let Some(v) = &self.value {
            let result = v.to_evaluation_result();
            // For primitive values, we need to preserve FHIR type information
            return match result {
                EvaluationResult::Boolean(b, _) => {
                    // Return FHIR boolean
                    EvaluationResult::fhir_boolean(b)
                }
                EvaluationResult::Integer(i, _) => {
                    // Return FHIR integer
                    EvaluationResult::fhir_integer(i)
                }
                #[cfg(not(any(feature = "R4", feature = "R4B")))]
                EvaluationResult::Integer64(i, _) => {
                    // Return FHIR integer64 (R5 and above)
                    EvaluationResult::fhir_integer64(i)
                }
                EvaluationResult::String(s, _) => {
                    // Determine the FHIR type name based on V's type
                    let fhir_type_name = if TypeId::of::<V>() == TypeId::of::<String>() {
                        // For strings, we need more context to determine the exact FHIR type
                        // Default to "string" but this could be date, dateTime, etc.
                        "string"
                    } else {
                        // Default fallback
                        "string"
                    };
                    EvaluationResult::fhir_string(s, fhir_type_name)
                }
                EvaluationResult::DateTime(dt, type_info) => {
                    // Check if V is PrecisionInstant - if so, this is an instant
                    if TypeId::of::<V>() == TypeId::of::<PrecisionInstant>() {
                        // Return as FHIR instant
                        EvaluationResult::DateTime(dt, Some(TypeInfoResult::new("FHIR", "instant")))
                    } else {
                        // Preserve original type info for PrecisionDateTime
                        EvaluationResult::DateTime(dt, type_info)
                    }
                }
                _ => result, // For other types, return as-is
            };
        } else if self.id.is_some() || self.extension.is_some() {
            // If value is None, but id or extension exist, return an Object with those
            let mut map = std::collections::HashMap::new();
            if let Some(id) = &self.id {
                map.insert("id".to_string(), EvaluationResult::string(id.clone()));
            }
            if let Some(ext) = &self.extension {
                let ext_collection: Vec<EvaluationResult> =
                    ext.iter().map(|e| e.to_evaluation_result()).collect();
                if !ext_collection.is_empty() {
                    map.insert(
                        "extension".to_string(),
                        EvaluationResult::collection(ext_collection),
                    );
                }
            }
            // Only return Object if map is not empty (i.e., id or extension was actually present)
            if !map.is_empty() {
                return EvaluationResult::typed_object(map, "FHIR", "Element");
            }
        }
        // If value, id, and extension are all None, return Empty
        EvaluationResult::Empty
    }
}