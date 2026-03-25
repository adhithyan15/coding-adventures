//! # Verilog Preprocessor — handling compiler directives before tokenization.
//!
//! Verilog, like C, has a preprocessor that runs before the main compilation
//! phase. The preprocessor handles textual substitution and conditional
//! compilation — operations that work on raw source text, not parsed tokens.
//!
//! ## Why preprocess before lexing?
//!
//! Consider this Verilog code:
//!
//! ```text
//! `define WIDTH 8
//! reg [`WIDTH-1:0] data;
//! ```
//!
//! After preprocessing, it becomes:
//!
//! ```text
//!
//! reg [8-1:0] data;
//! ```
//!
//! The lexer sees `8-1` as three tokens (NUMBER, MINUS, NUMBER), not the
//! raw directive text. Without preprocessing, `` `WIDTH `` would be lexed as
//! a DIRECTIVE token and the parser would have to handle macro expansion —
//! mixing concerns that should be separate.
//!
//! ## Supported directives
//!
//! | Directive                    | Action                                    |
//! |------------------------------|-------------------------------------------|
//! | `` `define NAME value ``     | Define a simple text macro                |
//! | `` `define NAME(a,b) body `` | Define a parameterized macro              |
//! | `` `undef NAME ``            | Remove a macro definition                 |
//! | `` `ifdef NAME ``            | Include following lines if NAME is defined |
//! | `` `ifndef NAME ``           | Include following lines if NAME is NOT defined |
//! | `` `else ``                  | Toggle conditional inclusion              |
//! | `` `endif ``                 | End conditional block                     |
//! | `` `include "file" ``        | Stubbed — replaced with a comment         |
//! | `` `timescale ... ``         | Stripped (simulation-only directive)       |
//!
//! ## Line number preservation
//!
//! When a directive line is consumed (e.g., `` `define `` or `` `ifdef ``),
//! we replace it with an empty line to preserve line numbering. This ensures
//! that error messages from the lexer/parser still point to the correct line
//! in the original source file.

use std::collections::HashMap;

// ===========================================================================
// Macro definitions — simple and parameterized
// ===========================================================================

/// A macro can be either a simple text substitution or a parameterized macro.
///
/// Simple macro:
///   `define WIDTH 8
///   Stored as: Macro { params: None, body: "8" }
///
/// Parameterized macro:
///   `define MAX(a, b) ((a) > (b) ? (a) : (b))
///   Stored as: Macro { params: Some(["a", "b"]), body: "((a) > (b) ? (a) : (b))" }
#[derive(Debug, Clone)]
struct Macro {
    /// Parameter names for parameterized macros, or None for simple macros.
    params: Option<Vec<String>>,

    /// The replacement text. For parameterized macros, parameter names in
    /// the body are replaced with actual arguments at expansion time.
    body: String,
}

// ===========================================================================
// Conditional compilation state
// ===========================================================================

/// Tracks the state of nested `ifdef/`ifndef/`else/`endif blocks.
///
/// Verilog allows nesting:
///
/// ```text
/// `ifdef FEATURE_A
///   `ifdef FEATURE_B
///     // Only if both A and B are defined
///   `endif
/// `endif
/// ```
///
/// We use a stack of `CondState` values. Each entry tells us:
/// - Are we currently emitting lines? (`active`)
/// - Have we already seen the "true" branch? (`seen_true`)
/// - Was the enclosing block active? (`parent_active`)
#[derive(Debug, Clone)]
struct CondState {
    /// True if we should emit lines in the current branch.
    active: bool,

    /// True if we've already found a true branch (so `else should be false).
    seen_true: bool,

    /// True if the enclosing conditional block was active. A nested block
    /// can only be active if its parent is also active.
    parent_active: bool,
}

// ===========================================================================
// Public API
// ===========================================================================

