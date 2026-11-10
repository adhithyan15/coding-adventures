//! Function-level, strict-mode and template early errors (V8C09 conformance).
//!
//! The grammar accepts a program if its *shape* is right. ECMA-262 then lists
//! a long tail of **early errors**: programs of the right shape that must
//! still be refused before any code runs. This module is the part of that
//! list that belongs to functions and to strict mode. [`crate::early_errors`]
//! holds blocks, `yield` and assignment targets.
//!
//! ```text
//! function f(a, a) { "use strict"; }      duplicate parameter in strict code
//! function f(a = 1) { "use strict"; }     "use strict" with non-simple params
//! (async function (...a,) {})             a rest parameter takes no comma
//! ({ m(a, a) {} })                        methods never allow duplicates
//! (a, a) => 1                             arrows never allow duplicates
//! function f(a) { let a; }                parameter and lexical name clash
//! async function f() { var await; }       `await` is reserved in async code
//! (async function () { super.x; })        `super` needs a method
//! `\07`                                   a template has no octal escapes
//! a?.b`x`                                 an optional chain takes no template
//! ```
//!
//! ## Why this was missing
//!
//! Until the object-method and `async function` expression grammar landed
//! (#19068), every one of these programs failed to *parse*, so the test262
//! negative tests for them passed for the wrong reason (see the lesson on
//! test262 pass lists). Accepting more syntax means accepting these programs
//! too, unless they are refused on purpose, here.
//!
//! ## How it works
//!
//! One pass over the CST with an explicit stack (no recursion, so deep
//! nesting cannot overflow the stack). Each stack entry carries a small
//! [`Ctx`]: is the code strict, is `await` reserved, may `super.x` appear.
//! Entering a function recomputes the context for its parameters and body;
//! entering a class makes everything inside strict.

use std::collections::HashSet;

use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::early_errors::{
    bound_names, children, collect_tokens, has_token, has_use_strict_directive, is_name, is_string,
    lexically_declared, nodes, Lexical,
};

/// What the code being walked is allowed to contain.
#[derive(Clone, Copy)]
struct Ctx {
    /// Strict mode code (ECMA-262 §11.2.2): `"use strict"`, a class, or
    /// the body and parameters of a function whose body says so.
    strict: bool,
    /// Inside an async function: `await` is a keyword, never an identifier.
    in_async: bool,
    /// Inside the parameter list of an async function: even an `await`
    /// *expression* is an error there (§15.8.1).
    in_async_params: bool,
    /// `super.x` is allowed: methods, class fields and static blocks, and the
    /// arrow functions inside them (§13.3.7.1).
    super_prop: bool,
}

/// Words that are identifiers in sloppy code and reserved in strict code
/// (§13.1.1). `yield` and `let` have their own rules elsewhere.
const STRICT_RESERVED: [&str; 7] = [
    "implements",
    "interface",
    "package",
    "private",
    "protected",
    "public",
    "static",
];

/// How a function-like node is shaped, from its rule name.
struct FunctionKind {
    is_async: bool,
    is_arrow: bool,
    is_method: bool,
    /// A function *expression*: its own name is a binding inside the body.
    is_expression: bool,
}

fn function_kind(node: &GrammarASTNode) -> Option<FunctionKind> {
    let (is_async, is_arrow, is_method, is_expression) = match node.rule_name.as_str() {
        "function_declaration" | "generator_declaration" => (false, false, false, false),
        "function_expression" | "generator_expression" => (false, false, false, true),
        "async_function_declaration" | "async_generator_declaration" => (true, false, false, false),
        "async_function_expression" | "async_generator_expression" => (true, false, false, true),
        "arrow_function" => (false, true, false, false),
        "async_arrow_function" => (true, true, false, false),
        "method_definition" => (false, false, true, false),
        "async_method" => (true, false, true, false),
        "private_method_definition" => (has_token(node, "async"), false, true, false),
        _ => return None,
    };
    Some(FunctionKind {
        is_async,
        is_arrow,
        is_method,
        is_expression,
    })
}

