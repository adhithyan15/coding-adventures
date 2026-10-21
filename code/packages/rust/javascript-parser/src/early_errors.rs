//! Early errors for redeclarations in a block (#17877).
//!
//! ECMA-262 §14.2.1 (Block) and §14.12.1 (CaseBlock) make two things
//! Syntax Errors before any code runs:
//!
//! 1. Its **LexicallyDeclaredNames** contain a duplicate. Those are the names
//!    bound by `let`, `const`, `using`, `class` and, inside a block, every
//!    kind of function declaration.
//! 2. A lexically declared name also occurs in its **VarDeclaredNames**: the
//!    names bound by any `var` inside it, including nested blocks and `for`
//!    heads, but not inside a nested function or class.
//!
//! Annex B.3.2.4 relaxes rule 1 in sloppy code: two *plain* `function`
//! declarations of the same name in one block are allowed. Async functions,
//! generators and classes are not.
//!
//! ```text
//! { let f; async function f(){} }     error, rule 1
//! { async function f(){}; var f; }    error, rule 2
//! { function f(){} function f(){} }   error in strict code only
//! switch (0) { case 1: let f; default: class f {} }   error, the case
//!                                     clauses share one scope
//! ```
//!
//! ## Why this lives in the parser
//!
//! Before #17877 the grammar rejected every declaration inside a block, so
//! these programs failed to parse by accident: test262 counted ~200 of its
//! redeclaration tests as passing for the wrong reason. Once blocks hold
//! declarations, the check has to be explicit, or closurec would accept (and
//! compile) programs every engine rejects. Upstream Closure refuses them too.
//!
//! The walk is over the CST from [`crate::parse_javascript_typed`], with an
//! explicit stack (no recursion), so it is linear in the tree and can't
//! overflow on deep nesting. Only block and switch scopes are checked; the
//! same rules at function-body and program level are a follow-up.

use std::collections::HashMap;

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

/// Check every block and switch case block under `root`. Returns the first
/// violation as a located message.
pub fn check(root: &GrammarASTNode) -> Result<(), String> {
    // (node, strict). Strictness only matters for the Annex B exception.
    let root_strict = has_use_strict_directive(root);
    let mut stack: Vec<(&GrammarASTNode, bool)> = vec![(root, root_strict)];
    while let Some((node, strict)) = stack.pop() {
        match node.rule_name.as_str() {
            "block" => check_scope(&block_items(node), strict)?,
            "switch_statement" => check_scope(&case_block_items(node), strict)?,
            _ => {}
        }
        for child in nodes(node) {
            let child_strict = strict
                || match child.rule_name.as_str() {
                    // A function body's own directive prologue.
                    "function_body" => has_use_strict_directive(child),
                    // All parts of a class are strict.
                    "class_body" => true,
                    _ => false,
                };
            stack.push((child, child_strict));
        }
    }
    Ok(())
}

/// One lexically declared name.
struct Lexical<'a> {
    name: &'a str,
    /// A plain `function` declaration: the only kind Annex B lets repeat.
    plain_function: bool,
    line: usize,
    column: usize,
}

fn check_scope(items: &[&GrammarASTNode], strict: bool) -> Result<(), String> {
    let mut lexical: Vec<Lexical> = Vec::new();
    for item in items {
        lexically_declared(item, &mut lexical);
    }
    if lexical.is_empty() {
        return Ok(());
    }
    // Rule 1: duplicates.
    let mut seen: HashMap<&str, bool> = HashMap::new();
    for l in &lexical {
        if let Some(&prev_plain) = seen.get(l.name) {
            let annex_b = !strict && prev_plain && l.plain_function;
            if !annex_b {
                return Err(redeclared(l, "is declared twice in one block"));
            }
        }
        seen.insert(l.name, l.plain_function);
    }
    // Rule 2: a lexical name also declared with `var`.
    let mut var_names: Vec<&str> = Vec::new();
    for item in items {
        var_declared(item, &mut var_names);
    }
    for l in &lexical {
        if var_names.contains(&l.name) {
            return Err(redeclared(l, "is declared with both var and let/const/class/function"));
        }
    }
    Ok(())
}

fn redeclared(l: &Lexical, what: &str) -> String {
    format!(
        "early error at {}:{}: `{}` {what} (redeclaration, ECMA-262 §14.2.1)",
        l.line, l.column, l.name
    )
}

/// The statement-list items of a `block`: its node children. A declaration
/// item is the declaration node itself (the grammar's group is inlined), a
/// statement is a `statement` node.
fn block_items(block: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    nodes(block)
}

/// Every item of every `case`/`default` clause: one CaseBlock, one scope.
fn case_block_items(switch: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    let mut out = Vec::new();
    for clause in nodes(switch) {
        match clause.rule_name.as_str() {
            // The first node child of a `case` clause is its test expression.
            "case_clause" => out.extend(nodes(clause).into_iter().skip(1)),
            "default_clause" => out.extend(nodes(clause)),
            _ => {}
        }
    }
    out
}