/// Preprocess Verilog source code by expanding macros and evaluating
/// conditional compilation directives.
///
/// This function processes the source text line by line, handling:
/// - `` `define `` / `` `undef `` — macro definition and removal
/// - `` `ifdef `` / `` `ifndef `` / `` `else `` / `` `endif `` — conditional compilation
/// - `` `include `` — replaced with a comment (stub)
/// - `` `timescale `` — stripped entirely
/// - Macro expansion — `` `NAME `` occurrences are replaced with their definitions
///
/// Lines consumed by directives are replaced with empty lines to preserve
/// line numbering in downstream error messages.
///
/// # Example
///
/// ```
/// use coding_adventures_verilog_lexer::preprocessor::verilog_preprocess;
///
/// let source = r#"`define WIDTH 8
/// reg [`WIDTH-1:0] data;"#;
///
/// let result = verilog_preprocess(source);
/// assert!(result.contains("reg [8-1:0] data;"));
/// ```
pub fn verilog_preprocess(source: &str) -> String {
    let mut macros: HashMap<String, Macro> = HashMap::new();
    let mut cond_stack: Vec<CondState> = Vec::new();
    let mut output_lines: Vec<String> = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();

        // -----------------------------------------------------------------
        // Step 1: Check if this line is a preprocessor directive.
        //
        // Directives start with a backtick followed by a keyword.
        // We handle them in order of specificity.
        // -----------------------------------------------------------------

        if trimmed.starts_with("`define ") || trimmed.starts_with("`define\t") {
            // `define — only process if we're in an active conditional block
            if is_active(&cond_stack) {
                parse_define(trimmed, &mut macros);
            }
            // Replace directive line with empty to preserve line numbers
            output_lines.push(String::new());
        } else if trimmed.starts_with("`undef ") || trimmed.starts_with("`undef\t") {
            if is_active(&cond_stack) {
                parse_undef(trimmed, &mut macros);
            }
            output_lines.push(String::new());
        } else if trimmed.starts_with("`ifdef ") || trimmed.starts_with("`ifdef\t") {
            let name = extract_directive_arg(trimmed, "`ifdef");
            let parent_active = is_active(&cond_stack);
            let defined = macros.contains_key(&name);
            let active = parent_active && defined;
            cond_stack.push(CondState {
                active,
                seen_true: active,
                parent_active,
            });
            output_lines.push(String::new());
        } else if trimmed.starts_with("`ifndef ") || trimmed.starts_with("`ifndef\t") {
            let name = extract_directive_arg(trimmed, "`ifndef");
            let parent_active = is_active(&cond_stack);
            let defined = macros.contains_key(&name);
            let active = parent_active && !defined;
            cond_stack.push(CondState {
                active,
                seen_true: active,
                parent_active,
            });
            output_lines.push(String::new());
        } else if trimmed == "`else" {
            if let Some(state) = cond_stack.last_mut() {
                // `else toggles: if we haven't seen a true branch yet and
                // the parent is active, this branch becomes active.
                if state.parent_active && !state.seen_true {
                    state.active = true;
                    state.seen_true = true;
                } else {
                    state.active = false;
                }
            }
            output_lines.push(String::new());
        } else if trimmed == "`endif" {
            cond_stack.pop();
            output_lines.push(String::new());
        } else if trimmed.starts_with("`include") {
            // `include is stubbed — we replace it with a comment placeholder.
            //
            // A real implementation would read the included file and
            // recursively preprocess it. For now, we leave a breadcrumb
            // so the user knows an include was here.
            if is_active(&cond_stack) {
                output_lines.push(format!("// [preprocessor] {}", trimmed));
            } else {
                output_lines.push(String::new());
            }
        } else if trimmed.starts_with("`timescale") {
            // `timescale is a simulation-only directive. It specifies the
            // time unit and precision for simulation:
            //   `timescale 1ns / 1ps
            //
            // It has no effect on synthesis, so we strip it entirely.
            output_lines.push(String::new());
        } else if is_active(&cond_stack) {
            // -----------------------------------------------------------------
            // Step 2: Not a directive — expand macros in the line.
            //
            // We look for backtick-prefixed names (e.g., `WIDTH) and replace
            // them with their defined values. Parameterized macros like
            // `MAX(a, b) are also expanded.
            // -----------------------------------------------------------------
            let expanded = expand_macros(line, &macros);
            output_lines.push(expanded);
        } else {
            // Inside a false conditional block — emit empty line to
            // preserve line numbering.
            output_lines.push(String::new());
        }
    }

    output_lines.join("\n")
}

