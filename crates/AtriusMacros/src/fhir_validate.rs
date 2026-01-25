//! # `#[derive(FhirValidate)]` macro
//!
//! This derive macro generates a *resource-specific validator* that enforces:
//!
//! 1. **FHIR invariants** (StructureDefinition.constraint) expressed as FHIRPath boolean expressions.
//! 2. **ValueSet bindings** declared via `#[fhir_binding(...)]`, using:
//!    - a fast **local membership** check against generated ValueSet code (when available)
//!    - an optional **remote terminology fallback** via `FhirPathEngine::validate_code_in_valueset()`
//! 3. **Recursive validation** of nested complex types (but not primitives / leaf types).
//! 4. **Stable instance paths** (e.g. `contact[0].telecom[1]....`) and **de-duplication** of issues.
//!
//! ## Runtime contract
//! The generated impl targets `atrius_fhirpath_support::validate::{FhirValidate, FhirPathEngine}`.
//! At runtime, the caller constructs a `FhirPathEngine` (e.g. `AtriusFhirPathEngine`) and invokes:
//!
//! ```ignore
//! let issues = resource.validate_with_engine(&engine);
//! ```
//!
//! The engine must be capable of evaluating FHIRPath booleans via `eval_bool()`. If bindings require
//! remote terminology validation, the engine may also implement `validate_code_in_valueset()`.
//!
//! ## Invariants (constraints)
//! Invariants may appear:
//! - **On the type** (struct-level attributes): constraints whose `path` refers to the resource root.
//! - **On fields**: constraints whose `path` refers to the element being constrained.
//!
//! The macro converts `#[fhir_invariant(...)]` attributes into `&'static [Invariant]` values and
//! emits runtime code that evaluates each invariant via `engine.eval_bool()` against an
//! `EvaluationResult` focus node for that value.
//!
//! ## ValueSet bindings
//! Binding declarations are attached to fields using:
//!
//! ```ignore
//! #[fhir_binding(strength="required", valueset="http://hl7.org/fhir/ValueSet/administrative-gender")]
//! ```
//!
//! The macro emits a binding check that follows this policy:
//!
//! 1. Attempt **local membership** via generated binding ops (`binding_ops_by_url(url)`).
//! 2. If local membership fails:
//!    - If the ValueSet indicates non-local rules or is not locally-enumerated,
//!      attempt **remote** `$validate-code` via `engine.validate_code_in_valueset(url, system, code)`.
//!    - Otherwise treat local failure as authoritative.
//! 3. Remote validation returns `Option<bool>`:
//!    - `Some(true)`  => confirmed member (no issue)
//!    - `Some(false)` => confirmed non-member (emit violation using binding strength)
//!    - `None`        => unknown (emit warning: "could not be verified")
//!
//! This design keeps FHIRPath evaluation (pure expressions) separate from terminology lookups.
//!
//! ## Containers and recursion
//! Fields may be wrapped in `Option<T>`, `Vec<T>`, `Option<Vec<T>>`, `Box<T>`, `Option<Box<T>>`.
//! The macro emits the correct traversal code for each container, so invariants, bindings, and recursion
//! operate on `&T` without cloning.
//!
//! Recursive validation is applied only to *complex* types. Primitive wrappers and leaf types
//! (including `Element<...>` and `...::primitives::...`) are treated as leaves.
//!
//! ## Instance paths and de-duplication
//! The generated validator builds an `instance_path` as it traverses fields/indices and then
//! rewrites child issue paths to be absolute (e.g. `contact[0].name.family`).
//! Issues are de-duplicated with a signature `(key|path|instance_path|expression)` to avoid repeats
//! from overlapping invariants, recursion, or repeated traversal.
//!
//! ## Supported binding shapes
//! Binding checks are generated only for the standard coded shapes:
//! - `Code`
//! - `Coding`
//! - `CodeableConcept`
//! and their container-wrapped variants (Option/Vec/etc).
//!
//! Any other field type with `#[fhir_binding]` is currently ignored (no binding check emitted).
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Attribute, Data, DeriveInput, Fields, LitStr, Meta, MetaList, MetaNameValue, Type, Expr, Lit};
use syn::ext::IdentExt;
use syn::parse::Parser;


struct InvariantLit {
    key: Option<LitStr>,
    severity: Option<LitStr>,
    human: Option<LitStr>,
    expr: Option<LitStr>,
    path: Option<LitStr>,
}

