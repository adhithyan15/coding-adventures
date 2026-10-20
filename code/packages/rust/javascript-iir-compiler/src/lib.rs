//! JavaScript parse tree (CST) to InterpreterIR lowering for a bounded
//! numeric slice.
//!
//! Semantic IR is not part of the execution path. Unsupported JavaScript
//! syntax is rejected rather than silently assigned another language's rules.
//!
//! # Pipeline: the compiler walks the parse tree
//!
//! ```text
//! source
//!   │  coding_adventures_javascript_parser::parse_javascript_cst_division_only
//!   │    GrammarLexer over es2020.tokens (REGEX off) → GrammarParser over
//!   │    es2020.grammar, with the ASI retry loop and the flat-spine bound
//!   ▼
//! GrammarASTNode  (the CST: one node per grammar rule that matched)
//!   │  compile_ast — this crate
//!   │    check_cst   validate, and keep every error the old typed-AST stage gave
//!   │    pass 1      collect the script's lexical names (early errors)
//!   │    pass 2      charge the node/depth budget, lower to IIR
//!   ▼
//! IIRModule  ──▶ run_source executes it on vm-core
//! ```
//!
//! LANG84 decision 6 says *the IIR compiler walks the parse tree*: nothing may
//! turn the CST into an owned, language-specific AST before IIR compilation.
//! Up to 0.2.0 this crate compiled `javascript_ast::Program`, which the parser
//! crate's hand-written bridge built from the CST. Since 0.3.0 the only input
//! is the `GrammarASTNode` itself. The typed AST and its bridge still exist for
//! other consumers (closurec); this crate never names them.
//!
//! # Reading expressions straight off the CST
//!
//! The grammar spells precedence as a ladder of rules, so even a bare `1` is
//! wrapped in some twenty single-child nodes:
//!
//! ```text
//! expression → assignment_expression → conditional_expression → … →
//! multiplicative_expression → exponentiation_expression → unary_expression →
//! postfix_expression → left_hand_side_expression → optional_chain_expression →
//! member_expression → primary_expression → NUMBER "1"
//! ```
//!
//! and a binary chain is one flat node, `additive_expression = A { (+|-) A }`:
//!
//! ```text
//! 8 - 2 + 1    additive_expression [ A(8), "-", A(2), "+", A(1) ]
//! ```
//!
//! The compiler never copies these into a second tree. An [`Expr`] is a small
//! `Copy` handle *into* the CST, and [`classify`] answers one question about
//! it — "what kind of expression is this?" — by looking at a handful of
//! children:
//!
//! | CST                                     | `Expr` handle           | `Shape`                         |
//! | --------------------------------------- | ----------------------- | ------------------------------- |
//! | single-child wrapper, `( e )`           | `Node(wrapper)`         | whatever the inner node is      |
//! | `A op B op C`, a flat binary node       | `Node(n)`               | `Binary(Fold(n, ..B), op, C)`   |
//! | first *k* operands of that node         | `Fold { node, last }`   | `Binary(Fold(..), op, operand)` |
//! | `f(a).b[c]`, a flat suffix chain        | `Node(n)`               | `Member(Chain(n, ..), [c])`     |
//! | first *k* children of that chain        | `Chain { node, end }`   | `Call` / `Member` of a prefix   |
//! | `. NAME` property token                 | `Name(token)`           | `Identifier`                    |
//!
//! Left associativity is read off the flat list by index arithmetic: the
//! left operand of `A - B + C` is "the first two operands of the same node",
//! which is just `Fold { node, last: 2 }`. Every accessor is O(1) or
//! O(children of one node), as LANG84 C1.6 requires of a borrowed view.
//!
//! The shapes mirror, one for one, the typed-AST variants the 0.2.0 compiler
//! matched on (`Identifier`, `NumericLiteral`, `UnaryExpression`,
//! `BinaryExpression`, `CallExpression`, `MemberExpression`, and "anything
//! else"). That is what keeps the node budget, the depth limit, every error
//! and every emitted instruction exactly as they were.
//!
//! # The accepted program shape
//!
//! A program is a flat list of top-level items, each one of:
//!
//! ```text
//! expression statement   1 + 2;   console.log(x, x * 4);
//! lexical declaration    let x = 1 / 2;   const y = -0;
//! ```
//!
//! There is no control flow and no assignment yet. That makes every binding's
//! state *statically* known at every program point, which is what lets the
//! compiler decide ReferenceErrors ahead of time:
//!
//! ```text
//! console.log(a);   // a is declared below → TDZ read  → ReferenceError
//! let a = 2;        // a becomes initialized here
//! console.log(a);   // a is initialized   → plain register read
//! console.log(z);   // z is declared nowhere → ReferenceError
//! ```
//!
//! Each binding lives in an IIR register named `js.<name>`. A `.` can never
//! appear in a JavaScript IdentifierName, so that namespace cannot meet the
//! compiler's temporaries (`js0`, `js1`, ...) or the runner's builtins
//! (`js_console_log`, ...). The table below is the whole register namespace:
//!
//! | Spelling        | Owner                 | Third character |
//! | --------------- | --------------------- | --------------- |
//! | `js<digits>`    | compiler temporaries  | a digit         |
//! | `js_<name>`     | runner builtins       | `_`             |
//! | `js.<name>`     | user `let` / `const`  | `.`             |
//!
//! # Editions
//!
//! [`compile_source`] parses ECMAScript 2020, as it always has (LANG84 phase
//! 6d makes the edition selectable). [`compile_ast`] keys only on rule names
//! and token types that the `es*.grammar` files share, so a CST from another
//! edition plugs in unchanged: es5's `arguments` rule lists its expressions
//! directly instead of through `argument_list`, which the argument reader
//! accepts; an edition without `let`/`const` simply never produces a
//! `lexical_declaration`; and a rule this file does not know classifies as
//! "unsupported", never as something it is not.

use coding_adventures_javascript_parser::parse_javascript_cst_division_only;
#[cfg(test)]
use coding_adventures_javascript_parser::parse_javascript_typed;
use coding_adventures_javascript_tokens::EsVersion;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use vm_core::{value::Value, VMCore};

/// Bound source read and parse work for the first native interpreter pilot.
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MAX_AST_NODES: usize = 16_384;
const MAX_EXPRESSION_DEPTH: usize = 64;

/// The ECMAScript edition whose `.tokens`/`.grammar` pair [`compile_source`]
/// parses with. Unchanged from the typed-AST pipeline.
const SOURCE_EDITION: EsVersion = EsVersion::Es2020;

/// Runner builtin raised by reading a name that no declaration binds.
const NOT_DEFINED_BUILTIN: &str = "js_reference_error_not_defined";
/// Runner builtin raised by reading a lexical binding inside its temporal dead
/// zone: after the script starts but before its declaration has executed.
const BEFORE_INITIALIZATION_BUILTIN: &str = "js_reference_error_before_initialization";

/// Names the native runner's global object is defined to hold.
///
/// ECMAScript resolves an identifier that no `let`/`const` binds against the
/// global object. Every name here *is* a global in a real engine — `NaN`
/// reads as a Number, `Math` as an object — but the pilot implements none of
/// them as values yet. If the compiler treated them as undeclared it would
/// report `ReferenceError: NaN is not defined`, a lie. So reading one, or
/// declaring one with `let`/`const`, is a compile-time "unsupported" error.
///
/// The list is the ECMA-262 global object (value, function, constructor and
/// namespace properties, including recent additions), Annex B's `escape` and
/// `unescape`, ECMA-402's `Intl`, and the runner's own host object `console`.
/// Listing a name that some engine lacks only rejects more programs, so the
/// list errs toward inclusion. Node-only host globals (`process`, `require`,
/// `Buffer`, ...) are deliberately absent: this runner is a different host
/// and does not define them, exactly as a bare engine shell does not.
const RUNNER_GLOBAL_NAMES: &[&str] = &[
    // Host object provided by this runner.
    "console",
    // Value properties of the global object.
    "globalThis",
    "Infinity",
    "NaN",
    "undefined",
    // Function properties.
    "eval",
    "isFinite",
    "isNaN",
    "parseFloat",
    "parseInt",
    "decodeURI",
    "decodeURIComponent",
    "encodeURI",
    "encodeURIComponent",
    "escape",
    "unescape",
    // Constructor properties.
    "AggregateError",
    "Array",
    "ArrayBuffer",
    "AsyncDisposableStack",
    "BigInt",
    "BigInt64Array",
    "BigUint64Array",
    "Boolean",
    "DataView",
    "Date",
    "DisposableStack",
    "Error",
    "EvalError",
    "FinalizationRegistry",
    "Float16Array",
    "Float32Array",
    "Float64Array",
    "Function",
    "Int8Array",
    "Int16Array",
    "Int32Array",
    "Iterator",
    "Map",
    "Number",
    "Object",
    "Promise",
    "Proxy",
    "RangeError",
    "ReferenceError",
    "RegExp",
    "Set",
    "SharedArrayBuffer",
    "String",
    "SuppressedError",
    "Symbol",
    "SyntaxError",
    "TypeError",
    "Uint8Array",
    "Uint8ClampedArray",
    "Uint16Array",
    "Uint32Array",
    "URIError",
    "WeakMap",
    "WeakRef",
    "WeakSet",
    // Namespace objects.
    "Atomics",
    "Intl",
    "JSON",
    "Math",
    "Reflect",
];

/// Parse source and compile the currently supported JavaScript subset.
pub fn compile_source(source: &str, module_name: &str) -> Result<IIRModule, String> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "JavaScript source exceeds the {MAX_SOURCE_BYTES}-byte native pilot limit"
        ));
    }
    // Regex literals are outside the native subset. Select the division goal
    // before tokenization so two arithmetic slashes cannot become one regex.
    let cst = parse_javascript_cst_division_only(source, SOURCE_EDITION)?;
    compile_ast(&cst, module_name)
}