// ===========================================================================
// Internal helpers
// ===========================================================================

/// Check if we're currently in an active (emitting) context.
///
/// If the conditional stack is empty, we're at the top level — always active.
/// Otherwise, the top of the stack determines whether we're emitting.
fn is_active(cond_stack: &[CondState]) -> bool {
    cond_stack.last().map_or(true, |s| s.active)
}

/// Parse a `define directive and add the macro to the table.
///
/// Handles two forms:
///
/// Simple: `define WIDTH 8
///   -> name = "WIDTH", body = "8", params = None
///
/// Parameterized: `define MAX(a, b) ((a) > (b) ? (a) : (b))
///   -> name = "MAX", params = ["a", "b"], body = "((a) > (b) ? (a) : (b))"
///
/// Note: the opening parenthesis must immediately follow the name (no space)
/// to be recognized as a parameterized macro. This matches real Verilog
/// preprocessor behavior.
fn parse_define(trimmed: &str, macros: &mut HashMap<String, Macro>) {
    // Strip the `define prefix to get "NAME value" or "NAME(params) body"
    let rest = trimmed
        .strip_prefix("`define")
        .unwrap()
        .trim_start();

    if rest.is_empty() {
        return;
    }

    // Find where the macro name ends. The name is [a-zA-Z_][a-zA-Z0-9_]*
    let name_end = rest
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());

    let name = &rest[..name_end];
    if name.is_empty() {
        return;
    }

    let after_name = &rest[name_end..];

    // Check if this is a parameterized macro: name immediately followed by '('
    if after_name.starts_with('(') {
        // Find the closing parenthesis
        if let Some(close_paren) = after_name.find(')') {
            let params_str = &after_name[1..close_paren];
            let params: Vec<String> = params_str
                .split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();

            let body = after_name[close_paren + 1..].trim().to_string();

            macros.insert(name.to_string(), Macro {
                params: Some(params),
                body,
            });
        }
    } else {
        // Simple macro — body is everything after the name (trimmed)
        let body = after_name.trim().to_string();
        macros.insert(name.to_string(), Macro {
            params: None,
            body,
        });
    }
}

/// Parse a `undef directive and remove the macro from the table.
fn parse_undef(trimmed: &str, macros: &mut HashMap<String, Macro>) {
    let name = extract_directive_arg(trimmed, "`undef");
    macros.remove(&name);
}

/// Extract the single argument from a directive like `ifdef NAME or `undef NAME.
fn extract_directive_arg(trimmed: &str, directive: &str) -> String {
    trimmed
        .strip_prefix(directive)
        .unwrap_or("")
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

/// Expand all macro references in a line of source code.
///
/// We scan for backtick-prefixed identifiers (`` `NAME ``) and replace them.
///
/// For simple macros, we just substitute the body text.
/// For parameterized macros, we also parse the argument list and substitute
/// each parameter in the body.
///
/// We limit expansion passes to prevent infinite recursion from circular
/// macro definitions. In practice, well-formed Verilog doesn't have circular
/// macros, but we guard against it anyway.
fn expand_macros(line: &str, macros: &HashMap<String, Macro>) -> String {
    let mut result = line.to_string();

    // Multiple expansion passes to handle macros that expand to other macros.
    // Cap at 10 passes to prevent infinite loops.
    for _ in 0..10 {
        let mut new_result = String::with_capacity(result.len());
        let mut chars = result.chars().peekable();
        let mut changed = false;

        while let Some(ch) = chars.next() {
            if ch == '`' {
                // Collect the identifier name after the backtick
                let mut name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }

                if let Some(mac) = macros.get(&name) {
                    changed = true;

                    if let Some(ref params) = mac.params {
                        // Parameterized macro — parse the argument list.
                        //
                        // We expect: `NAME(arg1, arg2, ...)
                        // The opening '(' should be the next non-whitespace char.
                        if chars.peek() == Some(&'(') {
                            chars.next(); // consume '('
                            let args = parse_macro_args(&mut chars);
                            let mut body = mac.body.clone();
                            for (i, param) in params.iter().enumerate() {
                                if let Some(arg) = args.get(i) {
                                    body = body.replace(param.as_str(), arg.as_str());
                                }
                            }
                            new_result.push_str(&body);
                        } else {
                            // Parameterized macro used without args — just insert body
                            new_result.push_str(&mac.body);
                        }
                    } else {
                        // Simple macro — direct substitution
                        new_result.push_str(&mac.body);
                    }
                } else {
                    // Not a known macro — keep the backtick + name as-is.
                    // This handles directives that aren't macros (e.g., `timescale
                    // that weren't caught earlier, or user-defined directives).
                    new_result.push('`');
                    new_result.push_str(&name);
                }
            } else {
                new_result.push(ch);
            }
        }

        result = new_result;
        if !changed {
            break;
        }
    }

    result
}

