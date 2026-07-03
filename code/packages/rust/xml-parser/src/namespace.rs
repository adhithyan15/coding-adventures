//! Namespace resolution and entity decoding — the two "context-sensitive"
//! jobs the grammar can't do.
//!
//! The grammar in `xml.grammar` produces a raw tree of tokens; it knows
//! nothing about namespaces or entities. This module holds the two helpers
//! that turn raw lexeme text into resolved names and decoded strings.

use std::collections::HashMap;

use crate::ast::ParseError;

// ===========================================================================
// The namespace scope stack
// ===========================================================================

/// A stack of namespace bindings, one frame per open element.
///
/// # Why a stack?
///
/// Namespace declarations are *scoped* to the element they appear on and to
/// that element's descendants. When we enter `<w:body>` we push a frame; when
/// we leave it we pop, so a binding declared inside `<w:body>` cannot leak out
/// to a later sibling. Resolution walks the stack from the top (innermost)
/// down, so an inner declaration shadows an outer one with the same prefix.
///
/// Two prefixes are always bound, per the XML Namespaces spec, and callers
/// never declare them:
///
/// - `xml`   → `http://www.w3.org/XML/1998/namespace`
/// - `xmlns` → `http://www.w3.org/2000/xmlns/`
#[derive(Debug, Default)]
pub struct NamespaceStack {
    /// One map per scope. `frames[0]` is the outermost element's declarations.
    /// A key of `""` is the *default* namespace (declared with `xmlns="..."`).
    frames: Vec<HashMap<String, String>>,
}

/// The reserved URI bound to the `xml` prefix.
pub const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";
/// The reserved URI bound to the `xmlns` prefix.
pub const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";

impl NamespaceStack {
    /// Create an empty stack (no user declarations yet).
    pub fn new() -> Self {
        NamespaceStack { frames: Vec::new() }
    }

    /// Push a new scope. Call this on entering a start tag, *before* recording
    /// that tag's `xmlns` declarations, so those declarations land in the new
    /// frame and are popped when the element closes.
    pub fn push(&mut self, declarations: HashMap<String, String>) {
        self.frames.push(declarations);
    }

    /// Pop the innermost scope. Call this when an element closes.
    pub fn pop(&mut self) {
        self.frames.pop();
    }

    /// Resolve a *prefix* to a namespace URI.
    ///
    /// - An empty prefix (`""`) asks for the default namespace; if none is in
    ///   scope the result is `None` (the "no namespace" case).
    /// - The reserved `xml` / `xmlns` prefixes always resolve.
    /// - Any other unbound prefix is an error (a caller decides whether to
    ///   surface it).
    pub fn resolve_prefix(&self, prefix: &str) -> Result<Option<String>, ParseError> {
        if prefix == "xml" {
            return Ok(Some(XML_NAMESPACE.to_string()));
        }
        if prefix == "xmlns" {
            return Ok(Some(XMLNS_NAMESPACE.to_string()));
        }
        // Walk from innermost frame outward.
        for frame in self.frames.iter().rev() {
            if let Some(uri) = frame.get(prefix) {
                // A binding to the empty string "undeclares" the default
                // namespace (`xmlns=""`), which means "no namespace".
                if uri.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(uri.clone()));
            }
        }
        if prefix.is_empty() {
            // No default namespace in scope → element is in no namespace.
            Ok(None)
        } else {
            Err(ParseError {
                message: format!("unbound namespace prefix '{prefix}'"),
                line: None,
                column: None,
            })
        }
    }
}

// ===========================================================================
// Splitting a qualified name
// ===========================================================================

/// Split a raw name like `w:p` into `(prefix, local)`.
///
/// A name with no colon has an empty prefix. A name is malformed if it has
/// more than one colon, an empty prefix before a colon (`:p`), or an empty
/// local part (`w:`).
pub fn split_qname(qname: &str) -> Result<(&str, &str), ParseError> {
    match qname.split_once(':') {
        None => Ok(("", qname)),
        Some((prefix, local)) => {
            if prefix.is_empty() || local.is_empty() || local.contains(':') {
                Err(ParseError {
                    message: format!("malformed qualified name '{qname}'"),
                    line: None,
                    column: None,
                })
            } else {
                Ok((prefix, local))
            }
        }
    }
}

// ===========================================================================
// Entity and character reference decoding
// ===========================================================================

/// Decode XML entity and character references inside a run of text.
///
/// This handles the five predefined entities and both flavours of numeric
/// character reference:
///
/// | Reference        | Decodes to                     |
/// |------------------|--------------------------------|
/// | `&amp;`          | `&`                            |
/// | `&lt;`           | `<`                            |
/// | `&gt;`           | `>`                            |
/// | `&apos;`         | `'`                            |
/// | `&quot;`         | `"`                            |
/// | `&#N;` (decimal) | the Unicode code point `N`     |
/// | `&#xH;` (hex)    | the Unicode code point `0xH`   |
///
/// CDATA content must **not** be passed through this function — it is verbatim.
///
/// An `&` that does not begin a well-formed reference is an error, which is
/// what a strict XML parser should do.
pub fn decode_entities(input: &str) -> Result<String, ParseError> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if ch != '&' {
            out.push(ch);
            continue;
        }
        // Gather everything up to the terminating ';'.
        let mut name = String::new();
        let mut terminated = false;
        for (_, c) in chars.by_ref() {
            if c == ';' {
                terminated = true;
                break;
            }
            name.push(c);
        }
        if !terminated {
            return Err(ParseError {
                message: "unterminated entity reference (missing ';')".to_string(),
                line: None,
                column: None,
            });
        }
        out.push_str(&decode_one_reference(&name)?);
    }

    Ok(out)
}