/// Compile a JavaScript parse tree (a `program` CST) directly to InterpreterIR.
///
/// `ast` is the `GrammarASTNode` that the generic `GrammarParser` builds from
/// an `es*.grammar` file — for instance from
/// `coding_adventures_javascript_parser::parse_javascript_cst_division_only`
/// or `parse_javascript_typed`. Compilation runs three passes:
///
/// 0. **Validation** ([`check_cst`]). Read every top-level item off the tree
///    and reject what the pre-0.3 typed-AST stage rejected, with the same
///    messages, in the same order. That stage converted the whole program
///    before the compiler saw any of it, so these errors still win over
///    everything below.
/// 1. **Declaration instantiation.** Collect every top-level lexical name.
///    A duplicate is an early `SyntaxError`, so the program never starts and
///    produces no output. Knowing all names up front is also what separates a
///    temporal-dead-zone read (declared later) from an undeclared one.
/// 2. **Lowering.** Walk the items in source order, charging the node budget
///    and emitting IIR. A declaration marks its name initialized only after
///    its initializer is lowered, so `let a = a;` reads `a` in its dead zone.
///
/// A caller may build the tree by hand, so every pass is iterative or
/// depth-bounded: a malformed or hostile tree yields an error, never a panic
/// or a stack overflow.
pub fn compile_ast(ast: &GrammarASTNode, module_name: &str) -> Result<IIRModule, String> {
    let items = check_cst(ast)?;
    // Every item costs at least one node, so a longer body can never fit the
    // budget. Checking first also bounds the declaration pre-pass below.
    if items.len() > MAX_AST_NODES {
        return Err("JavaScript AST exceeds the native pilot node limit".into());
    }
    let mut compiler = Compiler {
        lexical_names: collect_lexical_names(&items)?,
        ..Compiler::default()
    };
    let mut remaining_nodes = MAX_AST_NODES;
    for item in &items {
        if remaining_nodes == 0 {
            return Err("JavaScript AST exceeds the native pilot node limit".into());
        }
        remaining_nodes -= 1;
        match item {
            Item::Expression(expression) => {
                check_expression_budget(*expression, &mut remaining_nodes)?;
                compiler.compile_statement(*expression)?;
            }
            Item::Declaration(declaration) => {
                let (name, init) = lexical_binding(declaration)?;
                // One node for the declarator and one for its binding name,
                // then the initializer pays like any other expression.
                if remaining_nodes < 2 {
                    return Err("JavaScript AST exceeds the native pilot node limit".into());
                }
                remaining_nodes -= 2;
                check_expression_budget(init, &mut remaining_nodes)?;
                compiler.compile_lexical_binding(name, init)?;
            }
            Item::Unsupported => {
                return Err("unsupported JavaScript statement in native VM pilot".into())
            }
        }
    }
    compiler.emit("ret_void", None, vec![], "void");
    let mut module = IIRModule::new(module_name, "javascript");
    module.entry_point = Some("main".into());
    module.add_or_replace(IIRFunction::new(
        "main",
        vec![],
        "void",
        compiler.instructions,
    ));
    let errors = module.validate();
    if !errors.is_empty() {
        return Err(format!("invalid JavaScript IIR: {}", errors.join("; ")));
    }
    Ok(module)
}

// Pass 1: the script's LexicallyDeclaredNames. ECMAScript makes a repeated
// name an early error (Script static semantics), and V8 words it exactly as
// below. Each declaration's shape is validated here too, so pass 2 can rely
// on `lexical_binding` succeeding for every name collected.
fn collect_lexical_names(items: &[Item<'_>]) -> Result<HashSet<String>, String> {
    let mut names = HashSet::new();
    for item in items {
        let Item::Declaration(declaration) = item else {
            continue;
        };
        let (name, _) = lexical_binding(declaration)?;
        if !names.insert(name.to_string()) {
            return Err(format!(
                "SyntaxError: Identifier '{name}' has already been declared"
            ));
        }
    }
    Ok(names)
}

// Accept exactly `let NAME = INIT;` or `const NAME = INIT;` and return its
// parts. Every other declaration shape is outside this slice:
//
//   var x = 1;        function-scoped hoisting with `undefined` — not modelled
//   let x;            would need the `undefined` value
//   let a = 1, b = 2; kept to one declarator to keep the slice minimal
//   let [a] = xs;     destructuring needs arrays/objects (rejected by check_cst)
fn lexical_binding<'a>(declaration: &Declaration<'a>) -> Result<(&'a str, Expr<'a>), String> {
    if declaration.kind == DeclarationKind::Var {
        return Err(
            "unsupported JavaScript `var` declaration in native VM pilot; use `let` or `const`"
                .into(),
        );
    }
    let mut declarators = node_children(declaration.list);
    let (Some(declarator), None) = (declarators.next(), declarators.next()) else {
        return Err(
            "native let/const pilot requires exactly one declarator per declaration".into(),
        );
    };
    let name = binding_name(declarator)?;
    check_binding_name(name)?;
    // `const k;` is a real JavaScript early error, worded here as V8 words
    // it; `let x;` is valid JavaScript that this slice merely cannot run yet.
    let init = initializer(declarator).ok_or(match declaration.kind {
        DeclarationKind::Const => "SyntaxError: Missing initializer in const declaration",
        _ => "native let/const pilot requires an initializer",
    })?;
    Ok((name, init))
}

fn check_binding_name(name: &str) -> Result<(), String> {
    check_identifier_spelling(name)?;
    if RUNNER_GLOBAL_NAMES.contains(&name) {
        return Err(format!(
            "native let/const pilot cannot bind the global name `{name}`"
        ));
    }
    Ok(())
}

// The source lexer only produces ASCII `NAME` tokens, but `compile_ast` also
// takes trees built by other code. Accept only a plain IdentifierName
// spelling, so a forged token cannot smuggle `.`, spaces or control
// characters into register names and diagnostics. Unicode escapes (`a`)
// are rejected too: a token keeps its raw spelling, so `a` and `a` would
// otherwise be two different names although JavaScript treats them as one.
fn check_identifier_spelling(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let starts = chars
        .next()
        .is_some_and(|c| c == '$' || c == '_' || c.is_alphabetic());
    let continues = chars
        .all(|c| c == '$' || c == '_' || c == '\u{200C}' || c == '\u{200D}' || c.is_alphanumeric());
    if starts && continues {
        Ok(())
    } else {
        Err("unsupported JavaScript identifier spelling in native VM pilot".into())
    }
}

// Charge an expression against the node budget and the depth limit without
// recursing. The parser's depth guard protects source input, but compile_ast
// also accepts trees built by other code. The node count bounds generated IIR
// and VM work; the depth limit bounds the recursion of `compile_number`.
//
// One node is one `Shape`: wrappers, parentheses and the operator tokens of a
// flat chain are free, exactly as they were invisible in the typed AST.
fn check_expression_budget(expr: Expr<'_>, remaining: &mut usize) -> Result<(), String> {
    let mut pending = vec![(expr, 1_usize)];
    while let Some((expr, depth)) = pending.pop() {
        if depth > MAX_EXPRESSION_DEPTH {
            return Err("JavaScript AST exceeds the native pilot depth limit".into());
        }
        if *remaining == 0 {
            return Err("JavaScript AST exceeds the native pilot node limit".into());
        }
        *remaining -= 1;
        // Children are pushed right to left so they pop in source order.
        match classify(expr)? {
            Shape::Unary { argument, .. } => pending.push((argument, depth + 1)),
            Shape::Binary { left, right, .. } => {
                pending.push((right, depth + 1));
                pending.push((left, depth + 1));
            }
            Shape::Call { callee, arguments } => {
                for argument in arguments.into_iter().rev() {
                    pending.push((argument, depth + 1));
                }
                pending.push((callee, depth + 1));
            }
            Shape::Member {
                object, property, ..
            } => {
                pending.push((property, depth + 1));
                pending.push((object, depth + 1));
            }
            Shape::Identifier(_) | Shape::Number(_) | Shape::Other => {}
        }
    }
    Ok(())
}

// =========================================================================
// Borrowed views over the CST
// =========================================================================
//
// Everything in this section reads the `GrammarASTNode` in place. Nothing
// owns a copy of the tree; the only allocations are small `Vec`s of handles
// (the argument list of one call, the items of the program).

/// One top-level item of the script, read off a `source_element`.
#[derive(Clone, Copy)]
enum Item<'a> {
    /// `expression_statement = expression SEMICOLON`: the expression.
    Expression(Expr<'a>),
    /// `lexical_declaration` (`let`/`const`) or `variable_statement` (`var`).
    Declaration(Declaration<'a>),
    /// Any other statement or declaration (`if`, `function`, `;`, …).
    Unsupported,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DeclarationKind {
    Var,
    Let,
    Const,
}

/// A declaration and its declarator list: `binding_list` (whose children are
/// `lexical_binding` nodes) or `variable_declaration_list` (whose children
/// are `variable_declaration` nodes). Both declarator rules have the shape
/// `( NAME | binding_pattern ) [ EQUALS assignment_expression ]`.
#[derive(Clone, Copy)]
struct Declaration<'a> {
    kind: DeclarationKind,
    list: &'a GrammarASTNode,
}

/// A handle on one expression inside the CST. `Copy`, two words, no tree.
#[derive(Clone, Copy)]
enum Expr<'a> {
    /// Any expression-rule node; single-child wrappers are seen through.
    Node(&'a GrammarASTNode),
    /// The left fold of a flat binary node's children `0..=last`, where
    /// `last` is the (even) index of an operand. `A - B + C` is
    /// `[A, -, B, +, C]`; its left operand is `Fold { last: 2 }` = `A - B`.
    Fold {
        node: &'a GrammarASTNode,
        last: usize,
    },
    /// The first `end` children of a flat suffix chain (`call_expression`,
    /// `member_expression`, `optional_chain_expression`): the base followed
    /// by whole suffixes. `f(a).b` is `[f, (a), ., b]`; its object is
    /// `Chain { end: 2 }` = `f(a)`.
    Chain {
        node: &'a GrammarASTNode,
        end: usize,
    },
    /// The `NAME` token after a `.`: a property name, never a variable read.
    Name(&'a Token),
    /// A spread argument `...xs`. The pilot has no spread; it is one opaque
    /// node, as `SpreadElement` was in the typed AST.
    Spread,
}

/// What an [`Expr`] *is*, one level deep. Children are again handles.
///
/// The variants are the expression kinds the 0.2.0 compiler distinguished;
/// `Other` stands for every other kind (strings, assignments, `?:`, `&&`,
/// arrow functions, …), all of which the pilot rejects.
enum Shape<'a> {
    Identifier(&'a str),
    Number(f64),
    Unary {
        negate: bool,
        argument: Expr<'a>,
    },
    Binary {
        op: BinaryOp,
        left: Expr<'a>,
        right: Expr<'a>,
    },
    Call {
        callee: Expr<'a>,
        arguments: Vec<Expr<'a>>,
    },
    Member {
        object: Expr<'a>,
        property: Expr<'a>,
        computed: bool,
    },
    Other,
}

/// The binary operators the pilot lowers, plus `Other` for the rest
/// (`%`, `<`, `===`, `in`, `<<`, `&`, `**`, …), which it rejects.
#[derive(Clone, Copy)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Other,
}

// ---- small CST helpers ----------------------------------------------------

/// The `Node` children of `node`, in order (tokens skipped).
fn node_children(node: &GrammarASTNode) -> impl Iterator<Item = &GrammarASTNode> {
    node.children.iter().filter_map(|child| match child {
        ASTNodeOrToken::Node(inner) => Some(inner),
        ASTNodeOrToken::Token(_) => None,
    })
}

/// The `Token` children of `node`, in order (nodes skipped).
fn token_children(node: &GrammarASTNode) -> impl Iterator<Item = &Token> {
    node.children.iter().filter_map(|child| match child {
        ASTNodeOrToken::Token(token) => Some(token),
        ASTNodeOrToken::Node(_) => None,
    })
}

/// The only `Node` child of `node`, if it has exactly one.
fn sole_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    let mut nodes = node_children(node);
    match (nodes.next(), nodes.next()) {
        (Some(only), None) => Some(only),
        _ => None,
    }
}