/// Parse a comma-separated list of macro arguments, handling nested parentheses.
///
/// Called after consuming the opening '('. Reads characters until the matching
/// closing ')' is found, splitting on commas at the top nesting level.
///
/// Example: `MAX(a + b, c * d)` -> ["a + b", "c * d"]
///
/// Nested parens are handled: `MAX((a+b), (c+d))` -> ["(a+b)", "(c+d)"]
fn parse_macro_args(chars: &mut std::iter::Peekable<std::str::Chars>) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut depth = 1; // We've already consumed the opening '('

    for ch in chars.by_ref() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    // End of argument list
                    let trimmed = current.trim().to_string();
                    if !trimmed.is_empty() || !args.is_empty() {
                        args.push(trimmed);
                    }
                    break;
                }
                current.push(ch);
            }
            ',' if depth == 1 => {
                // Comma at top level separates arguments
                args.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }

    args
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Simple `define and expansion
    // -----------------------------------------------------------------------

    #[test]
    fn test_simple_define() {
        let source = "`define WIDTH 8\nreg [`WIDTH-1:0] data;";
        let result = verilog_preprocess(source);
        assert!(result.contains("reg [8-1:0] data;"));
        // First line should be empty (directive consumed)
        assert!(result.starts_with('\n'));
    }

    #[test]
    fn test_define_no_value() {
        // `define with no value — used as a flag for `ifdef
        let source = "`define FEATURE_X\n`ifdef FEATURE_X\nwire x;\n`endif";
        let result = verilog_preprocess(source);
        assert!(result.contains("wire x;"));
    }

    // -----------------------------------------------------------------------
    // Parameterized macros
    // -----------------------------------------------------------------------

    #[test]
    fn test_parameterized_macro() {
        let source = "`define MAX(a, b) ((a) > (b) ? (a) : (b))\nassign out = `MAX(x, y);";
        let result = verilog_preprocess(source);
        assert!(result.contains("((x) > (y) ? (x) : (y))"));
    }

    #[test]
    fn test_parameterized_macro_nested_parens() {
        let source = "`define ADD(a, b) (a + b)\nassign out = `ADD((x+1), (y+2));";
        let result = verilog_preprocess(source);
        assert!(result.contains("((x+1) + (y+2))"));
    }

    // -----------------------------------------------------------------------
    // `undef
    // -----------------------------------------------------------------------

    #[test]
    fn test_undef() {
        let source = "`define WIDTH 8\n`undef WIDTH\nreg [`WIDTH-1:0] data;";
        let result = verilog_preprocess(source);
        // After undef, `WIDTH should remain unexpanded
        assert!(result.contains("`WIDTH"));
    }

    // -----------------------------------------------------------------------
    // `ifdef / `ifndef / `else / `endif
    // -----------------------------------------------------------------------

    #[test]
    fn test_ifdef_defined() {
        let source = "`define FEATURE\n`ifdef FEATURE\nwire a;\n`endif";
        let result = verilog_preprocess(source);
        assert!(result.contains("wire a;"));
    }

    #[test]
    fn test_ifdef_not_defined() {
        let source = "`ifdef FEATURE\nwire a;\n`endif";
        let result = verilog_preprocess(source);
        assert!(!result.contains("wire a;"));
    }

    #[test]
    fn test_ifdef_else() {
        let source = "`ifdef FEATURE\nwire a;\n`else\nwire b;\n`endif";
        let result = verilog_preprocess(source);
        assert!(!result.contains("wire a;"));
        assert!(result.contains("wire b;"));
    }

    #[test]
    fn test_ifdef_defined_else() {
        let source = "`define FEATURE\n`ifdef FEATURE\nwire a;\n`else\nwire b;\n`endif";
        let result = verilog_preprocess(source);
        assert!(result.contains("wire a;"));
        assert!(!result.contains("wire b;"));
    }

    #[test]
    fn test_ifndef_not_defined() {
        let source = "`ifndef MISSING\nwire a;\n`endif";
        let result = verilog_preprocess(source);
        assert!(result.contains("wire a;"));
    }

    #[test]
    fn test_ifndef_defined() {
        let source = "`define PRESENT\n`ifndef PRESENT\nwire a;\n`endif";
        let result = verilog_preprocess(source);
        assert!(!result.contains("wire a;"));
    }

    #[test]
    fn test_nested_ifdef() {
        let source = "\
`define A
`define B
`ifdef A
`ifdef B
wire both;
`endif
`endif";
        let result = verilog_preprocess(source);
        assert!(result.contains("wire both;"));
    }

    #[test]
    fn test_nested_ifdef_outer_false() {
        let source = "\
`ifdef A
`ifdef B
wire both;
`endif
`endif";
        let result = verilog_preprocess(source);
        assert!(!result.contains("wire both;"));
    }

    // -----------------------------------------------------------------------
    // `include (stubbed)
    // -----------------------------------------------------------------------

    #[test]
    fn test_include_stubbed() {
        let source = "`include \"types.vh\"";
        let result = verilog_preprocess(source);
        assert!(result.contains("// [preprocessor] `include \"types.vh\""));
    }

    // -----------------------------------------------------------------------
    // `timescale (stripped)
    // -----------------------------------------------------------------------

    #[test]
    fn test_timescale_stripped() {
        let source = "`timescale 1ns / 1ps\nmodule top;";
        let result = verilog_preprocess(source);
        assert!(!result.contains("timescale"));
        assert!(result.contains("module top;"));
    }

    // -----------------------------------------------------------------------
    // Line number preservation
    // -----------------------------------------------------------------------

    #[test]
    fn test_line_number_preservation() {
        let source = "`define WIDTH 8\n`define DEPTH 16\nmodule top;";
        let result = verilog_preprocess(source);
        let lines: Vec<&str> = result.lines().collect();
        // First two lines should be empty (directives consumed)
        assert_eq!(lines[0], "");
        assert_eq!(lines[1], "");
        // Third line should be the module declaration
        assert_eq!(lines[2], "module top;");
        // Total line count should match
        assert_eq!(lines.len(), 3);
    }

    // -----------------------------------------------------------------------
    // Multiple macro expansions on one line
    // -----------------------------------------------------------------------

    #[test]
    fn test_multiple_macros_one_line() {
        let source = "`define A 1\n`define B 2\nassign x = `A + `B;";
        let result = verilog_preprocess(source);
        assert!(result.contains("assign x = 1 + 2;"));
    }

    // -----------------------------------------------------------------------
    // Macro expanding to another macro
    // -----------------------------------------------------------------------

    #[test]
    fn test_chained_macro_expansion() {
        let source = "`define INNER 42\n`define OUTER `INNER\nassign x = `OUTER;";
        let result = verilog_preprocess(source);
        assert!(result.contains("assign x = 42;"));
    }

    // -----------------------------------------------------------------------
    // Unknown directive left as-is
    // -----------------------------------------------------------------------

    #[test]
    fn test_unknown_macro_left_alone() {
        let source = "assign x = `UNKNOWN;";
        let result = verilog_preprocess(source);
        assert!(result.contains("`UNKNOWN"));
    }

    // -----------------------------------------------------------------------
    // Define inside false ifdef is ignored
    // -----------------------------------------------------------------------

    #[test]
    fn test_define_in_false_branch() {
        let source = "`ifdef NOPE\n`define X 5\n`endif\nassign y = `X;";
        let result = verilog_preprocess(source);
        // X should NOT be defined since the `define was in a false branch
        assert!(result.contains("`X"));
    }
}
