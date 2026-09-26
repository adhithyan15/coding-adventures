//! # Reading a test262 header
//!
//! Every test262 file carries its instructions in a YAML block between `/*---`
//! and `---*/` (V8C09 §2):
//!
//! ```text
//! /*---
//! esid: sec-let-and-const-declarations
//! description: redeclaring a let binding is an early error
//! negative:
//!   phase: parse
//!   type: SyntaxError
//! flags: [onlyStrict]
//! features: [let]
//! includes: [compareArray.js]
//! ---*/
//! ```
//!
//! test262 uses a small, regular slice of YAML, so this module parses exactly
//! that slice rather than all of YAML:
//!
//! | shape | example | read as |
//! |---|---|---|
//! | scalar | `esid: sec-foo` | a string |
//! | flow list | `flags: [onlyStrict, async]` | a list of strings |
//! | block list | `features:` then `  - let` lines | a list of strings |
//! | nested map | `negative:` then `  phase: parse` lines | `negative.phase`, `negative.type` |
//! | block scalar | `info: \|` then indented prose | skipped (prose for humans) |
//!
//! Anything else is an error that names the line, so a header this reader
//! misunderstands is reported rather than silently misread. A misread header
//! is the one failure mode a conformance runner must never have: it would
//! judge a test by the wrong rules.

use std::fmt;

/// When a negative test must fail (the `negative.phase` key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Rejected before any code runs: syntax errors and early errors.
    Parse,
    /// Rejected while linking modules.
    Resolution,
    /// Throws while running.
    Runtime,
}

/// A negative test's expectation: *this* program must fail, in *this* phase,
/// with an error of *this* constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Negative {
    pub phase: Phase,
    /// The error constructor's name, e.g. `SyntaxError`.
    pub error_type: String,
}

/// The flags that change how a test runs (V8C09 §2). Unknown flags are kept
/// verbatim so a new upstream flag is visible, not dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Flag {
    /// Run only with `"use strict";` prepended.
    OnlyStrict,
    /// Run only as sloppy script.
    NoStrict,
    /// Run exactly as written: no harness, no strict prefix.
    Raw,
    /// Parse and run as a module.
    Module,
    /// Completes asynchronously and prints `Test262:AsyncTestComplete`.
    Async,
    /// Generated from a template; runs like any other test.
    Generated,
    /// `Atomics.wait` may not block.
    CanBlockIsFalse,
    /// `Atomics.wait` may block.
    CanBlockIsTrue,
    /// Anything this runner does not know yet.
    Other(String),
}

impl Flag {
    fn from_name(name: &str) -> Flag {
        match name {
            "onlyStrict" => Flag::OnlyStrict,
            "noStrict" => Flag::NoStrict,
            "raw" => Flag::Raw,
            "module" => Flag::Module,
            "async" => Flag::Async,
            "generated" => Flag::Generated,
            "CanBlockIsFalse" => Flag::CanBlockIsFalse,
            "CanBlockIsTrue" => Flag::CanBlockIsTrue,
            other => Flag::Other(other.to_string()),
        }
    }
}

/// Everything the runner reads from one header.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    pub description: Option<String>,
    pub negative: Option<Negative>,
    pub flags: Vec<Flag>,
    pub features: Vec<String>,
    pub includes: Vec<String>,
}

impl Metadata {
    pub fn has_flag(&self, flag: &Flag) -> bool {
        self.flags.contains(flag)
    }
}

/// Why a header could not be read. Each variant names what to fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderError {
    /// No `/*---` … `---*/` block: not a test262 test.
    Missing,
    /// A line the supported YAML slice does not cover, with its 1-based
    /// number inside the header.
    Unsupported { line: usize, text: String },
    /// `negative:` without both `phase` and `type`, or an unknown phase.
    BadNegative(String),
}

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeaderError::Missing => write!(f, "no /*--- ... ---*/ header"),
            HeaderError::Unsupported { line, text } => {
                write!(f, "header line {line} is outside the supported YAML subset: {text:?}")
            }
            HeaderError::BadNegative(why) => write!(f, "bad `negative` block: {why}"),
        }
    }
}