impl InvariantLit {
    fn new() -> Self {
        Self {
            key: None,
            severity: None,
            human: None,
            expr: None,
            path: None,
        }
    }
}
// Parses #[fhir_invariant(key="...", severity="...", human="...", expr="...", path="...")]
// into a strongly typed structure used to generate &'static [Invariant] in expanded code.
// We parse tokens manually to accept a comma-separated list of `MetaNameValue` pairs.
//
// NOTE: We require string literals so generated code can store &'static str references.
fn parse_fhir_invariant(attr: &Attribute) -> syn::Result<InvariantLit> {
    // Better approach:
    let meta = attr.meta.clone();
    let Meta::List(MetaList { tokens, .. }) = meta else {
        return Err(syn::Error::new(attr.span(), "expected #[fhir_invariant(...)]"));
    };

    // Parse tokens into a comma-separated list of MetaNameValue
    let parser = syn::punctuated::Punctuated::<MetaNameValue, syn::Token![,]>::parse_terminated;
    let list = parser.parse2(tokens)?;

    let mut out = InvariantLit::new();

    for nv in list {
        let span = nv.span();
        let ident = nv
            .path
            .get_ident()
            .map(|i| i.to_string())
            .unwrap_or_default();

        let lit = match nv.value {
            syn::Expr::Lit(expr_lit) => match expr_lit.lit {
                syn::Lit::Str(s) => s,
                _ => return Err(syn::Error::new(span, "expected string literal")),
            },
            _ => return Err(syn::Error::new(span, "expected string literal")),
        };

        match ident.as_str() {
            "key" => out.key = Some(lit),
            "severity" => out.severity = Some(lit),
            "human" => out.human = Some(lit),
            "expr" => out.expr = Some(lit),
            "path" => out.path = Some(lit),
            _ => {}
        }
    }

    let missing = |name: &str| syn::Error::new(attr.span(), format!("missing required fhir_invariant field: {name}"));

    Ok(InvariantLit {
        key: Some(out.key.ok_or_else(|| missing("key"))?),
        severity: Some(out.severity.ok_or_else(|| missing("severity"))?),
        human: Some(out.human.ok_or_else(|| missing("human"))?),
        expr: Some(out.expr.ok_or_else(|| missing("expr"))?),
        path: Some(out.path.ok_or_else(|| missing("path"))?),
    })
}

struct BindingLit {
    strength: Option<LitStr>,
    valueset: Option<LitStr>,
}
// Parses #[fhir_binding(strength="required|extensible|preferred|example", valueset="...url...")]
// into BindingLit used to emit binding validation code.
//
// The macro *does not* validate the valueset URL at compile-time; it is treated as a string key
// passed into generated runtime dispatch (binding_ops_by_url / terminology fallback).
fn parse_fhir_binding(attr: &Attribute) -> syn::Result<BindingLit> {
    let meta = attr.meta.clone();
    let Meta::List(MetaList { tokens, .. }) = meta else {
        return Err(syn::Error::new(attr.span(), "expected #[fhir_binding(...)]"));
    };

    let parser = syn::punctuated::Punctuated::<MetaNameValue, syn::Token![,]>::parse_terminated;
    let list = parser.parse2(tokens)?;

    let mut out = BindingLit {
        strength: None,
        valueset: None,
    };

    for nv in list {
        let span = nv.span();
        let ident = nv
            .path
            .get_ident()
            .map(|i| i.to_string())
            .unwrap_or_default();

        let lit = match nv.value {
            Expr::Lit(expr_lit) => match expr_lit.lit {
                Lit::Str(s) => s,
                _ => return Err(syn::Error::new(span, "expected string literal")),
            },
            _ => return Err(syn::Error::new(span, "expected string literal")),
        };

        match ident.as_str() {
            "strength" => out.strength = Some(lit),
            "valueset" => out.valueset = Some(lit),
            _ => {}
        }
    }

    let missing = |name: &str| syn::Error::new(attr.span(), format!("missing required fhir_binding field: {name}"));

    Ok(BindingLit {
        strength: Some(out.strength.ok_or_else(|| missing("strength"))?),
        valueset: Some(out.valueset.ok_or_else(|| missing("valueset"))?),
    })
}

// Helper to sanitize a syn::Ident for use in a const identifier
fn sanitize_ident_for_const(ident: &syn::Ident) -> String {
    // syn::Ident may be raw like r#type / r#use
    let s = ident.unraw().to_string();
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    // const names must not start with a digit
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

// Detect Option<T>, Vec<T>, Option<Vec<T>>, Box<T>, Option<Box<T>>
enum ContainerKind {
    Plain,
    Option,
    Vec,
    OptionVec,
    Box,
    OptionBox,
}
// Many FHIR fields are wrapped in containers. We generate traversal code for each wrapper
// to validate values by reference without cloning.
//
// ContainerKind drives the expanded code shape:
// - Plain:      validate a single `&T`
// - Option:     validate `&T` if present
// - Vec:        validate each `&T` with an index-based instance path
// - OptionVec:  validate each `&T` if vec is present
// - Box/OptionBox: validate inner `&T` (heap allocation irrelevant to validation)
fn container_kind(ty: &Type) -> ContainerKind {
    fn last_seg<'a>(ty: &'a Type) -> Option<&'a syn::PathSegment> {
        let Type::Path(p) = ty else { return None; };
        p.path.segments.last()
    }

    // Option<...>
    if let Some(seg) = last_seg(ty) {
        if seg.ident == "Option" {
            if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = ab.args.first() {
                    // Option<Vec<T>> ?
                    if let Some(inner_seg) = last_seg(inner) {
                        if inner_seg.ident == "Vec" {
                            if let syn::PathArguments::AngleBracketed(ab2) = &inner_seg.arguments {
                                if let Some(syn::GenericArgument::Type(_t)) = ab2.args.first() {
                                    return ContainerKind::OptionVec;
                                }
                            }
                        }
                        // Option<Box<T>> ?
                        if inner_seg.ident == "Box" {
                            if let syn::PathArguments::AngleBracketed(ab2) = &inner_seg.arguments {
                                if let Some(syn::GenericArgument::Type(_t)) = ab2.args.first() {
                                    return ContainerKind::OptionBox;
                                }
                            }
                        }
                    }
                    return ContainerKind::Option;
                }
            }
        }
        if seg.ident == "Vec" {
            if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(_inner)) = ab.args.first() {
                    return ContainerKind::Vec;
                }
            }
        }
        if seg.ident == "Box" {
            if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(_inner)) = ab.args.first() {
                    return ContainerKind::Box;
                }
            }
        }
    }

    ContainerKind::Plain
}