/// Is `child` a token whose effective type (LANG84 C1.5) is `kind`?
fn is_token(child: Option<&ASTNodeOrToken>, kind: &str) -> bool {
    matches!(child, Some(ASTNodeOrToken::Token(token)) if token.effective_type_name() == kind)
}

/// Is `token` the reserved word `word` (a `KEYWORD` token spelled `word`)?
fn is_keyword(token: &Token, word: &str) -> bool {
    token.effective_type_name() == "KEYWORD" && token.value == word
}

/// Does `node` have a token child of effective type `kind`?
fn has_token(node: &GrammarASTNode, kind: &str) -> bool {
    token_children(node).any(|token| token.effective_type_name() == kind)
}

/// Does `node` have the keyword token `word` among its children?
fn has_keyword(node: &GrammarASTNode, word: &str) -> bool {
    token_children(node).any(|token| is_keyword(token, word))
}

/// The child at `index`, if it is a node.
fn node_at(node: &GrammarASTNode, index: usize) -> Option<&GrammarASTNode> {
    match node.children.get(index) {
        Some(ASTNodeOrToken::Node(inner)) => Some(inner),
        _ => None,
    }
}

/// The child at `index`, if it is a node of rule `rule`.
fn rule_at<'a>(node: &'a GrammarASTNode, index: usize, rule: &str) -> Option<&'a GrammarASTNode> {
    node_at(node, index).filter(|inner| inner.rule_name == rule)
}

// ---- errors, in the wording of the typed-AST stage ------------------------
//
// Up to 0.2.0, `compile_source` ran the parser crate's CST → typed-AST
// conversion before compiling, and that conversion rejected a few things the
// compiler supports the *shape* of: a numeric literal that does not decode, a
// destructuring pattern in a declaration, a reserved word used as a value,
// and malformed trees. `check_cst` keeps rejecting them at the same point and
// with the same text, so a caller sees byte-identical errors. The text names
// the old stage ("bridge", "typed AST"); it is kept verbatim on purpose, as a
// stable error format, not because this crate builds a typed AST.

/// `node`'s start position as `line:column`, `?` where unknown.
fn location(node: &GrammarASTNode) -> String {
    match (node.start_line, node.start_column) {
        (Some(line), Some(column)) => format!("{line}:{column}"),
        (Some(line), None) => format!("{line}:?"),
        _ => "?:?".to_string(),
    }
}

/// Valid JavaScript outside the subset the old typed stage represented.
fn unsupported_syntax(rule: &str, at: &GrammarASTNode) -> String {
    format!(
        "UnsupportedSyntax: grammar rule '{rule}' at {} is not yet covered by Phase 1 of the typed AST",
        location(at)
    )
}

/// A tree whose shape the grammar cannot produce (or a literal that does not
/// decode): the old stage's "internal error".
fn malformed(node: &GrammarASTNode, detail: &str) -> String {
    format!(
        "InternalError in bridge for rule '{}': {detail}",
        node.rule_name
    )
}

// ---- validation: check_cst --------------------------------------------------

/// Read the program's items off the CST and reject what the typed-AST stage
/// rejected, before any compilation.
///
/// That stage converted *every* item before the compiler ran, so its errors
/// beat the compiler's own — including the early `SyntaxError`s of pass 1.
/// To keep that precedence this pass also walks every item completely.
///
/// For the item and expression kinds the pilot can compile, the checks and
/// their order are the old stage's own (left to right, depth first). Inside a
/// construct the pilot rejects anyway (a `function`, a string, `a ? b : c`),
/// the walk stops: the compiler reports that construct as unsupported, which
/// is equivalent — both are compile errors with no output — though the text
/// may differ from what the old stage would have said about its insides.
fn check_cst(root: &GrammarASTNode) -> Result<Vec<Item<'_>>, String> {
    if root.rule_name != "program" {
        return Err(malformed(
            root,
            &format!("expected root rule 'program', got '{}'", root.rule_name),
        ));
    }
    let mut items = Vec::new();
    for element in node_children(root) {
        let item = item_of(element)?;
        match item {
            Item::Expression(expression) => check_expression(expression)?,
            Item::Declaration(declaration) => {
                for declarator in node_children(declaration.list) {
                    check_declarator(declarator)?;
                }
            }
            Item::Unsupported => {}
        }
        items.push(item);
    }
    Ok(items)
}

/// Classify one `source_element`.
///
/// ```text
/// source_element ─ statement ─ expression_statement ─ expression
///                             ├ lexical_declaration  ─ "let"|"const" binding_list
///                             └ variable_statement   ─ "var" variable_declaration_list
/// ```
///
/// A declaration directly under `source_element` (no `statement` wrapper) is
/// accepted as well; the grammar never builds one, but the typed AST had that
/// position too, and a hand-built tree may use it.
fn item_of(element: &GrammarASTNode) -> Result<Item<'_>, String> {
    let child = sole_node(element).ok_or_else(|| malformed(element, "expected 1 child"))?;
    let statement = match child.rule_name.as_str() {
        "statement" => {
            sole_node(child).ok_or_else(|| malformed(child, "expected 1 child in statement"))?
        }
        "lexical_declaration" | "variable_statement" => child,
        _ => return Ok(Item::Unsupported),
    };
    let first_node = node_children(statement).next();
    match statement.rule_name.as_str() {
        "expression_statement" => first_node
            .map(|expression| Item::Expression(Expr::Node(expression)))
            .ok_or_else(|| malformed(statement, "expression_statement: missing expression")),
        "lexical_declaration" => {
            let list = first_node
                .ok_or_else(|| malformed(statement, "lexical_declaration: missing binding_list"))?;
            // `lexical_declaration = ( "let" | "const" ) binding_list SEMICOLON`
            let kind = if has_keyword(statement, "let") {
                DeclarationKind::Let
            } else {
                DeclarationKind::Const
            };
            Ok(Item::Declaration(Declaration { kind, list }))
        }
        "variable_statement" => {
            let list = first_node.ok_or_else(|| {
                malformed(statement, "variable_statement: missing declaration list")
            })?;
            Ok(Item::Declaration(Declaration {
                kind: DeclarationKind::Var,
                list,
            }))
        }
        _ => Ok(Item::Unsupported),
    }
}

/// One declarator: no destructuring, a name, and a valid initializer if any.
fn check_declarator(declarator: &GrammarASTNode) -> Result<(), String> {
    if let Some(pattern) = node_children(declarator).find(|n| n.rule_name == "binding_pattern") {
        return Err(unsupported_syntax("binding_pattern", pattern));
    }
    binding_name(declarator)?;
    if let Some(init) = initializer(declarator) {
        check_expression(init)?;
    }
    Ok(())
}

/// The declarator's binding name: its first token that is not the `=`.
fn binding_name(declarator: &GrammarASTNode) -> Result<&str, String> {
    token_children(declarator)
        .find(|token| token.effective_type_name() != "EQUALS")
        .map(|token| token.value.as_str())
        .ok_or_else(|| malformed(declarator, "variable declarator: missing name"))
}

/// The declarator's initializer: its first node child, when that is an
/// expression rule (`assignment_expression` in every edition).
fn initializer(declarator: &GrammarASTNode) -> Option<Expr<'_>> {
    node_children(declarator)
        .next()
        .filter(|init| init.rule_name.contains("expression"))
        .map(Expr::Node)
}

/// Visit every expression the pilot could compile inside `expr`, depth
/// first and left to right, so that the first problem reported is the one
/// the typed-AST stage met first. Iterative: a hand-built tree may be deep.
fn check_expression(expr: Expr<'_>) -> Result<(), String> {
    let mut pending = vec![expr];
    while let Some(expr) = pending.pop() {
        match classify(expr)? {
            Shape::Unary { argument, .. } => pending.push(argument),
            Shape::Binary { left, right, .. } => {
                pending.push(right);
                pending.push(left);
            }
            Shape::Call { callee, arguments } => {
                pending.extend(arguments.into_iter().rev());
                pending.push(callee);
            }
            Shape::Member {
                object, property, ..
            } => {
                pending.push(property);
                pending.push(object);
            }
            Shape::Identifier(_) | Shape::Number(_) | Shape::Other => {}
        }
    }
    Ok(())
}

// ---- classify: what kind of expression is this? -----------------------------

/// The flat left-associative binary rules, `A { op A }`, and the operator
/// tokens each may contain (by effective type; `in`/`instanceof` are
/// keywords). The grammar ladder orders them loosest-binding first.
fn binary_operator(rule: &str, token: &Token) -> Option<BinaryOp> {
    let kind = token.effective_type_name();
    let allowed: &[&str] = match rule {
        "bitwise_or_expression" => &["PIPE"],
        "bitwise_xor_expression" => &["CARET"],
        "bitwise_and_expression" => &["AMPERSAND"],
        "equality_expression" => &[
            "EQUALS_EQUALS",
            "NOT_EQUALS",
            "STRICT_EQUALS",
            "STRICT_NOT_EQUALS",
        ],
        "relational_expression" => {
            if is_keyword(token, "in") || is_keyword(token, "instanceof") {
                return Some(BinaryOp::Other);
            }
            &["LESS_THAN", "GREATER_THAN", "LESS_EQUALS", "GREATER_EQUALS"]
        }
        "shift_expression" => &["LEFT_SHIFT", "RIGHT_SHIFT", "UNSIGNED_RIGHT_SHIFT"],
        "additive_expression" => &["PLUS", "MINUS"],
        "multiplicative_expression" => &["STAR", "SLASH", "PERCENT"],
        _ => return None,
    };
    if !allowed.contains(&kind) {
        return None;
    }
    Some(match kind {
        "PLUS" => BinaryOp::Add,
        "MINUS" => BinaryOp::Sub,
        "STAR" => BinaryOp::Mul,
        "SLASH" => BinaryOp::Div,
        _ => BinaryOp::Other,
    })
}

const LEFT_FOLD_BINARY_RULES: &[&str] = &[
    "bitwise_or_expression",
    "bitwise_xor_expression",
    "bitwise_and_expression",
    "equality_expression",
    "relational_expression",
    "shift_expression",
    "additive_expression",
    "multiplicative_expression",
];

