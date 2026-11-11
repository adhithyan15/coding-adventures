//! Refuse what Closure refuses (CCR-075, #15860; spec CLOC44).
//!
//! The owner decided on 2026-10-19 that when the pinned reference compiler
//! (`closure-compiler-v20260915`) rejects an input, closurec rejects it the same
//! way. That means the same diagnostic key and message, the same location, the
//! same exit status, and no JavaScript, output file or sidecar. closurec does
//! not compile a superset of what Closure compiles.
//!
//! This module is that gate. It scans every input before any compilation work
//! starts, so a refused program never reaches the pipeline and nothing is ever
//! written.
//!
//! # What is refused
//!
//! | Construct                         | Upstream diagnostic                     |
//! |-----------------------------------|-----------------------------------------|
//! | any private element (`#x`)        | `JSC_UNSUPPORTED_LANGUAGE_FEATURE` …    |
//! |                                   | "Private elements." (slice 1)           |
//! | `return` outside a function       | `JSC_PARSE_ERROR` "Parse error. return  |
//! |                                   | must be inside function" (slice 2)      |
//! | `new.target` outside a function   | `JSC_PARSE_ERROR` "Parse error.         |
//! |                                   | new.target must be inside a function"   |
//! |                                   | (slice 2)                               |
//! | `super` outside a method          | `JSC_INVALID_SUPER_ACCESS` "'super'     |
//! |                                   | may only be accessed within a method"   |
//! |                                   | (slice 3)                               |
//! | `with` structure                  | `JSC_USE_OF_WITH` "The with             |
//! |                                   | statement cannot be used in strict      |
//! |                                   | mode." (slice 4)                        |
//!
//! Upstream reports **every** occurrence: the `#x` of a field, method or
//! accessor declaration, every `this.#x` use, and `#x in o`. Each one is a
//! separate error at that token's position, at every compilation level.
//! Later slices add the rest of the refusals captured in
//! `tests/oracle/upstream-refusals-v20260915.json`.
//!
//! # The output format, byte for byte
//!
//! Measured against the jar:
//!
//! ```text
//! a.js:12:10: ERROR - [JSC_UNSUPPORTED_LANGUAGE_FEATURE] This language feature is not currently supported by the compiler: Private elements.
//!   12| class B { static #s = 1; … }
//!                 ^
//!
//! 1 error(s), 0 warning(s)
//! ```
//!
//! - The header is `file:line:column`, with a 1-based line and a 0-based
//!   column. The file is spelled as the user passed it.
//! - The excerpt is two spaces, the line number and `| `, then the source line
//!   verbatim. The caret is indented by that prefix's width plus the column.
//! - A blank line follows each error, and the run ends with
//!   `N error(s), 0 warning(s)`.
//! - Errors appear in source-name order (Java `String.compareTo`), then by
//!   line and column. The exit status is `min(N, 127)`.

use coding_adventures_javascript_lexer::tokenize_javascript_typed;
use crate::config::CompilationLevel;
use coding_adventures_javascript_parser::parse_javascript_typed;
use coding_adventures_javascript_tokens::EsVersion;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

/// One refused construct, located the way upstream reports it. It holds
/// positions only. The excerpt is read from the source at render time, so
/// memory stays proportional to the input however many errors there are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// Index of the input file in the list passed to [`render`].
    pub file: usize,
    /// 1-based line.
    pub line: usize,
    /// 0-based column, in UTF-16 code units.
    pub column: usize,
    /// The diagnostic key, e.g. `JSC_UNSUPPORTED_LANGUAGE_FEATURE`.
    pub key: &'static str,
    /// The diagnostic message, exactly as upstream prints it.
    pub message: &'static str,
}

/// Rendered diagnostics stop growing past this many bytes. Upstream prints
/// every error with a full excerpt, so on a hostile single-line input (`#a;`
/// repeated) its output grows with errors × line length: 30 KB of input gives
/// about 450 MB. Below this budget closurec matches upstream byte for byte.
/// Past it, the remaining error blocks are omitted, and the summary line and
/// exit status still count all of them. That is a deliberate divergence,
/// since parity isn't worth gigabytes of stderr. See CLOC44.
pub const MAX_DIAGNOSTIC_BYTES: usize = 16 << 20;