/// Check every function, strict-mode rule and template under `root`.
/// Returns the first violation as a located message.
pub fn check(root: &GrammarASTNode) -> Result<(), String> {
    let start = Ctx {
        strict: has_use_strict_directive(root),
        in_async: false,
        in_async_params: false,
        // The program itself is not inside a function, so `super` here is
        // left alone: the grammar has no Script/Module goals yet, and
        // top-level `super.x` is a known gap (V8C09), not a regression.
        super_prop: true,
    };
    let mut stack: Vec<(&GrammarASTNode, Ctx)> = vec![(root, start)];
    while let Some((node, ctx)) = stack.pop() {
        check_node(node, ctx)?;
        let child_ctx = child_contexts(node, ctx);
        for child in node.children.iter().rev() {
            if let ASTNodeOrToken::Node(child) = child {
                stack.push((child, child_ctx(child)));
            }
        }
    }
    Ok(())
}

/// The context each child of `node` is walked in. Returned as a closure so
/// the per-function work (directive lookup) is done once, not per child.
fn child_contexts<'a>(
    node: &'a GrammarASTNode,
    ctx: Ctx,
) -> Box<dyn Fn(&GrammarASTNode) -> Ctx + 'a> {
    if let Some(kind) = function_kind(node) {
        let body = function_body_of(node);
        let strict = ctx.strict || body.is_some_and(has_use_strict_directive);
        let base = Ctx {
            strict,
            in_async: kind.is_async,
            in_async_params: false,
            super_prop: if kind.is_method {
                true
            } else if kind.is_arrow {
                ctx.super_prop
            } else {
                false
            },
        };
        let is_arrow = kind.is_arrow;
        let is_async = kind.is_async;
        let outer_async = ctx.in_async;
        return Box::new(move |child| match child.rule_name.as_str() {
            "formal_parameters" | "formal_parameter" | "arrow_parameters" => Ctx {
                // A plain arrow's parameters are read in the enclosing
                // function's context for `await`.
                in_async: if is_arrow && !is_async {
                    outer_async
                } else {
                    base.in_async
                },
                in_async_params: is_async,
                ..base
            },
            // A plain arrow's body is not part of its enclosing async function.
            "concise_body" | "function_body" if is_arrow && !is_async => Ctx {
                in_async: false,
                ..base
            },
            _ => base,
        });
    }
    match node.rule_name.as_str() {
        "class_declaration" | "class_expression" => Box::new(move |child| Ctx {
            strict: true,
            // Computed keys and fields are not part of an enclosing async function.
            in_async: if child.rule_name == "class_body" {
                false
            } else {
                ctx.in_async
            },
            in_async_params: false,
            super_prop: if child.rule_name == "class_body" {
                false
            } else {
                ctx.super_prop
            },
        }),
        "class_field_declaration" | "static_block" => Box::new(move |_| Ctx {
            strict: true,
            in_async: false,
            in_async_params: false,
            super_prop: true,
        }),
        _ => Box::new(move |_| ctx),
    }
}

/// The `function_body` of a function-like node (an arrow's sits inside its
/// `concise_body`).
fn function_body_of(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    for child in children(node) {
        match child.rule_name.as_str() {
            "function_body" => return Some(child),
            "concise_body" => {
                return children(child).find(|n| n.rule_name == "function_body");
            }
            _ => {}
        }
    }
    None
}

fn first_name(node: &GrammarASTNode) -> Option<&Token> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) if is_name(t) => Some(t),
        _ => None,
    })
}

fn at(token: &Token) -> String {
    format!("early error at {}:{}", token.line, token.column)
}