/// Say what `expr` is, one level deep.
///
/// Single-child wrappers (most of the precedence ladder) and parentheses are
/// walked through in a loop, not by recursion, so a hand-built tower of
/// wrappers cannot exhaust the stack. Each step looks at one node's children
/// only, and a multi-operand node is checked once, here, so that the `Fold`
/// and `Chain` handles it hands out can be read by index alone.
fn classify(expr: Expr<'_>) -> Result<Shape<'_>, String> {
    let mut node = match expr {
        Expr::Node(node) => node,
        Expr::Fold { node, last } => return Ok(fold_shape(node, last)),
        Expr::Chain { node, end } => return Ok(chain_shape(node, end)),
        Expr::Name(token) => return Ok(Shape::Identifier(&token.value)),
        Expr::Spread => return Ok(Shape::Other),
    };
    loop {
        let rule = node.rule_name.as_str();
        let operands = node_children(node).count();
        match rule {
            // expression = assignment_expression { COMMA assignment_expression }
            "expression" => match operands {
                0 => return Err(malformed(node, "expression: no children")),
                1 => {}
                _ => return Ok(Shape::Other), // `a, b`: the comma operator
            },
            // One operand passes through; `lhs op rhs` is an assignment.
            "assignment_expression" => match operands {
                1 => {}
                3 => return Ok(Shape::Other),
                n => {
                    return Err(malformed(
                        node,
                        &format!("assignment_expression: unexpected {n} node children"),
                    ))
                }
            },
            "conditional_expression" => match operands {
                1 => {}
                3 => return Ok(Shape::Other), // `a ? b : c`
                n => {
                    return Err(malformed(
                        node,
                        &format!("conditional_expression: {n} node children"),
                    ))
                }
            },
            // `??`, `||`, `&&` short-circuit: not binary arithmetic.
            "nullish_coalescing_expression"
            | "logical_or_expression"
            | "logical_and_expression" => match operands {
                0 => return Err(malformed(node, "logical expression: no children")),
                1 => {}
                _ => return Ok(Shape::Other),
            },
            _ if LEFT_FOLD_BINARY_RULES.contains(&rule) => match operands {
                0 => return Err(malformed(node, "binary expression: no children")),
                1 => {}
                _ => {
                    check_fold_layout(node)?;
                    return Ok(fold_shape(node, node.children.len() - 1));
                }
            },
            // exponentiation_expression = unary_expression [ STAR_STAR exponentiation_expression ]
            // Right-associative, so it is one binary node, never a fold.
            "exponentiation_expression" => match operands {
                1 => {}
                2 => {
                    let mut operand = node_children(node);
                    let (Some(left), Some(right)) = (operand.next(), operand.next()) else {
                        unreachable!("two operands were just counted");
                    };
                    return Ok(Shape::Binary {
                        op: BinaryOp::Other,
                        left: Expr::Node(left),
                        right: Expr::Node(right),
                    });
                }
                n => {
                    return Err(malformed(
                        node,
                        &format!("exponentiation_expression: {n} nodes"),
                    ))
                }
            },
            "unary_expression" => {
                // The operator is a token, the operand a node:
                //   postfix_expression          [ Node ]           pass through
                //   MINUS unary_expression      [ "-", Node ]      negation
                //   PLUS_PLUS unary_expression  [ "++", Node ]     an update
                let argument = node_children(node)
                    .next()
                    .ok_or_else(|| malformed(node, "unary_expression: missing argument"))?;
                let operator =
                    token_children(node).find_map(|token| match token.effective_type_name() {
                        "MINUS" => Some(true),
                        "PLUS" | "BANG" | "TILDE" => Some(false),
                        _ if is_keyword(token, "typeof")
                            || is_keyword(token, "void")
                            || is_keyword(token, "delete") =>
                        {
                            Some(false)
                        }
                        _ => None,
                    });
                if let Some(negate) = operator {
                    return Ok(Shape::Unary {
                        negate,
                        argument: Expr::Node(argument),
                    });
                }
                if has_token(node, "PLUS_PLUS") || has_token(node, "MINUS_MINUS") {
                    return Ok(Shape::Other); // `++x` mutates; not a unary value
                }
                node = argument;
                continue;
            }
            // postfix_expression = left_hand_side_expression [ PLUS_PLUS | MINUS_MINUS ]
            "postfix_expression" => {
                if operands != 1 {
                    return Err(malformed(node, "postfix_expression: unexpected shape"));
                }
                if has_token(node, "PLUS_PLUS") || has_token(node, "MINUS_MINUS") {
                    return Ok(Shape::Other); // `x++`
                }
            }
            "left_hand_side_expression" => {
                if operands != 1 {
                    return Err(malformed(node, "lhs_expression: expected 1 child"));
                }
            }
            "call_expression" | "member_expression" | "optional_chain_expression" => {
                match chain_base(node)? {
                    ChainBase::PassThrough(inner) => {
                        node = inner;
                        continue;
                    }
                    ChainBase::Opaque => return Ok(Shape::Other),
                    ChainBase::Suffixes => {
                        return Ok(chain_shape(node, node.children.len()));
                    }
                }
            }
            // new_expression = member_expression | "new" new_expression
            "new_expression" => {
                if has_keyword(node, "new") {
                    return Ok(Shape::Other); // `new X` without arguments
                }
                if operands != 1 {
                    return Err(malformed(node, "new_expression: expected 1 child"));
                }
            }
            "primary_expression" => {
                let parenthesized = has_token(node, "LPAREN");
                let inner = node_children(node).next();
                match inner {
                    // `( expression )`, or a rule alternative such as
                    // `array_literal`, which classifies on the next turn.
                    Some(inner) if parenthesized || operands == 1 => {
                        node = inner;
                        continue;
                    }
                    _ => {}
                }
                return match token_children(node).next() {
                    Some(token) => primary_token(token, node),
                    None => Err(malformed(node, "primary_expression: unrecognised shape")),
                };
            }
            // Function and class expressions, literals other than numbers,
            // templates, `yield`, `await`, `import(…)`, `import.meta`, a
            // spread element, … — and any rule this file does not know.
            _ => return Ok(Shape::Other),
        }
        // A pass-through rule with exactly one operand: step into it.
        node = node_children(node)
            .next()
            .expect("a pass-through rule was checked to have one operand");
    }
}

/// A flat binary node must alternate operand and operator,
/// `[A, op, B, op, C]`, with every operator one its rule allows. The parser
/// always builds it so; checking once lets [`fold_shape`] index blindly.
fn check_fold_layout(node: &GrammarASTNode) -> Result<(), String> {
    let alternates = node.children.len() % 2 == 1
        && node
            .children
            .iter()
            .enumerate()
            .all(|(index, child)| match child {
                ASTNodeOrToken::Node(_) => index % 2 == 0,
                ASTNodeOrToken::Token(token) => {
                    index % 2 == 1 && binary_operator(&node.rule_name, token).is_some()
                }
            });
    if alternates {
        Ok(())
    } else {
        Err(malformed(
            node,
            "convert_left_fold_binary: operator count < node count",
        ))
    }
}

/// `children[0..=last]` of a checked flat binary node, as one binary step:
///
/// ```text
/// [A, -, B, +, C]   last = 4  →  Binary(Fold(last 2), +, C)
///                   last = 2  →  Binary(Node(A),      -, B)
/// ```
fn fold_shape(node: &GrammarASTNode, last: usize) -> Shape<'_> {
    let operator = match (last >= 2).then(|| node.children.get(last - 1)).flatten() {
        Some(ASTNodeOrToken::Token(operator)) => operator,
        _ => return Shape::Other, // not a handle `classify` hands out
    };
    let (Some(left), Some(right)) = (
        node_at(node, if last == 2 { 0 } else { last - 2 }),
        node_at(node, last),
    ) else {
        return Shape::Other;
    };
    Shape::Binary {
        op: binary_operator(&node.rule_name, operator).unwrap_or(BinaryOp::Other),
        left: if last == 2 {
            Expr::Node(left)
        } else {
            Expr::Fold {
                node,
                last: last - 2,
            }
        },
        right: Expr::Node(right),
    }
}

/// How a suffix-chain node begins.
enum ChainBase<'a> {
    /// No suffixes: the node only wraps its base.
    PassThrough(&'a GrammarASTNode),
    /// A form the pilot never compiles (`new.target`, an optional chain, …).
    Opaque,
    /// A base followed by a checked run of suffixes; see [`chain_shape`].
    Suffixes,
}

/// Check a suffix chain once, and say how it begins.
///
/// The three chain rules share one layout — a base, then suffixes:
///
/// ```text
/// call_expression           = member_expression arguments { suffix }
/// member_expression         = primary_expression { suffix }
///                           | "new" member_expression arguments
///                           | "super" (DOT NAME | LBRACKET expression RBRACKET)
///                           | "new" DOT "target"
/// optional_chain_expression = member_expression { suffix | OPTIONAL_CHAIN … }
///
/// suffix = arguments                         a call      f(…)
///        | DOT NAME                          a property  o.p
///        | LBRACKET expression RBRACKET      an index    o[k]
///        | template_literal                  a tag       f`…`
/// ```
///
/// After this check every suffix is whole, so [`chain_shape`] can peel the
/// last one off by looking at the final one to three children.
fn chain_base(node: &GrammarASTNode) -> Result<ChainBase<'_>, String> {
    let rule = node.rule_name.as_str();
    let children = &node.children;
    let first_token = match children.first() {
        Some(ASTNodeOrToken::Token(token)) => Some(token),
        _ => None,
    };
    let has_operand = node_children(node).next().is_some();
    let optional = has_token(node, "OPTIONAL_CHAIN");
    let base_end = match rule {
        "member_expression" => {
            let super_base = first_token.is_some_and(|token| is_keyword(token, "super"));
            let new_base = first_token.is_some_and(|token| is_keyword(token, "new"));
            // `new . target` is three tokens and no operand.
            let new_target = new_base && !has_operand;
            if !has_operand && !super_base && !new_target {
                return Err(malformed(node, "member_expression: no children"));
            }
            if optional {
                return Err(unsupported_syntax("OptionalMemberExpression", node));
            }
            if new_target {
                return Ok(ChainBase::Opaque);
            }
            if new_base {
                // `new X(args)`: [new, callee, arguments, suffix…]
                if node_at(node, 1).is_none() || rule_at(node, 2, "arguments").is_none() {
                    return Err(malformed(
                        node,
                        "member_expression: `new` form missing callee or arguments",
                    ));
                }
                3
            } else if super_base || node_at(node, 0).is_some() {
                // `super.x` (an opaque `super` base) or `primary.x`; then
                // the suffixes begin at child 1.
                1
            } else {
                return Err(malformed(node, "member_expression: expected primary base"));
            }
        }
        "call_expression" => {
            if !has_operand {
                return Err(malformed(node, "call_expression: no children"));
            }
            if optional {
                return Err(unsupported_syntax(rule, node));
            }
            if node_at(node, 0).is_none() {
                return Err(malformed(
                    node,
                    "call_expression: expected a base expression",
                ));
            }
            1
        }
        _ => {
            // optional_chain_expression
            if !has_operand {
                return Err(malformed(node, "optional_chain_expression: no children"));
            }
            if let Some(tagged) = node_children(node)
                .skip(1)
                .find(|n| n.rule_name == "template_literal" || n.rule_name == "argument_list")
            {
                return Err(unsupported_syntax(&tagged.rule_name, tagged));
            }
            if optional {
                return Ok(ChainBase::Opaque); // `a?.b`: short-circuits to undefined
            }
            if node_at(node, 0).is_none() {
                return Err(malformed(
                    node,
                    "optional_chain_expression: expected a base",
                ));
            }
            1
        }
    };
    // Every suffix after the base must be whole.
    let mut index = base_end;
    while index < children.len() {
        index += match (&children[index], children.get(index + 1)) {
            (ASTNodeOrToken::Node(n), _) if n.rule_name == "arguments" => 1,
            (ASTNodeOrToken::Node(n), _) if n.rule_name == "template_literal" => {
                if rule == "call_expression" {
                    return Err(unsupported_syntax(rule, node)); // f()`…`
                }
                1
            }
            (ASTNodeOrToken::Token(dot), Some(ASTNodeOrToken::Token(name)))
                if dot.effective_type_name() == "DOT" && is_property_name(name) =>
            {
                2
            }
            (ASTNodeOrToken::Token(open), Some(ASTNodeOrToken::Node(_)))
                if open.effective_type_name() == "LBRACKET"
                    && is_token(children.get(index + 2), "RBRACKET") =>
            {
                3
            }
            _ => return Err(malformed(node, "suffix chain: unexpected child")),
        };
    }
    if base_end == 1 && children.len() == 1 {
        if let Some(only) = node_at(node, 0) {
            return Ok(ChainBase::PassThrough(only));
        }
        return Ok(ChainBase::Opaque); // a lone `super`
    }
    Ok(ChainBase::Suffixes)
}