impl std::error::Error for HeaderError {}

/// Extract and parse the header of a test file's source.
pub fn parse_header(source: &str) -> Result<Metadata, HeaderError> {
    let start = source.find("/*---").ok_or(HeaderError::Missing)?;
    let body_start = start + "/*---".len();
    let end = source[body_start..].find("---*/").ok_or(HeaderError::Missing)? + body_start;
    parse_yaml_subset(&source[body_start..end])
}

/// Indentation of a line in spaces (test262 headers never use tabs).
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// `[a, b, c]` → `["a", "b", "c"]`. Items are trimmed; quotes are removed.
fn flow_list(value: &str) -> Option<Vec<String>> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    Some(
        inner
            .split(',')
            .map(|item| unquote(item.trim()).to_string())
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value.strip_prefix(quote).and_then(|v| v.strip_suffix(quote)) {
            return inner;
        }
    }
    value
}

/// The top-level keys whose value is a list of names.
fn is_list_key(key: &str) -> bool {
    matches!(key, "flags" | "features" | "includes")
}

fn parse_yaml_subset(body: &str) -> Result<Metadata, HeaderError> {
    let lines: Vec<&str> = body.lines().collect();
    let mut metadata = Metadata::default();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            index += 1;
            continue;
        }
        let line_number = index + 1;
        let unsupported = || HeaderError::Unsupported {
            line: line_number,
            text: line.to_string(),
        };
        // Top-level entries start in column 0; anything indented here belongs
        // to a block this loop did not open.
        if indent_of(line) != 0 {
            return Err(unsupported());
        }
        let (key, value) = trimmed.split_once(':').ok_or_else(unsupported)?;
        let key = key.trim();
        let value = value.trim();
        index += 1;

        // Collect the indented lines that belong to this key, if any.
        let mut block = Vec::new();
        while index < lines.len() && (lines[index].trim().is_empty() || indent_of(lines[index]) > 0) {
            if !lines[index].trim().is_empty() {
                block.push((index, lines[index]));
            }
            index += 1;
        }

        match key {
            "negative" => {
                if !value.is_empty() {
                    return Err(HeaderError::BadNegative(format!("expected a nested map, got {value:?}")));
                }
                metadata.negative = Some(parse_negative(&block)?);
            }
            _ if is_list_key(key) => {
                let items = if value.is_empty() {
                    block_list(&block)?
                } else {
                    flow_list(value).ok_or_else(unsupported)?
                };
                let target = match key {
                    "flags" => {
                        metadata.flags = items.iter().map(|name| Flag::from_name(name)).collect();
                        continue;
                    }
                    "features" => &mut metadata.features,
                    _ => &mut metadata.includes,
                };
                *target = items;
            }
            "description" => {
                // `description: |` is prose in a block scalar; a plain value is
                // kept. Either way it is for people, never for judging.
                if !value.is_empty() && value != "|" && value != ">" {
                    metadata.description = Some(unquote(value).to_string());
                } else if !block.is_empty() {
                    let text = block.iter().map(|(_, l)| l.trim()).collect::<Vec<_>>().join(" ");
                    metadata.description = Some(text);
                }
            }
            // Keys the runner does not act on (`esid`, `es5id`, `info`,
            // `author`, `locale`, `defines`, …). Their blocks are skipped.
            _ => {}
        }
    }
    Ok(metadata)
}

/// `  - a` lines → `["a", …]`.
fn block_list(block: &[(usize, &str)]) -> Result<Vec<String>, HeaderError> {
    block
        .iter()
        .map(|(number, line)| {
            line.trim()
                .strip_prefix('-')
                .map(|item| unquote(item.trim()).to_string())
                .ok_or(HeaderError::Unsupported {
                    line: number + 1,
                    text: line.to_string(),
                })
        })
        .collect()
}

