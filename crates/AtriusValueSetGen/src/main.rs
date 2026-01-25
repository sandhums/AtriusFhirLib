//! AtriusValueSetGen
//!
//! This crate generates **FHIR R5 terminology artifacts** (CodeSystems + ValueSets) into the
//! `AtriusFhir` crate so that:
//! - CodeSystem enums can be used as strongly-typed codes (when the CodeSystem enumerates concepts).
//! - ValueSet wrappers can provide **best-effort local membership checks** for `Code`, `Coding`, and
//!   `CodeableConcept` (used by `FhirValidate` binding enforcement).
//! - The validation layer can route bindings by **ValueSet canonical URL** via a tiny runtime
//!   registry (`binding_ops_by_url`, `valueset_has_nonlocal_rules`, `valueset_is_locally_enumerated`).
//!
//! ## Inputs
//! - `resources/R5/valuesets.json` (FHIR bundle containing `CodeSystem` and `ValueSet` resources).
//!   This is the single source of truth for *what can be generated locally*.
//!
//! ## Outputs
//! Generated files are written into `crates/AtriusFhir/src/r5/terminology`:
//! - `code_systems/*.rs` : one module per CodeSystem
//! - `code_systems/mod.rs` : `pub mod` + `pub use` re-exports
//! - `value_sets/*.rs` : one module per ValueSet wrapper
//! - `value_sets/mod.rs` : `pub mod` + `pub use` plus runtime URL registries
//! - `bindings.rs` : shared binding traits/enums used by validation
//! - `mod.rs` : terminology module root
//!
//! ## Local vs remote terminology
//! Many ValueSets in FHIR cannot be fully validated locally because their `compose` rules include:
//! - `filter` rules (terminology server evaluation)
//! - `include.valueSet` references
//! - whole-system includes where the CodeSystem is **not locally enumerated**
//!
//! For those cases, the generator marks the wrapper with:
//! - `HAS_NONLOCAL_RULES: bool`
//! - `include_whole_systems()` (systems requiring a terminology server)
//! - `is_locally_enumerated()` (fast path for purely local ValueSets)
//!
//! The `FhirValidate` derive uses these signals to decide when to fall back to remote
//! `$validate-code` (via the `FhirPathEngine::validate_code_in_valueset` hook).
//!
//! ## Determinism
//! Generation uses stable ordering (`BTreeMap`) and module-name deconfliction to ensure that
//! rebuilds produce consistent output.
use anyhow::{anyhow, Context, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

fn main() -> Result<()> {
    // ---------------------------------------------------------------------
    // 1) Locate inputs + outputs
    // ---------------------------------------------------------------------
    // Input: crates/AtriusValueSetGen/resources/R5/valuesets.json
    let input_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("R5")
        .join("valuesets.json");

    if !input_path.exists() {
        return Err(anyhow!(
            "valuesets.json not found at: {}",
            input_path.display()
        ));
    }

    // Output: crates/AtriusFhir/src/r5/terminology
    let terminology_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("expected crates/AtriusValueSetGen to have a parent directory")?
        .join("AtriusFhir")
        .join("src")
        .join("r5")
        .join("terminology");

    let code_systems_dir = terminology_dir.join("code_systems");
    let value_sets_dir = terminology_dir.join("value_sets");

    fs::create_dir_all(&code_systems_dir)
        .with_context(|| format!("failed to create output dir {}", code_systems_dir.display()))?;

    fs::create_dir_all(&value_sets_dir)
        .with_context(|| format!("failed to create output dir {}", value_sets_dir.display()))?;

    // ---------------------------------------------------------------------
    // 2) Parse the FHIR bundle and split resources into CodeSystems + ValueSets
    // ---------------------------------------------------------------------
    let json = fs::read_to_string(&input_path)
        .with_context(|| format!("failed reading {}", input_path.display()))?;

    let bundle: Bundle = serde_json::from_str(&json)
        .with_context(|| format!("failed parsing JSON bundle {}", input_path.display()))?;

    let entries = bundle.entry.unwrap_or_default();

    let mut code_systems: Vec<CodeSystem> = Vec::new();
    let mut value_sets: Vec<ValueSet> = Vec::new();

    for e in entries {
        match e.resource {
            Resource::CodeSystem(cs) => code_systems.push(cs),
            Resource::ValueSet(vs) => value_sets.push(vs),
            _ => {}
        }
    }

    // ---------------------------------------------------------------------
    // 3) CodeSystem generation
    //    - Prefer enumerated CodeSystems as Rust enums (finite concept lists)
    //    - Emit stubs for non-enumerated systems (content=not-present, etc.)
    // ---------------------------------------------------------------------
    // Deterministic ordering by module name (derived from CodeSystem.id)
    // Map: module_name -> (enum_name, codesystem, has_enumerated_concepts)
    let mut by_module: BTreeMap<String, (String, CodeSystem, bool)> = BTreeMap::new();

    for cs in code_systems {
        let enum_name = match codesystem_enum_name(&cs) {
            Some(n) => n,
            None => continue,
        };

        let base_module = match codesystem_module_name(&cs) {
            Some(m) => m,
            None => continue,
        };

        // Ensure uniqueness of module names (rare but safe)
        let mut module = base_module.clone();
        let mut suffix: usize = 2;
        while by_module.contains_key(&module) {
            module = format!("{}_{}", base_module, suffix);
            suffix += 1;
        }

        let has_enumerated_concepts = cs.concept.as_ref().map(|v| !v.is_empty()).unwrap_or(false);
        by_module.insert(module, (enum_name, cs, has_enumerated_concepts));
    }

    // Map canonical CodeSystem URL -> generated type information.
    // These maps are used during ValueSet generation to:
    // - validate whole-system includes locally when possible
    // - "pull" finite CodeSystem concepts into ValueSets that include an entire CodeSystem
    let mut cs_by_url: HashMap<String, (String, String)> = HashMap::new();
    for (module, (enum_name, cs, has_enum)) in &by_module {
        if *has_enum {
            cs_by_url.insert(cs.url.clone(), (module.clone(), enum_name.clone()));
        }
    }

    // Map CodeSystem canonical URL -> concept list (when present and finite).
    // Used to “pull in” codes for ValueSets that include whole CodeSystems without listing concepts.
    let mut cs_concepts_by_url: HashMap<String, Vec<Concept>> = HashMap::new();
    for (_module, (_enum_name, cs, has_enum)) in &by_module {
        if *has_enum {
            if let Some(concepts) = cs.concept.as_ref() {
                if !concepts.is_empty() {
                    cs_concepts_by_url.insert(cs.url.clone(), concepts.clone());
                }
            }
        }
    }

    // Write one file per CodeSystem into terminology/code_systems/<id>.rs
    for (module, (enum_name, cs, has_enum)) in &by_module {
        // Enumerated CodeSystems become Rust enums with:
        // - `as_code()` for serialization
        // - `to_code()`, `to_coding()`, `to_codeable_concept()` helpers
        // - `TryFrom<&str>` for parsing
        //
        // Non-enumerated CodeSystems get a stub type with:
        // - `URL`, `system()`, `version()`
        // - optional grammar validation for special systems (e.g. `color-rgb`)
        if *has_enum {
            if let Some(tokens) = generate_codesystem_enum_tokens(enum_name, cs) {
                let file_ast: syn::File = syn::parse2(tokens)
                    .with_context(|| format!("failed parsing tokens for {}", enum_name))?;

                let mut src = String::new();
                src.push_str("// @generated by AtriusValueSetGen\n");
                src.push_str("// DO NOT EDIT MANUALLY\n\n");
                src.push_str("#![allow(non_camel_case_types)]\n");
                src.push_str("#![allow(non_snake_case)]\n");
                src.push_str("#![allow(clippy::upper_case_acronyms)]\n\n");
                src.push_str("use serde::{Deserialize, Serialize};\n\n");
                // We are generated under `crate::r5::terminology::code_systems`, so `super::super::super` is the `r5` module.
                src.push_str(
                    "use super::super::super::{Boolean, Code, CodeableConcept, Coding, Element, Uri};\n\n",
                );
                src.push_str("use super::super::super::string::String as FhirString;\n\n");

                src.push_str(&prettyplease::unparse(&file_ast));
                src.push('\n');

                let path = code_systems_dir.join(format!("{}.rs", module));
                fs::write(&path, src)
                    .with_context(|| format!("failed writing {}", path.display()))?;
            }
        } else {
            let tokens = generate_codesystem_stub_tokens(enum_name, cs);
            let file_ast: syn::File = syn::parse2(tokens)
                .with_context(|| format!("failed parsing stub tokens for {}", enum_name))?;

            let mut src = String::new();
            src.push_str("// @generated by AtriusValueSetGen\n");
            src.push_str("// DO NOT EDIT MANUALLY\n\n");
            src.push_str("#![allow(non_camel_case_types)]\n");
            src.push_str("#![allow(non_snake_case)]\n");
            src.push_str("#![allow(clippy::upper_case_acronyms)]\n\n");
            // We are generated under `crate::r5::terminology::code_systems`, so `super::super::super` is the `r5` module.
            src.push_str(
                "use super::super::super::{Boolean, Code, CodeableConcept, Coding, Element, Uri};\n\n",
            );
            src.push_str("use super::super::super::string::String as FhirString;\n\n");

            src.push_str(&prettyplease::unparse(&file_ast));
            src.push('\n');

            let path = code_systems_dir.join(format!("{}.rs", module));
            fs::write(&path, src)
                .with_context(|| format!("failed writing {}", path.display()))?;
        }
    }

    // ---------------------------------------------------------------------
    // 4) ValueSet generation
    //
    // We generate lightweight wrapper types with local membership helpers:
    // - `contains_code(&Code)`
    // - `contains_coding(&Coding)`
    // - `contains_codeable_concept(&CodeableConcept)`
    //
    // These are intentionally **best-effort**:
    // - If a ValueSet has an explicit expansion or inline concepts, local validation can be definitive.
    // - If it contains filters/include.valueSet/whole-system non-enumerated includes, local validation
    //   is incomplete and the wrapper will be marked with `HAS_NONLOCAL_RULES`.
    // ---------------------------------------------------------------------

    let mut vs_by_module: BTreeMap<String, (String, ValueSet)> = BTreeMap::new();

    for vs in value_sets {
        if let Some(id) = vs.id.as_deref() {
            if id.starts_with("example-") {
                continue;
            }
        }
        let enum_name = match valueset_type_name(&vs) {
            Some(n) => n,
            None => continue,
        };

        let base_module = match valueset_module_name(&vs) {
            Some(m) => m,
            None => continue,
        };

        let mut module = base_module.clone();
        let mut suffix: usize = 2;
        while vs_by_module.contains_key(&module) {
            module = format!("{}_{}", base_module, suffix);
            suffix += 1;
        }

        vs_by_module.insert(module, (enum_name, vs));
    }

    for (module, (type_name, vs)) in &vs_by_module {
        if let Some(tokens) = generate_valueset_wrapper_tokens(type_name, vs, &cs_by_url, &cs_concepts_by_url) {
            let file_ast: syn::File = syn::parse2(tokens)
                .with_context(|| format!("failed parsing tokens for ValueSet {}", type_name))?;

            let mut src = String::new();
            src.push_str("// @generated by AtriusValueSetGenerator\n");
            src.push_str("// DO NOT EDIT MANUALLY\n\n");
            src.push_str("#![allow(non_camel_case_types)]\n");
            src.push_str("#![allow(non_snake_case)]\n");
            src.push_str("#![allow(clippy::upper_case_acronyms)]\n\n");

            // This module is `crate::r5::terminology::value_sets::<module>`.
            src.push_str(
                "use super::super::super::{Code, CodeableConcept, Coding, Element, Uri};\n\n",
            );
// Generate a stub module for CodeSystems that do not enumerate concepts.
fn generate_codesystem_stub_tokens(type_name: &str, cs: &CodeSystem) -> TokenStream {
    let type_ident = format_ident!("{}", type_name);
    let system_lit = syn::LitStr::new(&cs.url, proc_macro2::Span::call_site());

    let version_tokens: TokenStream = match cs.version.as_deref() {
        Some(v) => {
            let v_lit = syn::LitStr::new(v, proc_macro2::Span::call_site());
            quote!(Some(#v_lit))
        }
        None => quote!(None),
    };

    let mut docs = vec![
        format!("FHIR CodeSystem (non-enumerated): {}", type_name),
        format!("Canonical URL: {}", cs.url),
    ];
    if let Some(v) = &cs.version { docs.push(format!("Version: {}", v)); }
    if let Some(t) = &cs.title { docs.push(format!("Title: {}", t)); }
    if let Some(st) = &cs.status { docs.push(format!("Status: {}", st)); }
    if let Some(desc) = &cs.description { docs.push(desc.clone()); }

    // Note about why no enum exists
    docs.push("NOTE: This CodeSystem does not enumerate concepts in valuesets.json (e.g. content=not-present or empty concept list).".to_string());
    docs.push("A full terminology server is required to validate membership for arbitrary codes.".to_string());

    let doc_tokens = doc_attrs(&docs);

    let rgb = cs.url == "http://hl7.org/fhir/color-rgb";
    let is_valid_body: TokenStream = if rgb {
        quote! {
            if code.len() != 7 { return false; }
            let bytes = code.as_bytes();
            if bytes[0] != b'#' { return false; }
            code[1..].bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F'))
        }
    } else {
        quote!({ let _ = code; false })
    };

    quote! {
        #doc_tokens
        pub struct #type_ident;

        impl #type_ident {
            pub const URL: &'static str = #system_lit;

            pub fn system() -> &'static str {
                Self::URL
            }

            pub fn version() -> Option<&'static str> {
                #version_tokens
            }

            /// Best-effort local validation.
            ///
            /// Returns `true` only for systems where we can validate by grammar locally.
            /// Otherwise returns `false` (unknown locally).
            pub fn is_valid_code(code: &str) -> bool {
                #is_valid_body
            }
        }
    }
}
            src.push_str("use super::super::super::string::String as FhirString;\n\n");

            src.push_str(&prettyplease::unparse(&file_ast));
            src.push('\n');

            let path = value_sets_dir.join(format!("{}.rs", module));
            fs::write(&path, src)
                .with_context(|| format!("failed writing {}", path.display()))?;
        }
    }

    // Emit `terminology/value_sets/mod.rs`.
    //
    // In addition to `pub mod` + `pub use`, we generate a small **runtime registry** used by
    // `FhirValidate` binding enforcement. The registry allows the validator to:
    // - locate `contains_*` function pointers by canonical ValueSet URL
    // - determine whether local membership checks are definitive or require terminology fallback
    let mut vs_mod = String::new();
    vs_mod.push_str("// @generated by AtriusValueSetGen\n// DO NOT EDIT MANUALLY\n\n");

    // Module declarations
    for module in vs_by_module.keys() {
        vs_mod.push_str(&format!("pub mod {};\n", module));
    }

    vs_mod.push('\n');

    // Re-exports
    for module in vs_by_module.keys() {
        vs_mod.push_str(&format!("pub use {}::*;\n", module));
    }

    vs_mod.push_str(
        "\n// ---- Runtime binding registry (generated) ----\n\n",
    );

    // Bring types into scope for function pointer signatures
    vs_mod.push_str(
        "use super::super::{Code, Coding, CodeableConcept};\n\n",
    );

    vs_mod.push_str(
        "/// Function pointers for best-effort ValueSet membership checks.\n",
    );
    vs_mod.push_str(
        "///\n/// These are used by `FhirValidate` to enforce `#[fhir_binding(...)]` by URL at runtime.\n",
    );
    vs_mod.push_str(
        "pub struct ValueSetBindingOps {\n",
    );
    vs_mod.push_str(
        "    pub contains_code: fn(&Code) -> bool,\n",
    );
    vs_mod.push_str(
        "    pub contains_coding: fn(&Coding) -> bool,\n",
    );
    vs_mod.push_str(
        "    pub contains_codeable_concept: fn(&CodeableConcept) -> bool,\n",
    );
    vs_mod.push_str(
        "}\n\n",
    );

    // Emit one static ops table per ValueSet wrapper.
    // We reference the wrapper type by module + type name (module is the file/module name).
    for (module, (type_name, _vs)) in &vs_by_module {
        let ops_name = format!("__OPS_{}", module.to_ascii_uppercase());
        vs_mod.push_str(&format!(
            "static {}: ValueSetBindingOps = ValueSetBindingOps {{\n",
            ops_name
        ));
        vs_mod.push_str(&format!(
            "    contains_code: {}::{}::contains_code,\n",
            module, type_name
        ));
        vs_mod.push_str(&format!(
            "    contains_coding: {}::{}::contains_coding,\n",
            module, type_name
        ));
        vs_mod.push_str(&format!(
            "    contains_codeable_concept: {}::{}::contains_codeable_concept,\n",
            module, type_name
        ));
        vs_mod.push_str("};\n\n");
    }

    // Emit the URL->ops matcher.
    vs_mod.push_str(
        "/// Lookup binding operations by canonical ValueSet URL.\n",
    );
    vs_mod.push_str(
        "///\n",
    );
    vs_mod.push_str(
        "/// NOTE: URLs are expected to be version-stripped (no `|x.y.z` suffix).\n",
    );
    vs_mod.push_str(
        "pub fn binding_ops_by_url(url: &str) -> Option<&'static ValueSetBindingOps> {\n",
    );
    vs_mod.push_str("    match url {\n");

    for (module, (type_name, vs)) in &vs_by_module {
        let ops_name = format!("__OPS_{}", module.to_ascii_uppercase());
        let url = vs.url.as_str();
        vs_mod.push_str(&format!(
            "        {}::{}::URL => Some(&{}),\n",
            module, type_name, ops_name
        ));
    }

    vs_mod.push_str("        _ => None,\n");
    vs_mod.push_str("    }\n");
    vs_mod.push_str("}\n");

    // ---- valueset_has_nonlocal_rules ----
    vs_mod.push_str("\n/// Lookup whether a ValueSet has any non-local compose rules (filters, include.valueSet, or whole-system includes without local enumeration).\n");
    vs_mod.push_str("pub fn valueset_has_nonlocal_rules(url: &str) -> Option<bool> {\n");
    vs_mod.push_str("    match url {\n");
    for (module, (type_name, _vs)) in &vs_by_module {
        vs_mod.push_str(&format!("        {}::{}::URL => Some({}::{}::HAS_NONLOCAL_RULES),\n", module, type_name, module, type_name));
    }
    vs_mod.push_str("        _ => None,\n");
    vs_mod.push_str("    }\n");
    vs_mod.push_str("}\n");

    // ---- valueset_is_locally_enumerated ----
    vs_mod.push_str("\n/// Lookup whether a ValueSet is fully locally enumerable (i.e., membership can be decided without a terminology server).\n");
    vs_mod.push_str("pub fn valueset_is_locally_enumerated(url: &str) -> Option<bool> {\n");
    vs_mod.push_str("    match url {\n");
    for (module, (type_name, _vs)) in &vs_by_module {
        vs_mod.push_str(&format!("        {}::{}::URL => Some({}::{}::is_locally_enumerated()),\n", module, type_name, module, type_name));
    }
    vs_mod.push_str("        _ => None,\n");
    vs_mod.push_str("    }\n");
    vs_mod.push_str("}\n");

    fs::write(value_sets_dir.join("mod.rs"), vs_mod)
        .with_context(|| "failed writing terminology/value_sets/mod.rs")?;

    eprintln!(
        "Generated {} ValueSet wrappers into {}",
        vs_by_module.len(),
        value_sets_dir.display()
    );

    // Emit terminology/bindings.rs
    let bindings_rs = r#"// @generated by AtriusValueSetGen
// DO NOT EDIT MANUALLY

/// Trait implemented by generated ValueSet wrappers to provide membership checks
/// for different bound FHIR datatypes (`Code`, `Coding`, `CodeableConcept`). Generated wrappers implement this trait for all three.
pub trait ValueSetMembership<T> {
    fn contains(v: &T) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingStrength {
    Required,
    Extensible,
    Preferred,
    Example,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingIssue {
    /// Definitive violation (e.g. required binding not satisfied).
    Error { message: &'static str },
    /// Non-fatal issue (e.g. extensible/preferred not satisfied).
    Warning { message: &'static str },
    /// Membership could not be determined (e.g. terminology server unavailable).
    /// This should be surfaced as a warning by validators.
    Unknown { message: &'static str },
}

pub struct Binding<VS> {
    strength: BindingStrength,
    _vs: core::marker::PhantomData<VS>,
}

impl<VS> Binding<VS> {
    pub fn required() -> Self {
        Self {
            strength: BindingStrength::Required,
            _vs: core::marker::PhantomData,
        }
    }

    pub fn extensible() -> Self {
        Self {
            strength: BindingStrength::Extensible,
            _vs: core::marker::PhantomData,
        }
    }

    pub fn preferred() -> Self {
        Self {
            strength: BindingStrength::Preferred,
            _vs: core::marker::PhantomData,
        }
    }

    pub fn example() -> Self {
        Self {
            strength: BindingStrength::Example,
            _vs: core::marker::PhantomData,
        }
    }

    pub fn strength(&self) -> BindingStrength {
        self.strength
    }
}

/// Binding validator for a particular bound datatype `T`.
///
/// This API supports both purely local membership checks and an optional remote
/// terminology fallback.
pub trait BindingValidator<T> {
    /// Local-only check.
    fn check(self, value: &T) -> Result<(), BindingIssue>;

    /// Local-first check with optional remote fallback.
    ///
    /// Semantics (recommended usage):
    /// - First, evaluate local membership via `ValueSetMembership`.
    /// - If local passes -> Ok.
    /// - If local fails and `has_nonlocal_rules == true` (or `is_locally_enumerated == false`),
    ///   then attempt remote `$validate-code` using `remote_validate`.
    /// - If remote returns `Some(true)` -> Ok.
    /// - If remote returns `Some(false)` -> treat as a definitive failure.
    /// - If remote returns `None` -> Unknown (do not emit Error).
    ///
    /// `remote_validate` must implement ValueSet `$validate-code` semantics:
    ///
    /// NOTE: Remote validation requires both `system` and `code` (most terminology servers reject `code` without `system`).
    /// `remote_validate(valueset_url, system, code) -> Option<bool>`.
    fn check_with_remote(
        self,
        value: &T,
        valueset_url: &str,
        has_nonlocal_rules: bool,
        is_locally_enumerated: bool,
        system_and_code: Option<(&str, &str)>,
        remote_validate: Option<&dyn Fn(&str, &str, &str) -> Option<bool>>,
    ) -> Result<(), BindingIssue>;
}

impl<VS, T> BindingValidator<T> for Binding<VS>
where
    VS: ValueSetMembership<T>,
{
    fn check(self, value: &T) -> Result<(), BindingIssue> {
        let ok = VS::contains(value);
        match self.strength {
            BindingStrength::Required => {
                if ok {
                    Ok(())
                } else {
                    Err(BindingIssue::Error {
                        message: "Required binding violated",
                    })
                }
            }
            BindingStrength::Extensible => {
                if ok {
                    Ok(())
                } else {
                    Err(BindingIssue::Warning {
                        message: "Extensible binding not satisfied",
                    })
                }
            }
            BindingStrength::Preferred => {
                if ok {
                    Ok(())
                } else {
                    Err(BindingIssue::Warning {
                        message: "Preferred binding not satisfied",
                    })
                }
            }
            BindingStrength::Example => Ok(()),
        }
    }

    fn check_with_remote(
        self,
        value: &T,
        valueset_url: &str,
        has_nonlocal_rules: bool,
        is_locally_enumerated: bool,
        system_and_code: Option<(&str, &str)>,
        remote_validate: Option<&dyn Fn(&str, &str, &str) -> Option<bool>>,
    ) -> Result<(), BindingIssue> {
        // 1) Local first
        if VS::contains(value) {
            return Ok(());
        }

        // Example bindings are never enforced.
        if matches!(self.strength, BindingStrength::Example) {
            return Ok(());
        }

        // 2) Decide whether remote fallback is appropriate.
        // If the ValueSet is fully locally enumerable, a local failure is definitive.
        let should_try_remote = has_nonlocal_rules || !is_locally_enumerated;

        if should_try_remote {
            if let (Some((system, code)), Some(remote)) = (system_and_code, remote_validate) {
                match remote(valueset_url, system, code) {
                    Some(true) => return Ok(()),
                    Some(false) => {
                        // definitive remote failure -> fall through to strength mapping
                    }
                    None => {
                        // unknown (e.g. unreachable) -> never error
                        return Err(BindingIssue::Unknown {
                            message: "ValueSet membership could not be determined (terminology unavailable)",
                        });
                    }
                }
            } else {
                // We wanted remote but cannot call it due to missing (system,code) or client.
                return Err(BindingIssue::Unknown {
                    message: "ValueSet requires terminology validation but inputs/client were unavailable",
                });
            }
        }

        // 3) Final mapping by strength
        match self.strength {
            BindingStrength::Required => Err(BindingIssue::Error {
                message: "Required binding violated",
            }),
            BindingStrength::Extensible => Err(BindingIssue::Warning {
                message: "Extensible binding not satisfied",
            }),
            BindingStrength::Preferred => Err(BindingIssue::Warning {
                message: "Preferred binding not satisfied",
            }),
            BindingStrength::Example => Ok(()),
        }
    }
}
"#;

    fs::write(terminology_dir.join("bindings.rs"), bindings_rs)
        .with_context(|| "failed writing terminology/bindings.rs")?;

    // Emit terminology/mod.rs
    fs::write(
        terminology_dir.join("mod.rs"),
        "// @generated by AtriusValueSetGen\n// DO NOT EDIT MANUALLY\n\npub mod bindings;\npub mod code_systems;\npub mod value_sets;\n\npub use bindings::*;\n",
    )
    .with_context(|| "failed writing terminology/mod.rs")?;

    // Emit terminology/code_systems/mod.rs (pub mod + pub use for each module)
    let mut cs_mod = String::new();
    cs_mod.push_str("// @generated by AtriusValueSetGen\n// DO NOT EDIT MANUALLY\n\n");

    for module in by_module.keys() {
        cs_mod.push_str(&format!("pub mod {};\n", module));
    }
    cs_mod.push('\n');
    for module in by_module.keys() {
        cs_mod.push_str(&format!("pub use {}::*;\n", module));
    }

    fs::write(code_systems_dir.join("mod.rs"), cs_mod)
        .with_context(|| "failed writing terminology/code_systems/mod.rs")?;

    eprintln!(
        "Generated {} CodeSystem enums into {}",
        by_module.len(),
        code_systems_dir.display()
    );

    Ok(())
}

// -------------------- JSON models --------------------

#[derive(Debug, Deserialize)]
struct Bundle {
    entry: Option<Vec<BundleEntry>>,
}

#[derive(Debug, Deserialize)]
struct BundleEntry {
    #[serde(rename = "fullUrl")]
    _full_url: Option<String>,
    resource: Resource,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "resourceType")]
enum Resource {
    CodeSystem(CodeSystem),
    ValueSet(ValueSet),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize, Clone)]
struct CodeSystem {
    id: Option<String>,
    url: String,
    version: Option<String>,
    name: Option<String>,
    title: Option<String>,
    status: Option<String>,
    description: Option<String>,
    concept: Option<Vec<Concept>>,
}

#[derive(Debug, Deserialize, Clone)]
struct ValueSet {
    id: Option<String>,
    url: String,
    version: Option<String>,
    name: Option<String>,
    title: Option<String>,
    status: Option<String>,
    description: Option<String>,
    compose: Option<ValueSetCompose>,
    expansion: Option<ValueSetExpansion>,
}

#[derive(Debug, Deserialize, Clone)]
struct ValueSetCompose {
    include: Option<Vec<ValueSetInclude>>,
    exclude: Option<Vec<ValueSetInclude>>,
}

#[derive(Debug, Deserialize, Clone)]
struct ValueSetInclude {
    system: Option<String>,
    #[serde(rename = "valueSet")]
    value_set: Option<Vec<String>>,
    concept: Option<Vec<ValueSetConcept>>,
    filter: Option<Vec<ValueSetFilter>>,
}

#[derive(Debug, Deserialize, Clone)]
struct ValueSetConcept {
    code: String,
    display: Option<String>,
    extension: Option<Vec<ValueSetConceptExtension>>,
}

#[derive(Debug, Deserialize, Clone)]
struct ValueSetConceptExtension {
    url: String,
    #[serde(rename = "valueString")]
    value_string: Option<String>,
}
#[derive(Debug, Deserialize, Clone)]
struct ValueSetFilter {
    property: Option<String>,
    op: Option<String>,
    value: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct ValueSetExpansion {
    contains: Option<Vec<ValueSetExpansionContains>>,
}

#[derive(Debug, Deserialize, Clone)]
struct ValueSetExpansionContains {
    system: Option<String>,
    code: Option<String>,
    display: Option<String>,
    contains: Option<Vec<ValueSetExpansionContains>>,
}
#[derive(Debug, Deserialize, Clone)]
struct Concept {
    code: String,
    display: Option<String>,
    definition: Option<String>,
    extension: Option<Vec<Extension>>,
}

#[derive(Debug, Deserialize, Clone)]
struct Extension {
    url: String,
    #[serde(rename = "valueString")]
    value_string: Option<String>,
}

// -------------------- Generation helpers --------------------

fn rust_type_from_fhir_name(name: &str) -> String {
    // FHIR `name` is already in PascalCase/CamelCase and often contains acronyms.
    // We preserve existing case for alphanumerics and treat separators as word breaks.
    // We also ensure the result is a valid Rust identifier.

    // First, keep only alphanumerics, and turn everything else into spaces.
    let cleaned: String = name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect();

    let mut out = String::new();
    for w in cleaned.split_whitespace() {
        // Preserve the word as-is (case), but ensure it doesn't start with a digit.
        if w.is_empty() {
            continue;
        }
        if w.chars().next().unwrap().is_ascii_digit() {
            out.push('N');
        }
        out.push_str(w);
    }

    if out.is_empty() {
        return "ValueSet".to_string();
    }

    // Avoid a few problematic identifiers
    match out.as_str() {
        "Self" | "Type" | "Super" | "Crate" | "Mod" | "Move" => format!("{}__", out),
        _ => out,
    }
}

fn rust_type_from_title(title: &str, fallback: &str) -> String {
    // Titles can contain spaces and punctuation; normalize to PascalCase.
    let cleaned: String = title
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect();

    let mut out = String::new();
    for w in cleaned.split_whitespace() {
        let mut chars = w.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(chars.map(|c| c.to_ascii_lowercase()));
        }
    }

    if out.is_empty() { fallback.to_string() } else { out }
}

fn codesystem_enum_name(cs: &CodeSystem) -> Option<String> {
    if let Some(n) = cs.name.as_deref() {
        return Some(rust_type_from_fhir_name(n));
    }
    if let Some(t) = cs.title.as_deref() {
        return Some(rust_type_from_title(t, "CodeSystem"));
    }
    cs.id.as_deref().map(|id| rust_type_from_title(id, "CodeSystem"))
}

fn codesystem_module_name(cs: &CodeSystem) -> Option<String> {
    // Prefer CodeSystem.id as requested (stable, kebab-case in FHIR).
    let id = cs.id.as_deref()?;

    // Convert kebab-case to snake_case for Rust module/file name.
    let mut s = id.replace('-', "_");

    // If still contains non-identifier chars, replace them with '_'
    s = s
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '_' { ch } else { '_' })
        .collect();

    // Must not start with a digit for a Rust module name.
    if s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        s = format!("cs_{}", s);
    }

    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn valueset_type_name(vs: &ValueSet) -> Option<String> {
    // Prefer `name` (FHIR PascalCase, often with acronyms) and preserve it.
    if let Some(n) = vs.name.as_deref() {
        return Some(rust_type_from_fhir_name(n));
    }

    // Fall back to `title` (normalize to PascalCase), then `id`.
    if let Some(t) = vs.title.as_deref() {
        return Some(rust_type_from_title(t, "ValueSet"));
    }

    vs.id.as_deref()
        .map(|id| rust_type_from_title(id, "ValueSet"))
}

fn valueset_module_name(vs: &ValueSet) -> Option<String> {
    // Prefer ValueSet.id (stable, kebab-case in FHIR).
    let id = vs.id.as_deref()?;
    let mut s = id.replace('-', "_");
    s = s
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '_' { ch } else { '_' })
        .collect();
    if s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        s = format!("vs_{}", s);
    }
    if s.is_empty() { None } else { Some(s) }
}