/// The punctuation that delimits suffixes can never be a property name.
fn is_property_name(token: &Token) -> bool {
    !matches!(
        token.effective_type_name(),
        "DOT" | "LBRACKET" | "RBRACKET" | "OPTIONAL_CHAIN" | "LPAREN" | "RPAREN"
    )
}

/// `children[0..end]` of a checked chain, as its outermost suffix applied to
/// the shorter prefix:
///
/// ```text
/// console . log (1)      end 4: [.., arguments]   → Call(Chain(end 3), [1])
///                        end 3: [.., DOT, NAME]   → Member(Chain(end 1), log)
///                        end 1: [base]            → the base itself
/// o [ k ]                end 4: [.., [, k, ]]     → Member(o, k, computed)
/// ```
fn chain_shape(node: &GrammarASTNode, end: usize) -> Shape<'_> {
    let children = &node.children;
    let prefix = |end: usize| -> Expr<'_> {
        match node_at(node, 0) {
            Some(base) if end == 1 => Expr::Node(base),
            _ => Expr::Chain { node, end },
        }
    };
    // The base alone: a node (which `prefix` hands out as `Expr::Node`),
    // `super`, or `new X(…)` — the last two are opaque to the pilot.
    let suffix_start = match children.first() {
        Some(ASTNodeOrToken::Token(token)) if is_keyword(token, "new") => 3,
        _ => 1,
    };
    if end <= suffix_start {
        return Shape::Other;
    }
    match (
        children.get(end - 1),
        end.checked_sub(2).and_then(|i| children.get(i)),
    ) {
        (Some(ASTNodeOrToken::Node(last)), _) if last.rule_name == "arguments" => Shape::Call {
            callee: prefix(end - 1),
            arguments: arguments_of(last),
        },
        (Some(ASTNodeOrToken::Token(name)), Some(ASTNodeOrToken::Token(dot)))
            if dot.effective_type_name() == "DOT" =>
        {
            Shape::Member {
                object: prefix(end - 2),
                property: Expr::Name(name),
                computed: false,
            }
        }
        // `object [ key ]`: the key node sits between `[` and `]`, so the
        // object ends three children before `end`.  `chain_base` already
        // rejects layouts where that underflows, but the subtraction is
        // checked anyway so a hand-built tree can never wrap it.
        (Some(ASTNodeOrToken::Token(_)), Some(ASTNodeOrToken::Node(key))) => match end.checked_sub(3) {
            Some(start) => Shape::Member {
                object: prefix(start),
                property: Expr::Node(key),
                computed: true,
            },
            None => Shape::Other,
        },
        // A tagged template `` tag`…` ``.
        _ => Shape::Other,
    }
}

/// The argument expressions of an `arguments` node, in source order.
///
/// ```text
/// arguments     = LPAREN [ argument_list [ COMMA ] ] RPAREN      es2015+
/// argument_list = ( spread_element | assignment_expression ) { COMMA … }
/// arguments     = LPAREN [ assignment_expression { COMMA … } ] RPAREN   es5
/// ```
fn arguments_of(arguments: &GrammarASTNode) -> Vec<Expr<'_>> {
    let mut found = Vec::new();
    for child in node_children(arguments) {
        if child.rule_name == "argument_list" {
            found.extend(node_children(child).map(|argument| {
                if has_token(argument, "ELLIPSIS") {
                    Expr::Spread
                } else {
                    Expr::Node(argument)
                }
            }));
        } else if child.rule_name.contains("expression") {
            found.push(Expr::Node(child));
        }
    }
    found
}

/// A `primary_expression` that is a single token.
///
/// Dispatch is on the token's effective type (LANG84 C1.5): a `STRING` that
/// spells `this` is a string, not `this`.
fn primary_token<'a>(token: &'a Token, primary: &GrammarASTNode) -> Result<Shape<'a>, String> {
    let kind = token.effective_type_name();
    if matches!(kind, "NAME" | "KEYWORD")
        && matches!(
            token.value.as_str(),
            "this" | "null" | "undefined" | "true" | "false"
        )
    {
        // Values the pilot does not have yet. `undefined` is an identifier in
        // JavaScript, but it names a global the runner does not model.
        return Ok(Shape::Other);
    }
    match kind {
        "NUMBER" => decode_number(&token.value)
            .map(Shape::Number)
            .map_err(|()| {
                malformed(
                    primary,
                    &format!("failed to parse numeric literal '{}'", token.value),
                )
            }),
        "NAME" => Ok(Shape::Identifier(&token.value)),
        "KEYWORD" => Err(unsupported_syntax(
            &format!("keyword-expression:{}", token.value),
            primary,
        )),
        "BIGINT" | "REGEX" | "STRING" => Ok(Shape::Other),
        // A token no `primary_expression` alternative produces; read as a
        // name, whose spelling check then rejects it.
        _ => Ok(Shape::Identifier(&token.value)),
    }
}

/// The value of a `NUMBER` token, exactly as the typed-AST stage decoded it.
///
/// ```text
/// spelling      rule                                value
/// 1_000         separators dropped (ES2021)         1000
/// 0x1F, 0X1f    hexadecimal, must fit in u64        31
/// 0b101         binary, must fit in u64             5
/// 0o17          octal, must fit in u64              15
/// 017           legacy octal: leading 0, all 0–7    15
/// 019, 0.5, 1e3 decimal (f64 parsing)               19, 0.5, 1000
/// ```
///
/// A radix literal wider than 64 bits is an error, not an approximation;
/// that is the typed stage's rule and is kept so errors stay the same.
fn decode_number(spelling: &str) -> Result<f64, ()> {
    let cleaned: String = spelling.chars().filter(|&c| c != '_').collect();
    for (prefixes, radix) in [(["0x", "0X"], 16), (["0b", "0B"], 2), (["0o", "0O"], 8)] {
        if let Some(digits) = prefixes.iter().find_map(|p| cleaned.strip_prefix(p)) {
            return u64::from_str_radix(digits, radix)
                .map(|n| n as f64)
                .map_err(|_| ());
        }
    }
    if cleaned.starts_with('0')
        && cleaned.len() > 1
        && !cleaned.contains('.')
        && !cleaned.to_ascii_lowercase().contains('e')
        && cleaned[1..].chars().all(|c| c.is_ascii_digit() && c < '8')
    {
        return u64::from_str_radix(&cleaned[1..], 8)
            .map(|n| n as f64)
            .map_err(|_| ());
    }
    cleaned.parse::<f64>().map_err(|_| ())
}

/// VM failure with console output completed by earlier JavaScript statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaScriptRunError {
    pub output: String,
    pub message: String,
}

impl std::fmt::Display for JavaScriptRunError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for JavaScriptRunError {}

/// Execute source on this repository's Rust VM and collect console output.
pub fn run_source(source: &str) -> Result<String, JavaScriptRunError> {
    let mut module =
        compile_source(source, "javascript-script").map_err(|message| JavaScriptRunError {
            output: String::new(),
            message,
        })?;
    let output = Arc::new(Mutex::new(String::new()));
    let captured = Arc::clone(&output);
    let mut vm = VMCore::new();
    vm.max_instructions = Some(100_000);
    vm.builtins_mut().register("js_console_log", move |args| {
        let rendered = match args {
            [] => String::new(),
            [Value::Float(number)] => {
                format_js_number(*number).map_err(vm_core::errors::VMError::Custom)?
            }
            [Value::Float(first), Value::Float(second)] => {
                let first = format_js_number(*first).map_err(vm_core::errors::VMError::Custom)?;
                let second = format_js_number(*second).map_err(vm_core::errors::VMError::Custom)?;
                format!("{first} {second}")
            }
            [Value::Float(first), Value::Float(second), Value::Float(third)] => {
                // Build a complete line before locking or changing the sink.
                // A display failure in any argument leaves earlier calls intact.
                let first = format_js_number(*first).map_err(vm_core::errors::VMError::Custom)?;
                let second = format_js_number(*second).map_err(vm_core::errors::VMError::Custom)?;
                let third = format_js_number(*third).map_err(vm_core::errors::VMError::Custom)?;
                format!("{first} {second} {third}")
            }
            _ => {
                return Err(vm_core::errors::VMError::Custom(
                    "js_console_log expects zero, one, two, or three JavaScript Numbers".into(),
                ))
            }
        };
        let mut sink = captured.lock().map_err(|_| {
            vm_core::errors::VMError::Custom("JavaScript console lock poisoned".into())
        })?;
        let appended_bytes = rendered.len() + 1;
        if sink
            .len()
            .checked_add(appended_bytes)
            .is_none_or(|length| length > 1_000_000)
        {
            return Err(vm_core::errors::VMError::Custom(
                "JavaScript console output limit exceeded".into(),
            ));
        }
        sink.push_str(&rendered);
        sink.push('\n');
        Ok(Value::Null)
    });
    // ReferenceErrors are decided by the compiler (see `compile_reference`);
    // these builtins only raise them, carrying the JavaScript name as a string
    // operand. Messages follow V8's wording. Output from earlier completed
    // `console.log` calls stays in the sink, as for every other VM error.
    vm.builtins_mut().register(NOT_DEFINED_BUILTIN, |args| {
        raise_reference_error(args, |name| {
            format!("ReferenceError: {name} is not defined")
        })
    });
    vm.builtins_mut()
        .register(BEFORE_INITIALIZATION_BUILTIN, |args| {
            raise_reference_error(args, |name| {
                format!("ReferenceError: Cannot access '{name}' before initialization")
            })
        });
    let execution = vm.execute(&mut module, "main", &[]);
    let result = output
        .lock()
        .map_err(|_| JavaScriptRunError {
            output: String::new(),
            message: "JavaScript console lock poisoned".into(),
        })?
        .clone();
    match execution {
        Ok(_) => Ok(result),
        Err(error) => Err(JavaScriptRunError {
            output: result,
            message: error.to_string(),
        }),
    }
}