/// LexicallyDeclaredNames of one statement-list item.
fn lexically_declared<'a>(item: &'a GrammarASTNode, out: &mut Vec<Lexical<'a>>) {
    // A `statement` wrapper only matters when it holds a lexical declaration.
    let item = match item.rule_name.as_str() {
        "statement" => match nodes(item).as_slice() {
            [inner] => *inner,
            _ => return,
        },
        _ => item,
    };
    match item.rule_name.as_str() {
        "function_declaration" => push_decl_name(item, true, out),
        "generator_declaration" | "async_function_declaration" | "async_generator_declaration"
        | "class_declaration" => push_decl_name(item, false, out),
        "decorated_class_declaration" => {
            if let Some(cd) = nodes(item).into_iter().find(|n| n.rule_name == "class_declaration") {
                push_decl_name(cd, false, out);
            }
        }
        "lexical_declaration" | "using_declaration" | "await_using_declaration" => {
            let mut names = Vec::new();
            for list in nodes(item).into_iter().filter(|n| n.rule_name == "binding_list") {
                bound_names(list, &mut names);
            }
            out.extend(names.into_iter().map(|(name, line, column)| Lexical {
                name,
                plain_function: false,
                line,
                column,
            }));
        }
        _ => {}
    }
}

/// The declared name of a function or class declaration: the first NAME
/// token after the `function` or `class` keyword. (`async` lexes as a NAME
/// too, so the first NAME overall would be wrong for `async function f`.)
fn push_decl_name<'a>(decl: &'a GrammarASTNode, plain_function: bool, out: &mut Vec<Lexical<'a>>) {
    let name = decl
        .children
        .iter()
        .skip_while(|c| {
            !matches!(c, ASTNodeOrToken::Token(t) if !is_string(t)
                && (t.value == "function" || t.value == "class"))
        })
        .find_map(|c| match c {
            ASTNodeOrToken::Token(t) if is_name(t) => Some(t),
            _ => None,
        });
    if let Some(t) = name {
        out.push(Lexical { name: &t.value, plain_function, line: t.line, column: t.column });
    }
}

/// VarDeclaredNames of `item`: names bound by `var` anywhere inside it,
/// crossing blocks and statement bodies but not function or class
/// boundaries.
fn var_declared<'a>(item: &'a GrammarASTNode, out: &mut Vec<&'a str>) {
    let mut stack = vec![item];
    while let Some(node) = stack.pop() {
        match node.rule_name.as_str() {
            // New var scopes (or no statements at all).
            "function_declaration" | "generator_declaration" | "async_function_declaration"
            | "async_generator_declaration" | "function_expression" | "generator_expression"
            | "async_function_expression" | "async_generator_expression" | "arrow_function"
            | "async_arrow_function" | "method_definition" | "async_method"
            | "private_method_definition" | "class_declaration" | "class_expression"
            | "decorated_class_declaration" | "static_block" => continue,
            "variable_statement" => {
                let mut names = Vec::new();
                bound_names(node, &mut names);
                out.extend(names.into_iter().map(|(n, _, _)| n));
                continue;
            }
            // `for (var …)` heads: the `var` token followed by the declaration(s).
            "for_statement" | "for_in_statement" | "for_of_statement" | "for_await_of_statement"
                if has_token(node, "var") =>
            {
                for decl in nodes(node).into_iter().take_while(|n| {
                    matches!(n.rule_name.as_str(), "variable_declaration_list" | "variable_declaration")
                }) {
                    let mut names = Vec::new();
                    bound_names(decl, &mut names);
                    out.extend(names.into_iter().map(|(n, _, _)| n));
                }
            }
            _ => {}
        }
        stack.extend(nodes(node));
    }
}