const UNSUPPORTED_FEATURE: &str = "JSC_UNSUPPORTED_LANGUAGE_FEATURE";
const PRIVATE_ELEMENTS: &str =
    "This language feature is not currently supported by the compiler: Private elements.";
const PARSE_ERROR: &str = "JSC_PARSE_ERROR";
const RETURN_OUTSIDE_FUNCTION: &str = "Parse error. return must be inside function";
const NEW_TARGET_OUTSIDE_FUNCTION: &str = "Parse error. new.target must be inside a function";
const INVALID_SUPER_ACCESS: &str = "JSC_INVALID_SUPER_ACCESS";
const SUPER_OUTSIDE_METHOD: &str = "'super' may only be accessed within a method";
const USE_OF_WITH: &str = "JSC_USE_OF_WITH";
const WITH_IN_STRICT_MODE: &str = "The with statement cannot be used in strict mode.";

/// Convert a 1-based character (scalar) column from the lexer/parser into
/// a 0-based column in UTF-16 code units, matching upstream Java String indexing.
fn utf16_col(lines: &[&str], line: usize, char_col_1based: usize) -> usize {
    let char_offset = char_col_1based.saturating_sub(1);
    if let Some(line_str) = lines.get(line.saturating_sub(1)) {
        line_str.chars().take(char_offset).map(|c| c.len_utf16()).sum()
    } else {
        char_offset
    }
}

/// Every refusal in one input file, in source order.
///
/// A file the lexer can't tokenize yields none here, because the ordinary
/// pipeline reports that failure itself.
pub fn scan(
    file: usize,
    source: &str,
    version: EsVersion,
    level: CompilationLevel,
) -> Vec<Refusal> {
    let Ok(tokens) = tokenize_javascript_typed(source, version) else {
        return Vec::new();
    };
    let lines: Vec<&str> = source.split('\n').collect();
    let at = |t: &lexer::token::Token| Refusal {
        file,
        line: t.line,
        column: utf16_col(&lines, t.line, t.column),
        key: UNSUPPORTED_FEATURE,
        message: PRIVATE_ELEMENTS,
    };
    let mut out = Vec::new();
    // Slices 2, 3 & 4: `return` / `new.target` outside a function, `super` outside
    // a method, and `with` statements. They need the parse tree, so only parse
    // when a candidate KEYWORD token is present. A string `"with"` or a member
    // name `x.with` is not one. Note `with` is permitted at WHITESPACE_ONLY.
    let refuse_with = level != CompilationLevel::WhitespaceOnly;
    let candidate = tokens.iter().enumerate().any(|(i, t)| {
        let keyword = matches!(t.type_, lexer::token::TokenType::Keyword)
            && !(i > 0 && matches!(tokens[i - 1].value.as_str(), "." | "?."));
        keyword
            && (t.value == "return"
                || t.value == "super"
                || (refuse_with && t.value == "with")
                || (t.value == "new"
                    && tokens.get(i + 1).is_some_and(|d| d.value == ".")
                    && tokens.get(i + 2).is_some_and(|n| n.value == "target")))
    });
    if candidate {
        if let Ok(root) = parse_javascript_typed(source, version) {
            out.extend(outside_function(file, &root, &lines, refuse_with));
        }
    }
    for (i, t) in tokens.iter().enumerate() {
        if t.type_name.as_deref() != Some("PRIVATE_NAME") {
            continue;
        }
        // A `static` private FIELD is reported twice upstream: once at the
        // `static` keyword (the member) and once at the name. A private
        // method, accessor or non-static field is reported once, at the
        // name. Measured: `static #x = 1` errors at both 10 and 17, while
        // `static #m(){}` errors only at 17.
        let is_method = tokens.get(i + 1).is_some_and(|n| n.value == "(");
        if !is_method && i > 0 && tokens[i - 1].value == "static" {
            out.push(at(&tokens[i - 1]));
        }
        out.push(at(t));
    }
    // Upstream reports every error in source order, interleaving the kinds
    // (`class A{#x}return;` gives the private element at 8, then the
    // `return` at 11).
    out.sort_by_key(|r| (r.line, r.column));
    out
}