// Shared body of the two ReferenceError builtins: always an error, never a
// value. A malformed call (not exactly one string name) is still an error,
// just a less informative one.
fn raise_reference_error(
    args: &[Value],
    render: impl Fn(&str) -> String,
) -> Result<Value, vm_core::errors::VMError> {
    match args {
        [Value::Str(name)] => Err(vm_core::errors::VMError::Custom(render(name))),
        _ => Err(vm_core::errors::VMError::Custom(
            "JavaScript ReferenceError builtin expects one binding name".into(),
        )),
    }
}

fn format_js_number(number: f64) -> Result<String, String> {
    if number.is_nan() {
        Ok("NaN".into())
    } else if number == f64::INFINITY {
        Ok("Infinity".into())
    } else if number == f64::NEG_INFINITY {
        Ok("-Infinity".into())
    } else if number == 0.0 && number.is_sign_negative() {
        Ok("-0".into())
    } else if number != 0.0 && !(1e-6..1e21).contains(&number.abs()) {
        Err("JavaScript number display outside the native pilot's supported range".into())
    } else {
        Ok(number.to_string())
    }
}

#[derive(Default)]
struct Compiler {
    instructions: Vec<IIRInstr>,
    next_register: usize,
    /// Every top-level `let`/`const` name in the script (pass 1).
    lexical_names: HashSet<String>,
    /// The subset whose declaration has already been lowered. A name in
    /// `lexical_names` but not here is in its temporal dead zone.
    initialized: HashSet<String>,
}

/// IIR register holding the JavaScript binding `name`; see the module docs
/// for why the `js.` prefix cannot collide with anything else.
fn binding_register(name: &str) -> String {
    format!("js.{name}")
}

impl Compiler {
    fn emit(&mut self, op: &str, dest: Option<String>, srcs: Vec<Operand>, type_hint: &str) {
        self.instructions
            .push(IIRInstr::new(op, dest, srcs, type_hint));
    }

    fn register(&mut self) -> String {
        let result = format!("js{}", self.next_register);
        self.next_register += 1;
        result
    }

    fn compile_statement(&mut self, expr: Expr<'_>) -> Result<(), String> {
        if let Shape::Call { callee, arguments } = classify(expr)? {
            if is_console_log(callee)? {
                if arguments.len() > 3 {
                    return Err("native console.log pilot requires at most three arguments".into());
                }
                let mut operands = vec![Operand::Var("js_console_log".into())];
                for argument in arguments {
                    operands.push(self.compile_number(argument)?);
                }
                self.emit("call_builtin", None, operands, "void");
                return Ok(());
            }
        }
        self.compile_number(expr)?;
        Ok(())
    }

    // `let NAME = INIT;` evaluates INIT, then copies the result into the
    // binding's own register. Only after that is NAME readable.
    fn compile_lexical_binding(&mut self, name: &str, init: Expr<'_>) -> Result<(), String> {
        let value = self.compile_number(init)?;
        self.emit("mov", Some(binding_register(name)), vec![value], "f64");
        self.initialized.insert(name.to_string());
        Ok(())
    }

    // Resolve an identifier read. With no control flow, the binding's state
    // here is the same on every execution, so the decision is static:
    //
    //   state at this point            lowering
    //   ----------------------------   -------------------------------------
    //   declared and initialized       read register `js.NAME`
    //   a runner global (NaN, Math…)   compile error: not implemented yet
    //   declared later in the script   call js_reference_error_before_initialization
    //   declared nowhere               call js_reference_error_not_defined
    //
    // The error builtins never return. Their call still names a destination
    // register so the enclosing expression lowers normally; the VM stops
    // before anything could read it.
    fn compile_reference(&mut self, name: &str) -> Result<Operand, String> {
        check_identifier_spelling(name)?;
        if self.initialized.contains(name) {
            return Ok(Operand::Var(binding_register(name)));
        }
        if RUNNER_GLOBAL_NAMES.contains(&name) {
            return Err(format!(
                "unsupported JavaScript global `{name}` in native VM pilot"
            ));
        }
        let builtin = if self.lexical_names.contains(name) {
            BEFORE_INITIALIZATION_BUILTIN
        } else {
            NOT_DEFINED_BUILTIN
        };
        let dest = self.register();
        self.emit(
            "call_builtin",
            Some(dest.clone()),
            vec![Operand::Var(builtin.into()), Operand::Str(name.into())],
            "f64",
        );
        Ok(Operand::Var(dest))
    }

    // Lower a numeric expression. Recursion here is bounded: the budget pass
    // has already checked that no path is deeper than MAX_EXPRESSION_DEPTH
    // shapes, and wrappers between shapes are crossed by `classify`'s loop.
    fn compile_number(&mut self, expr: Expr<'_>) -> Result<Operand, String> {
        match classify(expr)? {
            Shape::Identifier(name) => self.compile_reference(name),
            Shape::Number(value) => {
                let dest = self.register();
                self.emit(
                    "const",
                    Some(dest.clone()),
                    vec![Operand::Float(value)],
                    "f64",
                );
                Ok(Operand::Var(dest))
            }
            Shape::Unary {
                negate: true,
                argument,
            } => {
                let source = self.compile_number(argument)?;
                let dest = self.register();
                self.emit("neg", Some(dest.clone()), vec![source], "f64");
                Ok(Operand::Var(dest))
            }
            Shape::Binary { op, left, right } => {
                let op = match op {
                    BinaryOp::Add => "add",
                    BinaryOp::Sub => "sub",
                    BinaryOp::Mul => "mul",
                    BinaryOp::Div => "div",
                    BinaryOp::Other => {
                        return Err(
                            "unsupported JavaScript numeric operator in native VM pilot".into()
                        )
                    }
                };
                let left = self.compile_number(left)?;
                let right = self.compile_number(right)?;
                let dest = self.register();
                self.emit(op, Some(dest.clone()), vec![left, right], "f64");
                Ok(Operand::Var(dest))
            }
            _ => Err("unsupported JavaScript expression in native VM pilot".into()),
        }
    }
}

