// crates/fhir-macro/src/fhir_validate.rs
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Attribute, Data, DeriveInput, Fields, LitStr, Meta, MetaList, MetaNameValue, Type};
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

// Detect Option<T>, Vec<T>, Option<Vec<T>>
enum ContainerKind {
    Plain,
    Option,
    Vec,
    OptionVec,
}

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
    }

    ContainerKind::Plain
}

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let Data::Struct(ds) = &input.data else {
        return syn::Error::new(input.span(), "FhirValidate only supports structs")
            .to_compile_error()
            .into();
    };

    let Fields::Named(fields) = &ds.fields else {
        return syn::Error::new(input.span(), "FhirValidate requires named fields")
            .to_compile_error()
            .into();
    };

    // Build per-field invariant arrays and validation statements
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

    for f in &fields.named {
        let Some(ident) = &f.ident else { continue; };

        let inv_attrs: Vec<_> = f
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("fhir_invariant"))
            .collect();

        if inv_attrs.is_empty() {
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

        // Generate traversal code based on container kind
        let ty = &f.ty;
        let access = quote! { &self.#ident };

        let validate_value = quote! {
            {
                // local helper: run invs against a value
                let focus = value.to_evaluation_result();
                for inv in #const_name {
                    match engine.eval_bool(&focus, inv.expr) {
                        Ok(true) => {}
                        Ok(false) => issues.push(atrius_fhirpath_support::validate::ValidationIssue {
                            key: inv.key,
                            severity: inv.severity,
                            path: inv.path,
                            instance_path: __fhir_instance_path.clone(),
                            expression: inv.expr,
                            message: inv.human,
                        }),
                        Err(_) => issues.push(atrius_fhirpath_support::validate::ValidationIssue {
                            key: inv.key,
                            severity: inv.severity,
                            path: inv.path,
                            instance_path: __fhir_instance_path.clone(),
                            expression: inv.expr,
                            message: inv.human,
                        }),
                    }
                }
            }
        };
        let ident_str = LitStr::new(&ident.unraw().to_string(), ident.span());
        let stmt = match container_kind(ty) {
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
            ContainerKind::Plain => quote! {
                {
                    let value = #access;
                    let __fhir_instance_path = #ident_str.to_string();
                    #validate_value
                }
            },
        };
        field_validate_stmts.push(stmt);
    }

    // Type-level invariants support (optional; empty by default)
    let expanded = quote! {
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

                // type-level invariants (defined on the struct itself)
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

                // field-level invariants
                #(#field_invariant_consts)*
                #(#field_validate_stmts)*

                issues
            }
        }
    };

    expanded.into()
}