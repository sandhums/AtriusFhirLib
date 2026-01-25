use once_cell::sync::Lazy;
use roxmltree::{Document, Node};
use std::collections::HashSet;
use atrius_fhirpath_support::evaluation_error::EvaluationError;
use atrius_fhirpath_support::evaluation_result::EvaluationResult;// adjust paths to your crate

static XHTML_NS: &str = "http://www.w3.org/1999/xhtml";

static ALLOWED_TAGS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // basic structure
        "div", "p", "span", "br", "hr",

        // headings
        "h1","h2","h3","h4","h5","h6",

        // inline formatting
        "b","i","u","em","strong","small","big",
        "sub","sup","tt","code","pre","blockquote","q","cite","dfn","samp","kbd","var",

        // lists
        "ul","ol","li","dl","dt","dd",

        // tables
        "table","thead","tbody","tfoot","tr","th","td","caption","colgroup","col",

        // special allowed
        "a", "img",
    ]
        .into_iter()
        .collect()
});

static GLOBAL_ATTRS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    // Keep tight; expand later if you need more
    ["id", "class", "title", "lang", "dir", "style", "xml:lang"]
        .into_iter()
        .collect()
});

static A_ATTRS: Lazy<HashSet<&'static str>> = Lazy::new(|| ["href", "name"].into_iter().collect());

static IMG_ATTRS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    // Keep conservative
    ["src", "alt", "title"].into_iter().collect()
});

static TD_TH_ATTRS: Lazy<HashSet<&'static str>> =
    Lazy::new(|| ["colspan", "rowspan"].into_iter().collect());

/// Implements the FHIRPath htmlChecks() function used by Narrative invariants (txt-1, txt-2).
///
/// - Requires singleton input.
/// - Returns Boolean(true/false) if input is String/XHTML-ish.
/// - Returns Empty if input is Empty.
/// - Returns Empty for non-string-ish inputs (or you can return Boolean(false) instead).
pub fn html_checks_function(
    invocation_base: &EvaluationResult,
) -> Result<EvaluationResult, EvaluationError> {
    // Singleton check (matches your style)
    if invocation_base.count() > 1 {
        return Err(EvaluationError::SingletonEvaluationError(
            "htmlChecks requires a singleton input".to_string(),
        ));
    }

    Ok(match invocation_base {
        EvaluationResult::Empty => EvaluationResult::Empty,

        // If you have a dedicated XHTML variant, handle it here too.
        // (I’m guessing you likely store Narrative.div as String anyway.)
        EvaluationResult::String(s, _) => {
            let ok = html_checks_impl(s);
            EvaluationResult::boolean(ok)
        }

        // If your engine represents XHTML as some other primitive, add cases here:
        // EvaluationResult::Xhtml(s, _) => EvaluationResult::boolean(html_checks_impl(s)),

        // Not convertible → Empty (or Boolean(false) if you want strict)
        _ => EvaluationResult::Empty,
    })
}

/// Returns true if the xhtml fragment passes both:
/// - allowed-tags/allowed-attrs safety checks (txt-1)
/// - non-whitespace content requirement (txt-2)
fn html_checks_impl(xhtml: &str) -> bool {
    if xhtml.trim().is_empty() {
        return false;
    }

    // Must be parseable XML/XHTML
    let doc = match Document::parse(xhtml) {
        Ok(d) => d,
        Err(_) => return false,
    };

    let root = doc.root_element();

    // FHIR Narrative.div is an XHTML div
    if root.tag_name().name() != "div" {
        return false;
    }
    if root.tag_name().namespace() != Some(XHTML_NS) {
        return false;
    }

    let mut has_non_ws_text = false;
    let mut has_img = false;

    for node in doc.descendants() {
        match node.node_type() {
            roxmltree::NodeType::Element => {
                if !check_element(node, &mut has_img) {
                    return false;
                }
            }
            roxmltree::NodeType::Text => {
                if node.text().map(|t| !t.trim().is_empty()).unwrap_or(false) {
                    has_non_ws_text = true;
                }
            }
            // safest: disallow comments & processing instructions
            roxmltree::NodeType::Comment | roxmltree::NodeType::PI => return false,
            _ => {}
        }
    }

    // txt-2: some non-whitespace content (text OR an image)
    has_non_ws_text || has_img
}

fn check_element(node: Node<'_, '_>, has_img: &mut bool) -> bool {
    let name = node.tag_name().name();

    // Restrict to XHTML namespace only
    if node.tag_name().namespace() != Some(XHTML_NS) {
        return false;
    }

    // Restrict tag allow-list
    if !ALLOWED_TAGS.contains(name) {
        return false;
    }

    if name == "img" {
        *has_img = true;
    }

    // Attribute allow-list + safety checks
    for attr in node.attributes() {
        let an = attr.name();

        // allow xmlns="http://www.w3.org/1999/xhtml" on root
        if an == "xmlns" {
            if node.is_root() && attr.value() == XHTML_NS {
                continue;
            }
            return false;
        }

        // forbid any event handler attrs (onclick, onload, etc.)
        if an.len() >= 2 && an[..2].eq_ignore_ascii_case("on") {
            return false;
        }

        // forbid other namespaced attrs except xml:lang
        if an.contains(':') && an != "xml:lang" {
            return false;
        }

        let allowed = GLOBAL_ATTRS.contains(an)
            || (name == "a" && A_ATTRS.contains(an))
            || (name == "img" && IMG_ATTRS.contains(an))
            || ((name == "td" || name == "th") && TD_TH_ATTRS.contains(an));

        if !allowed {
            return false;
        }

        // href/src should not allow javascript:
        if (name == "a" && an.eq_ignore_ascii_case("href"))
            || (name == "img" && an.eq_ignore_ascii_case("src"))
        {
            let v = attr.value().trim().to_ascii_lowercase();
            if v.starts_with("javascript:") {
                return false;
            }
        }

        // style should not allow obvious active content vectors
        if an.eq_ignore_ascii_case("style") {
            let s = attr.value().to_ascii_lowercase();
            if s.contains("url(") || s.contains("expression(") || s.contains("@import") {
                return false;
            }
        }
    }

    // <a> must have either href or name
    if name == "a" {
        let has_href = node.attribute("href").map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_name = node.attribute("name").map(|s| !s.trim().is_empty()).unwrap_or(false);
        if !(has_href || has_name) {
            return false;
        }
    }

    true
}