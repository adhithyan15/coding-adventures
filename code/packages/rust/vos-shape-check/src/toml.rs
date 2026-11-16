//! # TOML, read with the repository's grammar-driven crates
//!
//! `Cargo.toml` decides things the Rust source never shows: a `build = ".."`
//! key compiles a script before the crate, `proc-macro = true` makes the crate
//! run at compile time, a dependency table pulls in foreign code.  To police
//! those we must *read* the manifest, and a line-by-line `grep` is easy to
//! evade (`package = { build = ".." }`, a quoted table name `["lib"]`, a
//! multi-line array whose body looks like a header).
//!
//! The repository forbids hand-written lexers and parsers, so this module is a
//! thin wrapper over `coding-adventures-toml-parser` (which runs the generic
//! `GrammarLexer` / `GrammarParser` on `code/grammars/toml/toml.{tokens,grammar}`).
//! It does three things the grammar crates leave to the consumer:
//!
//! 1. **Flatten every value to a full key path**, so the rules look at paths,
//!    however the author spelled them:
//!
//! ```text
//!   [package]                      path ["package","build"]  = Str("x")
//!   build = "x"
//!   package.build = "x"            same path
//!   package = { build = "x" }      same path
//!   "build" = "x"                  same path (quotes are not part of the key)
//!   bin = [{ path = "x.rs" }]      path ["bin","[]","path"] = Str("x.rs")
//! ```
//!
//! 2. **Fail closed on duplicates.**  The grammar accepts `a = 1` then `a = 2`
//!    (its header says key uniqueness is a post-parse pass that the crate does
//!    not run), so a repeated non-table path is an error here, never a silent
//!    last-wins.
//!
//! 3. **Refuse escapes.**  The TOML lexer strips the quotes of a string token
//!    but leaves its escapes undecoded (`"package"` arrives as the text
//!    `package`).  Decoding is a second implementation to trust, so a
//!    backslash in a basic or multi-line basic string (a key or a value) is an
//!    error: `"package".build` cannot slip past as a key we failed to
//!    normalise.  Literal strings (`'..'`) have no escapes and are fine.
//!
//! The parser's own recursion cap (165 levels) bounds nesting; anything it
//! refuses is a [`TomlError`] and so a finding (fail closed, VOS03 section 3).

use coding_adventures_toml_parser::{try_parse_toml, TomlParseError};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

/// A flattened value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// A string.
    Str(String),
    /// `true` or `false`.
    Bool(bool),
    /// A number, date or time, kept as text.
    Other(String),
    /// An array (elements that are tables are flattened separately, see
    /// [`Entry`]; they appear here as [`Value::Table`]).
    Array(Vec<Value>),
    /// An inline table or a table header; its members are separate entries
    /// with a longer path.
    Table,
}

/// One flattened `path = value` with the 1-based line it was written on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Full key path from the document root.
    pub path: Vec<String>,
    /// The value.
    pub value: Value,
    /// Line of the key.
    pub line: u32,
}

/// A parse failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TomlError {
    /// 1-based line (1 when the grammar crates give no position, as for a
    /// lexical or syntax error).
    pub line: u32,
    /// Reason.
    pub message: String,
}

fn err<T>(line: u32, message: &str) -> Result<T, TomlError> {
    Err(TomlError {
        line,
        message: message.to_string(),
    })
}

fn line_of(n: &GrammarASTNode) -> u32 {
    u32::try_from(n.start_line.unwrap_or(1)).unwrap_or(u32::MAX)
}

fn child_nodes(n: &GrammarASTNode) -> impl Iterator<Item = &GrammarASTNode> {
    n.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Node(m) => Some(m),
        ASTNodeOrToken::Token(_) => None,
    })
}

/// The one token of a node that wraps exactly one token.
fn only_token(n: &GrammarASTNode) -> Option<(&str, &str)> {
    match n.children.as_slice() {
        [ASTNodeOrToken::Token(t)] => {
            Some((t.type_name.as_deref().unwrap_or(""), t.value.as_str()))
        }
        _ => None,
    }
}

/// `a . "b" . 'c'` as its parts.
fn key_path(key: &GrammarASTNode) -> Result<Vec<String>, TomlError> {
    let line = line_of(key);
    let mut parts = Vec::new();
    for simple in child_nodes(key) {
        let Some((kind, text)) = only_token(simple) else {
            return err(line, "unsupported key form");
        };
        match kind {
            "BARE_KEY" | "LITERAL_STRING" | "TRUE" | "FALSE" | "INTEGER" => {
                parts.push(text.to_string())
            }
            "BASIC_STRING" => {
                if text.contains('\\') {
                    return err(
                        line,
                        "escape in a quoted key is not supported (fail closed)",
                    );
                }
                parts.push(text.to_string());
            }
            _ => return err(line, "unsupported key form (fail closed)"),
        }
    }
    if parts.is_empty() {
        return err(line, "empty key");
    }
    Ok(parts)
}