/// The per-node rules.
fn check_node(node: &GrammarASTNode, ctx: Ctx) -> Result<(), String> {
    for child in &node.children {
        if let ASTNodeOrToken::Token(t) = child {
            check_literal(t, ctx)?;
        }
    }
    if let Some(kind) = function_kind(node) {
        check_function(node, &kind, ctx)?;
    }
    match node.rule_name.as_str() {
        "with_statement" if ctx.strict => {
            return Err(
                "early error: `with` statement is not allowed in strict mode code (ECMA-262 §14.11.1)"
                    .to_string(),
            );
        }
        // An identifier reference.
        "primary_expression" => {
            if let Some(t) = first_name(node).filter(|_| node.children.len() == 1) {
                check_reference(t, ctx)?;
            }
            // An untagged template: every escape must be valid (§13.2.8.1).
            // A tagged template is a template that follows another operand,
            // so it never sits directly in a `primary_expression`.
            for template in children(node).filter(|n| n.rule_name == "template_literal") {
                for piece in &template.children {
                    if let ASTNodeOrToken::Token(t) = piece {
                        if let Some(why) = template_escape_error(&t.value) {
                            return Err(format!(
                                "{}: {why} in a template literal (ECMA-262 §13.2.8.1)",
                                at(t)
                            ));
                        }
                    }
                }
            }
        }
        "labelled_statement" => {
            if let Some(t) = first_name(node) {
                check_reference(t, ctx)?;
            }
        }
        // A name being bound.
        "variable_declaration"
        | "lexical_binding"
        | "binding_element"
        | "formal_parameter"
        | "object_rest_property"
        | "catch_clause" => {
            if let Some(t) = first_name(node) {
                check_binding(t, ctx)?;
            }
        }
        "binding_property" if !has_token(node, ":") => {
            if let Some(t) = first_name(node) {
                check_binding(t, ctx)?;
            }
        }
        "await_expression" if ctx.in_async_params => {
            return Err(
                "early error: `await` expression cannot be used in async function parameters (ECMA-262 §15.8.1)"
                    .to_string(),
            );
        }
        // `a?.b`x`` and the ASI twin `a?.b⏎`x``.
        "optional_chain_expression" => {
            let mut after_chain = false;
            for child in &node.children {
                match child {
                    ASTNodeOrToken::Token(t) if t.value == "?." && !is_string(t) => {
                        after_chain = true;
                    }
                    ASTNodeOrToken::Node(n) if after_chain && n.rule_name == "template_literal" => {
                        return Err(
                            "early error: a template literal cannot follow an optional chain (ECMA-262 §13.3.1)"
                                .to_string(),
                        );
                    }
                    _ => {}
                }
            }
        }
        "member_expression" => {
            let is_super = matches!(
                node.children.first(),
                Some(ASTNodeOrToken::Token(t)) if t.value == "super" && !is_string(t)
            );
            if is_super && !ctx.super_prop {
                return Err(
                    "early error: `super` property access is only allowed in methods (ECMA-262 §13.3.7.1)"
                        .to_string(),
                );
            }
        }
        // `[...a, b]` and `[...a,]`: the rest element comes last.
        "array_binding_pattern" => {
            let list: Vec<&ASTNodeOrToken> = node
                .children
                .iter()
                .filter(
                    |c| !matches!(c, ASTNodeOrToken::Token(t) if t.value == "[" || t.value == "]"),
                )
                .collect();
            for (index, item) in list.iter().enumerate() {
                if let ASTNodeOrToken::Node(n) = item {
                    if n.rule_name == "binding_element"
                        && has_token(n, "...")
                        && index + 1 < list.len()
                    {
                        return Err(
                            "early error: a rest element must be last in an array binding pattern (ECMA-262 §14.3.3)"
                                .to_string(),
                        );
                    }
                }
            }
        }
        // `for (let x = 1 of y)`: a for-of head declares no initializer, and
        // neither does a for-in head with `let` or `const` (Annex B.3.5 keeps
        // only `for (var x = 1 in y)` in sloppy code).
        "for_of_statement" | "for_await_of_statement" => {
            if children(node).any(|n| {
                matches!(
                    n.rule_name.as_str(),
                    "binding_element" | "variable_declaration"
                ) && has_token(n, "=")
            }) {
                return Err(
                    "early error: a for-of loop variable cannot have an initializer (ECMA-262 §14.7.5.1)"
                        .to_string(),
                );
            }
        }
        "for_in_statement"
            if children(node).any(|n| n.rule_name == "binding_element" && has_token(n, "=")) =>
        {
            return Err(
                "early error: a for-in loop variable cannot have an initializer (ECMA-262 §14.7.5.1)"
                    .to_string(),
            );
        }
        _ => {}
    }
    // `async⏎function`, `async⏎m(){}` and `async⏎x => 1`: no line
    // terminator after `async` (§15.8, §15.9).
    if function_kind(node).is_some_and(|k| k.is_async) {
        check_async_line(node)?;
    }
    Ok(())
}