fn parse_negative(block: &[(usize, &str)]) -> Result<Negative, HeaderError> {
    let mut phase = None;
    let mut error_type = None;
    for (number, line) in block {
        let (key, value) = line.trim().split_once(':').ok_or(HeaderError::Unsupported {
            line: number + 1,
            text: line.to_string(),
        })?;
        let value = unquote(value.trim());
        match key.trim() {
            "phase" => {
                phase = Some(match value {
                    "parse" | "early" => Phase::Parse,
                    "resolution" => Phase::Resolution,
                    "runtime" => Phase::Runtime,
                    other => return Err(HeaderError::BadNegative(format!("unknown phase {other:?}"))),
                })
            }
            "type" => error_type = Some(value.to_string()),
            _ => {}
        }
    }
    match (phase, error_type) {
        (Some(phase), Some(error_type)) => Ok(Negative { phase, error_type }),
        _ => Err(HeaderError::BadNegative("needs both `phase` and `type`".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LET_REDECLARED: &str = "// comment\n/*---\nesid: sec-let\ndescription: redeclared let\nnegative:\n  phase: parse\n  type: SyntaxError\nflags: [onlyStrict]\nfeatures: [let]\nincludes: [compareArray.js]\n---*/\n$DONOTEVALUATE();\nlet x; let x;\n";

    #[test]
    fn reads_every_key_the_runner_uses() {
        let metadata = parse_header(LET_REDECLARED).unwrap();
        assert_eq!(metadata.description.as_deref(), Some("redeclared let"));
        assert_eq!(
            metadata.negative,
            Some(Negative {
                phase: Phase::Parse,
                error_type: "SyntaxError".into()
            })
        );
        assert_eq!(metadata.flags, vec![Flag::OnlyStrict]);
        assert_eq!(metadata.features, vec!["let"]);
        assert_eq!(metadata.includes, vec!["compareArray.js"]);
    }

    #[test]
    fn reads_block_lists_and_skips_prose() {
        let source = "/*---\ninfo: |\n  Several lines\n  of prose: with colons.\nfeatures:\n  - Symbol\n  - 'class'\nflags: []\n---*/";
        let metadata = parse_header(source).unwrap();
        assert_eq!(metadata.features, vec!["Symbol", "class"]);
        assert!(metadata.flags.is_empty());
        assert_eq!(metadata.negative, None);
    }

    #[test]
    fn keeps_unknown_flags_visible() {
        let metadata = parse_header("/*---\nflags: [raw, somethingNew]\n---*/").unwrap();
        assert_eq!(metadata.flags, vec![Flag::Raw, Flag::Other("somethingNew".into())]);
    }

    #[test]
    fn a_header_it_cannot_read_is_an_error_not_a_guess() {
        assert_eq!(parse_header("no header here"), Err(HeaderError::Missing));
        assert!(matches!(
            parse_header("/*---\nnegative:\n  phase: parse\n---*/"),
            Err(HeaderError::BadNegative(_))
        ));
        assert!(matches!(
            parse_header("/*---\nnegative:\n  phase: sometime\n  type: X\n---*/"),
            Err(HeaderError::BadNegative(_))
        ));
        assert!(matches!(
            parse_header("/*---\nflags: onlyStrict\n---*/"),
            Err(HeaderError::Unsupported { .. })
        ));
        assert!(matches!(
            parse_header("/*---\nfeatures:\n  let\n---*/"),
            Err(HeaderError::Unsupported { .. })
        ));
    }

    #[test]
    fn early_is_the_old_name_for_parse() {
        let metadata = parse_header("/*---\nnegative:\n  phase: early\n  type: SyntaxError\n---*/").unwrap();
        assert_eq!(metadata.negative.unwrap().phase, Phase::Parse);
    }
}