/// Is `callee` exactly the non-computed member `console.log`? Parentheses
/// are transparent, so `(console).log` and `(console.log)` qualify too, as
/// they do in JavaScript; `console["log"]` and `console?.log` do not.
fn is_console_log(callee: Expr<'_>) -> Result<bool, String> {
    let Shape::Member {
        object,
        property,
        computed: false,
    } = classify(callee)?
    else {
        return Ok(false);
    };
    Ok(matches!(classify(object)?, Shape::Identifier("console"))
        && matches!(classify(property)?, Shape::Identifier("log")))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- CST fixtures ----------------------------------------------------
    //
    // These tests hand the compiler parse trees. `cst` is the ordinary
    // (regex-aware) ES2020 parse; a few tests then edit the tree by hand to
    // forge shapes source text cannot reach, reusing real nodes as templates
    // so no token is fabricated.

    fn cst(source: &str) -> GrammarASTNode {
        parse_javascript_typed(source, EsVersion::Es2020).unwrap()
    }

    /// The first node of rule `rule`, depth first.
    fn find<'t>(node: &'t GrammarASTNode, rule: &str) -> &'t GrammarASTNode {
        fn walk<'t>(node: &'t GrammarASTNode, rule: &str) -> Option<&'t GrammarASTNode> {
            if node.rule_name == rule {
                return Some(node);
            }
            node_children(node).find_map(|child| walk(child, rule))
        }
        walk(node, rule).unwrap_or_else(|| panic!("no `{rule}` node"))
    }

    fn find_mut<'t>(node: &'t mut GrammarASTNode, rule: &str) -> &'t mut GrammarASTNode {
        fn walk<'t>(node: &'t mut GrammarASTNode, rule: &str) -> Option<&'t mut GrammarASTNode> {
            if node.rule_name == rule {
                return Some(node);
            }
            node.children.iter_mut().find_map(|child| match child {
                ASTNodeOrToken::Node(inner) => walk(inner, rule),
                ASTNodeOrToken::Token(_) => None,
            })
        }
        walk(node, rule).unwrap_or_else(|| panic!("no `{rule}` node"))
    }

    /// The `index`-th node child of `node` (tokens skipped).
    fn nth_node_mut(node: &mut GrammarASTNode, index: usize) -> &mut GrammarASTNode {
        node.children
            .iter_mut()
            .filter_map(|child| match child {
                ASTNodeOrToken::Node(inner) => Some(inner),
                ASTNodeOrToken::Token(_) => None,
            })
            .nth(index)
            .expect("node child")
    }

    /// `expr` negated `levels` times: `-(-(…expr))`, one unary shape each.
    fn negated(expr: GrammarASTNode, levels: usize) -> GrammarASTNode {
        let template = find(&cst("-1;"), "unary_expression").clone();
        let mut node = expr;
        for _ in 0..levels {
            let mut wrapper = template.clone();
            wrapper.children[1] = ASTNodeOrToken::Node(node);
            node = wrapper;
        }
        node
    }

    /// `expr + expr`, doubled `levels` times: a balanced tree of
    /// `2^levels * (size of expr) + (2^levels - 1)` shapes.
    fn doubled(expr: GrammarASTNode, levels: usize) -> GrammarASTNode {
        let template = find(&cst("1 + 1;"), "additive_expression").clone();
        let mut node = expr;
        for _ in 0..levels {
            let mut sum = template.clone();
            sum.children[0] = ASTNodeOrToken::Node(node.clone());
            sum.children[2] = ASTNodeOrToken::Node(node);
            node = sum;
        }
        node
    }

    #[test]
    fn native_source_repeated_division_reaches_the_vm() {
        for (source, expected) in [
            ("console.log(-0, 1 / 0, 0 / 0)", "-0 Infinity NaN\n"),
            ("console.log(8/2/2, 9/3, 12/4);", "2 3 3\n"),
            ("console.log(8 /* / ignored / */ / 2 / 2);", "2\n"),
            ("console.log(8 /\n2 / 2)\nconsole.log(6 / 3 / 2)", "2\n1\n"),
        ] {
            assert_eq!(run_source(source).unwrap(), expected, "{source}");
        }
        let module = compile_source("console.log(8 / 2 / 2, 9 / 3, 12 / 4)", "div-order").unwrap();
        assert_eq!(
            module.functions[0]
                .instructions
                .iter()
                .filter(|i| i.op == "div")
                .count(),
            4
        );
    }

    #[test]
    fn native_division_goal_rejects_regex_and_malformed_operands() {
        for source in [
            "console.log(/1/);",
            "console.log(/1/ / 2);",
            "console.log(1, /2/g, 3);",
            "/1/;",
            "console.log(8 /= 2);",
            "console.log(8 /);",
            "console.log(8 / / 2);",
            "console.log(1); console.log(8 / 2, /3/, 4);",
        ] {
            let error = run_source(source).unwrap_err();
            assert!(error.output.is_empty(), "{source}");
        }
        for source in [
            "console.log(7); console.log(1e21 / 1, 8 / 2, 9 / 3);",
            "console.log(7); console.log(8 / 2, 1e21 / 1, 9 / 3);",
            "console.log(7); console.log(8 / 2, 9 / 3, 1e21 / 1);",
        ] {
            let error = run_source(source).unwrap_err();
            assert_eq!(error.output, "7\n", "{source}");
            assert!(error.message.contains("number display outside"));
        }
    }

    #[test]
    fn native_division_spine_guard_preserves_semantic_depth_boundary() {
        let chain = |length| vec!["1"; length].join("/");
        assert_eq!(
            run_source(&format!("console.log({});", chain(63))).unwrap(),
            "1\n"
        );
        let error = run_source(&format!("console.log({});", chain(64))).unwrap_err();
        assert!(error.output.is_empty());
        assert!(error.message.contains("depth limit"));
        assert_eq!(run_source(&format!("{};", chain(64))).unwrap(), "");
        for length in [5_000, 20_000] {
            let error = run_source(&format!("console.log({});", chain(length))).unwrap_err();
            assert!(error.output.is_empty());
            assert!(error.message.contains("spine limit"));
        }
        for length in [200, 5_000] {
            let source = format!("console.log({});", vec!["1"; length].join("**"));
            let error = run_source(&source).unwrap_err();
            assert!(error.output.is_empty());
            assert!(error.message.contains("exponentiation is unsupported"));
        }
    }

    #[test]
    fn executes_javascript_numbers_on_our_vm() {
        assert_eq!(
            run_source("console.log(1 + 2); console.log(1 / 2);").unwrap(),
            "3\n0.5\n"
        );
        assert_eq!(run_source("console.log(-(1 + 2) * 3);").unwrap(), "-9\n");
    }

    #[test]
    fn empty_console_log_lowers_directly_and_preserves_effect_order() {
        let ast = cst("console.log();");
        let module = compile_ast(&ast, "empty-log").unwrap();
        let calls: Vec<_> = module.functions[0]
            .instructions
            .iter()
            .filter(|instruction| instruction.op == "call_builtin")
            .collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].srcs.len(),
            1,
            "only the builtin name is an operand"
        );
        assert_eq!(
            run_source("console.log(); console.log(7);").unwrap(),
            "\n7\n"
        );
    }

    #[test]
    fn two_numeric_arguments_lower_in_order_and_print_with_one_space() {
        let ast = cst("console.log(1 + 2, 3 / 2);");
        let module = compile_ast(&ast, "two-log").unwrap();
        let call = module.functions[0]
            .instructions
            .iter()
            .find(|instruction| instruction.op == "call_builtin")
            .unwrap();
        assert_eq!(call.srcs.len(), 3);
        assert_eq!(
            run_source("console.log(); console.log(1 + 2, 3 / 2); console.log(4);").unwrap(),
            "\n3 1.5\n4\n"
        );
    }

    #[test]
    fn three_numeric_arguments_lower_in_source_order_and_execute_on_our_vm() {
        let ast = cst("console.log(11, 22, 33);");
        let module = compile_ast(&ast, "three-log").unwrap();
        let instructions = &module.functions[0].instructions;
        let call = instructions
            .iter()
            .find(|instruction| instruction.op == "call_builtin")
            .unwrap();
        assert_eq!(call.srcs.len(), 4);
        assert_eq!(call.srcs[0], Operand::Var("js_console_log".into()));
        for (index, number) in [11.0, 22.0, 33.0].into_iter().enumerate() {
            // Each source argument's constant must precede the call and feed
            // the corresponding operand, not merely produce the right arity.
            assert_eq!(instructions[index].op, "const");
            assert_eq!(instructions[index].srcs, vec![Operand::Float(number)]);
            assert_eq!(
                call.srcs[index + 1],
                Operand::Var(instructions[index].dest.clone().unwrap())
            );
        }
        assert_eq!(
            run_source("console.log(); console.log(7); console.log(1, 2); console.log(1 + 2, 3 / 2, -(4 * 2));").unwrap(),
            "\n7\n1 2\n3 1.5 -8\n"
        );
        assert_eq!(
            run_source("console.log(-0, 1 / 0, 0);").unwrap(),
            "-0 Infinity 0\n"
        );
        assert_eq!(
            run_source("console.log(0 / 0, -0, 0);").unwrap(),
            "NaN -0 0\n"
        );
        assert_eq!(
            run_source("console.log(0.000001, 100000000000000000000, -1 / 0);").unwrap(),
            "0.000001 100000000000000000000 -Infinity\n"
        );
    }

    #[test]
    fn three_argument_display_failure_appends_none_of_the_failed_call() {
        for source in [
            "console.log(); console.log(7); console.log(1e21, 2, 3);",
            "console.log(); console.log(7); console.log(1, 1e21, 3);",
            "console.log(); console.log(7); console.log(1, 2, 1e21);",
            "console.log(); console.log(7); console.log(1, 2, 0.0000001);",
        ] {
            let error = run_source(source).unwrap_err();
            assert_eq!(error.output, "\n7\n", "{source}");
            assert!(error.message.contains("number display outside"), "{error}");
        }
    }

    #[test]
    fn unsupported_third_arguments_and_four_arguments_emit_no_output() {
        for source in [
            "console.log(7); console.log(1, 2, 'three');",
            "console.log(7); console.log(1, 2, console.log(3));",
            "console.log(7); console.log(1, 2, 3 % 2);",
            "console.log(7); console.log(1, 2, 3, 4);",
        ] {
            let error = run_source(source).unwrap_err();
            assert!(error.output.is_empty(), "{source}");
            assert!(
                error.message.contains("unsupported") || error.message.contains("at most three"),
                "{error}"
            );
        }
    }

    #[test]
    fn direct_ast_node_budget_checks_the_third_console_argument() {
        let mut ast = cst("console.log(1, 2, 3);");
        let arguments = find_mut(&mut ast, "argument_list");
        // A shallow balanced tree reaches 16,383 nodes. Adding the other
        // arguments, callee, and statement exceeds the whole-AST budget.
        let third = nth_node_mut(arguments, 2);
        *third = doubled(third.clone(), 13);
        let error = compile_ast(&ast, "forged-wide-third-argument").unwrap_err();
        assert!(error.contains("node limit"), "{error}");
    }

    #[test]
    fn two_argument_display_failure_keeps_only_prior_completed_output() {
        let error = run_source("console.log(7); console.log(1e21, 2);").unwrap_err();
        assert_eq!(error.output, "7\n");
        assert!(error.message.contains("number display outside"));
        let error = run_source("console.log(7); console.log(1, 1e21);").unwrap_err();
        assert_eq!(error.output, "7\n");
        assert!(error.message.contains("number display outside"));
        let error = run_source("console.log(7); console.log(1, 2, 3, 4);").unwrap_err();
        assert!(error.output.is_empty());
        assert!(error.message.contains("requires"));
    }

    #[test]
    fn direct_ast_budget_checks_every_console_argument() {
        for argument_index in 0..3 {
            let mut ast = cst("console.log(1, 2, 3);");
            let argument = nth_node_mut(find_mut(&mut ast, "argument_list"), argument_index);
            *argument = negated(argument.clone(), MAX_EXPRESSION_DEPTH);
            let error = compile_ast(&ast, "forged-deep-argument").unwrap_err();
            assert!(error.contains("depth limit"), "{error}");
        }
    }

    #[test]
    fn empty_console_output_survives_later_vm_error_but_compile_errors_emit_nothing() {
        let error = run_source("console.log(); console.log(1e21);").unwrap_err();
        assert_eq!(error.output, "\n");
        assert!(error.message.contains("number display outside"));
        let error = run_source("console.log(); console.log(1, 2, 3, 4);").unwrap_err();
        assert!(error.output.is_empty());
        assert!(error.message.contains("requires"));
    }

    #[test]
    fn retains_completed_console_output_before_vm_error() {
        let error = run_source("console.log(7); console.log(1e21);").unwrap_err();
        assert_eq!(error.output, "7\n");
        assert!(error.message.contains("number display outside"));

        // `let x = 1;` joined the subset in 0.2.0; a statement form that is
        // still outside it keeps this compile-error probe meaningful.
        let error = run_source("if (1) 1;").unwrap_err();
        assert!(error.output.is_empty());
        assert!(error.message.contains("unsupported JavaScript statement"));
    }

    #[test]
    fn unsupported_semantics_are_rejected() {
        assert!(compile_source("console.log('a' + 'b');", "bad").is_err());
        assert!(compile_source("var x = 1;", "bad").is_err());
        assert!(run_source("console.log(1e21);").is_err());
        assert!(compile_source(&" ".repeat(MAX_SOURCE_BYTES + 1), "too-large").is_err());
        let ast = cst(&"1;".repeat(MAX_AST_NODES + 1));
        assert!(compile_ast(&ast, "too-many-nodes").is_err());
    }

    // ---- top-level numeric let/const (0.2.0) -------------------------------

    fn assert_runs(source: &str, expected: &str) {
        assert_eq!(run_source(source).unwrap(), expected, "{source}");
    }

    fn assert_compile_error(source: &str, needle: &str) {
        // Prefixing completed output proves a compile error runs nothing.
        let source = format!("console.log(7); {source}");
        let error = run_source(&source).unwrap_err();
        assert!(error.output.is_empty(), "{source}");
        assert!(error.message.contains(needle), "{source}: {error}");
    }

    #[test]
    fn let_and_const_bindings_execute_on_our_vm() {
        assert_runs("let x = 1 / 2; console.log(x, x * 4)", "0.5 2\n");
        assert_runs("const y = -0; console.log(y)", "-0\n");
        assert_runs(
            "let a = 2; const b = a * 3; console.log(a, b, -b / a); console.log(b - a);",
            "2 6 -3\n4\n",
        );
        assert_runs(
            "let n = 0 / 0; const i = 1 / n; console.log(n, i);",
            "NaN NaN\n",
        );
        assert_runs(
            "const z = 1 / -0; let w = -z; console.log(z, w);",
            "-Infinity Infinity\n",
        );
        // A bare reference is an expression statement with no output.
        assert_runs("let x = 1; x; -x + x;", "");
        // Declarations need no trailing semicolon thanks to ASI.
        assert_runs("let x = 3\nconsole.log(x)", "3\n");
    }

    #[test]
    fn user_names_cannot_meet_temporaries_or_builtins() {
        // `js0` is also the compiler's first temporary and `js_console_log`
        // the console builtin; user bindings live under `js.` instead.
        assert_runs(
            "let js0 = 5; const js_console_log = 6; let $ = 1; let _ = 2; \
             console.log(js0, js_console_log, js0 + $ + _);",
            "5 6 8\n",
        );
        let module = compile_source("let js0 = 5; console.log(js0);", "namespace").unwrap();
        let instructions = &module.functions[0].instructions;
        let mov = instructions.iter().find(|i| i.op == "mov").unwrap();
        assert_eq!(mov.dest.as_deref(), Some("js.js0"));
        assert_eq!(mov.srcs, vec![Operand::Var("js0".into())]);
        let call = instructions
            .iter()
            .find(|i| i.op == "call_builtin")
            .unwrap();
        assert_eq!(
            call.srcs,
            vec![
                Operand::Var("js_console_log".into()),
                Operand::Var("js.js0".into())
            ]
        );
    }

    #[test]
    fn undeclared_reads_raise_reference_error_after_prior_output() {
        let error = run_source("console.log(1); console.log(z)").unwrap_err();
        assert_eq!(error.output, "1\n");
        assert_eq!(error.message, "ReferenceError: z is not defined");
        // Arguments evaluate before the call, so the failing call adds nothing.
        let error = run_source("console.log(1); let a = 2; console.log(a, a + q, 3);").unwrap_err();
        assert_eq!(error.output, "1\n");
        assert_eq!(error.message, "ReferenceError: q is not defined");
        // A bare read throws too, and so does one inside an initializer.
        let error = run_source("console.log(); z; console.log(2);").unwrap_err();
        assert_eq!(error.output, "\n");
        assert_eq!(error.message, "ReferenceError: z is not defined");
        let error = run_source("console.log(4); let b = 1 + y;").unwrap_err();
        assert_eq!(error.output, "4\n");
        assert_eq!(error.message, "ReferenceError: y is not defined");
        // Node-only host globals are not part of this runner's host.
        let error = run_source("console.log(process);").unwrap_err();
        assert_eq!(error.message, "ReferenceError: process is not defined");
    }

    #[test]
    fn temporal_dead_zone_reads_raise_reference_error_after_prior_output() {
        let error = run_source("console.log(1); console.log(a); let a = 2;").unwrap_err();
        assert_eq!(error.output, "1\n");
        assert_eq!(
            error.message,
            "ReferenceError: Cannot access 'a' before initialization"
        );
        // A binding is not readable inside its own initializer.
        let error = run_source("console.log(1); const a = a + 1;").unwrap_err();
        assert_eq!(error.output, "1\n");
        assert_eq!(
            error.message,
            "ReferenceError: Cannot access 'a' before initialization"
        );
        // TDZ is per binding: an initialized neighbour still reads fine.
        let error =
            run_source("let a = 1; console.log(a); console.log(a, b); let b = 2;").unwrap_err();
        assert_eq!(error.output, "1\n");
        assert!(error.message.contains("Cannot access 'b'"), "{error}");
        // The IIR raises at the read site and never reaches the late `mov`.
        let module = compile_source("console.log(a); let a = 2;", "tdz").unwrap();
        let instructions = &module.functions[0].instructions;
        assert_eq!(instructions[0].op, "call_builtin");
        assert_eq!(
            instructions[0].srcs,
            vec![
                Operand::Var(BEFORE_INITIALIZATION_BUILTIN.into()),
                Operand::Str("a".into())
            ]
        );
    }

    #[test]
    fn duplicate_lexical_declarations_are_early_errors_with_no_output() {
        for source in [
            "let a = 1; let a = 2;",
            "console.log(1); let a = 1; let a = 2;",
            "const a = 1; let a = 2;",
            "let a = 1; console.log(a); const a = 2;",
            // Early errors win over runtime errors that would come first.
            "console.log(1); console.log(z); let a = 1; let a = 2;",
        ] {
            let error = run_source(source).unwrap_err();
            assert!(error.output.is_empty(), "{source}");
            assert_eq!(
                error.message, "SyntaxError: Identifier 'a' has already been declared",
                "{source}"
            );
            assert!(compile_source(source, "duplicate").is_err());
        }
    }

    #[test]
    fn unsupported_declaration_forms_are_rejected_at_compile_time() {
        assert_compile_error("var x = 1;", "`var` declaration");
        assert_compile_error("let x;", "requires an initializer");
        assert_compile_error(
            "const k;",
            "SyntaxError: Missing initializer in const declaration",
        );
        assert_compile_error("let a = 1, b = 2;", "exactly one declarator");
        assert_compile_error("let console = 1;", "global name `console`");
        assert_compile_error("const NaN = 1;", "global name `NaN`");
        assert_compile_error("let Math = 1;", "global name `Math`");
        assert_compile_error("let s = 'a';", "unsupported JavaScript expression");
        assert_compile_error("const t = true;", "unsupported JavaScript expression");
        assert_compile_error("let f = console.log;", "unsupported JavaScript expression");
        assert_compile_error("let m = 5 % 2;", "unsupported JavaScript numeric operator");
        // Assignment is not in the subset, so `const` reassignment cannot arise.
        assert_compile_error("x = 2;", "unsupported JavaScript expression");
        assert_compile_error("let x = 1; x = 2;", "unsupported JavaScript expression");
        assert_compile_error("const c = 1; c = 2;", "unsupported JavaScript expression");
        assert_compile_error(
            "let x = 1; console.log(x = 2);",
            "unsupported JavaScript expression",
        );
        // Known globals are not implemented as values yet; they must not read
        // as undeclared names.
        assert_compile_error("console.log(NaN);", "unsupported JavaScript global `NaN`");
        assert_compile_error("console.log(Infinity);", "global `Infinity`");
        assert_compile_error("console;", "global `console`");
        assert_compile_error("let x = globalThis;", "global `globalThis`");
        // Shapes the parser or AST bridge declines still emit nothing.
        for source in [
            "console.log(7); const [a] = [1];",
            "console.log(7); let {a} = {a: 1};",
            "console.log(7); let undefined = 1;",
            "console.log(7); console.log(undefined);",
            "console.log(7); let \\u0061 = 1;",
        ] {
            let error = run_source(source).unwrap_err();
            assert!(error.output.is_empty(), "{source}: {error}");
        }
    }

    /// The `lexical_binding` of the program's first declaration.
    fn single_declarator(ast: &mut GrammarASTNode) -> &mut GrammarASTNode {
        find_mut(ast, "lexical_binding")
    }

    /// The binding-name token of a `lexical_binding`.
    fn binding_token(declarator: &mut GrammarASTNode) -> &mut Token {
        match &mut declarator.children[0] {
            ASTNodeOrToken::Token(token) => token,
            ASTNodeOrToken::Node(_) => panic!("expected the binding name token"),
        }
    }

    #[test]
    fn cst_declarations_lower_directly_in_both_item_positions() {
        let ast = cst("let x = 2; console.log(x, -x);");
        let module = compile_ast(&ast, "cst-let").unwrap();
        let ops: Vec<_> = module.functions[0]
            .instructions
            .iter()
            .map(|i| i.op.as_str())
            .collect();
        assert_eq!(ops, ["const", "mov", "neg", "call_builtin", "ret_void"]);

        // A caller may also place the declaration directly under
        // `source_element`, without the `statement` wrapper.
        let mut moved = ast.clone();
        let element = nth_node_mut(&mut moved, 0);
        let declaration = find(element, "lexical_declaration").clone();
        element.children = vec![ASTNodeOrToken::Node(declaration)];
        let shape = |module: &IIRModule| -> Vec<_> {
            module.functions[0]
                .instructions
                .iter()
                .map(|i| (i.op.clone(), i.dest.clone(), i.srcs.clone()))
                .collect()
        };
        assert_eq!(
            shape(&compile_ast(&moved, "program-level-let").unwrap()),
            shape(&module)
        );

        // Caller-built names must be plain IdentifierName spellings.
        for forged in ["", "a b", "a.b", "1a", "a\n", "\\u0061"] {
            let mut bad = ast.clone();
            binding_token(single_declarator(&mut bad)).value = forged.into();
            let error = compile_ast(&bad, "forged-binding").unwrap_err();
            assert!(error.contains("identifier spelling"), "{forged:?}: {error}");
        }
        let mut bad = cst("let x = y; console.log(x, -x);");
        let reference = find_mut(single_declarator(&mut bad), "primary_expression");
        let ASTNodeOrToken::Token(name) = &mut reference.children[0] else {
            panic!("expected the NAME token of `y`");
        };
        name.value = "a.b".into();
        let error = compile_ast(&bad, "forged-reference").unwrap_err();
        assert!(error.contains("identifier spelling"), "{error}");
        let mut bad = ast.clone();
        single_declarator(&mut bad).children.truncate(1); // drop `= 2`
        assert!(compile_ast(&bad, "forged-no-init")
            .unwrap_err()
            .contains("requires an initializer"));
    }

    #[test]
    fn direct_ast_budget_checks_declarator_initializers() {
        let base = cst("let x = 1;");

        let mut deep = base.clone();
        let init = nth_node_mut(single_declarator(&mut deep), 0);
        *init = negated(init.clone(), MAX_EXPRESSION_DEPTH);
        let error = compile_ast(&deep, "forged-deep-initializer").unwrap_err();
        assert!(error.contains("depth limit"), "{error}");

        // 2^14 - 1 = 16,383 initializer nodes plus the item, declarator and
        // binding charges exceed the 16,384-node budget.
        let mut wide = base.clone();
        let init = nth_node_mut(single_declarator(&mut wide), 0);
        *init = doubled(init.clone(), 13);
        let error = compile_ast(&wide, "forged-wide-initializer").unwrap_err();
        assert!(error.contains("node limit"), "{error}");

        // Declarations alone pay three nodes each (item, declarator, binding
        // name) plus their one-node initializer: 4,096 fit, one more does not.
        let fits: String = (0..MAX_AST_NODES / 4)
            .map(|i| format!("let v{i} = 1;"))
            .collect();
        let ast = cst(&fits);
        assert!(compile_ast(&ast, "declaration-budget").is_ok());
        let over = format!("{fits}let last = 1;");
        let ast = cst(&over);
        assert!(compile_ast(&ast, "declaration-budget")
            .unwrap_err()
            .contains("node limit"));
    }

    #[test]
    fn reference_error_builtins_reject_malformed_calls() {
        let render = |name: &str| format!("ReferenceError: {name} is not defined");
        for args in [
            vec![],
            vec![Value::Float(1.0)],
            vec![Value::Str("a".into()); 2],
        ] {
            let error = raise_reference_error(&args, render).unwrap_err();
            assert!(error.to_string().contains("expects one binding name"));
        }
        let error = raise_reference_error(&[Value::Str("a".into())], render).unwrap_err();
        assert_eq!(error.to_string(), "ReferenceError: a is not defined");
    }
}
