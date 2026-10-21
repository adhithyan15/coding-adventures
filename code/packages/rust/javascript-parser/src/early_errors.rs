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
//! explicit stack (no recursion) in a single pass, so it is linear in the
//! tree however blocks nest, and can't overflow on deep nesting. Only block and switch scopes are checked; the
//! same rules at function-body and program level are a follow-up.

use std::collections::HashMap;

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

/// Check every block and switch case block under `root`. Returns the first
/// violation as a located message.
///
/// One pass over the tree, linear however blocks nest (security review):
///
/// - On entering a block or case block, its own lexical names are checked
///   for duplicates (rule 1) and added to `active`, a count of the names
///   lexically declared by every block scope open since the nearest function
///   boundary. They're removed again when the walk leaves that scope.
/// - Each `var` binding found (a `var` statement or `for (var …)` head) is
///   looked up in `active` (rule 2). A `var` hoists through every block up to
///   its function, so it clashes with a lexical name of any enclosing block on
///   the way.
/// - A `function_body` or `class_body` starts a new var scope: `active` is set
///   aside and restored on exit. Matching on these two rules, not on the many
///   function-ish rules above them, also covers grammars that spell methods
///   differently (ES2015 inlines them in `property_definition`).
pub fn check(root: &GrammarASTNode) -> Result<(), String> {
    enum Step<'a> {
        Enter(&'a GrammarASTNode, bool),
        /// Leave a block scope: forget its lexical names.
        LeaveScope(Vec<&'a str>),
        /// Leave a function or class body: restore the outer scopes.
        LeaveBoundary,
    }
    let mut active: HashMap<&str, usize> = HashMap::new();
    let mut saved: Vec<HashMap<&str, usize>> = Vec::new();
    let mut stack: Vec<Step> = vec![Step::Enter(root, has_use_strict_directive(root))];
    while let Some(step) = stack.pop() {
        let (node, strict) = match step {
            Step::Enter(node, strict) => (node, strict),
            Step::LeaveScope(names) => {
                for name in names {
                    if let Some(count) = active.get_mut(name) {
                        *count -= 1;
                        if *count == 0 {
                            active.remove(name);
                        }
                    }
                }
                continue;
            }
            Step::LeaveBoundary => {
                active = saved.pop().unwrap_or_default();
                continue;
            }
        };
        match node.rule_name.as_str() {
            rule @ ("block" | "switch_statement") => {
                let items =
                    if rule == "block" { block_items(node) } else { case_block_items(node) };
                let names = check_duplicates(&items, strict)?;
                for &name in &names {
                    *active.entry(name).or_insert(0) += 1;
                }
                stack.push(Step::LeaveScope(names));
            }
            "function_body" | "class_body" => {
                saved.push(std::mem::take(&mut active));
                stack.push(Step::LeaveBoundary);
            }
            "variable_statement" => check_vars(node, &active)?,
            "for_statement" | "for_in_statement" | "for_of_statement" | "for_await_of_statement"
                if has_token(node, "var") =>
            {
                for decl in children(node).take_while(|n| {
                    matches!(n.rule_name.as_str(), "variable_declaration_list" | "variable_declaration")
                }) {
                    check_vars(decl, &active)?;
                }
            }
            _ => {}
        }
        // Children in reverse, so they are entered in source order.
        let class = matches!(node.rule_name.as_str(), "class_declaration" | "class_expression");
        for child in node.children.iter().rev() {
            if let ASTNodeOrToken::Node(child) = child {
                let child_strict = strict
                    // All parts of a class, its heritage included, are strict.
                    || class
                    || child.rule_name == "class_body"
                    // A function body's own directive prologue.
                    || (child.rule_name == "function_body" && has_use_strict_directive(child));
                stack.push(Step::Enter(child, child_strict));
            }
        }
    }
    Ok(())
}

/// Rule 2 for one `var` binding list: none of its names may be lexically
/// declared by an enclosing block scope since the nearest function boundary.
fn check_vars(decl: &GrammarASTNode, active: &HashMap<&str, usize>) -> Result<(), String> {
    let mut names = Vec::new();
    bound_names(decl, &mut names);
    for (name, line, column) in names {
        if active.contains_key(name) {
            return Err(format!(
                "early error at {line}:{column}: `{name}` is declared with both var and \
                 let/const/class/function (redeclaration, ECMA-262 §14.2.1)"
            ));
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

/// Rule 1 for one scope's items. Returns the scope's lexical names.
fn check_duplicates<'a>(items: &[&'a GrammarASTNode], strict: bool) -> Result<Vec<&'a str>, String> {
    let mut lexical: Vec<Lexical> = Vec::new();
    for item in items {
        lexically_declared(item, &mut lexical);
    }
    let mut seen: HashMap<&str, bool> = HashMap::new();
    for l in &lexical {
        if let Some(&prev_plain) = seen.get(l.name) {
            let annex_b = !strict && prev_plain && l.plain_function;
            if !annex_b {
                return Err(format!(
                    "early error at {}:{}: `{}` is declared twice in one block \
                     (redeclaration, ECMA-262 §14.2.1)",
                    l.line, l.column, l.name
                ));
            }
        }
        seen.insert(l.name, l.plain_function);
    }
    Ok(seen.into_keys().collect())
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

/// `node`'s node children, without allocating.
fn children(node: &GrammarASTNode) -> impl Iterator<Item = &GrammarASTNode> {
    node.children.iter().filter_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        ASTNodeOrToken::Token(_) => None,
    })
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
        // A var in a sibling or already-closed scope doesn't clash.
        assert!(ok("{ { let f; } var f; }"));
        assert!(ok("{ var f; { let f; } }"));
        // A class field's or static block's var scope is its own.
        assert!(ok("{ let f; class C { static { var f; } m(){ var f; } } }"));
    }

    #[test]
    fn object_methods_are_var_boundaries_in_every_grammar() {
        // ES2015 spells object methods inline in `property_definition`
        // (security review): `function_body` is the boundary that holds.
        for v in [EsVersion::Es2015, EsVersion::Es2025] {
            for src in ["{ let f; x = { m(){ var f; } }; }", "{ let f; x = { get y(){ var f; } }; }"] {
                if let Err(e) = parse_javascript_typed(src, v) {
                    assert!(!e.contains("early error"), "{v:?} {src:?}: {e}");
                }
            }
        }
    }

    #[test]
    fn class_heritage_is_strict() {
        assert!(!ok("class C extends (function(){ { function f(){} function f(){} } }) {}"));
    }

    #[test]
    fn the_check_is_linear() {
        // Many lets and vars in one block: a Vec lookup made this O(L x V).
        let n = 5_000;
        let lets: String = (0..n).map(|i| format!("let l{i};")).collect();
        let vars: String = (0..n).map(|i| format!("var v{i};")).collect();
        let src = format!("{{{lets}{vars}}}");
        let ast = crate::parse_javascript_typed(&src, EsVersion::Es2025);
        assert!(ast.is_ok());
        // Deeply nested blocks over a large body: each block used to re-walk
        // its whole subtree.
        let src = format!("{}{}{}", "{let a;".repeat(40), "x=1;".repeat(10_000), "}".repeat(40));
        let root = crate::parse_javascript(&src, "es2025").expect("parses");
        let start = std::time::Instant::now();
        assert!(super::check(&root).is_ok());
        assert!(start.elapsed() < std::time::Duration::from_secs(5), "{:?}", start.elapsed());
    }
}