fn value(node: &GrammarASTNode, path: &[String], out: &mut Vec<Entry>) -> Result<Value, TomlError> {
    let line = line_of(node);
    if let Some((kind, text)) = only_token(node) {
        return Ok(match kind {
            "BASIC_STRING" | "ML_BASIC_STRING" => {
                if text.contains('\\') {
                    return err(
                        line,
                        "escape in a TOML string is not supported (fail closed)",
                    );
                }
                Value::Str(text.to_string())
            }
            "LITERAL_STRING" | "ML_LITERAL_STRING" => Value::Str(text.to_string()),
            "TRUE" => Value::Bool(true),
            "FALSE" => Value::Bool(false),
            _ => Value::Other(text.to_string()),
        });
    }
    // array or inline table
    let Some(inner) = child_nodes(node).next() else {
        return err(line, "empty value");
    };
    match inner.rule_name.as_str() {
        "array" => {
            let mut sub = path.to_vec();
            sub.push("[]".to_string());
            let mut items = Vec::new();
            for values in child_nodes(inner) {
                for v in child_nodes(values) {
                    items.push(value(v, &sub, out)?);
                }
            }
            Ok(Value::Array(items))
        }
        "inline_table" => {
            for kv in child_nodes(inner) {
                let mut full = path.to_vec();
                let mut nodes = child_nodes(kv);
                let (Some(k), Some(v)) = (nodes.next(), nodes.next()) else {
                    return err(line, "malformed inline table member");
                };
                full.extend(key_path(k)?);
                let val = value(v, &full, out)?;
                out.push(Entry {
                    path: full,
                    value: val,
                    line: line_of(kv),
                });
            }
            Ok(Value::Table)
        }
        _ => err(line, "unsupported value form"),
    }
}

/// Parse `text` into flattened entries, in document order.
pub fn parse(text: &str) -> Result<Vec<Entry>, TomlError> {
    let ast = try_parse_toml(text).map_err(|e| TomlError {
        line: 1,
        message: match e {
            TomlParseError::Lexical => "TOML tokenization failed".to_string(),
            TomlParseError::Syntax => "TOML parsing failed".to_string(),
        },
    })?;
    let mut out: Vec<Entry> = Vec::new();
    let mut prefix: Vec<String> = Vec::new();
    // Each `[[x]]` header starts a NEW table of the array `x`, so the same
    // key under two of them is not a duplicate.
    let mut generation = 0usize;
    let mut generations: Vec<usize> = Vec::new();
    for expr in child_nodes(&ast) {
        let Some(inner) = child_nodes(expr).next() else {
            continue;
        };
        let line = line_of(inner);
        let before = out.len();
        match inner.rule_name.as_str() {
            "table_header" | "array_table_header" => {
                if inner.rule_name == "array_table_header" {
                    generation += 1;
                }
                let Some(key) = child_nodes(inner).next() else {
                    return err(line, "malformed table header");
                };
                prefix = key_path(key)?;
                out.push(Entry {
                    path: prefix.clone(),
                    value: Value::Table,
                    line,
                });
            }
            "keyval" => {
                let mut nodes = child_nodes(inner);
                let (Some(k), Some(v)) = (nodes.next(), nodes.next()) else {
                    return err(line, "malformed key/value");
                };
                let mut full = prefix.clone();
                full.extend(key_path(k)?);
                let val = value(v, &full, &mut out)?;
                out.push(Entry {
                    path: full,
                    value: val,
                    line,
                });
            }
            _ => return err(line, "unsupported expression"),
        }
        generations.extend(std::iter::repeat_n(generation, out.len() - before));
    }
    // The grammar crate does not enforce key uniqueness; do it here.
    let mut seen: std::collections::BTreeSet<(usize, &Vec<String>)> =
        std::collections::BTreeSet::new();
    for (e, g) in out.iter().zip(&generations) {
        // elements of an array of inline tables share the path `.. [] ..`
        let in_array = e.path.iter().any(|p| p == "[]");
        if e.value != Value::Table && !in_array && !seen.insert((*g, &e.path)) {
            return err(e.line, "duplicate key");
        }
    }
    Ok(out)
}