/// Strict-mode rules for a literal token: legacy octal escapes in strings and
/// legacy octal numbers (§12.9.3.1, §12.9.4.1).
fn check_literal(token: &Token, ctx: Ctx) -> Result<(), String> {
    if !ctx.strict {
        return Ok(());
    }
    match token.type_ {
        TokenType::String if has_legacy_octal_escape(&token.value) => Err(format!(
            "{}: octal escape sequences are not allowed in strict mode code (ECMA-262 §12.9.4.1)",
            at(token)
        )),
        TokenType::Number
            if {
                let mut chars = token.value.chars();
                chars.next() == Some('0') && chars.next().is_some_and(|c| c.is_ascii_digit())
            } =>
        {
            Err(format!(
                "{}: legacy octal numeric literals are not allowed in strict mode code (ECMA-262 §12.9.3.1)",
                at(token)
            ))
        }
        _ => Ok(()),
    }
}

/// Does a string literal's source spelling contain `\1`..`\9`, or `\0`
/// followed by a digit? (`\0` alone is the NUL escape and stays legal.)
fn has_legacy_octal_escape(spelling: &str) -> bool {
    let chars: Vec<char> = spelling.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\\' {
            i += 1;
            continue;
        }
        match chars.get(i + 1) {
            Some('0') if !chars.get(i + 2).is_some_and(char::is_ascii_digit) => {}
            Some(c) if c.is_ascii_digit() => return true,
            _ => {}
        }
        i += 2;
    }
    false
}

/// The first invalid escape sequence in a template piece, if any. A piece is
/// the raw source of one `TEMPLATE_*` token, delimiters included.
///
/// Unlike a string, a template has no legacy octal escapes in any mode:
/// `\1`..`\9` and `\0` followed by a digit are errors, as are a short `\x`
/// and a malformed or out-of-range `\u`. (A *tagged* template accepts them,
/// its cooked value being `undefined`; the caller never checks those.)
fn template_escape_error(raw: &str) -> Option<&'static str> {
    let chars: Vec<char> = raw.chars().collect();
    let hex = |i: usize| chars.get(i).is_some_and(char::is_ascii_hexdigit);
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\\' {
            i += 1;
            continue;
        }
        match chars.get(i + 1) {
            Some('0') => {
                if chars.get(i + 2).is_some_and(char::is_ascii_digit) {
                    return Some("octal escape sequence");
                }
                i += 2;
            }
            Some('1'..='9') => return Some("octal escape sequence"),
            Some('x') => {
                if !(hex(i + 2) && hex(i + 3)) {
                    return Some("invalid hexadecimal escape sequence");
                }
                i += 4;
            }
            Some('u') if chars.get(i + 2) == Some(&'{') => {
                let mut j = i + 3;
                let mut value: u32 = 0;
                let mut digits = 0;
                while hex(j) {
                    value = value
                        .saturating_mul(16)
                        .saturating_add(chars[j].to_digit(16).unwrap_or(0));
                    digits += 1;
                    j += 1;
                }
                if digits == 0 || chars.get(j) != Some(&'}') || value > 0x10_FFFF {
                    return Some("invalid Unicode escape sequence");
                }
                i = j + 1;
            }
            Some('u') => {
                if !(hex(i + 2) && hex(i + 3) && hex(i + 4) && hex(i + 5)) {
                    return Some("invalid Unicode escape sequence");
                }
                i += 6;
            }
            _ => i += 2,
        }
    }
    None
}

/// An identifier *reference* or label in the current context.
fn check_reference(token: &Token, ctx: Ctx) -> Result<(), String> {
    if ctx.in_async && token.value == "await" {
        return Err(format!(
            "{}: `await` cannot be used as an identifier in an async function (ECMA-262 §13.1.1)",
            at(token)
        ));
    }
    if ctx.strict && STRICT_RESERVED.contains(&token.value.as_str()) {
        return Err(format!(
            "{}: `{}` is a reserved word in strict mode code (ECMA-262 §13.1.1)",
            at(token),
            token.value
        ));
    }
    Ok(())
}

/// A name being *bound* (a parameter, `var`, `let`, `catch` parameter...).
fn check_binding(token: &Token, ctx: Ctx) -> Result<(), String> {
    check_reference(token, ctx)?;
    if ctx.strict && matches!(token.value.as_str(), "eval" | "arguments") {
        return Err(format!(
            "{}: `{}` cannot be a binding name in strict mode code (ECMA-262 §13.1.1)",
            at(token),
            token.value
        ));
    }
    Ok(())
}