/// Slices 2 & 3 (CLOC44): `return` / `new.target` outside a function, and
/// `super` outside a method.
///
/// Upstream's rule, measured on the jar, is not quite ECMA-262's, and it
/// treats the constructs differently:
///
/// | Where                                       | `return`  | `new.target` | `super`      |
/// |---------------------------------------------|-----------|--------------|--------------|
/// | top level, a block, a label                 | refused   | refused      | refused      |
/// | a standalone function                       | allowed   | allowed      | refused      |
/// | a method, getter, setter                    | allowed   | allowed      | allowed      |
/// | an arrow (at top level or in function)      | allowed   | allowed      | as outside   |
/// | an arrow inside a method                    | allowed   | allowed      | allowed      |
/// | a class `static {}` block                   | refused   | as outside   | allowed      |
/// | a class field initialiser (`x = …`)         | n/a       | as outside   | allowed      |
/// | a method's computed key (`[k](){}`)         | n/a       | as outside   | as outside   |
///
/// "As outside" means the context around the class/function: `class A{x=new.target}`
/// is refused at top level, but `function f(){class A{x=new.target}}`
/// compiles. So three flags travel down the tree:
///
/// - functions set `return_ok` and `new_target_ok`, but clear `super_ok`;
/// - methods set `return_ok`, `new_target_ok`, and `super_ok`;
/// - arrows set `return_ok` and `new_target_ok`, and inherit `outer.super_ok`;
/// - `static_block` and `class_field_declaration` clear `return_ok`, inherit
///   `new_target_ok`, and set `super_ok`;
/// - a method's `property_name` child (its key, computed or not) keeps the
///   flags from outside the method.
///
/// Each refusal is reported at the statement's or expression's first token.
/// The walk is iterative, with an explicit stack, so a deeply nested input
/// can't overflow the native stack.
fn outside_function(
    file: usize,
    root: &GrammarASTNode,
    lines: &[&str],
    refuse_with: bool,
) -> Vec<Refusal> {
    #[derive(Clone, Copy)]
    struct Context {
        return_ok: bool,
        new_target_ok: bool,
        super_ok: bool,
    }
    let mut out = Vec::new();
    let top = Context {
        return_ok: false,
        new_target_ok: false,
        super_ok: false,
    };
    let mut stack: Vec<(&GrammarASTNode, Context)> = vec![(root, top)];
    while let Some((node, outer)) = stack.pop() {
        let rule = node.rule_name.as_str();
        let method_like = matches!(rule, "method_definition" | "async_method" | "private_method_definition");
        let here = match rule {
            "function_declaration"
            | "function_expression"
            | "generator_declaration"
            | "generator_expression"
            | "async_function_declaration"
            | "async_function_expression"
            | "async_generator_declaration"
            | "async_generator_expression" => Context {
                return_ok: true,
                new_target_ok: true,
                super_ok: false,
            },
            "arrow_function" | "async_arrow_function" => Context {
                return_ok: true,
                new_target_ok: true,
                super_ok: outer.super_ok,
            },
            _ if method_like => Context {
                return_ok: true,
                new_target_ok: true,
                super_ok: true,
            },
            "static_block" | "class_field_declaration" => Context {
                return_ok: false,
                new_target_ok: outer.new_target_ok,
                super_ok: true,
            },
            _ => outer,
        };
        let diag = match rule {
            "return_statement" if !here.return_ok => Some((PARSE_ERROR, RETURN_OUTSIDE_FUNCTION)),
            "member_expression" if !here.new_target_ok && is_new_target(node) => {
                Some((PARSE_ERROR, NEW_TARGET_OUTSIDE_FUNCTION))
            }
            "member_expression" if !here.super_ok && is_super(node) => {
                Some((INVALID_SUPER_ACCESS, SUPER_OUTSIDE_METHOD))
            }
            "with_statement" if refuse_with => Some((USE_OF_WITH, WITH_IN_STRICT_MODE)),
            _ => None,
        };
        if let (Some((key, message)), Some(line), Some(column)) = (diag, node.start_line, node.start_column) {
            out.push(Refusal {
                file,
                line,
                // Upstream reports column in UTF-16 code units (0-based).
                column: utf16_col(lines, line, column),
                key,
                message,
            });
        }
        for child in node.children.iter().rev() {
            if let ASTNodeOrToken::Node(n) = child {
                // A method's key is evaluated outside the method.
                let ctx = if method_like && n.rule_name == "property_name" { outer } else { here };
                stack.push((n, ctx));
            }
        }
    }
    out
}