fn concept_comments(c: &Concept) -> Vec<String> {
    const COMMENTS_URL: &str = "http://hl7.org/fhir/StructureDefinition/codesystem-concept-comments";

    c.extension
        .as_ref()
        .into_iter()
        .flat_map(|v| v.iter())
        .filter(|e| e.url == COMMENTS_URL)
        .filter_map(|e| e.value_string.clone())
        .collect()
}

fn to_rust_ident_pascal(code: &str) -> String {
    // Handle common symbolic codes explicitly (FHIR has a few like QuantityComparator)
    match code {
        "<" => return "Lt".to_string(),
        "<=" => return "Le".to_string(),
        ">" => return "Gt".to_string(),
        ">=" => return "Ge".to_string(),
        "=" => return "Eq".to_string(),
        "!=" => return "Ne".to_string(),
        _ => {}
    }

    // Convert non-alnum to spaces
    let cleaned: String = code
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect();

    // PascalCase words
    let mut out = String::new();
    for w in cleaned.split_whitespace() {
        let mut chars = w.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(chars.map(|c| c.to_ascii_lowercase()));
        }
    }

    // If it ends up empty (pure symbols), fall back to a deterministic name
    if out.is_empty() {
        // Encode original code bytes into a safe identifier.
        // Example: "<=" -> Sym_3c_3d
        let mut s = String::from("Sym");
        for b in code.as_bytes() {
            s.push('_');
            s.push_str(&format!("{:02x}", b));
        }
        out = s;
    }

    // Ensure it doesn't start with a digit
    if out.chars().next().unwrap().is_ascii_digit() {
        out = format!("N{}", out);
    }

    // Avoid a small set of common Rust keywords / special idents
    match out.as_str() {
        "Self" | "Type" | "Super" | "Crate" | "Mod" | "Move" => format!("{}__", out),
        _ => out,
    }
}