/// No line terminator between `async` and what follows it.
fn check_async_line(node: &GrammarASTNode) -> Result<(), String> {
    let Some(ASTNodeOrToken::Token(first)) = node.children.first() else {
        return Ok(());
    };
    if first.value != "async" || is_string(first) {
        return Ok(());
    }
    let next_line = match node.children.get(1) {
        Some(ASTNodeOrToken::Token(t)) => Some(t.line),
        Some(ASTNodeOrToken::Node(n)) => {
            let mut tokens = Vec::new();
            collect_tokens(n, &mut tokens, 1);
            tokens.first().map(|t| t.line)
        }
        None => None,
    };
    if next_line.is_some_and(|line| line != first.line) {
        return Err(format!(
            "{}: no line terminator is allowed after `async` (ECMA-262 §15.8)",
            at(first)
        ));
    }
    Ok(())
}

/// The parameters of a function-like node: the `formal_parameter` nodes and
/// the container that holds them (for the rest-element rule).
fn parameters(node: &GrammarASTNode) -> (Vec<&GrammarASTNode>, Option<&GrammarASTNode>) {
    // An arrow's parameters sit one level down, in `arrow_parameters`.
    let holder = children(node)
        .find(|n| n.rule_name == "arrow_parameters")
        .unwrap_or(node);
    if let Some(list) = children(holder).find(|n| n.rule_name == "formal_parameters") {
        return (
            children(list)
                .filter(|n| n.rule_name == "formal_parameter")
                .collect(),
            Some(list),
        );
    }
    // A setter's lone parameter is not wrapped in `formal_parameters`.
    (
        children(holder)
            .filter(|n| n.rule_name == "formal_parameter")
            .collect(),
        None,
    )
}

/// The names bound by one `formal_parameter`.
fn parameter_names<'a>(param: &'a GrammarASTNode, out: &mut Vec<(&'a str, usize, usize)>) {
    for child in &param.children {
        match child {
            ASTNodeOrToken::Token(t) if is_name(t) => {
                out.push((&t.value, t.line, t.column));
                return;
            }
            ASTNodeOrToken::Node(n) if n.rule_name == "binding_pattern" => {
                bound_names(n, out);
                return;
            }
            _ => {}
        }
    }
}

/// A `formal_parameter` is simple when it is a bare identifier: no default,
/// no rest, no pattern (§15.1.1 IsSimpleParameterList).
fn is_simple_parameter(param: &GrammarASTNode) -> bool {
    matches!(param.children.as_slice(), [ASTNodeOrToken::Token(t)] if is_name(t))
}