// Helper: extract the "inner" type for container wrappers
fn inner_type<'a>(ty: &'a Type, kind: &ContainerKind) -> Option<&'a Type> {
    match kind {
        ContainerKind::Option => {
            // Option<T>
            if let Type::Path(p) = ty {
                if let Some(seg) = p.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = ab.args.first() {
                            return Some(inner);
                        }
                    }
                }
            }
            None
        }
        ContainerKind::Vec => {
            // Vec<T>
            if let Type::Path(p) = ty {
                if let Some(seg) = p.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = ab.args.first() {
                            return Some(inner);
                        }
                    }
                }
            }
            None
        }
        ContainerKind::OptionVec => {
            // Option<Vec<T>>
            if let Type::Path(p) = ty {
                if let Some(seg) = p.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = ab.args.first() {
                            // inner is Vec<T>
                            if let Type::Path(p2) = inner {
                                if let Some(seg2) = p2.path.segments.last() {
                                    if let syn::PathArguments::AngleBracketed(ab2) = &seg2.arguments {
                                        if let Some(syn::GenericArgument::Type(inner2)) = ab2.args.first() {
                                            return Some(inner2);
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
        ContainerKind::Box => {
            // Box<T>
            if let Type::Path(p) = ty {
                if let Some(seg) = p.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = ab.args.first() {
                            return Some(inner);
                        }
                    }
                }
            }
            None
        }
        ContainerKind::OptionBox => {
            // Option<Box<T>>
            if let Type::Path(p) = ty {
                if let Some(seg) = p.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = ab.args.first() {
                            // inner is Box<T>
                            if let Type::Path(p2) = inner {
                                if let Some(seg2) = p2.path.segments.last() {
                                    if let syn::PathArguments::AngleBracketed(ab2) = &seg2.arguments {
                                        if let Some(syn::GenericArgument::Type(inner2)) = ab2.args.first() {
                                            return Some(inner2);
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
        ContainerKind::Plain => Some(ty),
    }
}

// Decide whether to recursively validate nested children (complex datatypes/resources).
//
// Important: We must NOT recurse into primitives or leaf wrappers (Element<...> and
// generated primitive modules under `...::primitives::...`) otherwise the macro would
// attempt to treat them like structs and generate invalid field access (system/code/coding).
fn should_recurse(ty: &Type) -> bool {
    let syn::Type::Path(p) = ty else { return false; };
    let Some(seg) = p.path.segments.last() else { return false; };

    // NEW: treat any type under ...::primitives::... as a leaf (covers aliases like r5::primitives::string::String)
    {
        let full = p.path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if full.contains("::primitives::") {
            return false;
        }
    }

    // Hard stop: any Element<...> is a leaf (covers direct use, not aliases)
    if seg.ident == "Element" {
        return false;
    }

    let ident = seg.ident.to_string();
    match ident.as_str() {
        // Rust primitives and std types
        "String" | "str" | "bool" |
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
        "f32" | "f64" |
        // Common leaf FHIR structs
        "Extension"
        => false,
        _ => true,
    }
}

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // We support structs (named fields) and choice enums (tuple variants with a single value)
    let (struct_fields_named, enum_data) = match &input.data {
        Data::Struct(ds) => {
            let Fields::Named(fields) = &ds.fields else {
                return syn::Error::new(input.span(), "FhirValidate requires named fields")
                    .to_compile_error()
                    .into();
            };
            (Some(fields), None)
        }
        Data::Enum(de) => (None, Some(de)),
        _ => {
            return syn::Error::new(input.span(), "FhirValidate only supports structs and enums")
                .to_compile_error()
                .into();
        }
    };

    // Build per-field invariant arrays and validation statements (structs only)
    let mut field_invariant_consts = Vec::new();
    let mut field_validate_stmts = Vec::new();

    // Collect struct-level invariants (constraints on the type/root element, e.g. Attachment.att-1)
    let type_inv_attrs: Vec<_> = input
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("fhir_invariant"))
        .collect();

    let mut type_invs = Vec::new();
    for a in type_inv_attrs {
        match parse_fhir_invariant(a) {
            Ok(v) => type_invs.push(v),
            Err(e) => return e.to_compile_error().into(),
        }
    }

    let type_inv_elems = type_invs.iter().map(|i| {
        let key = i.key.as_ref().expect("invariant key");
        let severity = i.severity.as_ref().expect("invariant severity");
        let human = i.human.as_ref().expect("invariant human");
        let expr = i.expr.as_ref().expect("invariant expr");
        let path = i.path.as_ref().expect("invariant path");

        let sev = match severity.value().as_str() {
            "warning" => quote! { atrius_fhirpath_support::validate::ValidationSeverity::Warning },
            _ => quote! { atrius_fhirpath_support::validate::ValidationSeverity::Error },
        };

        quote! {
            atrius_fhirpath_support::validate::Invariant {
                key: #key,
                severity: #sev,
                human: #human,
                expr: #expr,
                path: #path,
            }
        }
    });

    if let Some(fields) = struct_fields_named {
        for f in &fields.named {
            let Some(ident) = &f.ident else { continue; };

            let binding_attr = f
                .attrs
                .iter()
                .find(|a| a.path().is_ident("fhir_binding"));

            let binding_lit = if let Some(a) = binding_attr {
                match parse_fhir_binding(a) {
                    Ok(v) => Some(v),
                    Err(e) => return e.to_compile_error().into(),
                }
            } else {
                None
            };

            let inv_attrs: Vec<_> = f
                .attrs
                .iter()
                .filter(|a| a.path().is_ident("fhir_invariant"))
                .collect();

            if inv_attrs.is_empty() && binding_lit.is_none() {
                continue;
            }

            let mut invs = Vec::new();
            for a in inv_attrs {
                match parse_fhir_invariant(a) {
                    Ok(v) => invs.push(v),
                    Err(e) => return e.to_compile_error().into(),
                }
            }

            let const_suffix = sanitize_ident_for_const(ident);
            let const_name = syn::Ident::new(
                &format!("__fhir_invariants_field_{}", const_suffix),
                ident.span(),
            );

            // Always define a field-level invariant slice (may be empty) so later code can reference it.
            if invs.is_empty() {
                field_invariant_consts.push(quote! {
                    let #const_name: &[atrius_fhirpath_support::validate::Invariant] = &[];
                });
            }
            if !invs.is_empty() {
                // Generate a &'static [Invariant] constant for this field
                let inv_elems = invs.iter().map(|i| {
                    let key = i.key.as_ref().expect("invariant key");
                    let severity = i.severity.as_ref().expect("invariant severity");
                    let human = i.human.as_ref().expect("invariant human");
                    let expr = i.expr.as_ref().expect("invariant expr");
                    let path = i.path.as_ref().expect("invariant path");

                    // Map severity string -> enum at macro-expansion time (so the generated const is valid)
                    let sev = match severity.value().as_str() {
                        "warning" => quote! { atrius_fhirpath_support::validate::ValidationSeverity::Warning },
                        _ => quote! { atrius_fhirpath_support::validate::ValidationSeverity::Error },
                    };

                    quote! {
                        atrius_fhirpath_support::validate::Invariant {
                            key: #key,
                            severity: #sev,
                            human: #human,
                            expr: #expr,
                            path: #path,
                        }
                    }
                });
                field_invariant_consts.push(quote! {
                    let #const_name: &[atrius_fhirpath_support::validate::Invariant] = &[
                        #(#inv_elems),*
                    ];
                });
            }

            // Generate traversal code based on container kind
            let ty = &f.ty;
            let kind = container_kind(ty);
            // IMPORTANT: keep `access` as the *value expression* (no leading `&`).
            // Each container branch will take references as needed.
            let access = quote! { self.#ident };

            let inner_ty_for_kind = inner_type(ty, &kind);
            let do_recurse = inner_ty_for_kind.map(should_recurse).unwrap_or(false);

            let binding_stmt_tokens = if let Some(bl) = binding_lit.as_ref() {
                // NOTE: this is compile-time string extraction from the attribute
                let strength = bl.strength.as_ref().unwrap().value();
                let vs = bl.valueset.as_ref().unwrap().value();

                // Use unwrapped type that matches `value` in validate_value
                let bind_ty = inner_ty_for_kind.unwrap_or(ty);

                // A reasonable path string (you can refine later)
                let fhir_path = format!("{}.{}", name.unraw(), ident.unraw());

                binding_check_tokens(
                    quote!(value),
                    bind_ty,
                    &strength,
                    &vs,
                    &fhir_path,
                )
            } else {
                quote! {}
            };

            // Per-value validation pipeline (for a single field value):
            // 1) Evaluate field-level invariants (FHIRPath boolean expressions)
            // 2) Evaluate ValueSet binding (local membership, then optional remote fallback)
            // 3) Recurse into the value's children if it's a complex type
            //
            // This order matches semantics: invariants/binding apply to the value itself; recursion is separate.
            let validate_value = {
                let do_recurse_lit = do_recurse;
                quote! {
                    {
                        // local helper: run invs against a value
                        let focus = value.to_evaluation_result();
                        for inv in #const_name {
                            match engine.eval_bool(&focus, inv.expr) {
                                Ok(true) => {}
                                Ok(false) => __fhir_push_issue(atrius_fhirpath_support::validate::ValidationIssue {
                                    key: inv.key,
                                    severity: inv.severity,
                                    path: inv.path,
                                    instance_path: __fhir_instance_path.clone(),
                                    expression: inv.expr,
                                    message: inv.human,
                                },
                                &mut issues,
                                &mut __fhir_seen,
                            ),
                                Err(_) => __fhir_push_issue(atrius_fhirpath_support::validate::ValidationIssue {
                                    key: inv.key,
                                    severity: inv.severity,
                                    path: inv.path,
                                    instance_path: __fhir_instance_path.clone(),
                                    expression: inv.expr,
                                    message: inv.human,
                                },
                                &mut issues,
                                &mut __fhir_seen,
                            ),
                            }
                        }
                        #binding_stmt_tokens
                       
                        if #do_recurse_lit {
                            let child_issues = atrius_fhirpath_support::validate::FhirValidate::validate_with_engine(value, engine);
                            for mut ci in child_issues {
                                if ci.instance_path == ci.path {
                                    ci.instance_path.clear();
                                }
                                if ci.instance_path.is_empty() {
                                    ci.instance_path = __fhir_instance_path.clone();
                                } else if ci.instance_path.starts_with('[') {
                                    ci.instance_path = format!("{}{}", __fhir_instance_path, ci.instance_path);
                                } else {
                                    ci.instance_path = format!("{}.{}", __fhir_instance_path, ci.instance_path);
                                }
                                __fhir_push_issue(ci, &mut issues, &mut __fhir_seen);
                            }
                        }
                    }
                }
            };
            let ident_str = LitStr::new(&ident.unraw().to_string(), ident.span());
            let stmt = match kind {
                ContainerKind::Option => quote! {
                    if let Some(value) = #access.as_ref() {
                        let __fhir_instance_path = #ident_str.to_string();
                        #validate_value
                    }
                },
                ContainerKind::Vec => quote! {
                    for (i, value) in #access.iter().enumerate() {
                        let __fhir_instance_path = format!("{}[{}]", #ident_str, i);
                        #validate_value
                    }
                },
                ContainerKind::OptionVec => quote! {
                    if let Some(values) = #access.as_ref() {
                        for (i, value) in values.iter().enumerate() {
                            let __fhir_instance_path = format!("{}[{}]", #ident_str, i);
                            #validate_value
                        }
                    }
                },
                ContainerKind::Box => quote! {
                    {
                        let value = #access.as_ref();
                        let __fhir_instance_path = #ident_str.to_string();
                        #validate_value
                    }
                },
                ContainerKind::OptionBox => quote! {
                    if let Some(b) = #access.as_ref() {
                        let value = b.as_ref();
                        let __fhir_instance_path = #ident_str.to_string();
                        #validate_value
                    }
                },
                ContainerKind::Plain => quote! {
                    {
                        let value = &#access;
                        let __fhir_instance_path = #ident_str.to_string();
                        #validate_value
                    }
                },
            };
            field_validate_stmts.push(stmt);
        }
    }

    // Emit impls for structs vs enums
    let expanded = if let Some(_fields) = struct_fields_named {
        quote! {
            impl atrius_fhirpath_support::validate::FhirValidate for #name {
                fn invariants() -> &'static [atrius_fhirpath_support::validate::Invariant] {
                    &[#(#type_inv_elems),*]
                }

                fn validate_with_engine(
                    &self,
                    engine: &dyn atrius_fhirpath_support::validate::FhirPathEngine
                ) -> Vec<atrius_fhirpath_support::validate::ValidationIssue> {
                    use atrius_fhirpath_support::traits::IntoEvaluationResult;

                    let mut issues = Vec::new();

                    use std::collections::HashSet;

                    let mut __fhir_seen: HashSet<std::string::String> = HashSet::new();
                    // De-duplicate issues aggressively. Invariant evaluation + recursion can naturally produce
                    // repeats (e.g., overlapping constraints or repeated traversal). Signature uses invariant key,
                    // declared path, concrete instance_path, and expression.
                    fn __fhir_push_issue(
                        issue: atrius_fhirpath_support::validate::ValidationIssue,
                        issues: &mut Vec<atrius_fhirpath_support::validate::ValidationIssue>,
                        seen: &mut HashSet<std::string::String>,
                    ) {
                        let sig = format!(
                            "{}|{}|{}|{}",
                            issue.key,
                            issue.path,
                            issue.instance_path,
                            issue.expression
                        );
                        if seen.insert(sig) {
                            issues.push(issue);
                        }
                    }

                    // type-level invariants (defined on the struct itself)
                    {
                        let focus = self.to_evaluation_result();
                        for inv in <Self as atrius_fhirpath_support::validate::FhirValidate>::invariants() {
                            match engine.eval_bool(&focus, inv.expr) {
                                Ok(true) => {}
                                Ok(false) => __fhir_push_issue(atrius_fhirpath_support::validate::ValidationIssue {
                                    key: inv.key,
                                    severity: inv.severity,
                                    path: inv.path,
                                    instance_path: inv.path.to_string(),
                                    expression: inv.expr,
                                    message: inv.human,
                                },
                                &mut issues,
                                &mut __fhir_seen,
                            ),
                                Err(_) => __fhir_push_issue(atrius_fhirpath_support::validate::ValidationIssue {
                                    key: inv.key,
                                    severity: inv.severity,
                                    path: inv.path,
                                    instance_path: inv.path.to_string(),
                                    expression: inv.expr,
                                    message: inv.human,
                                },
                                &mut issues,
                                &mut __fhir_seen,
                            ),
                            }
                        }
                    }

                    // field-level invariants
                    #(#field_invariant_consts)*
                    #(#field_validate_stmts)*

                    issues
                }
            }
        }
    } else {
        // Enum: delegate validation to the contained value (choice enums are tuple variants like Variant(T)).
        let de = enum_data.expect("enum data");
        let mut arms = Vec::new();

        for v in &de.variants {
            let v_ident = &v.ident;
            match &v.fields {
                syn::Fields::Unnamed(unnamed) if unnamed.unnamed.len() == 1 => {
                    let inner_ty = &unnamed.unnamed.first().unwrap().ty;
                    let do_recurse = should_recurse(inner_ty);
                    if do_recurse {
                        arms.push(quote! {
                            Self::#v_ident(inner) => {
                                atrius_fhirpath_support::validate::FhirValidate::validate_with_engine(inner, engine)
                            }
                        });
                    } else {
                        arms.push(quote! {
                            Self::#v_ident(_inner) => Vec::new(),
                        });
                    }
                }
                // Unit variants: nothing to recurse into
                syn::Fields::Unit => {
                    arms.push(quote! {
                        Self::#v_ident => Vec::new(),
                    });
                }
                // Anything else is unsupported for now
                _ => {
                    return syn::Error::new(v.span(), "FhirValidate enum support expects tuple variants with exactly one field (e.g., Variant(T))")
                        .to_compile_error()
                        .into();
                }
            }
        }

        quote! {
            impl atrius_fhirpath_support::validate::FhirValidate for #name {
                fn invariants() -> &'static [atrius_fhirpath_support::validate::Invariant] {
                    &[#(#type_inv_elems),*]
                }

                fn validate_with_engine(
                    &self,
                    engine: &dyn atrius_fhirpath_support::validate::FhirPathEngine
                ) -> Vec<atrius_fhirpath_support::validate::ValidationIssue> {
                    use atrius_fhirpath_support::traits::IntoEvaluationResult;

                    let mut issues = Vec::new();

                    // type-level invariants (rare for choice enums, but supported)
                    {
                        let focus = self.to_evaluation_result();
                        for inv in <Self as atrius_fhirpath_support::validate::FhirValidate>::invariants() {
                            match engine.eval_bool(&focus, inv.expr) {
                                Ok(true) => {}
                                Ok(false) => issues.push(atrius_fhirpath_support::validate::ValidationIssue {
                                    key: inv.key,
                                    severity: inv.severity,
                                    path: inv.path,
                                    instance_path: inv.path.to_string(),
                                    expression: inv.expr,
                                    message: inv.human,
                                }),
                                Err(_) => issues.push(atrius_fhirpath_support::validate::ValidationIssue {
                                    key: inv.key,
                                    severity: inv.severity,
                                    path: inv.path,
                                    instance_path: inv.path.to_string(),
                                    expression: inv.expr,
                                    message: inv.human,
                                }),
                            }
                        }
                    }

                    // delegate to the contained value
                    let mut child = match self {
                        #(#arms)*
                    };

                    issues.append(&mut child);
                    issues
                }
            }
        }
    };

    expanded.into()
}
fn binding_check_tokens(
    field_access: proc_macro2::TokenStream,  // e.g. quote!(&self.gender)
    ty: &syn::Type,                          // the Rust type of the field
    strength: &str,                          // "required" | "extensible" | ...
    valueset_url: &str,                      // "http://hl7.org/fhir/ValueSet/administrative-gender"
    path_literal: &str,                      // e.g. "Practitioner.gender"
) -> proc_macro2::TokenStream {
    use quote::quote;

    // Helper: last ident of a TypePath (e.g., Option, Vec, Coding, CodeableConcept, Code)
    fn last_ident(ty: &syn::Type) -> Option<String> {
        match ty {
            syn::Type::Path(tp) => tp.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        }
    }

    // Unwrap Option<T> and Vec<T> recursively
    if let syn::Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            if seg.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        let inner_tokens = binding_check_tokens(
                            quote!(v),
                            inner,
                            strength,
                            valueset_url,
                            path_literal,
                        );
                        return quote! {
                            if let Some(v) = #field_access.as_ref() {
                                #inner_tokens
                            }
                        };
                    }
                }
            }

            if seg.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        let inner_tokens = binding_check_tokens(
                            quote!(v),
                            inner,
                            strength,
                            valueset_url,
                            path_literal,
                        );
                        return quote! {
                            for v in #field_access.iter() {
                                #inner_tokens
                            }
                        };
                    }
                }
            }
        }
    }

    // Base dispatch (ONLY ONE call). NOTE: `binding_ops_by_url` returns ops with fn pointers.
    let kind = last_ident(ty).unwrap_or_default();

    // Binding checks are emitted only for coded shapes (Code, Coding, CodeableConcept) because only
    // these carry the `system|code` data needed for membership checks and remote terminology calls.
    //
    // The macro expands to:
    // - local membership check via generated ValueSet binding ops (fn pointers)
    // - optional remote $validate-code via engine.validate_code_in_valueset(valueset_url, system, code)
    // - tri-state semantics (Some(true)/Some(false)/None) to support "unknown terminology" outcomes
    match kind.as_str() {
        "Coding" => {
            let membership_call = quote! { (ops.contains_coding)(#field_access) };
            quote! {
                if let Some(ops) = crate::r5::terminology::value_sets::binding_ops_by_url(#valueset_url) {
                    let ok_local = #membership_call;
                    if !ok_local {
                        let has_nonlocal = crate::r5::terminology::value_sets::valueset_has_nonlocal_rules(#valueset_url)
                            .unwrap_or(false);
                        let is_locally_enumerated = crate::r5::terminology::value_sets::valueset_is_locally_enumerated(#valueset_url)
                            .unwrap_or(true);
                        let should_try_remote = has_nonlocal || !is_locally_enumerated;

                        let sev_violation = match #strength {
                            "required" => atrius_fhirpath_support::validate::ValidationSeverity::Error,
                            "extensible" | "preferred" => atrius_fhirpath_support::validate::ValidationSeverity::Warning,
                            _ => atrius_fhirpath_support::validate::ValidationSeverity::Warning,
                        };

                        if should_try_remote {
                            let coding = #field_access;
                            let system = coding.system.as_ref().and_then(|u| u.value.as_deref());
                            let code = coding.code.as_ref().and_then(|c| c.value.as_deref());

                            let remote_result = if let (Some(system), Some(code)) = (system, code) {
                                engine.validate_code_in_valueset(#valueset_url, system, code)
                            } else {
                                None
                            };

                            match remote_result {
                                Some(true) => {}
                                Some(false) => {
                                    __fhir_push_issue(
                                        atrius_fhirpath_support::validate::ValidationIssue {
                                            key: "binding",
                                            severity: sev_violation,
                                            path: #path_literal,
                                            instance_path: __fhir_instance_path.clone(),
                                            expression: "valueset-membership",
                                            message: "ValueSet binding not satisfied",
                                        },
                                        &mut issues,
                                        &mut __fhir_seen,
                                    );
                                }
                                None => {
                                    __fhir_push_issue(
                                        atrius_fhirpath_support::validate::ValidationIssue {
                                            key: "binding",
                                            severity: atrius_fhirpath_support::validate::ValidationSeverity::Warning,
                                            path: #path_literal,
                                            instance_path: __fhir_instance_path.clone(),
                                            expression: "valueset-membership",
                                            message: "ValueSet binding could not be verified (terminology unavailable)",
                                        },
                                        &mut issues,
                                        &mut __fhir_seen,
                                    );
                                }
                            }
                        } else {
                            __fhir_push_issue(
                                atrius_fhirpath_support::validate::ValidationIssue {
                                    key: "binding",
                                    severity: sev_violation,
                                    path: #path_literal,
                                    instance_path: __fhir_instance_path.clone(),
                                    expression: "valueset-membership",
                                    message: "ValueSet binding not satisfied",
                                },
                                &mut issues,
                                &mut __fhir_seen,
                            );
                        }
                    }
                }
            }
        }
        // CodeableConcept binding validation
        //
        // Semantics:
        // - A CodeableConcept is valid if *any* of its codings is a member of the ValueSet.
        // - Local membership is checked first (fast, offline).
        // - If local membership fails AND the ValueSet requires external rules,
        //   we fall back to remote terminology validation ($validate-code).
        //
        // Remote validation is tri-state:
        //   Some(true)  => confirmed member (accept)
        //   Some(false) => confirmed non-member (violation depends on binding strength)
        //   None        => unknown / terminology unavailable (emit warning)
        //
        // Aggregation across codings:
        // - any_true  => overall true
        // - else any_false => overall false
        // - else any_none  => unknown
        "CodeableConcept" => {
            // membership_call expands to ops.contains_codeable_concept(value)
            let membership_call = quote! { (ops.contains_codeable_concept)(#field_access) };
            quote! {
                if let Some(ops) = crate::r5::terminology::value_sets::binding_ops_by_url(#valueset_url) {
                    // Local membership check (fast, offline) - checks all codings locally, using generated ValueSet code.
                    let ok_local = #membership_call;
                    if !ok_local {
                        let has_nonlocal = crate::r5::terminology::value_sets::valueset_has_nonlocal_rules(#valueset_url)
                            .unwrap_or(false);
                        let is_locally_enumerated = crate::r5::terminology::value_sets::valueset_is_locally_enumerated(#valueset_url)
                            .unwrap_or(true);
                        // Decide whether remote validation is needed
                        let should_try_remote = has_nonlocal || !is_locally_enumerated;
                        // Binding strength → severity mapping
                        let sev_violation = match #strength {
                            "required" => atrius_fhirpath_support::validate::ValidationSeverity::Error,
                            "extensible" | "preferred" => atrius_fhirpath_support::validate::ValidationSeverity::Warning,
                            _ => atrius_fhirpath_support::validate::ValidationSeverity::Warning,
                        };

                        if should_try_remote {
                            let cc = #field_access;
                            // Remote validation across multiple codings
                            // If there are no codings, we immediately return None (unknown).
                            let remote_result: Option<bool> = if let Some(codings) = cc.coding.as_ref() {
                                let mut any_none = false;
                                let mut any_false = false;
                                let mut any_true = false;
                                // Loop over codings, extract system and code
                                for coding in codings.iter() {
                                    let system = coding.system.as_ref().and_then(|u| u.value.as_deref());
                                    let code = coding.code.as_ref().and_then(|c| c.value.as_deref());

                                    if let (Some(system), Some(code)) = (system, code) {
                                        // call terminology server
                                        // this maps directly to ValueSet/$validate-code?url=...&system=...&code=...
                                        match engine.validate_code_in_valueset(#valueset_url, system, code) {
                                            Some(true) => {
                                                any_true = true;
                                                break;
                                            }
                                            Some(false) => any_false = true,
                                            None => any_none = true,
                                        }
                                    } else {
                                        any_none = true;
                                    }
                                }
                                // Aggregation rule:
                                // CodeableConcept is valid if ANY coding is valid.
                                // Unknowns are only returned if no definitive true/false result exists.
                                if any_true {
                                    Some(true)
                                } else if any_false {
                                    Some(false)
                                } else if any_none {
                                    None
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                           // Emit issues based on result
                            match remote_result {
                                // No issue — binding satisfied.
                                Some(true) => {}
                                Some(false) => {
                                    __fhir_push_issue(
                                        atrius_fhirpath_support::validate::ValidationIssue {
                                            key: "binding",
                                            severity: sev_violation,
                                            path: #path_literal,
                                            instance_path: __fhir_instance_path.clone(),
                                            expression: "valueset-membership",
                                            message: "ValueSet binding not satisfied",
                                        },
                                        &mut issues,
                                        &mut __fhir_seen,
                                    );
                                }
                                None => {
                                    __fhir_push_issue(
                                        atrius_fhirpath_support::validate::ValidationIssue {
                                            key: "binding",
                                            severity: atrius_fhirpath_support::validate::ValidationSeverity::Warning,
                                            path: #path_literal,
                                            instance_path: __fhir_instance_path.clone(),
                                            expression: "valueset-membership",
                                            message: "ValueSet binding could not be verified (terminology unavailable)",
                                        },
                                        &mut issues,
                                        &mut __fhir_seen,
                                    );
                                }
                            }
                        } else {
                            __fhir_push_issue(
                                atrius_fhirpath_support::validate::ValidationIssue {
                                    key: "binding",
                                    severity: sev_violation,
                                    path: #path_literal,
                                    instance_path: __fhir_instance_path.clone(),
                                    expression: "valueset-membership",
                                    message: "ValueSet binding not satisfied",
                                },
                                &mut issues,
                                &mut __fhir_seen,
                            );
                        }
                    }
                }
            }
        }

        "Code" => {
            let membership_call = quote! { (ops.contains_code)(#field_access) };
            quote! {
                if let Some(ops) = crate::r5::terminology::value_sets::binding_ops_by_url(#valueset_url) {
                    let ok_local = #membership_call;
                    if !ok_local {
                        let has_nonlocal = crate::r5::terminology::value_sets::valueset_has_nonlocal_rules(#valueset_url)
                            .unwrap_or(false);
                        let is_locally_enumerated = crate::r5::terminology::value_sets::valueset_is_locally_enumerated(#valueset_url)
                            .unwrap_or(true);
                        let should_try_remote = has_nonlocal || !is_locally_enumerated;

                        let sev_violation = match #strength {
                            "required" => atrius_fhirpath_support::validate::ValidationSeverity::Error,
                            "extensible" | "preferred" => atrius_fhirpath_support::validate::ValidationSeverity::Warning,
                            _ => atrius_fhirpath_support::validate::ValidationSeverity::Warning,
                        };

                        if should_try_remote {
                            __fhir_push_issue(
                                atrius_fhirpath_support::validate::ValidationIssue {
                                    key: "binding",
                                    severity: atrius_fhirpath_support::validate::ValidationSeverity::Warning,
                                    path: #path_literal,
                                    instance_path: __fhir_instance_path.clone(),
                                    expression: "valueset-membership",
                                    message: "ValueSet binding could not be verified (no system for primitive code)",
                                },
                                &mut issues,
                                &mut __fhir_seen,
                            );
                        } else {
                            __fhir_push_issue(
                                atrius_fhirpath_support::validate::ValidationIssue {
                                    key: "binding",
                                    severity: sev_violation,
                                    path: #path_literal,
                                    instance_path: __fhir_instance_path.clone(),
                                    expression: "valueset-membership",
                                    message: "ValueSet binding not satisfied",
                                },
                                &mut issues,
                                &mut __fhir_seen,
                            );
                        }
                    }
                }
            }
        }

        _ => quote! {},
    }
}