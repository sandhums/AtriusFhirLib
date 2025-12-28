use crate::element_definition;

/// Escapes a string so it can be safely embedded inside a Rust string literal.
///
/// Returns the escaped *contents* (no surrounding quotes).
fn escape_rust_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

/// Formats element constraints as executable Rust attributes.
///
/// This is separate from `format_constraints` (doc output). The returned string contains
/// raw attribute lines WITHOUT indentation; the caller should indent as needed.
pub(crate) fn format_constraint_attributes(
    constraints: &[element_definition::ElementDefinitionConstraint],
    fhir_path: &str,
) -> String {
    if constraints.is_empty() {
        return String::new();
    }

    let mut out = String::new();

    for c in constraints {
        // Only emit executable invariants when an expression is present.
        let Some(expr) = &c.expression else { continue; };

        let key = escape_rust_string_literal(&c.key);
        let severity = escape_rust_string_literal(&c.severity);
        let human = escape_rust_string_literal(&c.human);
        let expr = escape_rust_string_literal(expr);
        let path = escape_rust_string_literal(fhir_path);

        out.push_str("#[fhir_invariant(");
        out.push_str(&format!(
            "key=\"{}\", severity=\"{}\", human=\"{}\", expr=\"{}\", path=\"{}\"",
            key, severity, human, expr, path
        ));
        out.push_str(")]\n");
    }

    out
}
