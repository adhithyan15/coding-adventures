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
//! - Errors appear in input order, then by position. The exit status is
//!   `min(N, 127)`.

use coding_adventures_javascript_lexer::tokenize_javascript_typed;
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
    /// 0-based column, in characters.
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

/// Every refusal in one input file, in source order.
///
/// A file the lexer can't tokenize yields none here, because the ordinary
/// pipeline reports that failure itself.
pub fn scan(file: usize, source: &str, version: EsVersion) -> Vec<Refusal> {
    let Ok(tokens) = tokenize_javascript_typed(source, version) else {
        return Vec::new();
    };
    let at = |t: &lexer::token::Token| Refusal {
        file,
        line: t.line,
        // The lexer's columns are 1-based; upstream prints 0-based.
        column: t.column.saturating_sub(1),
        key: UNSUPPORTED_FEATURE,
        message: PRIVATE_ELEMENTS,
    };
    let mut out = Vec::new();
    // Slice 2: `return` / `new.target` outside a function. They need the
    // parse tree, so only parse when a candidate KEYWORD token is present.
    // A string `"return"` or a member name `x.return` is not one.
    let candidate = tokens.iter().enumerate().any(|(i, t)| {
        let keyword = matches!(t.type_, lexer::token::TokenType::Keyword)
            && !(i > 0 && matches!(tokens[i - 1].value.as_str(), "." | "?."));
        keyword
            && (t.value == "return"
                || (t.value == "new"
                    && tokens.get(i + 1).is_some_and(|d| d.value == ".")
                    && tokens.get(i + 2).is_some_and(|n| n.value == "target")))
    });
    if candidate {
        if let Ok(root) = parse_javascript_typed(source, version) {
            out.extend(outside_function(file, &root));
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

/// Slice 2 (CLOC44): every `return` and `new.target` that is not inside a
/// function, in the parse tree of one file.
///
/// Upstream's rule, measured on the jar, is not quite ECMA-262's, and it
/// treats the two keywords differently:
///
/// | Where                                       | `return`  | `new.target` |
/// |---------------------------------------------|-----------|--------------|
/// | top level, a block, a label                 | refused   | refused      |
/// | a function, method, getter, setter          | allowed   | allowed      |
/// | an arrow (even at top level)                | allowed   | allowed      |
/// | a class `static {}` block                   | refused   | as outside   |
/// | a class field initialiser (`x = …`)         | n/a       | as outside   |
/// | a method's computed key (`[k](){}`)         | n/a       | as outside   |
///
/// "As outside" means the context around the class: `class A{x=new.target}`
/// is refused at top level, but `function f(){class A{x=new.target}}`
/// compiles. So two flags travel down the tree:
///
/// - a function-like node (declaration, expression, method, accessor,
///   generator, async form, arrow) sets both, which covers its parameters
///   as well as its body;
/// - `static_block` clears `return_ok` only;
/// - a method's `property_name` child (its key, computed or not) keeps the
///   flags from outside the method.
///
/// Each refusal is reported at the statement's or expression's first token.
/// The walk is iterative, with an explicit stack, so a deeply nested input
/// can't overflow the native stack.
fn outside_function(file: usize, root: &GrammarASTNode) -> Vec<Refusal> {
    #[derive(Clone, Copy)]
    struct Context {
        return_ok: bool,
        new_target_ok: bool,
    }
    const FUNCTION: Context = Context { return_ok: true, new_target_ok: true };
    let mut out = Vec::new();
    let top = Context { return_ok: false, new_target_ok: false };
    let mut stack: Vec<(&GrammarASTNode, Context)> = vec![(root, top)];
    while let Some((node, outer)) = stack.pop() {
        let rule = node.rule_name.as_str();
        let method_like = matches!(rule, "method_definition" | "async_method" | "private_method_definition");
        let here = match rule {
            "function_declaration"
            | "function_expression"
            | "function_body"
            | "generator_declaration"
            | "generator_expression"
            | "async_function_declaration"
            | "async_function_expression"
            | "async_generator_declaration"
            | "async_generator_expression"
            | "arrow_function"
            | "async_arrow_function" => FUNCTION,
            _ if method_like => FUNCTION,
            "static_block" => Context { return_ok: false, ..outer },
            _ => outer,
        };
        let message = match rule {
            "return_statement" if !here.return_ok => Some(RETURN_OUTSIDE_FUNCTION),
            "member_expression" if !here.new_target_ok && is_new_target(node) => {
                Some(NEW_TARGET_OUTSIDE_FUNCTION)
            }
            _ => None,
        };
        if let (Some(message), Some(line), Some(column)) = (message, node.start_line, node.start_column) {
            out.push(Refusal {
                file,
                line,
                // 1-based in the tree; upstream prints 0-based.
                column: column.saturating_sub(1),
                key: PARSE_ERROR,
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

/// Render `refusals` exactly as upstream prints them to stderr, without the
/// final newline (the CLI adds it). `files` holds `(display name, source)` for
/// each input, indexed by [`Refusal::file`]. Output is capped at
/// [`MAX_DIAGNOSTIC_BYTES`].
pub fn render(refusals: &[Refusal], files: &[(String, String)]) -> String {
    // Each file's lines, split once and borrowed, never copied per error.
    let lines: Vec<Vec<&str>> = files.iter().map(|(_, src)| src.lines().collect()).collect();
    let mut out = String::new();
    for r in refusals {
        let excerpt = lines
            .get(r.file)
            .and_then(|l| l.get(r.line.saturating_sub(1)))
            .copied()
            .unwrap_or_default();
        let name = files.get(r.file).map(|(n, _)| n.as_str()).unwrap_or_default();
        let gutter = format!("  {}| ", r.line);
        let pad = gutter.chars().count() + r.column;
        // Header, excerpt line, caret line, blank line.
        let block = name.len() + r.key.len() + r.message.len() + 48 + gutter.len() + excerpt.len() + pad;
        if out.len().saturating_add(block) > MAX_DIAGNOSTIC_BYTES {
            break;
        }
        out.push_str(&format!(
            "{name}:{}:{}: ERROR - [{}] {}\n{gutter}{excerpt}\n{}^\n\n",
            r.line,
            r.column,
            r.key,
            r.message,
            " ".repeat(pad),
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
        scan(0, src, EsVersion::Es2025)
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
        let out = render(&scan(0, src, EsVersion::Es2025), &files);
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
        let r = scan(0, &src, EsVersion::Es2025);
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
        let r = scan(0, &src, EsVersion::Es2025);
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
}