/// Is `node` the `new . target` alternative of `member_expression`?
fn is_new_target(node: &GrammarASTNode) -> bool {
    let tokens: Vec<&str> = node
        .children
        .iter()
        .map(|c| match c {
            ASTNodeOrToken::Token(t) => t.value.as_str(),
            ASTNodeOrToken::Node(_) => "",
        })
        .collect();
    tokens == ["new", ".", "target"]
}

/// Is `node` a `member_expression` whose base is the `super` keyword?
fn is_super(node: &GrammarASTNode) -> bool {
    matches!(
        node.children.first(),
        Some(ASTNodeOrToken::Token(t))
            if t.value == "super" && matches!(t.type_, lexer::token::TokenType::Keyword)
    )
}

/// Render `refusals` exactly as upstream prints them to stderr, without the
/// final newline (the CLI adds it). `files` holds `(display name, source)` for
/// each input, indexed by [`Refusal::file`]. Output is capped at
/// [`MAX_DIAGNOSTIC_BYTES`].
pub fn render(refusals: &[Refusal], files: &[(String, String)]) -> String {
    // Each file's lines, split once and borrowed, never copied per error.
    // Using `split('\n')` rather than `lines()` preserves trailing `\r`
    // so CRLF files echo the carriage return on excerpt lines, exactly as
    // upstream does (CLOC44, #18159).
    let lines: Vec<Vec<&str>> = files.iter().map(|(_, src)| src.split('\n').collect()).collect();

    // Upstream sorts diagnostics by source name (Java String.compareTo,
    // which compares UTF-16 code units), then line, then column.
    let mut order: Vec<usize> = (0..refusals.len()).collect();
    order.sort_by(|&i, &j| {
        let ra = &refusals[i];
        let rb = &refusals[j];
        let name_a = files.get(ra.file).map(|(n, _)| n.as_str()).unwrap_or_default();
        let name_b = files.get(rb.file).map(|(n, _)| n.as_str()).unwrap_or_default();
        name_a
            .encode_utf16()
            .cmp(name_b.encode_utf16())
            .then_with(|| ra.line.cmp(&rb.line))
            .then_with(|| ra.column.cmp(&rb.column))
            .then_with(|| ra.key.cmp(rb.key))
            .then_with(|| ra.message.cmp(rb.message))
            .then_with(|| ra.file.cmp(&rb.file))
    });

    let mut out = String::new();
    for &idx in &order {
        let r = &refusals[idx];
        let excerpt = lines
            .get(r.file)
            .and_then(|l| l.get(r.line.saturating_sub(1)))
            .copied()
            .unwrap_or_default();
        let name = files.get(r.file).map(|(n, _)| n.as_str()).unwrap_or_default();
        let gutter = format!("  {}| ", r.line);

        // Build caret line: gutter is padded with spaces. For the excerpt up to
        // `r.column`, upstream preserves tab characters verbatim from the source line
        // (`\t\t^`), and replaces non-tabs with spaces (CLOC44, #18159).
        let mut caret = " ".repeat(gutter.chars().count());
        let mut col = 0;
        for c in excerpt.chars() {
            if col >= r.column {
                break;
            }
            let u16_len = c.len_utf16();
            if c == '\t' {
                caret.push('\t');
            } else {
                for _ in 0..u16_len {
                    caret.push(' ');
                }
            }
            col += u16_len;
        }
        while col < r.column {
            caret.push(' ');
            col += 1;
        }
        caret.push('^');

        // Header, excerpt line, caret line, blank line.
        let block = name.len() + r.key.len() + r.message.len() + 48 + gutter.len() + excerpt.len() + caret.len() + 4;
        if out.len().saturating_add(block) > MAX_DIAGNOSTIC_BYTES {
            break;
        }
        out.push_str(&format!(
            "{name}:{}:{}: ERROR - [{}] {}\n{gutter}{excerpt}\n{caret}\n\n",
            r.line,
            r.column,
            r.key,
            r.message,
        ));
    }
    out.push_str(&format!("{} error(s), 0 warning(s)", refusals.len()));
    out
}