/// Decode the body of a single reference — the text between `&` and `;`.
fn decode_one_reference(name: &str) -> Result<String, ParseError> {
    // Numeric character references start with '#'.
    if let Some(rest) = name.strip_prefix('#') {
        let code = if let Some(hex) = rest.strip_prefix('x').or_else(|| rest.strip_prefix('X')) {
            u32::from_str_radix(hex, 16).map_err(|_| bad_ref(name))?
        } else {
            rest.parse::<u32>().map_err(|_| bad_ref(name))?
        };
        let ch = char::from_u32(code).ok_or_else(|| ParseError {
            message: format!("character reference '&#{name};' is not a valid code point"),
            line: None,
            column: None,
        })?;
        return Ok(ch.to_string());
    }

    // Named entity references — only the five predefined ones are supported
    // (this parser has no DTD, so no custom entities exist).
    let decoded = match name {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "apos" => "'",
        "quot" => "\"",
        _ => {
            return Err(ParseError {
                message: format!("unknown entity reference '&{name};'"),
                line: None,
                column: None,
            })
        }
    };
    Ok(decoded.to_string())
}

fn bad_ref(name: &str) -> ParseError {
    ParseError {
        message: format!("malformed character reference '&{name};'"),
        line: None,
        column: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // --- NamespaceStack::resolve_prefix ---

    #[test]
    fn reserved_prefixes_resolve() {
        let ns = NamespaceStack::new();
        assert_eq!(ns.resolve_prefix("xml").unwrap().as_deref(), Some(XML_NAMESPACE));
        assert_eq!(
            ns.resolve_prefix("xmlns").unwrap().as_deref(),
            Some(XMLNS_NAMESPACE)
        );
    }

    #[test]
    fn empty_prefix_with_no_default_is_no_namespace() {
        let ns = NamespaceStack::new();
        assert_eq!(ns.resolve_prefix("").unwrap(), None);
    }

    #[test]
    fn unbound_prefix_errors() {
        let ns = NamespaceStack::new();
        assert!(ns.resolve_prefix("nope").is_err());
    }

    #[test]
    fn empty_uri_binding_undeclares_default() {
        // `xmlns=""` maps the default prefix to the empty string, which
        // resolve_prefix treats as "no namespace".
        let mut ns = NamespaceStack::new();
        let mut frame = HashMap::new();
        frame.insert(String::new(), String::new());
        ns.push(frame);
        assert_eq!(ns.resolve_prefix("").unwrap(), None);
    }

    #[test]
    fn push_and_pop_scope() {
        let mut ns = NamespaceStack::new();
        let mut frame = HashMap::new();
        frame.insert("p".to_string(), "uri".to_string());
        ns.push(frame);
        assert_eq!(ns.resolve_prefix("p").unwrap().as_deref(), Some("uri"));
        ns.pop();
        // After popping, the binding is gone.
        assert!(ns.resolve_prefix("p").is_err());
    }

    // --- split_qname ---

    #[test]
    fn split_qname_plain() {
        assert_eq!(split_qname("name").unwrap(), ("", "name"));
    }

    #[test]
    fn split_qname_prefixed() {
        assert_eq!(split_qname("w:p").unwrap(), ("w", "p"));
    }

    #[test]
    fn split_qname_rejects_malformed() {
        assert!(split_qname(":local").is_err()); // empty prefix
        assert!(split_qname("prefix:").is_err()); // empty local
        assert!(split_qname("a:b:c").is_err()); // two colons
    }

    // --- decode_entities ---

    #[test]
    fn decode_named_entities() {
        assert_eq!(decode_entities("a&amp;b&lt;c&gt;d&apos;e&quot;f").unwrap(), "a&b<c>d'e\"f");
    }

    #[test]
    fn decode_decimal_and_hex_refs() {
        assert_eq!(decode_entities("&#65;&#x42;&#X43;").unwrap(), "ABC");
    }

    #[test]
    fn decode_plain_text_untouched() {
        assert_eq!(decode_entities("no entities here").unwrap(), "no entities here");
    }

    #[test]
    fn decode_rejects_unterminated() {
        assert!(decode_entities("a &amp b").unwrap_err().message.contains("unterminated"));
    }

    #[test]
    fn decode_rejects_unknown_entity() {
        assert!(decode_entities("&bogus;").unwrap_err().message.contains("unknown"));
    }

    #[test]
    fn decode_rejects_bad_numeric_ref() {
        assert!(decode_entities("&#zz;").is_err());
        assert!(decode_entities("&#xZZ;").is_err());
    }

    #[test]
    fn decode_rejects_out_of_range_code_point() {
        // 0xD800 is a surrogate — not a valid scalar value.
        assert!(decode_entities("&#xD800;").is_err());
        // Beyond the Unicode range.
        assert!(decode_entities("&#x110000;").is_err());
    }
}