/// Parameters, directive and body of one function.
fn check_function(node: &GrammarASTNode, kind: &FunctionKind, ctx: Ctx) -> Result<(), String> {
    let (params, container) = parameters(node);
    let body = function_body_of(node);
    let has_directive = body.is_some_and(has_use_strict_directive);
    let strict = ctx.strict || has_directive;

    // `(a, ...b,)` and `(...a, b)`: the rest parameter is last, with no comma.
    if let Some(list) = container {
        let mut seen_rest = false;
        for child in &list.children {
            if seen_rest {
                return Err(
                    "early error: a rest parameter must be the last formal parameter (ECMA-262 §15.1.1)"
                        .to_string(),
                );
            }
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "formal_parameter" && has_token(n, "...") {
                    seen_rest = true;
                }
            }
        }
    }

    // An arrow with one bare parameter (`x => 1`) has no `formal_parameter`.
    let bare_arrow_param = if kind.is_arrow {
        children(node)
            .find(|n| n.rule_name == "arrow_parameters")
            .and_then(first_name)
            .filter(|_| params.is_empty())
    } else {
        None
    };

    let simple = params.iter().all(|p| is_simple_parameter(p));
    if has_directive && !simple {
        return Err(
            "early error: a function with non-simple parameters cannot have a \"use strict\" directive (ECMA-262 §15.2.1)"
                .to_string(),
        );
    }

    let mut names: Vec<(&str, usize, usize)> = Vec::new();
    for param in &params {
        parameter_names(param, &mut names);
    }
    if let Some(t) = bare_arrow_param {
        names.push((&t.value, t.line, t.column));
    }

    // Methods and arrows are UniqueFormalParameters; any function becomes one
    // in strict code or with non-simple parameters (§15.2.1).
    let unique_required = strict || !simple || kind.is_arrow || kind.is_method;
    if unique_required {
        let mut seen: HashSet<&str> = HashSet::new();
        for &(name, line, column) in &names {
            if !seen.insert(name) {
                return Err(format!(
                    "early error at {line}:{column}: duplicate parameter name `{name}` (ECMA-262 §15.2.1)"
                ));
            }
        }
    }
    if kind.is_async {
        if let Some(&(_, line, column)) = names.iter().find(|(name, _, _)| *name == "await") {
            return Err(format!(
                "early error at {line}:{column}: `await` cannot be a parameter name of an async function (ECMA-262 §15.8.1)"
            ));
        }
    }

    // The function's own name: `function eval(){"use strict"}` and, for an
    // async function *expression*, `await`.
    if !kind.is_method && !kind.is_arrow {
        if let Some(name) = function_name(node) {
            if strict && matches!(name.value.as_str(), "eval" | "arguments") {
                return Err(format!(
                    "{}: `{}` cannot be a function name in strict mode code (ECMA-262 §15.2.1)",
                    at(name),
                    name.value
                ));
            }
            if strict && STRICT_RESERVED.contains(&name.value.as_str()) {
                return Err(format!(
                    "{}: `{}` is a reserved word in strict mode code (ECMA-262 §13.1.1)",
                    at(name),
                    name.value
                ));
            }
            if name.value == "await" && kind.is_async && kind.is_expression {
                return Err(format!(
                    "{}: `await` cannot name an async function expression (ECMA-262 §15.8.1)",
                    at(name)
                ));
            }
            if name.value == "await" && ctx.in_async && !kind.is_expression {
                return Err(format!(
                    "{}: `await` cannot be a function name in an async function (ECMA-262 §13.1.1)",
                    at(name)
                ));
            }
        }
    }

    // `function f(a) { let a; }`: a parameter and a top-level lexical name
    // of the body must differ (§15.2.1). Top-level `function` declarations
    // are var-like there, so they are not collected.
    if let Some(body) = body {
        let param_names: HashSet<&str> = names.iter().map(|(name, _, _)| *name).collect();
        if !param_names.is_empty() {
            let mut lexical: Vec<Lexical> = Vec::new();
            for element in children(body) {
                let mut item = element;
                while matches!(item.rule_name.as_str(), "source_element" | "statement") {
                    match nodes(item).as_slice() {
                        [inner] => item = *inner,
                        _ => break,
                    }
                }
                if matches!(
                    item.rule_name.as_str(),
                    "lexical_declaration"
                        | "using_declaration"
                        | "await_using_declaration"
                        | "class_declaration"
                        | "decorated_class_declaration"
                ) {
                    lexically_declared(item, &mut lexical);
                }
            }
            if let Some(l) = lexical.iter().find(|l| param_names.contains(l.name)) {
                return Err(format!(
                    "early error at {}:{}: `{}` is both a parameter and a lexical declaration of the body (ECMA-262 §15.2.1)",
                    l.line, l.column, l.name
                ));
            }
        }
    }
    Ok(())
}