/// BoundNames of a declaration list, binding, or pattern: the identifiers it
/// binds, with their positions. Property keys and default-value expressions
/// bind nothing.
fn bound_names<'a>(node: &'a GrammarASTNode, out: &mut Vec<(&'a str, usize, usize)>) {
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        match n.rule_name.as_str() {
            // `NAME` or a pattern first, then an optional `= initializer`.
            "lexical_binding" | "variable_declaration" | "binding_element" | "object_rest_property" => {
                for c in &n.children {
                    match c {
                        ASTNodeOrToken::Token(t) if is_name(t) => {
                            out.push((&t.value, t.line, t.column));
                            break;
                        }
                        ASTNodeOrToken::Node(p) if p.rule_name == "binding_pattern" => {
                            stack.push(p);
                            break;
                        }
                        ASTNodeOrToken::Token(t) if t.value == "..." => {}
                        _ => {}
                    }
                }
            }
            // `key: element` binds the element; shorthand `NAME [= expr]`
            // binds NAME.
            "binding_property" => {
                if has_token(n, ":") {
                    if let Some(el) = nodes(n).into_iter().find(|c| c.rule_name == "binding_element") {
                        stack.push(el);
                    }
                } else if let Some(t) = n.children.iter().find_map(|c| match c {
                    ASTNodeOrToken::Token(t) if is_name(t) => Some(t),
                    _ => None,
                }) {
                    out.push((&t.value, t.line, t.column));
                }
            }
            // Lists and patterns: their binding children.
            "binding_list" | "variable_declaration_list" | "variable_statement" | "binding_pattern"
            | "object_binding_pattern" | "array_binding_pattern" => {
                for c in nodes(n).into_iter().rev() {
                    if matches!(
                        c.rule_name.as_str(),
                        "lexical_binding" | "variable_declaration" | "variable_declaration_list"
                            | "binding_pattern" | "object_binding_pattern" | "array_binding_pattern"
                            | "binding_property" | "binding_element" | "object_rest_property"
                    ) {
                        stack.push(c);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Does `body` (a program or function body) open with a `"use strict"`
/// directive? A directive is a leading expression statement that is a lone
/// string literal.
fn has_use_strict_directive(body: &GrammarASTNode) -> bool {
    for element in nodes(body) {
        let mut tokens = Vec::new();
        collect_tokens(element, &mut tokens, 3);
        let is_directive = match tokens.as_slice() {
            [s] | [s, _] => is_string(s) && tokens.get(1).is_none_or(|t| t.value == ";"),
            _ => false,
        };
        if !is_directive {
            return false;
        }
        if tokens[0].value == "use strict" {
            return true;
        }
    }
    false
}

/// Up to `limit` tokens of `node`'s subtree, in source order: enough to tell
/// a lone string-literal statement from anything longer. Iterative, and it
/// stops as soon as it has `limit` tokens.
fn collect_tokens<'a>(node: &'a GrammarASTNode, out: &mut Vec<&'a lexer::token::Token>, limit: usize) {
    // Each frame: a node and the index of its next child to visit.
    let mut stack: Vec<(&GrammarASTNode, usize)> = vec![(node, 0)];
    while let Some((n, i)) = stack.pop() {
        if out.len() >= limit {
            return;
        }
        let Some(child) = n.children.get(i) else { continue };
        stack.push((n, i + 1));
        match child {
            ASTNodeOrToken::Token(t) => out.push(t),
            ASTNodeOrToken::Node(c) => stack.push((c, 0)),
        }
    }
}

fn nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

fn has_token(node: &GrammarASTNode, value: &str) -> bool {
    node.children
        .iter()
        .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == value && !is_string(t)))
}

fn is_string(t: &lexer::token::Token) -> bool {
    matches!(t.type_, lexer::token::TokenType::String)
}

/// An identifier token (not a keyword, punctuator or literal).
fn is_name(t: &lexer::token::Token) -> bool {
    matches!(t.type_, lexer::token::TokenType::Name) && t.type_name.as_deref().is_none_or(|n| n == "NAME")
}

#[cfg(test)]
mod tests {
    use crate::parse_javascript_typed;
    use coding_adventures_javascript_tokens::EsVersion;

    fn ok(src: &str) -> bool {
        parse_javascript_typed(src, EsVersion::Es2025).is_ok()
    }

    #[test]
    fn duplicate_lexical_names_in_a_block_are_errors() {
        assert!(!ok("{ let f; let f; }"));
        assert!(!ok("{ let f; async function f(){} }"));
        assert!(!ok("{ class f {} function f(){} }"));
        assert!(!ok("{ const f = 1; function* f(){} }"));
        assert!(!ok("{ let [a, {b: f}] = x; let f; }"));
    }

    #[test]
    fn a_name_both_lexical_and_var_is_an_error() {
        assert!(!ok("{ async function f(){}; var f; }"));
        assert!(!ok("{ let f; { var f; } }"));
        assert!(!ok("{ function f(){} for (var f in o); }"));
        assert!(!ok("function x(){ { class f {} var f; } }"));
    }

    #[test]
    fn switch_clauses_share_one_scope() {
        assert!(!ok("switch (0) { case 1: let f; default: async function f(){} }"));
        assert!(!ok("switch (0) { case 1: let f; case 2: var f; }"));
        assert!(ok("switch (0) { case 1: let f; } switch (0) { case 1: let f; }"));
    }

    #[test]
    fn annex_b_allows_duplicate_plain_functions_in_sloppy_code_only() {
        assert!(ok("{ function f(){} function f(){} }"));
        assert!(!ok("'use strict'; { function f(){} function f(){} }"));
        assert!(!ok("function g(){ 'use strict'; { function f(){} function f(){} } }"));
        assert!(!ok("class C { m(){ { function f(){} function f(){} } } }"));
        // Async functions and generators never qualify.
        assert!(!ok("{ async function f(){} async function f(){} }"));
    }

    #[test]
    fn valid_programs_are_untouched() {
        assert!(ok("{ let a; { let a; } }"));
        assert!(ok("{ var a; var a; }"));
        assert!(ok("{ let f; } var f;"));
        assert!(ok("{ let f; function g(){ var f; } }"));
        assert!(ok("{ let f; (function(){ var f; }); }"));
        assert!(ok("{ let f; x = () => { var f; }; }"));
        assert!(ok("{ let {a: b} = o; var a; }"));
        assert!(ok("{ let [a = f] = o; var f; }"));
        assert!(ok("{ function f(){} } { function f(){} }"));
        assert!(ok("var f; function f(){}"));
    }
}
