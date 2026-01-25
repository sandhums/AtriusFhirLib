# Atrius FHIR Validation Architecture

This document describes how Atrius performs FHIR validation using:
- generated constraints (FHIR invariants)
- generated ValueSet binding membership checks
- optional remote terminology validation

It is intended for developers working on:
- `atrius-macros` (`#[derive(FhirValidate)]`)
- `atrius-fhirpath-support` (validation contracts)
- `atrius-fhir-path` (engine + terminology provider)
- `atrius-value-set-gen` (ValueSet code generation)

---

## Components and Responsibilities

### 1) `atrius-fhirpath-support`
**Path:** `crates/AtriusFhirPathSupport/src/validate.rs`

Defines core validation contracts and data structures:

- `ValidationSeverity` — `Error | Warning`
- `ValidationIssue` — emitted issues with declared `path` and concrete `instance_path`
- `Invariant` — compiled constraint definition (`key`, `severity`, `expr`, `path`, `human`)
- `FhirPathEngine` — evaluation + terminology hooks:
    - `eval_bool(focus, expr) -> Result<bool, EvaluationError>`
    - `validate_code_in_valueset(valueset_url, system, code) -> Option<bool>`
- `FhirValidate` — trait for generated validators, defines `invariants()` and `validate_with_engine()`

**Design choice:** Terminology validation is separate from FHIRPath evaluation because FHIRPath is pure
expression evaluation; terminology requires IO and is environment-dependent.

---

### 2) `atrius-macros` (`#[derive(FhirValidate)]`)
**Path:** `crates/AtriusMacros/src/fhir_validate.rs`

Generates resource-specific validation logic at compile time.

For structs (resources / complex types), generated validation includes:
1. type-level invariants (struct-level `#[fhir_invariant]`)
2. field-level invariants (field-level `#[fhir_invariant]`)
3. ValueSet binding checks (field-level `#[fhir_binding]`)
4. recursion into nested complex types
5. instance path rewriting and issue de-duplication

For enums (FHIR choice types), generated validation delegates to the active variant.

---

### 3) ValueSet code generation (`atrius-value-set-gen`)
**Path:** `crates/AtriusValueSetGen/src/main.rs`

Generates ValueSet modules (e.g. `crates/AtriusFhir/src/r5/terminology/value_sets/*`) with:
- local membership logic (`contains_code`, `contains_coding`, `contains_codeable_concept`)
- per-ValueSet flags describing locality/nonlocal rules:
    - `HAS_NONLOCAL_RULES: bool`
    - `is_locally_enumerated() -> bool`
- dispatch tables keyed by ValueSet URL:
    - `binding_ops_by_url(url) -> Option<&'static ValueSetBindingOps>`
    - `valueset_has_nonlocal_rules(url) -> Option<bool>`
    - `valueset_is_locally_enumerated(url) -> Option<bool>`

Local checks are fast and work offline. Remote checks are required for ValueSets that reference
external CodeSystems or include complex rule composition.

---

### 4) `atrius-fhir-path`
**Path:** `crates/AtriusFhirPath/src/engine.rs`

Implements `AtriusFhirPathEngine`, which evaluates FHIRPath and optionally performs terminology lookups.

- `TerminologyProvider` — abstraction for code validation
- `HttpTerminologyProvider` — implementation that calls a FHIR terminology server (HAPI/Snowstorm/etc)
- `AtriusFhirPathEngine` — implements `FhirPathEngine::eval_bool()`
    - and provides `validate_code_in_valueset()` via configured terminology provider

Terminology implementation files:
- `crates/AtriusFhirPath/src/terminology_client.rs`
- `crates/AtriusFhirPath/src/terminology_functions.rs`

---

## Validation Pipeline (Runtime)

When a resource is validated:

```rust
let engine = AtriusFhirPathEngine::new()
  .with_terminology_provider(Arc::new(HttpTerminologyProvider::new(BASE_URL)));

let issues = patient.validate_with_engine(&engine);
```
The generated validator performs:

### Step 1 — Type-level invariants

Evaluates invariants attached to the struct itself.

### Step 2 — Field traversal

For each field, handles containers:
	•	T, Option<T>, Vec<T>, Option<Vec<T>>, Box<T>, Option<Box<T>>

Generates a concrete instance_path like:
	•	name
	•	contact[0]
	•	contact[0].telecom[1]

### Step 3 — Field invariants

Evaluates invariants attached to that field for each value instance.

### Step 4 — ValueSet binding checks

If #[fhir_binding] exists, check membership using:
1.	Local membership
Calls generated binding ops:
                                           
	•	contains_code(&Code)
	•	contains_coding(&Coding)
	•	contains_codeable_concept(&CodeableConcept)

2.	Remote terminology fallback (only when needed)
If local fails AND ValueSet indicates nonlocal rules or not locally enumerated:
call engine.validate_code_in_valueset(valueset_url, system, code).

### Tri-state result:
	•	Some(true) => valid
	•	Some(false) => invalid (emit based on binding strength)
	•	None => unknown (emit warning: “could not be verified”)

### Binding strength mapping:
	•	required => Error if confirmed invalid
	•	extensible / preferred => Warning if confirmed invalid
	•	example => typically no enforcement

### Step 5 — Recursion into children

If the value is a complex type, recurse into FhirValidate::validate_with_engine(value, engine).

Leaf rules:
	•	Element<...> and generated ...::primitives::... are treated as leaves (no recursion).

### Step 6 — De-duplicate issues

## Issues are de-duplicated by signature:
key|path|instance_path|expression

⸻

# Terminology Server Contract

Remote validation uses the FHIR operation:
ValueSet/$validate-code

Typical call:
{BASE}/ValueSet/$validate-code?url={vs_url}&system={system}&code={code}

The validator requires both:
	•	system
	•	code

because many servers enforce that pairing.

⸻

# Caching Strategy (Recommended)

Terminology validation can be expensive. The recommended place to cache is inside:
	•	terminology_client.rs (client layer), keyed by:
	•	(valueset_url, system, code, version?)

Cache values:
	•	Some(true) / Some(false) and optionally None with a short TTL.

⸻

# Troubleshooting

### “ValueSet binding could not be verified”

Usually means:
	•	terminology provider not configured, or
	•	network/HTTP error, or
	•	server returned unexpected payload

Because the result is None, the validator emits a warning rather than a hard failure.

### Local binding returns false unexpectedly

Verify:
•	the ValueSet URL used by #[fhir_binding(valueset="...")] matches the dispatch in
binding_ops_by_url(url)
•	the field’s coding actually includes the expected system and code

⸻

# Future Extensions
	•	Support CodeableConcept.text fallback behaviors (optional)
	•	Support validation with coding.version
	•	Batch $validate-code calls where servers support it
	•	Promote “unknown” outcomes to configurable policy (strict vs lenient mode)