/// The binding name of a function declaration or expression: the identifier
/// after the `function` keyword (and its `*`). `async` lexes as a name too,
/// so the first name overall would be wrong.
fn function_name(node: &GrammarASTNode) -> Option<&Token> {
    node.children
        .iter()
        .skip_while(
            |c| !matches!(c, ASTNodeOrToken::Token(t) if !is_string(t) && t.value == "function"),
        )
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) if is_name(t) => Some(t),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use crate::parse_javascript_typed;
    use coding_adventures_javascript_tokens::EsVersion;

    fn ok(src: &str) -> bool {
        parse_javascript_typed(src, EsVersion::Es2025).is_ok()
    }

    #[test]
    fn duplicate_parameters() {
        assert!(!ok("function f(a, a) { 'use strict'; }"));
        assert!(!ok("'use strict'; function f(a, a) {}"));
        assert!(!ok("function f(a, a = 1) {}"));
        assert!(!ok("function f(a, [a]) {}"));
        assert!(!ok("(a, a) => 1"));
        assert!(!ok("async (a, a) => 1"));
        assert!(!ok("({ m(a, a) {} })"));
        assert!(!ok("({ async m(a, a) {} })"));
        assert!(!ok("(async function (x = 0, x) {})"));
        assert!(!ok("(async function* (x = 0, x) {})"));
        assert!(!ok("class C { m(a, a) {} }"));
        // Sloppy functions with simple parameters may repeat a name.
        assert!(ok("function f(a, a) {}"));
        assert!(ok("(function (a, a) {})"));
        assert!(ok("(async function (a, a) {})"));
        assert!(ok("function f(a, b) { var a; }"));
    }

    #[test]
    fn use_strict_needs_simple_parameters() {
        assert!(!ok("function f(a = 1) { 'use strict'; }"));
        assert!(!ok("function f([a]) { 'use strict'; }"));
        assert!(!ok("function f(...a) { 'use strict'; }"));
        assert!(!ok("(async function ({ a }) { 'use strict'; })"));
        assert!(!ok("({ m(a = 0) { 'use strict'; } })"));
        assert!(!ok("({ set m(a = 0) { 'use strict'; } })"));
        assert!(!ok("(a = 1) => { 'use strict'; }"));
        assert!(ok("function f(a) { 'use strict'; }"));
        assert!(ok("function f(a = 1) { 'use sloppy'; }"));
        assert!(ok("function f(a = 1) { f(); 'use strict'; }"));
    }

    #[test]
    fn rest_parameter_is_last_without_a_comma() {
        assert!(!ok("function f(...a,) {}"));
        assert!(!ok("(async function (...a,) {})"));
        assert!(!ok("({ m(...a,) {} })"));
        assert!(!ok("function f(...a, b) {}"));
        assert!(ok("function f(a, ...b) {}"));
        assert!(ok("function f(a,) {}"));
    }

    #[test]
    fn rest_element_is_last_in_array_binding_pattern() {
        assert!(!ok("function f([...a, b]) {}"));
        assert!(!ok("function f([...a,]) {}"));
        assert!(!ok("({ m([...a, b]) {} })"));
        assert!(!ok("for (let [...a, b] of c) ;"));
        assert!(!ok("var [...a, b] = c;"));
        assert!(ok("function f([a, ...b]) {}"));
    }

    #[test]
    fn strict_binding_names() {
        assert!(!ok("'use strict'; function eval() {}"));
        assert!(!ok("function f(eval) { 'use strict'; }"));
        assert!(!ok("function arguments() { 'use strict'; }"));
        assert!(!ok("'use strict'; (async function arguments() {})"));
        assert!(!ok("({ set x(eval) { 'use strict'; } })"));
        assert!(!ok("'use strict'; ({ set x(arguments) {} })"));
        assert!(!ok("'use strict'; var eval;"));
        assert!(!ok("'use strict'; try {} catch (eval) {}"));
        assert!(!ok("'use strict'; var public;"));
        assert!(!ok("'use strict'; public = 1;"));
        assert!(!ok("({ get x() { 'use strict'; public = 42; } })"));
        assert!(ok("function eval() {}"));
        assert!(ok("var public = 1; ({ public: 1 }).public;"));
    }

    #[test]
    fn strict_literals_and_with() {
        assert!(!ok("'use strict'; with (a) {}"));
        assert!(!ok("({ m() { 'use strict'; with (a) {} } })"));
        assert!(!ok("class C { m() { with (a) {} } }"));
        assert!(!ok("'use strict'; '\\07';"));
        assert!(!ok("'use strict'; `${'\\07'}`;"));
        assert!(!ok("'use strict'; 010;"));
        assert!(ok("with (a) {}"));
        assert!(ok("'\\07'; 010;"));
        assert!(ok("'use strict'; '\\0'; 0.5; 0;"));
    }

    #[test]
    fn parameters_clash_with_body_lexical_names() {
        assert!(!ok("function f(a) { let a; }"));
        assert!(!ok("(async function (a) { const a = 0; })"));
        assert!(!ok("({ m(a) { class a {} } })"));
        assert!(!ok("(a) => { let a; }"));
        assert!(ok("function f(a) { var a; }"));
        assert!(ok("function f(a) { function a() {} }"));
        assert!(ok("function f(a) { { let a; } }"));
    }

    #[test]
    fn await_is_reserved_in_async_functions() {
        assert!(!ok("(async function () { var await; })"));
        assert!(!ok("(async function () { void await; })"));
        assert!(!ok("(async function () { await: ; })"));
        assert!(!ok("({ async m() { void await; } })"));
        assert!(!ok("(async function* () { var await; })"));
        assert!(!ok("(async function (await) {})"));
        assert!(!ok("(async function* await() {})"));
        assert!(!ok("({ async m(await) {} })"));
        assert!(ok("var await; function f() { var await; }"));
        assert!(ok("(async function () { ({ await: 1 }).await; })"));
        assert!(ok("(async function () { (() => { var await; }); })"));
    }

    #[test]
    fn async_takes_no_line_terminator() {
        assert!(!ok("({ async\n m() {} })"));
        assert!(ok("({ async m() {} })"));
        assert!(ok("({ async: 1, async\n })"));
    }

    #[test]
    fn super_property_needs_a_method() {
        assert!(!ok("(async function () { super.x; })"));
        assert!(!ok("(function () { super.x; })"));
        assert!(!ok("function f() { super.x; }"));
        assert!(!ok("(async function (a = super.x) {})"));
        assert!(!ok("function f() { (() => super.x); }"));
        assert!(ok("({ m() { super.x; } })"));
        assert!(ok("({ m() { (() => super.x); } })"));
        assert!(ok("({ get m() { return super.x; } })"));
        assert!(ok("class C { m() { super.x; } }"));
        assert!(ok("class C { static x = super.y; }"));
        assert!(ok("class C { static { super.y; } }"));
    }

    #[test]
    fn template_escapes() {
        assert!(!ok("`\\u{10FFFFF}`"));
        assert!(!ok("`\\u{10FFFFF}${1}right`"));
        assert!(!ok("`\\07`"));
        assert!(!ok("`\\1`"));
        assert!(!ok("`\\xg0`"));
        assert!(!ok("`\\u00g0`"));
        assert!(!ok("`a${1}\\u{}`"));
        assert!(ok("`\\u{10FFFF}\\x41\\u0041\\0\\n\\``"));
        assert!(ok("tag`\\07 \\u{10FFFFF} \\xg`"));
        assert!(ok("a.b`\\1`"));
    }

    #[test]
    fn optional_chain_takes_no_template() {
        assert!(!ok("a?.b`x`"));
        assert!(!ok("a?.b\n`x`"));
        assert!(!ok("a?.b.c`x`"));
        assert!(ok("(a?.b)`x`"));
        assert!(ok("a?.b + `x`"));
        assert!(ok("a.b`x`"));
    }

    #[test]
    fn for_of_heads_take_no_initializer() {
        assert!(!ok("for (let x = 1 of y) ;"));
        assert!(!ok("for (var x = 1 of y) ;"));
        assert!(!ok("for (const x = 1 in y) ;"));
        assert!(ok("for (let x of y) ;"));
    }

    #[test]
    fn assignment_targets() {
        assert!(!ok("(async function () {}) = 1"));
        assert!(!ok("(async function* () {}) = 1"));
        assert!(!ok("(async function foo() { } = 1)"));
        assert!(!ok("({ m() {} } = 1)"));
        assert!(!ok("[{ get x() {} }] = [{}]"));
        assert!(!ok("[...{ get x() {} }] = [[]]"));
        assert!(!ok("({ x: { get x() {} } } = {})"));
        assert!(!ok("[{ set y(v) {} }?.y = 42] = [23]"));
        assert!(!ok("({ x: { set y(v) {} }?.y = 42 } = {})"));
        assert!(!ok("for ({ m() {} } in {}) ;"));
        assert!(!ok("for ([{ get x() {} }] of []) ;"));
        assert!(!ok("a?.b = 1"));
        assert!(!ok("[...a, b] = c"));
        assert!(!ok("[...a,] = c"));
        assert!(!ok("({...a, b} = c)"));
        assert!(!ok("({...[a]} = c)"));
        assert!(!ok("[a] += 1"));
        assert!(!ok("1 = 2"));
        assert!(ok("[a, b = 1, ...c] = d"));
        assert!(ok("({ a, b: c, d: [e], ...f } = g)"));
        assert!(ok("[...[a, b]] = c"));
        assert!(ok("[a.b, c[d], ...e.f] = g"));
        assert!(ok("({ x: a.b = 1 } = c)"));
        assert!(ok("for ([a, b] of c) ;"));
        assert!(ok("a.b = 1; a[b] += 1; a = b = c"));
    }
}