fn doc_attrs(lines: &[String]) -> TokenStream {
    // FHIR text/definitions frequently contain CRLF/CR paragraph breaks (e.g. "\r\r").
    // If we embed raw '\r' in generated source it can split the line and break compilation.
    // Also, to preserve paragraph breaks in rustdoc, emit one #[doc = "..."] per line.

    let mut out: Vec<proc_macro2::TokenStream> = Vec::new();

    for l in lines {
        // Normalize newlines first
        let normalized = l.replace("\r\n", "\n").replace('\r', "\n");

        for part in normalized.split('\n') {
            // Preserve blank lines as an empty doc line
            if part.trim().is_empty() {
                out.push(quote!(#[doc = ""]));
            } else {
                let s = part.trim().to_string();
                out.push(quote!(#[doc = #s]));
            }
        }
    }

    quote!(#(#out)*)
}
fn flatten_expansion_contains(nodes: &[ValueSetExpansionContains], out: &mut Vec<(String, String)>) {
    for n in nodes {
        if let (Some(sys), Some(code)) = (n.system.as_ref(), n.code.as_ref()) {
            out.push((sys.clone(), code.clone()));
        }
        if let Some(children) = n.contains.as_ref() {
            flatten_expansion_contains(children, out);
        }
    }
}
fn valueset_concept_definition(c: &ValueSetConcept) -> Option<&str> {
    const DEF_URL: &str = "http://hl7.org/fhir/StructureDefinition/valueset-concept-definition";
    let exts = c.extension.as_ref()?;
    for e in exts {
        if e.url == DEF_URL {
            if let Some(v) = e.value_string.as_deref() {
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}
fn generate_codesystem_enum_tokens(enum_name: &str, cs: &CodeSystem) -> Option<TokenStream> {
    let concepts = cs.concept.as_ref()?;
    if concepts.is_empty() {
        return None;
    }

    let enum_ident = format_ident!("{}", enum_name);

    // NOTE: Don't interpolate `cs` directly in `quote!` (it doesn't implement `ToTokens`).
    // Convert needed values into string literal tokens first.
    let system_lit = syn::LitStr::new(&cs.url, proc_macro2::Span::call_site());

    let version_tokens: TokenStream = match cs.version.as_deref() {
        Some(v) => {
            let v_lit = syn::LitStr::new(v, proc_macro2::Span::call_site());
            quote!(Some(#v_lit))
        }
        None => quote!(None),
    };

    let mut docs = vec![
        format!("FHIR CodeSystem: {}", enum_name),
        format!("Canonical URL: {}", cs.url),
    ];
    if let Some(v) = &cs.version {
        docs.push(format!("Version: {}", v));
    }
    if let Some(t) = &cs.title {
        docs.push(format!("Title: {}", t));
    }
    if let Some(st) = &cs.status {
        docs.push(format!("Status: {}", st));
    }
    if let Some(desc) = &cs.description {
        docs.push(desc.clone());
    }
    let enum_docs = doc_attrs(&docs);

    let mut variant_idents: Vec<proc_macro2::Ident> = Vec::new();
    let mut variant_codes: Vec<String> = Vec::new();
    let mut variant_doc_attrs: Vec<proc_macro2::TokenStream> = Vec::new();

    for c in concepts {
        let base_name = to_rust_ident_pascal(&c.code);

        // Ensure unique variant identifiers within the enum
        let mut name = base_name.clone();
        let mut suffix: usize = 2;
        while variant_idents
            .iter()
            .any(|existing: &proc_macro2::Ident| existing.to_string() == name)
        {
            name = format!("{}_{}", base_name, suffix);
            suffix += 1;
        }

        let var_ident = format_ident!("{}", name);

        let mut vdocs = Vec::new();
        if let Some(d) = &c.display {
            vdocs.push(format!("Display: {}", d));
        }
        if let Some(defn) = &c.definition {
            vdocs.push(format!("Definition: {}", defn));
        }
        for cm in concept_comments(c) {
            vdocs.push(format!("Comment: {}", cm));
        }

        variant_idents.push(var_ident);
        variant_codes.push(c.code.clone());
        variant_doc_attrs.push(doc_attrs(&vdocs));
    }

    let as_code_arms = variant_idents
        .iter()
        .zip(variant_codes.iter())
        .map(|(ident, code)| quote!(#enum_ident::#ident => #code));

    let tryfrom_arms = variant_idents
        .iter()
        .zip(variant_codes.iter())
        .map(|(ident, code)| quote!(#code => Ok(#enum_ident::#ident)));

    Some(quote! {
        #enum_docs
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum #enum_ident {
            #(
                #variant_doc_attrs
                #[serde(rename = #variant_codes)]
                #variant_idents,
            )*
        }

        impl #enum_ident {
            pub fn as_code(&self) -> &'static str {
                match self {
                    #(#as_code_arms,)*
                }
            }
            pub fn system() -> &'static str {
                #system_lit
            }

            pub fn version() -> Option<&'static str> {
                #version_tokens
            }

            /// Convert this code into a FHIR `code` primitive (`Code = Element<String, Extension>`).
            ///
            /// Useful for elements like `Patient.gender` that are bound directly to a ValueSet and use
            /// the `code` datatype.
            pub fn to_code(self) -> Code {
                Element {
                    id: None,
                    extension: None,
                    value: Some(self.as_code().to_string()),
                }
            }

            /// Convert this code into a FHIR `Coding` with `system` and (if available) `version` set.
            ///
            /// Useful for `Coding` fields and for inclusion inside `CodeableConcept.coding`.
            pub fn to_coding(self) -> Coding {
                let system: Uri = Element {
                    id: None,
                    extension: None,
                    value: Some(Self::system().to_string()),
                };

                Coding {
                    id: None,
                    extension: None,
                    system: Some(system),
                    version: Self::version().map(|v| FhirString {
                            id: None,
                            extension: None,
                            value: Some(v.to_string()),
                        }),
                    code: Some(self.to_code()),
                    display: None,
                    user_selected: Option::<Boolean>::None,
                }
            }

            /// Convert this code into a minimal `CodeableConcept` (with a single `coding`).
            ///
            /// This is especially useful for bound elements that use `CodeableConcept` (e.g.
            /// extensible bindings).
            pub fn to_codeable_concept(self) -> CodeableConcept {
                CodeableConcept {
                    id: None,
                    extension: None,
                    coding: Some(vec![self.to_coding()]),
                    text: None,
                }
            }
            /// Parse a code string into this enum.
            ///
            /// This is a convenience wrapper around the `TryFrom<&str>` implementation.
            pub fn try_from_code(code: &str) -> Result<Self, ()> {
                <Self as core::convert::TryFrom<&str>>::try_from(code)
            }

            /// Parse from a FHIR `code` primitive (`Code = Element<String, Extension>`).
            pub fn try_from_code_element(code: &Code) -> Result<Self, ()> {
                match code.value.as_deref() {
                    Some(v) => Self::try_from_code(v),
                    None => Err(()),
                }
            }
        }

        impl core::convert::TryFrom<&str> for #enum_ident {
            type Error = ();

            fn try_from(s: &str) -> Result<Self, <Self as core::convert::TryFrom<&str>>::Error> {
                match s {
                    #(#tryfrom_arms,)*
                    _ => Err(()),
                }
            }
        }

        impl core::fmt::Display for #enum_ident {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(self.as_code())
            }
        }
    })
}

fn generate_valueset_wrapper_tokens(
    type_name: &str,
    vs: &ValueSet,
    cs_by_url: &HashMap<String, (String, String)>,
    cs_concepts_by_url: &HashMap<String, Vec<Concept>>,
) -> Option<TokenStream> {
    let type_ident = format_ident!("{}", type_name);

    let url_lit = syn::LitStr::new(&vs.url, proc_macro2::Span::call_site());

    let version_tokens: TokenStream = match vs.version.as_deref() {
        Some(v) => {
            let v_lit = syn::LitStr::new(v, proc_macro2::Span::call_site());
            quote!(Some(#v_lit))
        }
        None => quote!(None),
    };

    // -------- Collect membership rules --------
    // We support explicit membership when:
    // - expansion.contains is present (preferred)
    // - compose.include[].concept[] is present
    // We support explicit exclusion via compose.exclude[].concept[].
    // Any terminology filters or include.valueSet references are marked as non-local.

    let mut include_systems: Vec<String> = Vec::new();
    let mut include_pairs: Vec<(String, String)> = Vec::new();
    // Rich explicit concepts: (system, code, display, definition)
    let mut include_entries: Vec<(String, String, Option<String>, Option<String>)> = Vec::new();
    let mut exclude_pairs: Vec<(String, String)> = Vec::new();
    let mut has_nonlocal_rules = false;
    let mut include_value_sets: Vec<String> = Vec::new();
    // Systems included without an inline concept list AND without a locally-enumerated CodeSystem.
    // These require a terminology server for definitive membership checks.
    let mut include_whole_systems: Vec<String> = Vec::new();

    if let Some(compose) = vs.compose.as_ref() {
        if let Some(includes) = compose.include.as_ref() {
            for inc in includes {
                if let Some(sys) = inc.system.as_ref() {
                    include_systems.push(sys.clone());

                    if let Some(concepts) = inc.concept.as_ref() {
                        // Inline enumerated concepts defined directly in the ValueSet
                        for c in concepts {
                            include_pairs.push((sys.clone(), c.code.clone()));
                            include_entries.push((
                                sys.clone(),
                                c.code.clone(),
                                c.display.clone(),
                                valueset_concept_definition(c).map(|s| s.to_string()),
                            ));
                        }
                    } else if let Some(cs_concepts) = cs_concepts_by_url.get(sys) {
                        // Whole CodeSystem include without inline concepts.
                        // If the CodeSystem is present in the bundle and has a finite concept list,
                        // pull its codes into the local allow-list so helpers like `display()` work.
                        for c in cs_concepts {
                            include_pairs.push((sys.clone(), c.code.clone()));
                            include_entries.push((
                                sys.clone(),
                                c.code.clone(),
                                c.display.clone(),
                                c.definition.clone(),
                            ));
                        }
                    } else {
                        // Whole-system include, but we don't have a locally-enumerated CodeSystem.
                        // Mark as requiring terminology-server support.
                        include_whole_systems.push(sys.clone());
                        has_nonlocal_rules = true;
                    }
                }
                if inc.filter.as_ref().map(|v| !v.is_empty()).unwrap_or(false) {
                    has_nonlocal_rules = true;
                }
                if let Some(vss) = inc.value_set.as_ref() {
                    if !vss.is_empty() {
                        has_nonlocal_rules = true;
                        for u in vss {
                            include_value_sets.push(u.clone());
                        }
                    }
                }
            }
        }

        if let Some(excludes) = compose.exclude.as_ref() {
            for exc in excludes {
                if let Some(sys) = exc.system.as_ref() {
                    // Do NOT add excluded systems into include_systems.
                    if let Some(concepts) = exc.concept.as_ref() {
                        for c in concepts {
                            exclude_pairs.push((sys.clone(), c.code.clone()));
                        }
                    }
                }
                if exc.filter.as_ref().map(|v| !v.is_empty()).unwrap_or(false) {
                    has_nonlocal_rules = true;
                }
            }
        }
    }

    // expansion.contains explicit allow list (preferred)
    let mut expansion_pairs: Vec<(String, String)> = Vec::new();
    if let Some(exp) = vs.expansion.as_ref() {
        if let Some(nodes) = exp.contains.as_ref() {
            flatten_expansion_contains(nodes, &mut expansion_pairs);
        }
    }

    // De-duplicate systems
    include_systems.sort();
    include_systems.dedup();
    include_value_sets.sort();
    include_value_sets.dedup();
    include_whole_systems.sort();
    include_whole_systems.dedup();
    include_entries.sort_by(|a, b| (a.0.as_str(), a.1.as_str()).cmp(&(b.0.as_str(), b.1.as_str())));
    include_entries.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

    let sys_lits: Vec<syn::LitStr> = include_systems
        .iter()
        .map(|s| syn::LitStr::new(s, proc_macro2::Span::call_site()))
        .collect();
    let vs_lits: Vec<syn::LitStr> = include_value_sets
        .iter()
        .map(|s| syn::LitStr::new(s, proc_macro2::Span::call_site()))
        .collect();
    let whole_sys_lits: Vec<syn::LitStr> = include_whole_systems
        .iter()
        .map(|s| syn::LitStr::new(s, proc_macro2::Span::call_site()))
        .collect();

    // Render pairs as literal arrays for membership tests
    let include_pair_tokens: Vec<TokenStream> = include_pairs
        .iter()
        .map(|(s, c)| {
            let s = syn::LitStr::new(s, proc_macro2::Span::call_site());
            let c = syn::LitStr::new(c, proc_macro2::Span::call_site());
            quote!((#s, #c))
        })
        .collect();

    let include_entry_tokens: Vec<TokenStream> = include_entries
        .iter()
        .map(|(s, c, d, def)| {
            let s_lit = syn::LitStr::new(s, proc_macro2::Span::call_site());
            let c_lit = syn::LitStr::new(c, proc_macro2::Span::call_site());

            let d_tokens: TokenStream = match d.as_deref() {
                Some(v) if !v.is_empty() => {
                    let lit = syn::LitStr::new(v, proc_macro2::Span::call_site());
                    quote!(Some(#lit))
                }
                _ => quote!(None),
            };

            let def_tokens: TokenStream = match def.as_deref() {
                Some(v) if !v.is_empty() => {
                    let lit = syn::LitStr::new(v, proc_macro2::Span::call_site());
                    quote!(Some(#lit))
                }
                _ => quote!(None),
            };

            quote!((#s_lit, #c_lit, #d_tokens, #def_tokens))
        })
        .collect();

    let exclude_pair_tokens: Vec<TokenStream> = exclude_pairs
        .iter()
        .map(|(s, c)| {
            let s = syn::LitStr::new(s, proc_macro2::Span::call_site());
            let c = syn::LitStr::new(c, proc_macro2::Span::call_site());
            quote!((#s, #c))
        })
        .collect();

    let expansion_pair_tokens: Vec<TokenStream> = expansion_pairs
        .iter()
        .map(|(s, c)| {
            let s = syn::LitStr::new(s, proc_macro2::Span::call_site());
            let c = syn::LitStr::new(c, proc_macro2::Span::call_site());
            quote!((#s, #c))
        })
        .collect();

    // Build match arms for whole-system validation when we don't have explicit lists.
    // If we have a generated CodeSystem enum for that system, call Enum::try_from_code(code).
    // Special-case the RGB grammar system.
    let mut match_arms: Vec<TokenStream> = Vec::new();

    for sys in &include_systems {
        if sys == "http://hl7.org/fhir/color-rgb" {
            let sys_lit = syn::LitStr::new(sys, proc_macro2::Span::call_site());
            match_arms.push(quote! {
                #sys_lit => is_rgb_hex(code),
            });
            continue;
        }

        if let Some((module, enum_name)) = cs_by_url.get(sys) {
            let mod_ident = format_ident!("{}", module);
            let enum_ident = format_ident!("{}", enum_name);
            let sys_lit = syn::LitStr::new(sys, proc_macro2::Span::call_site());
            match_arms.push(quote! {
                #sys_lit => super::super::code_systems::#mod_ident::#enum_ident::try_from_code(code).is_ok(),
            });
        }
    }

    let has_nonlocal_rules_tokens = if has_nonlocal_rules {
        quote!(true)
    } else {
        quote!(false)
    };

    // Docs
    let mut docs = vec![
        format!("FHIR ValueSet: {}", type_name),
        format!("Canonical URL: {}", vs.url),
    ];
    if let Some(v) = &vs.version {
        docs.push(format!("Version: {}", v));
    }
    if let Some(t) = &vs.title {
        docs.push(format!("Title: {}", t));
    }
    if let Some(st) = &vs.status {
        docs.push(format!("Status: {}", st));
    }
    if let Some(desc) = &vs.description {
        docs.push(desc.clone());
    }

    if !expansion_pairs.is_empty() {
        docs.push(format!("Expansion contains {} explicit codes", expansion_pairs.len()));
    }
    if !include_pairs.is_empty() {
        docs.push(format!("Compose includes {} explicit concept codes", include_pairs.len()));
    }
    if !exclude_pairs.is_empty() {
        docs.push(format!("Compose excludes {} explicit concept codes", exclude_pairs.len()));
    }
    if has_nonlocal_rules {
        docs.push(
            "Contains terminology rules (filters and/or include.valueSet) that are not evaluated locally"
                .to_string(),
        );
    }
    if !include_systems.is_empty() {
        docs.push("Includes systems:".to_string());
        for s in &include_systems {
            docs.push(format!("- {}", s));
        }
    }
    if !include_value_sets.is_empty() {
        docs.push("Includes other ValueSets:".to_string());
        for u in &include_value_sets {
            docs.push(format!("- {}", u));
        }
    }
    if !include_whole_systems.is_empty() {
        docs.push("Includes non-enumerated whole systems (requires terminology server for definitive validation):".to_string());
        for s in &include_whole_systems {
            docs.push(format!("- {}", s));
        }
    }

    let doc_tokens = doc_attrs(&docs);

    Some(quote! {
        #doc_tokens
        pub struct #type_ident;

        impl #type_ident {
            pub const URL: &'static str = #url_lit;
            pub const HAS_NONLOCAL_RULES: bool = #has_nonlocal_rules_tokens;

            pub fn version() -> Option<&'static str> {
                #version_tokens
            }

            pub fn include_systems() -> &'static [&'static str] {
                &[#(#sys_lits),*]
            }

            pub fn include_value_sets() -> &'static [&'static str] {
                &[#(#vs_lits),*]
            }

            /// Systems that are included as whole CodeSystems but are not locally enumerable.
            ///
            /// If this is non-empty, callers should use a terminology server for definitive validation.
            pub fn include_whole_systems() -> &'static [&'static str] {
                &[#(#whole_sys_lits),*]
            }

            /// Returns true only when this ValueSet can be treated as fully locally checkable.
            ///
            /// If `HAS_NONLOCAL_RULES` is true (filters, include.valueSet, or non-enumerated whole-system includes),
            /// local checks are best-effort and may require a terminology server.
            pub fn is_locally_enumerated() -> bool {
                !Self::HAS_NONLOCAL_RULES
                    && (!Self::expansion_pairs().is_empty() || !Self::include_pairs().is_empty())
            }

            fn include_pairs() -> &'static [(&'static str, &'static str)] {
                &[#(#include_pair_tokens),*]
            }

            fn include_entries() -> &'static [(&'static str, &'static str, Option<&'static str>, Option<&'static str>)] {
                &[#(#include_entry_tokens),*]
            }

            /// Return the preferred display text for a (system, code) pair, if present in this ValueSet's explicit concepts.
            pub fn display(system: &str, code: &str) -> Option<&'static str> {
                for (s, c, d, _def) in Self::include_entries() {
                    if *s == system && *c == code {
                        return *d;
                    }
                }
                None
            }

            /// Return the ValueSet concept definition for a (system, code) pair, if present.
            ///
            /// In `valuesets.json` this often comes from the extension:
            /// `http://hl7.org/fhir/StructureDefinition/valueset-concept-definition`.
            pub fn definition(system: &str, code: &str) -> Option<&'static str> {
                for (s, c, _d, def) in Self::include_entries() {
                    if *s == system && *c == code {
                        return *def;
                    }
                }
                None
            }

            /// Create a minimal `Coding` from a (system, code) pair.
            ///
            /// If this ValueSet provides an explicit display for the code, it will be populated.
            pub fn to_coding(system: &str, code: &str) -> Coding {
                let sys_uri: Uri = Element {
                    id: None,
                    extension: None,
                    value: Some(system.to_string()),
                };

                let code_el: Code = Element {
                    id: None,
                    extension: None,
                    value: Some(code.to_string()),
                };

                let display = Self::display(system, code).map(|s| FhirString {
                    id: None,
                    extension: None,
                    value: Some(s.to_string()),
                });

                Coding {
                    id: None,
                    extension: None,
                    system: Some(sys_uri),
                    version: Self::version().map(|v| FhirString {
                        id: None,
                        extension: None,
                        value: Some(v.to_string()),
                    }),
                    code: Some(code_el),
                    display,
                    user_selected: None,
                }
            }

            fn exclude_pairs() -> &'static [(&'static str, &'static str)] {
                &[#(#exclude_pair_tokens),*]
            }

            fn expansion_pairs() -> &'static [(&'static str, &'static str)] {
                &[#(#expansion_pair_tokens),*]
            }

            /// Best-effort local validation for a `Coding` against this ValueSet.
            ///
            /// Rules:
            /// - Explicit excludes (compose.exclude.concept) always deny.
            /// - If expansion.contains exists, it is treated as an explicit allow-list.
            /// - Else if compose.include.concept exists, it is treated as an explicit allow-list.
            /// - Else: fall back to whole-system validation when possible (generated CodeSystem enums / RGB grammar).
            ///
            /// Note: terminology filters and include.valueSet references are NOT evaluated locally.
            pub fn contains_coding(coding: &Coding) -> bool {
                let system = match &coding.system {
                    Some(s) => s.value.as_deref().unwrap_or(""),
                    None => return false,
                };

                let code = match &coding.code {
                    Some(c) => c.value.as_deref().unwrap_or(""),
                    None => return false,
                };

                // Apply explicit excludes first
                if Self::exclude_pairs().iter().any(|(s, c)| *s == system && *c == code) {
                    return false;
                }

                // 1) Prefer expansion explicit list
                let exp = Self::expansion_pairs();
                if !exp.is_empty() {
                    return exp.iter().any(|(s, c)| *s == system && *c == code);
                }

                // 2) Then explicit include concepts
                let inc = Self::include_pairs();
                if !inc.is_empty() {
                    return inc.iter().any(|(s, c)| *s == system && *c == code);
                }

                // 3) Fallback to best-effort whole-system validation
                match system {
                    #(#match_arms)*
                    _ => false,
                }
            }

            /// Best-effort local validation for a `CodeableConcept` against this ValueSet.
            ///
            /// Returns true if any `coding` entry is contained in the ValueSet.
            pub fn contains_codeable_concept(cc: &CodeableConcept) -> bool {
                match cc.coding.as_ref() {
                    Some(codings) => codings.iter().any(|c| Self::contains_coding(c)),
                    None => false,
                }
            }

            /// Convenience: validate a FHIR `code` primitive when the binding is on `code`.
            ///
            /// This only works reliably when the ValueSet includes exactly one system.
            pub fn contains_code(code: &Code) -> bool {
                let code_str = match code.value.as_deref() {
                    Some(v) => v,
                    None => return false,
                };

                let systems = Self::include_systems();
                if systems.len() != 1 {
                    return false;
                }

                // Build a synthetic Coding and reuse contains_coding()
                let system: Uri = Element {
                    id: None,
                    extension: None,
                    value: Some(systems[0].to_string()),
                };

                let code: Code = Element {
                    id: None,
                    extension: None,
                    value: Some(code_str.to_string()),
                };

                let coding = Coding {
                    id: None,
                    extension: None,
                    system: Some(system),
                    version: None,
                    code: Some(code),
                    display: None,
                    user_selected: None,
                };

                Self::contains_coding(&coding)
            }
            /// Create a minimal `CodeableConcept` (single coding) from a (system, code) pair.
            pub fn to_codeable_concept(system: &str, code: &str) -> CodeableConcept {
                let coding = Self::to_coding(system, code);
                CodeableConcept {
                    id: None,
                    extension: None,
                    coding: Some(vec![coding]),
                    text: None,
                }
            }
        }

        impl super::super::bindings::ValueSetMembership<Code> for #type_ident {
            fn contains(v: &Code) -> bool {
                Self::contains_code(v)
            }
        }

        impl super::super::bindings::ValueSetMembership<Coding> for #type_ident {
            fn contains(v: &Coding) -> bool {
                Self::contains_coding(v)
            }
        }

        impl super::super::bindings::ValueSetMembership<CodeableConcept> for #type_ident {
            fn contains(v: &CodeableConcept) -> bool {
                Self::contains_codeable_concept(v)
            }
        }

        fn is_rgb_hex(code: &str) -> bool {
            // #RRGGBB, case-insensitive hex
            let b = code.as_bytes();
            if b.len() != 7 || b[0] != b'#' {
                return false;
            }
            fn is_hex(x: u8) -> bool {
                (b'0'..=b'9').contains(&x)
                    || (b'a'..=b'f').contains(&x)
                    || (b'A'..=b'F').contains(&x)
            }
            is_hex(b[1]) && is_hex(b[2]) && is_hex(b[3]) && is_hex(b[4]) && is_hex(b[5]) && is_hex(b[6])
        }
    })
}
// Generate a stub module for CodeSystems that do not enumerate concepts.
fn generate_codesystem_stub_tokens(type_name: &str, cs: &CodeSystem) -> TokenStream {
    let type_ident = format_ident!("{}", type_name);
    let system_lit = syn::LitStr::new(&cs.url, proc_macro2::Span::call_site());

    let version_tokens: TokenStream = match cs.version.as_deref() {
        Some(v) => {
            let v_lit = syn::LitStr::new(v, proc_macro2::Span::call_site());
            quote!(Some(#v_lit))
        }
        None => quote!(None),
    };

    let mut docs = vec![
        format!("FHIR CodeSystem (non-enumerated): {}", type_name),
        format!("Canonical URL: {}", cs.url),
    ];
    if let Some(v) = &cs.version { docs.push(format!("Version: {}", v)); }
    if let Some(t) = &cs.title { docs.push(format!("Title: {}", t)); }
    if let Some(st) = &cs.status { docs.push(format!("Status: {}", st)); }
    if let Some(desc) = &cs.description { docs.push(desc.clone()); }

    // Note about why no enum exists
    docs.push("NOTE: This CodeSystem does not enumerate concepts in valuesets.json (e.g. content=not-present or empty concept list).".to_string());
    docs.push("A full terminology server is required to validate membership for arbitrary codes.".to_string());

    let doc_tokens = doc_attrs(&docs);

    let rgb = cs.url == "http://hl7.org/fhir/color-rgb";
    let is_valid_body: TokenStream = if rgb {
        quote! {
            if code.len() != 7 { return false; }
            let bytes = code.as_bytes();
            if bytes[0] != b'#' { return false; }
            code[1..].bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F'))
        }
    } else {
        quote!({ let _ = code; false })
    };

    quote! {
        #doc_tokens
        pub struct #type_ident;

        impl #type_ident {
            pub const URL: &'static str = #system_lit;

            pub fn system() -> &'static str {
                Self::URL
            }

            pub fn version() -> Option<&'static str> {
                #version_tokens
            }

            /// Best-effort local validation.
            ///
            /// Returns `true` only for systems where we can validate by grammar locally.
            /// Otherwise returns `false` (unknown locally).
            pub fn is_valid_code(code: &str) -> bool {
                #is_valid_body
            }
        }
    }
}