/// The file name as upstream prints it. Java's `java.io.File` collapses runs
/// of `/`, so `T//a.js` is reported as `T/a.js`.
pub fn display_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        if c == '/' && out.ends_with('/') {
            continue;
        }
        out.push(c);
    }
    out
}

/// Upstream's process exit status for `errors` errors.
pub fn exit_status(errors: usize) -> u8 {
    errors.min(127) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan_es(src: &str) -> Vec<Refusal> {
        scan(0, src, EsVersion::Es2025, CompilationLevel::Simple)
    }

    #[test]
    fn every_private_element_occurrence_is_refused() {
        // Field declaration and the `this.#x` read: two errors, like upstream.
        let r = scan_es("class A { #x = 1; get(){ return this.#x; } }");
        let cols: Vec<usize> = r.iter().map(|r| r.column).collect();
        assert_eq!(cols, vec![10, 37]);
        assert!(r.iter().all(|r| r.line == 1 && r.key == UNSUPPORTED_FEATURE));
    }

    #[test]
    fn methods_accessors_statics_and_brand_checks_are_refused() {
        let r = scan_es("class B { static #s = 1; #m() { return #s in this; } get #g() { return 1; } }");
        let cols: Vec<usize> = r.iter().map(|r| r.column).collect();
        assert_eq!(cols, vec![10, 17, 25, 39, 57]);
    }

    #[test]
    fn a_static_private_field_is_also_reported_at_static() {
        // Measured against v20260915.
        let cols = |src| scan_es(src).iter().map(|r| r.column).collect::<Vec<_>>();
        assert_eq!(cols("class A { static #x = 1; }"), vec![10, 17]);
        assert_eq!(cols("class A { static #x; }"), vec![10, 17]);
        assert_eq!(cols("class A { static #m() {} }"), vec![17]);
        assert_eq!(cols("class A { static get #g() {} }"), vec![21]);
        assert_eq!(cols("class A { get #g() {} }"), vec![14]);
        assert_eq!(cols("class A { static async *#g() {} }"), vec![24]);
    }

    // ---- Slice 2: `return` / `new.target` outside a function ----------
    // Every expectation below was measured on the v20260915 jar.

    fn parse_refusals(src: &str) -> Vec<(usize, &'static str)> {
        scan_es(src).into_iter().map(|r| (r.column, r.message)).collect()
    }

    #[test]
    fn return_outside_a_function_is_refused_every_time() {
        assert_eq!(
            parse_refusals("return;return;"),
            vec![(0, RETURN_OUTSIDE_FUNCTION), (7, RETURN_OUTSIDE_FUNCTION)]
        );
        assert_eq!(parse_refusals("if(a){return}"), vec![(6, RETURN_OUTSIDE_FUNCTION)]);
        assert_eq!(parse_refusals("l:{return}"), vec![(3, RETURN_OUTSIDE_FUNCTION)]);
        assert_eq!(parse_refusals("class A{static{return}}"), vec![(15, RETURN_OUTSIDE_FUNCTION)]);
    }

    #[test]
    fn new_target_outside_a_function_is_refused() {
        assert_eq!(parse_refusals("f(new.target, 1 + 2);"), vec![(2, NEW_TARGET_OUTSIDE_FUNCTION)]);
        assert_eq!(parse_refusals("class A{static{new.target}}"), vec![(15, NEW_TARGET_OUTSIDE_FUNCTION)]);
        assert_eq!(parse_refusals("class A{x=new.target}"), vec![(10, NEW_TARGET_OUTSIDE_FUNCTION)]);
        // A method's computed key is outside the method.
        assert_eq!(parse_refusals("class A{static [new.target](){}}"), vec![(16, NEW_TARGET_OUTSIDE_FUNCTION)]);
        assert_eq!(parse_refusals("class A{x=class B{y=new.target}}"), vec![(20, NEW_TARGET_OUTSIDE_FUNCTION)]);
    }

    #[test]
    fn class_fields_and_static_blocks_follow_the_enclosing_function_for_new_target() {
        for src in [
            "function make(){return class{kind=new.target}}",
            "function f(){class A{static x=new.target}}",
            "function f(){class A{static{new.target}}}",
            "class A{m(){class B{x=new.target}}}",
            "var x=()=>{class B{static{new.target}}}",
            "function f(){class A{static{class B{static{new.target}}}}}",
            "function f(){class A{[new.target](){}}}",
        ] {
            assert!(scan_es(src).is_empty(), "{src:?}");
        }
        // ... but a static block still has no `return`.
        assert_eq!(
            parse_refusals("function f(){class A{static{return}}}"),
            vec![(28, RETURN_OUTSIDE_FUNCTION)]
        );
    }

    #[test]
    fn functions_methods_and_arrows_allow_both() {
        for src in [
            "function f(){return new.target}",
            "function f(a=new.target){}",
            "x=()=>{return 1}",
            "()=>new.target",
            "function f(){()=>{return new.target}}",
            "class A{m(){return new.target}}",
            "var x='return'; // return",
        ] {
            assert!(scan_es(src).is_empty(), "{src:?}");
        }
    }

    // ---- Slice 3: `super` outside a method ----------------------------
    // Every expectation below was measured on the v20260915 jar.

    #[test]
    fn super_outside_a_method_is_refused() {
        assert_eq!(
            parse_refusals("super.f(1 + 2);"),
            vec![(0, SUPER_OUTSIDE_METHOD)]
        );
        assert_eq!(
            parse_refusals("x=()=>super.x;"),
            vec![(6, SUPER_OUTSIDE_METHOD)]
        );
        assert_eq!(
            parse_refusals("super.x;super.y;"),
            vec![(0, SUPER_OUTSIDE_METHOD), (8, SUPER_OUTSIDE_METHOD)]
        );
        assert_eq!(
            parse_refusals("if(a){super.x;}"),
            vec![(6, SUPER_OUTSIDE_METHOD)]
        );
    }

    #[test]
    fn super_in_arrows_inherits_method_context() {
        for src in [
            "class A{m(){const f=()=>super.x;}}",
            "var o={m(){const f=()=>super.x;}};",
            "class A{static m(){const f=()=>super.x;}}",
        ] {
            assert!(scan_es(src).is_empty(), "{src:?}");
        }
    }

    #[test]
    fn methods_class_fields_and_static_blocks_allow_super() {
        for src in [
            "class A{m(){super.x;}}",
            "class A{constructor(){super();}}",
            "class A{get x(){return super.x;}}",
            "class A{set x(v){super.x=v;}}",
            "var o={m(){super.x;}};",
            "class A{x=super.y;}",
            "class A{static{super.y;}}",
            "var x='super'; // super",
        ] {
            assert!(scan_es(src).is_empty(), "{src:?}");
        }
    }

    #[test]
    fn computed_method_key_outside_method_refuses_super() {
        assert_eq!(
            parse_refusals("class A{[super.x](){}}"),
            vec![(9, SUPER_OUTSIDE_METHOD)]
        );
        assert_eq!(
            parse_refusals("class A{static [super.x](){}}"),
            vec![(16, SUPER_OUTSIDE_METHOD)]
        );
    }

    #[test]
    fn refusals_of_all_three_slices_come_out_in_source_order() {
        let r = scan_es("class A{#x}return;super.x;");
        assert_eq!(
            r.iter().map(|r| (r.column, r.key)).collect::<Vec<_>>(),
            vec![
                (8, UNSUPPORTED_FEATURE),
                (11, PARSE_ERROR),
                (18, INVALID_SUPER_ACCESS)
            ]
        );
    }

    // ---- Slice 4: `with` statement -----------------------------------
    // Every expectation below was measured on the v20260915 jar.

    #[test]
    fn with_statement_is_refused() {
        assert_eq!(
            parse_refusals("with (o) { x(1 + 2); }"),
            vec![(0, WITH_IN_STRICT_MODE)]
        );
        assert_eq!(
            parse_refusals("function f() { with (o) {} }"),
            vec![(15, WITH_IN_STRICT_MODE)]
        );
        assert_eq!(
            parse_refusals("with (a) {} with (b) {}"),
            vec![(0, WITH_IN_STRICT_MODE), (12, WITH_IN_STRICT_MODE)]
        );
        for src in [
            "var x = 'with'; // with",
            "obj.with = 1;",
        ] {
            assert!(scan_es(src).is_empty(), "{src:?}");
        }
    }

    #[test]
    fn refusals_of_all_four_slices_come_out_in_source_order() {
        let r = scan_es("class A{#x}return;super.x;with(o){}");
        assert_eq!(
            r.iter().map(|r| (r.column, r.key)).collect::<Vec<_>>(),
            vec![
                (8, UNSUPPORTED_FEATURE),
                (11, PARSE_ERROR),
                (18, INVALID_SUPER_ACCESS),
                (26, USE_OF_WITH),
            ]
        );
    }

    #[test]
    fn refusals_of_both_slices_come_out_in_source_order() {
        let r = scan_es("class A{#x}return;");
        assert_eq!(
            r.iter().map(|r| (r.column, r.key)).collect::<Vec<_>>(),
            vec![(8, UNSUPPORTED_FEATURE), (11, PARSE_ERROR)]
        );
        let r = scan_es("return;class A{#x}");
        assert_eq!(
            r.iter().map(|r| (r.column, r.key)).collect::<Vec<_>>(),
            vec![(0, PARSE_ERROR), (15, UNSUPPORTED_FEATURE)]
        );
    }

    #[test]
    fn deep_nesting_does_not_overflow_the_walk() {
        // The walk is iterative, so any tree the parser builds is walked
        // without recursion. (Past the parser's own rule-depth limit there
        // is no tree, and no slice-2 refusal; see CLOC44.)
        let src = format!("{}return;{}", "{".repeat(40), "}".repeat(40));
        assert_eq!(parse_refusals(&src).len(), 1);
    }

    #[test]
    fn programs_without_private_elements_are_not_refused() {
        assert!(scan_es("class A { x = 1; m() { return '#x'; } } // #y").is_empty());
    }

    #[test]
    fn render_matches_upstream_bytes() {
        // Captured from closure-compiler-v20260915 (two errors, exit 2).
        let src = "class A { #x = 1; get(){ return this.#x; } }\nconsole.log(new A().get());\n";
        let files = vec![("t.js".to_string(), src.to_string())];
        let out = render(&scan(0, src, EsVersion::Es2025, CompilationLevel::Simple), &files);
        let line = "class A { #x = 1; get(){ return this.#x; } }";
        let expected = format!(
            "t.js:1:10: ERROR - [{UNSUPPORTED_FEATURE}] {PRIVATE_ELEMENTS}\n  1| {line}\n{}^\n\n\
             t.js:1:37: ERROR - [{UNSUPPORTED_FEATURE}] {PRIVATE_ELEMENTS}\n  1| {line}\n{}^\n\n\
             2 error(s), 0 warning(s)",
            " ".repeat(15),
            " ".repeat(42),
        );
        assert_eq!(out, expected);
    }

    #[test]
    fn multi_digit_lines_widen_the_gutter() {
        let src = format!("{}class C {{\n  #y;\n}}\n", "var v;\n".repeat(11));
        let r = scan(0, &src, EsVersion::Es2025, CompilationLevel::Simple);
        assert_eq!((r[0].line, r[0].column), (13, 2));
        let files = vec![("b.js".to_string(), src.clone())];
        assert!(render(&r, &files).starts_with("b.js:13:2: ERROR"));
        assert!(render(&r, &files).contains("\n  13|   #y;\n        ^\n"));
    }

    #[test]
    fn display_path_collapses_repeated_slashes_like_java_file() {
        assert_eq!(display_path("/tmp//cc/a.js"), "/tmp/cc/a.js");
        assert_eq!(display_path("tests/diff/x/input/a.js"), "tests/diff/x/input/a.js");
    }

    #[test]
    fn rendering_is_bounded_on_a_hostile_single_line() {
        // `#a;` × 20000 on one line: upstream would print ~2.4 GB.
        let src = "#a;".repeat(20_000);
        let r = scan(0, &src, EsVersion::Es2025, CompilationLevel::Simple);
        assert_eq!(r.len(), 20_000);
        let files = vec![("h.js".to_string(), src)];
        let out = render(&r, &files);
        assert!(out.len() <= MAX_DIAGNOSTIC_BYTES + 64, "{}", out.len());
        assert!(out.ends_with("20000 error(s), 0 warning(s)"));
    }

    #[test]
    fn exit_status_is_the_error_count_capped_at_127() {
        assert_eq!(exit_status(1), 1);
        assert_eq!(exit_status(6), 6);
        assert_eq!(exit_status(200), 127);
    }

    #[test]
    fn multi_file_refusals_sort_by_java_string_order() {
        // Upstream sorts by source name using Java String.compareTo (UTF-16 code units),
        // then line, then column, rather than input order.
        let files = vec![
            ("z.js".to_string(), "return;".to_string()),
            ("a.js".to_string(), "return;".to_string()),
        ];
        let mut refusals = scan(0, &files[0].1, EsVersion::Es2025, CompilationLevel::Simple);
        refusals.extend(scan(1, &files[1].1, EsVersion::Es2025, CompilationLevel::Simple));
        let out = render(&refusals, &files);
        let a_pos = out.find("a.js:1:0: ERROR").expect("a.js present");
        let z_pos = out.find("z.js:1:0: ERROR").expect("z.js present");
        assert!(a_pos < z_pos, "a.js should precede z.js in diagnostic output");
    }

    #[test]
    fn utf16_columns_count_surrogate_pairs() {
        // `var s='😀';return;`: '😀' is 1 Unicode scalar, but 2 UTF-16 code units.
        // Upstream reports column 11 (0-based UTF-16 code units), not 10.
        let src = "var s='😀';return;";
        let r = scan(0, src, EsVersion::Es2025, CompilationLevel::Simple);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].line, 1);
        assert_eq!(r[0].column, 11);
    }

    #[test]
    fn tab_carets_preserve_tabs_from_source_line() {
        // Upstream's caret line copies tabs from the excerpt so carets align with tab stops.
        let src = "\t\treturn;";
        let files = vec![("tabs.js".to_string(), src.to_string())];
        let r = scan(0, src, EsVersion::Es2025, CompilationLevel::Simple);
        let out = render(&r, &files);
        // Gutter is "  1| " (5 spaces), followed by two tabs, then '^'.
        assert!(out.contains("  1| \t\treturn;\n     \t\t^\n"));
    }

    #[test]
    fn crlf_excerpts_keep_trailing_carriage_return() {
        // Upstream retains the trailing '\r' on excerpt lines when source has CRLF endings.
        let src = "var a=1;\r\nreturn;\r\n";
        let files = vec![("crlf.js".to_string(), src.to_string())];
        let r = scan(0, src, EsVersion::Es2025, CompilationLevel::Simple);
        let out = render(&r, &files);
        assert!(out.contains("  2| return;\r\n"));
    }

    #[test]
    fn with_statement_allowed_at_whitespace_only() {
        assert!(scan(0, "with (o) { a(); }", EsVersion::Es2025, CompilationLevel::WhitespaceOnly).is_empty());
    }
}
