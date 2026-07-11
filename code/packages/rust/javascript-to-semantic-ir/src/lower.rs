//! JavaScript `GrammarASTNode` (CST) → `semantic_ir::Module` lowering.
//!
//! # What this file does (milestones M1 + M2 + M3 + M4)
//!
//! The [`javascript-parser`](coding_adventures_javascript_parser) crate
//! hands us a *concrete syntax tree* (CST): a [`GrammarASTNode`] whose
//! shape mirrors the ECMAScript grammar one-for-one.  Even a bare
//! literal like `42;` produces a deep spine of single-child wrapper
//! nodes — `program → source_element → statement →
//! expression_statement → expression → assignment_expression → … →
//! primary_expression → <token>`.  Twenty-odd rule layers, all of which
//! exist only to encode operator precedence, and *none* of which carry
//! information until one of them *branches* (an operator with two
//! operands, an assignment with a target and value, …).
//!
//! ## Milestone history
//!
//! - **M1** lowered **literals only**: walk a statement down to its
//!   single leaf token, classify the token, emit the matching SIR
//!   literal.  Anything non-literal was rejected.
//! - **M2** (this build) adds **variables and operators**.  The
//!   single-child *peel* in [`Lowerer::lower_expression`] is still the
//!   entry point: when it stops at a *branching* node it now dispatches
//!   on the rule name (`additive_expression`, `relational_expression`,
//!   `logical_and_expression`, `unary_expression`, …) instead of
//!   rejecting.  And `program` now threads a per-scope set of declared
//!   names so a bare identifier resolves to a [`VarRef`] and a
//!   `let`/`const`/`var` becomes a binding statement.
//!
//! ## The literal truth table (learned by probing the real parser)
//!
//! | JS source     | leaf token                                  | SIR node            |
//! |---------------|---------------------------------------------|---------------------|
//! | `42`          | `Number` value `"42"`  (no `.`/`e`)         | `IntLit { 42 }`     |
//! | `3.25`        | `Number` value `"3.25"` (has `.`)           | `FloatLit { 3.25 }` |
//! | `1e3`         | `Number` value `"1e3"` (has `e`)            | `FloatLit { 1000 }` |
//! | `true`        | `Keyword` value `"true"`                    | `BoolLit { true }`  |
//! | `false`       | `Keyword` value `"false"`                   | `BoolLit { false }` |
//! | `null`        | `Keyword` value `"null"`                    | `NilLit`            |
//! | `undefined`   | `Name`   value `"undefined"` (an ident!)    | `NilLit`            |
//! | `"hi"`/`'hi'` | `String` value `hi` (already unescaped)     | `StrLit { "hi" }`   |
//!
//! ## The operator truth table (M2 — learned by probing the parser)
//!
//! Each row's *rule node* is where the precedence spine **branches**:
//! the children are `[lhs, op_token, rhs, op_token, rhs, …]` (binary
//! rules are flat, left-associative chains), or `[op_token, operand]`
//! for a prefix `unary_expression`.  The middle/first token's *value*
//! (not its `TokenType`, which varies) carries the operator spelling.
//!
//! | JS source     | branching rule              | op value | SIR node                       |
//! |---------------|-----------------------------|----------|--------------------------------|
//! | `a + b`       | `additive_expression`       | `+`      | `BuiltinCall("+", [a, b])`     |
//! | `a - b`       | `additive_expression`       | `-`      | `BuiltinCall("-", [a, b])`     |
//! | `a * b`       | `multiplicative_expression` | `*`      | `BuiltinCall("*", [a, b])`     |
//! | `a / b`       | `multiplicative_expression` | `/`      | `BuiltinCall("/", [a, b])`     |
//! | `a % b`       | `multiplicative_expression` | `%`      | `BuiltinCall("%", [a, b])`     |
//! | `a < b`       | `relational_expression`     | `<`      | `BuiltinCall("<", [a, b])`     |
//! | `a > b`       | `relational_expression`     | `>`      | `BuiltinCall(">", [a, b])`     |
//! | `a <= b`      | `relational_expression`     | `<=`     | `BuiltinCall("<=", [a, b])`    |
//! | `a >= b`      | `relational_expression`     | `>=`     | `BuiltinCall(">=", [a, b])`    |
//! | `a == b`      | `equality_expression`       | `==`     | `BuiltinCall("=", [a, b])`     |
//! | `a === b`     | `equality_expression`       | `===`    | `BuiltinCall("=", [a, b])`     |
//! | `a != b`      | `equality_expression`       | `!=`     | `BuiltinCall("!=", [a, b])`    |
//! | `a !== b`     | `equality_expression`       | `!==`    | `BuiltinCall("!=", [a, b])`    |
//! | `a && b`      | `logical_and_expression`    | `&&`     | `LogicalAnd { a, b }`          |
//! | `a \|\| b`    | `logical_or_expression`     | `\|\|`   | `LogicalOr { a, b }`           |
//! | `!a`          | `unary_expression`          | `!`      | `BuiltinCall("not", [a])`      |
//! | `-a`          | `unary_expression`          | `-`      | `BuiltinCall("neg", [a])`      |
//!
//! ### Equality normalisation (a deliberate semantic change)
//!
//! JS has *two* equality families: loose (`==`/`!=`, with coercion) and
//! strict (`===`/`!==`, no coercion).  The IR has a single `=` /`!=`.
//! We map **both** JS families to the strict-shaped IR comparison
//! (`BuiltinCall("=")` / `BuiltinCall("!=")`).  This *changes semantics*
//! for the coercion cases — `null == undefined` is `true` in JS but
//! `false` under strict comparison.  This loss is spec-sanctioned for v0
//! (see SIR19 "Equality normalisation"); programs relying on loose
//! coercion are out of scope.
//!
//! ### Unary `-` on a numeric literal (constant fold)
//!
//! `-5` parses as a prefix `unary_expression` whose operand is the
//! literal `5`.  Rather than emit `BuiltinCall("neg", [IntLit(5)])` we
//! *constant-fold* it to `IntLit(-5)` (and `-3.25` to `FloatLit(-3.25)`).
//! This keeps the spec's `-7 → IntLit` row exact.  Unary `-` on any
//! *non-literal* operand (e.g. `-x`) stays `BuiltinCall("neg", [x])`.
//!
//! ## Variable model (M2)
//!
//! Everything lives inside the synthetic top-level `main`, so every
//! top-level binding is a *local* of `main` (`Scope::Local`).  We track a
//! `declared_locals` set as we lower statements in source order:
//!
//! - First sighting of a name as `let`/`const`/`var x = …` (or a bare
//!   `x = …` that has no prior binding) emits a binding statement and
//!   records the name.
//! - A subsequent `x = …` to an already-declared name emits
//!   `Stmt::Assign` (`Feature::MutableBindings`).
//! - A bare identifier reference resolves to `VarRef { scope: Local }`
//!   if it is declared, to `NilLit` for the exact spelling `undefined`,
//!   and otherwise to a positioned "unresolved name" [`JsLowerError`].
//!
//! Bindings lower to [`Stmt::LetStarBinding`] (sequential `let*`), **not**
//! [`Stmt::LetBinding`].  The SIR validator treats a run of consecutive
//! `LetBinding`s as a *parallel* group whose right-hand sides may not see
//! one another; JS `let`/`const` are sequentially scoped, so a perfectly
//! ordinary `let x = 1; const y = x + 1;` must validate.  `let*`'s
//! sequential semantics match JS exactly.  (The SIR19 spec coverage table
//! writes "LetBinding" generically for both kinds; this divergence is
//! noted there.)  `const` vs `let` vs `var` are not distinguished in v0
//! (the IR models no immutability constraint).
//!
//! ## Control flow (M3 — learned by probing the parser)
//!
//! M3 adds the four counting/branching control-flow shapes.  The CST
//! rule names and child layouts (precedence-wrapper layers elided):
//!
//! | JS source                                   | CST rule           | children                                                                     |
//! |---------------------------------------------|--------------------|------------------------------------------------------------------------------|
//! | `if (c) S`                                  | `if_statement`     | `[Kw("if"), (, expression, ), statement]`                                    |
//! | `if (c) S else T`                           | `if_statement`     | `[…, statement, Kw("else"), statement]`                                      |
//! | `while (c) S`                               | `while_statement`  | `[Kw("while"), (, expression, ), statement]`                                 |
//! | `for (let i=0; c; u) S`                     | `for_statement`    | `[Kw("for"), (, Kw("let"), binding_list, ;, expr(cond), ;, expr(update), ), statement]` |
//! | `for (const x of xs) S`                     | `for_of_statement` | `[Kw("for"), (, Kw("const"), binding_element, Name("of"), assignment_expression, ), statement]` |
//! | `{ S1; S2; }`                               | `block`            | `[{, statement*, }]`                                                          |
//!
//! ### `if` → [`Expr::If`]
//!
//! The IR's conditional is an *expression* ([`Expr::If`]) with `then_branch`
//! and `else_branch` [`Block`]s — there is no statement-level `if`.  So a JS
//! `if` *statement* lowers to a `Stmt::ExprStmt` wrapping an `Expr::If`.  A
//! missing `else` becomes a synthetic nil-valued empty `Block`.  An
//! **else-if chain** (`else if (…)`) is just the grammar nesting another
//! `if_statement` inside the `else` `statement`, so it recurses naturally
//! into a *nested* `Expr::If` living in the outer `else_branch`'s tail value.
//!
//! ### `while` → [`Stmt::While`]
//!
//! Direct: lower the condition expression and the body block.
//!
//! ### C-style `for` → [`Stmt::ForRange`] (canonical counting loops only)
//!
//! The IR has no general three-clause `for`; it has a half-open counting
//! [`Stmt::ForRange`] (`for var in range(start, stop, step)`).  We accept a
//! C-style `for` **only** when it matches the canonical counting shape and
//! extract `var`/`start`/`stop`/`step`:
//!
//! - **init** must be `let i = <start>` (a single `lexical_binding`/`var`
//!   declaration binding `i` to the start expression).
//! - **cond** must be `i < <stop>` or `i <= <stop>` on the *same* `i`.
//!   `<=` is rewritten to a half-open `<` by bumping the stop to
//!   `<stop> + 1` (`BuiltinCall("+", [stop, IntLit(1)])`).
//! - **update** must increment `i` by a constant `step` in one of:
//!   `i = i + <step>`, `i += <step>`, or `i++` (step = 1).
//!
//! Anything else — a different loop variable across clauses, a decrementing
//! or multiplicative update, a missing clause, a multi-binding init — is a
//! *non-canonical* loop we cannot faithfully represent as a `ForRange`, so
//! it is a positioned [`JsLowerError`] (deferred), never silently mangled.
//!
//! ### `for … of` → [`Stmt::ForEach`]
//!
//! `for (const x of xs)` binds `x` over the iterable `xs`.  Only the
//! single-identifier binding form is supported (destructuring is deferred).
//!
//! ### Block scoping
//!
//! A `{ … }` block and every control-flow body lower to a [`Block`].  Names
//! bound *inside* a body are block-scoped: we snapshot `declared_locals`
//! before lowering a body and restore it afterwards, so an inner `let` does
//! not leak to the enclosing scope.  This mirrors the SIR validator, which
//! marks/rewinds its `LocalEnv` around each `Block` and around a loop's
//! body (with the loop variable added only for that body).  The loop
//! variable is likewise bound into the body scope only.
//!
//! ### Recursion bound
//!
//! Statement-block nesting is bounded by [`MAX_STMT_DEPTH`] exactly as
//! operator recursion is bounded by [`MAX_EXPR_DEPTH`]: each nested body is
//! lowered with `depth + 1`, and an over-deep nest becomes an ordinary
//! positioned error rather than a stack overflow.
//!
//! ## Functions, calls, closures (M4 — learned by probing the parser)
//!
//! M4 adds **functions** (declarations + arrows), **calls**, and
//! **closures**.  The CST rule names and child layouts (precedence-wrapper
//! layers elided):
//!
//! | JS source                       | CST rule               | children                                                                          |
//! |---------------------------------|------------------------|-----------------------------------------------------------------------------------|
//! | `function f(a, b) { … }`        | `function_declaration` | `[Kw("function"), Name(f), (, formal_parameters, ), {, function_body, }]`          |
//! | `function g() { … }`            | `function_declaration` | `[Kw("function"), Name(g), (, ), {, function_body, }]` (no `formal_parameters`)    |
//! | `a` / `a, b`                    | `formal_parameters`    | `[formal_parameter, (, formal_parameter)*]`, each `formal_parameter[ Name ]`       |
//! | `{ … }` (fn body)               | `function_body`        | `[source_element*]` (possibly empty)                                              |
//! | `return e;`                     | `return_statement`     | `[Kw("return"), expression, ;]` (or `[Kw("return"), ;]` for bare `return;`)        |
//! | `(a) => e` / `a => e`           | `arrow_function`       | `[arrow_parameters, Name("=>"), concise_body]`                                     |
//! | `(a)` / `()` / `a`              | `arrow_parameters`     | `[(, formal_parameters?, )]` **or** a bare `[Name]` for `a => …`                   |
//! | `=> e`                          | `concise_body`         | `[assignment_expression]` (expression body)                                       |
//! | `=> { … }`                      | `concise_body`         | `[{, function_body, }]` (block body)                                               |
//! | `f(1, 2)`                       | `call_expression`      | `[callee(member_expression), arguments]`                                           |
//! | `console.log(x)`                | `call_expression`      | `[member_expression[ console, ., log ], arguments]`                                |
//! | `(1, 2)`                        | `arguments`            | `[(, argument_list?, )]`, `argument_list[ assignment_expression (, …)* ]`          |
//!
//! ### Two-pass function collection
//!
//! Before lowering any body we walk the program collecting **every**
//! `function_declaration` name (top-level and nested) into
//! `function_names`.  This lets a call resolve to a `DirectCall` even when
//! the callee is defined *after* the call site (forward reference) and lets
//! mutual recursion (`isEven`/`isOdd`) work.  Nested function names are
//! global too: a nested `function inner` is lifted to a top-level
//! synthesised `Function`, so its name must be visible module-wide.
//!
//! ### `return` (tail-position only)
//!
//! The IR has no early-return node; a `Function`/closure body is a `Block`
//! whose `value` is the returned expression.  We accept a `return` **only**
//! in tail position — the last statement of the body.  `return expr` sets
//! `body.value = expr`; a body with no `return` (or a bare `return;`) gets
//! `body.value = NilLit`.  A `return` anywhere *other* than the final
//! statement is a positioned [`JsLowerError`] ("early return not supported
//! in v0").
//!
//! ### Arrow functions and nested functions → `MakeClosure`
//!
//! An arrow function or a *nested* `function` declaration is lifted to a
//! synthesised top-level `Function` with a gensym'd name (`__lambda_<N>`
//! for arrows, the source name for nested declarations) plus a
//! [`MakeClosure`](Expr::MakeClosure) at the source position.  Its free
//! variables — body references that resolve to an *enclosing* function's
//! local / param / capture, and are not params/locals of the closure
//! itself, nor module functions/globals/builtins — become
//! [`Capture`]s on the synthesised `Function` (resolved as
//! [`Scope::Capture`] inside the body) and [`CaptureValue`]s on the
//! `MakeClosure` (resolved in the *enclosing* scope).  Capture discovery is
//! **on-resolve**: we lower the body inside a fresh scope frame and, each
//! time a name resolves to an enclosing frame, record it as a capture.
//!
//! An expression-bodied arrow (`x => x + 1`) yields a `Block` with no
//! statements and `value = x + 1`; a block-bodied arrow / nested function
//! follows the same tail-`return` rule as a top-level function.
//!
//! ### Calls → `DirectCall` / `IndirectCall` / `BuiltinCall`
//!
//! `f(args)` dispatches on the callee:
//!   * a known top-level/synthesised `function` name → [`DirectCall`];
//!   * `console.log(x)` (and the M1–M3 builtin map) → [`BuiltinCall`];
//!   * any *other* dotted member call `recv.method(args…)` → the SIR
//!     method-dispatch envelope `BuiltinCall("__method__", [recv,
//!     StrLit("method"), args…])` (C3 — collection methods);
//!   * any other callee that resolves to a *value* (a local/param/captured
//!     closure handle) → [`IndirectCall`] on that value.
//!
//! Calling a *computed* member (`obj[expr](…)`) or a non-identifier callee
//! is deferred.
//!
//! ### Recursion bound
//!
//! Closure/function-body nesting reuses [`MAX_STMT_DEPTH`] (the body is a
//! statement sequence lowered at the caller's `depth + 1`) and operand
//! recursion stays bounded by [`MAX_EXPR_DEPTH`], so a pathologically deep
//! nest of functions or calls becomes a positioned error, not a stack
//! overflow.
//!
//! ## Collections (M5 — learned by probing the parser)
//!
//! M5 adds **arrays** and **objects** plus member / subscript access.  The
//! CST rule names and child layouts (precedence-wrapper layers elided):
//!
//! | JS source        | CST rule              | children                                                    |
//! |------------------|-----------------------|-------------------------------------------------------------|
//! | `[1, 2, 3]`      | `array_literal`       | `[LBracket, element_list, RBracket]`                        |
//! | `[]`             | `array_literal`       | `[LBracket, element_list(empty), RBracket]`                 |
//! | `{a: 1, "k": v}` | `object_literal`      | `[LBrace, property_definition*, (Comma), RBrace]`           |
//! | `{}`             | `object_literal`      | `[LBrace, RBrace]`                                          |
//! | `a:`/`"k":`      | `property_definition` | `[property_name(Name|String), Colon, assignment_expression]`|
//! | `obj.prop`       | `member_expression`   | `[receiver, Dot, Name(prop)]`                               |
//! | `xs[i]`/`obj["k"]`| `member_expression`  | `[receiver, LBracket, expression, RBracket]`                |
//! | `({…})`          | `primary_expression`  | `[LParen, expression, RParen]` (grouping)                   |
//!
//! ### The bracket-vs-dot disambiguation (the M5 design decision)
//!
//! The IR has *both* sequences (`SeqLit`/`SeqIndex`/`SeqLen`/`SeqSet`) and
//! maps (`MapLit`/`MapGet`/`MapSet`).  JS `x[i]` is structurally ambiguous —
//! it could index either — and `x.prop` is unambiguously object access.  The
//! IR is untyped at this layer, so we cannot infer the receiver's kind.  We
//! follow the SIR19 collections table, which fixes:
//!
//! - `[…]` → `SeqLit`, `{…}` → `MapLit` (identifier and `"string"` keys both
//!   lower to a **string** map key — the JS quoting distinction is syntactic);
//! - `xs.length` → `SeqLen`; **every other** `.prop` (dot) → `MapGet` with a
//!   string key;
//! - `xs[i]` → `SeqIndex`, `d[k]`/`d.k` → `MapGet`.
//!
//! The only genuinely ambiguous *source* form is `x[<expr>]`.  We resolve it
//! **by the index expression's kind**: a **string-literal** key
//! (`obj["k"]`) → `MapGet` (treating a quoted subscript exactly like the
//! equivalent dotted access `obj.k`); **any other** index (`xs[0]`, `xs[i]`,
//! `xs[i + 1]`) → `SeqIndex`.  Assignment targets mirror this exactly:
//! `obj.prop = v` / `obj["k"] = v` → `MapSet`, `xs[i] = v` → `SeqSet`.  This
//! default is spec-sanctioned (the table is the authority); it is documented
//! here and in the spec's collections section.
//!
//! ### Recursion bound
//!
//! Every M5 CST walk is depth-bounded.  Array elements, object-property
//! values, member-chain receivers, and bracket indices are all lowered
//! through [`lower_expression`](Lowerer::lower_expression) at `depth + 1`, so
//! a pathological `[[[[…]]]]`, `{a:{b:{c:…}}}`, or `a.b.c.d…` tower past
//! [`MAX_EXPR_DEPTH`] becomes a positioned [`JsLowerError`], never a native
//! stack overflow (CWE-674).  The object-property and array-element *iteration*
//! is strictly non-recursive (a `for` over direct children).
//!
//! ### Deferred past M5
//!
//! Spread (`[...xs]` / `{...o}`), array elisions (`[1, , 3]`), object
//! shorthand (`{x}`), computed keys (`{[e]: v}`), numeric keys (`{0: v}`),
//! object methods / getters / setters, `.length` *assignment* (a resize, with
//! no IR node) — note that array/collection method **calls** (`.map`/`.push`/…)
//! are now lowered to `__method__` dispatch (C3), routed to the backend's OOP
//! runtime; classes, `this`/`new`,
//! generators / `async`/`await`, **rest** params (`...args`),
//! destructuring, and template literals all remain positioned errors.
//!
//! ### Default parameters (P9, post-M5)
//!
//! A defaulted formal `name = <expr>` (in both `function` declarations and
//! arrow functions) lowers to `Param { default: Some(<lowered expr>) }`.
//! JS defaults are **call-time** and may reference **earlier** params
//! (`function f(a, b = a + 1)`), so the initializer is lowered *inside* the
//! function frame (after the params are in scope) — a reference to an
//! earlier param resolves as `Scope::Param`, which is exactly the SIR
//! `Param.default` model.  A call omitting a defaulted argument (`f(5)`)
//! lowers to a **partial** `DirectCall` carrying only the present args.
//! Observes `Feature::DefaultParams`.  Rest and destructuring stay deferred.

use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, Capture, CaptureValue, Effect, EffectSet, ExportName, Expr, Feature, FeatureManifest,
    Function, IndexArg, Metadata, Module, Param, ParamKind, Scope, Span, Stmt, CURRENT_SIR_VERSION,
};
use std::collections::HashSet;

/// A failure encountered during JavaScript → SIR lowering.
///
/// Carries 1-based `line`/`column` so callers can produce IDE-friendly
/// diagnostics.  When the position is unknown (the AST node had no
/// recorded span), the fields are zero.  Mirrors the error shape used by
/// the sibling [`ruby-to-semantic-ir`](https://example.invalid) and
/// [`twig-to-semantic-ir`] frontends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for JsLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "JsLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for JsLowerError {}

/// Hard ceiling on operator-expression recursion depth.
///
/// `lower_expression` descends the precedence spine iteratively, but each
/// operand of a branching node is lowered by a *recursive* call back into
/// `lower_expression`.  A pathological input — thousands of nested
/// parenthesised operators — could otherwise drive that recursion deep
/// enough to overflow the thread stack (an uncatchable abort, i.e. a DoS
/// for any host compiling untrusted source).  We cap the depth and turn
/// an over-deep tree into an ordinary positioned error.  The limit is
/// generous: real JavaScript almost never nests operators past a handful
/// of levels, and the CST's ~20 fixed precedence-wrapper layers per
/// "real" level are peeled iteratively (they do **not** count against
/// this budget — only genuine operand recursion does).
const MAX_EXPR_DEPTH: usize = 256;

/// Hard ceiling on *statement-block* nesting depth (M3).
///
/// Each control-flow body (`if`/`while`/`for` body, or a bare `{ … }`
/// block) is lowered by a recursive call that descends with `depth + 1`.
/// Deeply nested control flow — thousands of `if (c) { if (c) { … } }` —
/// could otherwise drive that recursion deep enough to overflow the thread
/// stack (an uncatchable abort, i.e. a DoS for any host compiling untrusted
/// source).  We cap the nesting and turn an over-deep tree into an ordinary
/// positioned error.  The limit is generous: real JavaScript almost never
/// nests blocks past a handful of levels.  This is the statement-side twin
/// of [`MAX_EXPR_DEPTH`].
const MAX_STMT_DEPTH: usize = 256;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed JavaScript program into a [`semantic_ir::Module`].
///
/// The root node must be a `program` rule node — that's what
/// [`coding_adventures_javascript_parser::parse_javascript`] always
/// emits.  The `module_name` becomes the SIR module identifier
/// (typically the source file's stem).
///
/// M2 admits literal expression statements, `let`/`const`/`var`
/// bindings, re-assignments, variable references, and unary/binary
/// operators.  M3 adds control flow: `if`/`else`, `while`, the canonical
/// counting C-style `for`, `for … of`, and bare `{ … }` blocks.  Any
/// other statement or expression shape produces a [`JsLowerError`] (see
/// module docs).
pub fn compile(program: &GrammarASTNode, module_name: &str) -> Result<Module, JsLowerError> {
    if program.rule_name != "program" {
        return Err(JsLowerError {
            message: format!("expected root rule `program`, got `{}`", program.rule_name),
            line: program.start_line.unwrap_or(0),
            column: program.start_column.unwrap_or(0),
        });
    }

    let mut lw = Lowerer {
        file_name: module_name.to_string(),
        features_used: FeatureManifest::new(),
        scopes: Vec::new(),
        function_names: HashSet::new(),
        user_functions: Vec::new(),
        synthesised: Vec::new(),
        lambda_counter: 0,
    };

    // ── Pass 1: collect every `function` name (top-level and nested) ──
    // so a call can resolve to a `DirectCall` even when the callee is
    // defined later (forward reference) or refers to itself / a sibling
    // (recursion, mutual recursion).  Nested declarations are lifted to
    // top-level synthesised functions, so their names are module-wide too.
    // Depth-bounded so an adversarially deep CST is a positioned error, not
    // a stack overflow.
    collect_function_names(program, &mut lw.function_names, 0)?;

    // ── Pass 2: lower bodies. ────────────────────────────────────────
    // `main` is the synthetic top-level scope frame (no params/captures);
    // its locals accumulate as we lower the top-level statement sequence.
    let block = lw.lower_program(program)?;

    // Every JS source becomes a synthetic `main` whose body is the
    // top-level statement sequence — matching SIR17 (Python) and the
    // Ruby frontend.  `main` has no params, so it never triggers the
    // validator's `DynamicTyping` observation; we only declare features
    // the body actually uses.
    let main = Function {
        name: "main".to_string(),
        params: Vec::new(),
        return_type: None,
        captures: Vec::new(),
        body: block,
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: lw.span_of(program),
    };

    // The module's public surface is `main` plus every user-visible
    // top-level `function` (a module-scoped `function f` in JS is the
    // analog of `def f` at Python module scope).  Capture the names *before*
    // the function vectors are moved into the table below.
    let mut exports = vec![ExportName {
        name: "main".to_string(),
        span: Span::synthetic(),
    }];
    for name in lw.top_level_function_names_in_order() {
        exports.push(ExportName {
            name,
            span: Span::synthetic(),
        });
    }

    // Assemble the function table: synthesised closure bodies first, then
    // the user's top-level `function` declarations, then `main` last.
    // Order is cosmetic — the validator resolves names against the whole
    // set — but keeping `main` last mirrors the sibling frontends.
    let mut functions: Vec<Function> =
        Vec::with_capacity(lw.synthesised.len() + lw.user_functions.len() + 1);
    functions.append(&mut lw.synthesised);
    functions.append(&mut lw.user_functions);
    functions.push(main);

    // Manifest fixup: `MutualRecursion`.  The validator never *observes*
    // this feature (there is no node for it), so we must declare it
    // ourselves — and only when it is genuinely present, otherwise the
    // validator reports a spurious "declared but unused" warning.  We
    // detect a real cycle in the static call graph of the user/synthesised
    // functions (see `has_mutual_recursion`).
    if has_mutual_recursion(&functions) {
        lw.features_used.add(Feature::MutualRecursion);
    }
    // `DynamicTyping`: any function with an untyped param (every JS param
    // is untyped in v0) makes the validator observe `DynamicTyping`, so we
    // must declare it to match.  `main` has no params; a user `function`
    // or arrow with ≥1 param does.
    if functions
        .iter()
        .any(|f| f.params.iter().any(|p| p.sir_type.is_none()))
    {
        lw.features_used.add(Feature::DynamicTyping);
    }

    // Materialise the manifest in a stable order.  The SIR validator
    // requires the manifest to *exactly* match what the body uses:
    // used-but-undeclared is an error, declared-but-unused a warning.
    // We tallied features while lowering, so we just hand the
    // accumulator over.
    let manifest = lw.features_used.clone();

    let metadata = Metadata::new()
        .with_source_language("javascript")
        .with_sir_version(CURRENT_SIR_VERSION);

    Ok(Module {
        name: module_name.to_string(),
        manifest,
        imports: Vec::new(),
        exports,
        functions,
        globals: Vec::new(),
        metadata,
        span: lw.span_of(program),
    })
}

// ---------------------------------------------------------------------------
// Lowerer — the small amount of mutable state M2 needs
// ---------------------------------------------------------------------------

/// One lexical *function frame* on the scope stack.
///
/// The bottom frame is the synthetic `main`; each arrow / nested
/// `function` pushes a new frame while its body is lowered.  Resolution
/// (`resolve_name`) walks the stack top-down: the current frame's
/// `locals`/`params`, then `captures`, then — for a name found in an
/// *enclosing* frame — a capture is recorded on the current closure.
/// A single formal parameter extracted from the CST, before it is lowered
/// into a [`Param`].  `default` borrows the initializer expression node (the
/// `assignment_expression` after `=`) so it can be lowered **inside** the
/// function frame — JS defaults are call-time and may reference earlier
/// params, so they must be lowered with the params in scope.
struct FormalParamSpec<'a> {
    name: String,
    default: Option<&'a GrammarASTNode>,
}

struct FnScope {
    /// Parameter names of this function (empty for `main`).
    params: HashSet<String>,
    /// Capture names discovered so far for this closure body (empty for
    /// `main` and for top-level `function` declarations, which capture
    /// nothing — they see only globals/functions/builtins).
    captures: HashSet<String>,
    /// `let`/`const`/`var`-bound names visible at this point, in source
    /// order of first binding.  Used both for resolution and (via the
    /// snapshot/restore dance) block scoping.
    locals: HashSet<String>,
    /// Whether this frame is a *closure* frame (an arrow or a nested
    /// `function`).  Only closure frames accumulate captures; `main` and
    /// top-level declarations are not closures.  When a name resolves to a
    /// frame *below* a closure frame, every intervening closure frame must
    /// capture it (transitive capture).
    is_closure: bool,
    /// Capture values to attach to this closure's `MakeClosure`, paired by
    /// name with `captures`.  Each is the resolution of the captured name
    /// in the *enclosing* scope, computed when the capture is first seen.
    capture_values: Vec<CaptureValue>,
}

impl FnScope {
    fn new(params: HashSet<String>, is_closure: bool) -> Self {
        FnScope {
            params,
            captures: HashSet::new(),
            locals: HashSet::new(),
            is_closure,
            capture_values: Vec::new(),
        }
    }
}

struct Lowerer {
    /// Logical filename stamped into every [`Span`].  We use the module
    /// name because the parser CST doesn't carry the original path.
    file_name: String,
    /// Features accumulated as we lower.  `FeatureManifest::add` is
    /// idempotent, so repeated `StrLit`s add `Strings` exactly once.
    features_used: FeatureManifest,
    /// The lexical function-frame stack (see [`FnScope`]).  The bottom is
    /// `main`; arrows and nested `function`s push/pop frames around their
    /// bodies.  Resolution walks it top-down.
    scopes: Vec<FnScope>,
    /// Every `function` name (top-level + nested) collected in pass 1, so
    /// a call resolves to a `DirectCall` regardless of source order and
    /// recursion / mutual recursion works.
    function_names: HashSet<String>,
    /// User-written top-level `function` declarations, lowered.  Kept
    /// separate from `main` and from the synthesised closures so the
    /// module's function table and export list can be assembled in a
    /// stable order.
    user_functions: Vec<Function>,
    /// Synthesised closure-body functions — one per arrow function and one
    /// per *nested* `function` declaration.  Referenced by `MakeClosure`.
    synthesised: Vec<Function>,
    /// Gensym counter for synthesised arrow names (`__lambda_<N>`).
    lambda_counter: usize,
}

impl Lowerer {
    /// Build a [`Span`] from a node's recorded 1-based position.  Falls
    /// back to a zero point when the parser left positions unset.
    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::new(
            self.file_name.clone(),
            node.start_line.unwrap_or(0),
            node.start_column.unwrap_or(0),
            node.end_line
                .unwrap_or_else(|| node.start_line.unwrap_or(0)),
            node.end_column
                .unwrap_or_else(|| node.start_column.unwrap_or(0)),
        )
    }

    /// Build a [`Span`] from a leaf [`Token`]'s 1-based position.  The
    /// width is one column (zero-width point), which is good enough for
    /// literal diagnostics.
    fn span_of_token(&self, tok: &Token) -> Span {
        Span::point(self.file_name.clone(), tok.line, tok.column)
    }

    // -----------------------------------------------------------------------
    // program → Block
    // -----------------------------------------------------------------------

    /// Lower the whole program into a single [`Block`].
    ///
    /// SIR `Block`s are "statements then a tail value": `Block.value` is
    /// the program's result.  Following SIR17/Ruby, binding and
    /// assignment statements accumulate in `stmts`, and the **final**
    /// top-level *expression* statement becomes the tail `value`.
    /// Earlier bare expression statements are pure (M2 has no calls yet),
    /// hence unobservable, so we drop them.  An empty program yields a
    /// `NilLit` value.
    fn lower_program(&mut self, program: &GrammarASTNode) -> Result<Block, JsLowerError> {
        // Push the synthetic `main` frame: no params, no captures, not a
        // closure.  Top-level `let`/`const`/`var` become its locals.
        self.scopes.push(FnScope::new(HashSet::new(), false));
        // The top-level statement sequence is `program`'s children, each a
        // `source_element` wrapping one `statement`.  Lowering it is exactly
        // lowering a statement list — the same routine used for `{ … }`
        // block bodies — at depth 0.
        let result = self.lower_stmt_seq(&program.children, self.span_of(program), 0);
        self.scopes.pop();
        result
    }

    // -----------------------------------------------------------------------
    // Scope stack helpers (M4)
    // -----------------------------------------------------------------------

    /// The current (innermost) function frame.  There is always at least
    /// one frame (`main`) while a body is being lowered.
    fn cur(&mut self) -> &mut FnScope {
        self.scopes
            .last_mut()
            .expect("scope stack is non-empty during lowering")
    }

    /// Is `name` a local already bound in the *current* frame?  Drives the
    /// binding-vs-assignment choice (first sighting binds, later
    /// re-assigns) at the current lexical level.
    fn is_current_local(&self, name: &str) -> bool {
        self.scopes
            .last()
            .map(|s| s.locals.contains(name))
            .unwrap_or(false)
    }

    /// Record a `let`/`const`/`var` binding in the current frame.
    fn declare_local(&mut self, name: &str) {
        self.cur().locals.insert(name.to_string());
    }

    /// Resolve a bare identifier to a [`Scope`]d [`VarRef`], discovering
    /// captures along the way.
    ///
    /// We walk the scope stack from the innermost frame outward:
    ///
    /// - a hit in the **current** frame's locals/params → `Local`/`Param`;
    /// - a hit in a frame's already-recorded captures → `Capture`;
    /// - a hit in an **enclosing** frame's locals/params → a *free
    ///   variable*.  Every closure frame between the use site and the
    ///   defining frame must capture it (transitive capture): we add the
    ///   name to each such frame's `captures` and, for the innermost
    ///   closure frame, record a `CaptureValue` resolving the name in the
    ///   enclosing scope.  The reference itself becomes `Capture`.
    ///
    /// Names not found in any frame fall through to the caller (which tries
    /// `undefined`, then module functions/globals/builtins, then errors).
    fn resolve_local_chain(&mut self, name: &str, span: &Span) -> Option<Expr> {
        let n = self.scopes.len();
        // Find the frame that *defines* `name` (as local/param/capture),
        // searching innermost-first.
        let mut def_idx: Option<(usize, Scope)> = None;
        for i in (0..n).rev() {
            let f = &self.scopes[i];
            if f.locals.contains(name) {
                def_idx = Some((i, Scope::Local));
                break;
            }
            if f.params.contains(name) {
                def_idx = Some((i, Scope::Param));
                break;
            }
            if f.captures.contains(name) {
                def_idx = Some((i, Scope::Capture));
                break;
            }
        }
        let (def_i, def_scope) = def_idx?;
        let cur_i = n - 1;

        if def_i == cur_i {
            // Defined right here — a plain reference.
            return Some(Expr::VarRef {
                name: name.to_string(),
                scope: def_scope,
                span: span.clone(),
            });
        }

        // Defined in an *enclosing* frame: this is a free variable.  Each
        // closure frame strictly above the defining frame must capture it.
        // We thread the capture *value* outward: in the defining frame the
        // value is a direct reference (Local/Param/Capture there); each
        // closure frame then captures from the frame just below it.
        //
        // Walk from def_i+1 up to cur_i; for every closure frame, ensure it
        // captures `name` (recording a CaptureValue that references the
        // name as it is seen *one frame down*).
        for i in (def_i + 1)..=cur_i {
            if !self.scopes[i].is_closure {
                continue;
            }
            if self.scopes[i].captures.contains(name) {
                continue; // already captured by this frame.
            }
            // The value of the capture, as resolved in frame i-1.  Frame
            // i-1 is the defining frame or an inner closure that already
            // captured it.
            let below = &self.scopes[i - 1];
            let below_scope = if below.locals.contains(name) {
                Scope::Local
            } else if below.params.contains(name) {
                Scope::Param
            } else {
                // Must be a capture of the frame below (already inserted on
                // a prior iteration, or the defining frame was a closure).
                Scope::Capture
            };
            let value = Expr::VarRef {
                name: name.to_string(),
                scope: below_scope,
                span: span.clone(),
            };
            self.scopes[i].captures.insert(name.to_string());
            self.scopes[i].capture_values.push(CaptureValue {
                name: name.to_string(),
                value,
            });
        }

        // Inside the current (closure) frame the reference is a capture.
        Some(Expr::VarRef {
            name: name.to_string(),
            scope: Scope::Capture,
            span: span.clone(),
        })
    }

    /// Resolve an identifier in *value* position to a [`VarRef`].
    ///
    /// Order: scope chain (local/param/capture, possibly recording a
    /// capture) → a top-level/synthesised `function` name used as a value
    /// (`Scope::Global`, which the validator resolves against the function
    /// table) → positioned "unresolved name" error.
    fn resolve_name(
        &mut self,
        name: &str,
        span: Span,
        line: usize,
        column: usize,
    ) -> Result<Expr, JsLowerError> {
        if let Some(e) = self.resolve_local_chain(name, &span) {
            return Ok(e);
        }
        if self.function_names.contains(name) {
            // A function name referenced as a value (e.g. `return inner;`
            // or `let g = f;`).  The validator accepts `Scope::Global` for
            // a name in the function table.
            return Ok(Expr::VarRef {
                name: name.to_string(),
                scope: Scope::Global,
                span,
            });
        }
        Err(JsLowerError {
            message: format!("unresolved name reference `{name}`"),
            line,
            column,
        })
    }

    /// Lower a slice of CST children that are statement-bearing nodes into a
    /// single [`Block`] (statements then a tail value).
    ///
    /// This is the shared workhorse for the top-level program body and every
    /// `{ … }` block / control-flow body.  Each child is lowered to a
    /// [`Lowered`]:
    ///
    /// - a [`Lowered::Stmt`] is pushed onto `stmts` (flushing any pending
    ///   bare-expression value as an `ExprStmt` first, so evaluation order
    ///   and side effects are preserved);
    /// - a [`Lowered::Expr`] becomes the *candidate* tail value, superseding
    ///   any earlier pure bare-expression value.
    ///
    /// The final candidate tail value becomes `Block.value`; an empty
    /// sequence yields a `NilLit` tail (matching SIR's "every block produces
    /// a value" rule).  `block_span` stamps the resulting `Block`.
    fn lower_stmt_seq(
        &mut self,
        children: &[ASTNodeOrToken],
        block_span: Span,
        depth: usize,
    ) -> Result<Block, JsLowerError> {
        let mut stmts: Vec<Stmt> = Vec::new();
        // The most recent bare-expression value seen.  Whatever it holds
        // at the end becomes the block's tail value.
        let mut tail: Option<Expr> = None;

        for child in children {
            match child {
                ASTNodeOrToken::Node(n) => {
                    match self.lower_source_element(n, depth)? {
                        Lowered::Stmt(s) => {
                            // A statement makes any pending tail
                            // expression unobservable as a *value*, but it
                            // may have a side effect — keep it as an
                            // `ExprStmt` so evaluation order is preserved.
                            if let Some(prev) = tail.take() {
                                let span = prev.span().clone();
                                stmts.push(Stmt::ExprStmt { expr: prev, span });
                            }
                            stmts.push(*s);
                        }
                        Lowered::Expr(e) => {
                            // A new bare expression supersedes the prior one
                            // as the candidate tail value.  The prior one is
                            // no longer observable *as a value*, but if it may
                            // have a side effect (e.g. `console.log(...)` →
                            // an effectful `print`, or a call) it must still
                            // run — flush it as an `ExprStmt`.  A provably
                            // pure prior (a literal, a var-ref, arithmetic on
                            // pure operands) is simply dropped.  Without this,
                            // two consecutive `console.log(...)` statements
                            // would lose all but the last.
                            if let Some(prev) = tail.take() {
                                if expr_may_have_effects(&prev) {
                                    let span = prev.span().clone();
                                    stmts.push(Stmt::ExprStmt { expr: prev, span });
                                }
                            }
                            tail = Some(e);
                        }
                    }
                }
                // Stray tokens (the `{`/`}` of a block, the `source_element`
                // separators, etc.) carry no statement; skip them.
                ASTNodeOrToken::Token(_) => {}
            }
        }

        let value = tail.unwrap_or(Expr::NilLit {
            span: block_span.clone(),
        });

        Ok(Block {
            stmts,
            value,
            span: block_span,
        })
    }

    /// Lower one statement-bearing item (a `source_element`, a `statement`
    /// wrapper, or a concrete statement node) to a [`Lowered`].
    ///
    /// `depth` bounds control-flow body nesting (see [`MAX_STMT_DEPTH`]); a
    /// nested body recurses with `depth + 1`.
    fn lower_source_element(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, JsLowerError> {
        // `source_element` → `statement` → `<concrete statement>`.
        // Descend through the single-child wrappers until we reach a
        // statement we recognise.
        let inner = single_child_node(node).unwrap_or(node);
        match inner.rule_name.as_str() {
            // `source_element` and `statement` are both single-child
            // wrappers; recurse through them to the concrete statement.
            "statement" => self.lower_source_element(inner, depth),
            other => self.lower_statement_inner(inner, other, depth),
        }
    }

    /// Lower a concrete statement node, dispatching on its `rule_name`.
    fn lower_statement_inner(
        &mut self,
        node: &GrammarASTNode,
        rule_name: &str,
        depth: usize,
    ) -> Result<Lowered, JsLowerError> {
        match rule_name {
            "expression_statement" => self.lower_expression_statement(node),
            "lexical_declaration" => self
                .lower_lexical_declaration(node)
                .map(|s| Lowered::Stmt(Box::new(s))),
            "variable_statement" => self
                .lower_variable_statement(node)
                .map(|s| Lowered::Stmt(Box::new(s))),
            // ── M3: control flow ────────────────────────────────────
            "if_statement" => self.lower_if(node, depth).map(Lowered::Expr),
            "while_statement" => self
                .lower_while(node, depth)
                .map(|s| Lowered::Stmt(Box::new(s))),
            "for_statement" => self
                .lower_for(node, depth)
                .map(|s| Lowered::Stmt(Box::new(s))),
            "for_of_statement" => self
                .lower_for_of(node, depth)
                .map(|s| Lowered::Stmt(Box::new(s))),
            "block" => self.lower_block(node, depth).map(|b| {
                // A bare `{ … }` block is a value-producing expression in
                // SIR (`Expr::Block`); at statement position its tail value
                // is unobservable but its statements run for effect.
                Lowered::Expr(Expr::Block(Box::new(b)))
            }),
            // ── M4: functions ───────────────────────────────────────
            "function_declaration" => self.lower_function_declaration(node, depth),
            // A `return` outside a function body is invalid JS; inside one
            // it is handled positionally by `lower_function_body` (only the
            // tail statement may be a `return`).  Reaching it here means it
            // was *not* in tail position → reject with the spec's message.
            "return_statement" => Err(self.early_return_error(node)),
            // deferred to a later milestone: switch, try, do-while,
            // labeled, break/continue, …
            other => Err(self.unsupported(node, other)),
        }
    }

    /// The positioned "early return" error (SIR19 "Return statement").
    fn early_return_error(&self, node: &GrammarASTNode) -> JsLowerError {
        JsLowerError {
            message: "early return not supported in v0 (only a trailing tail-position \
                      `return` is accepted)"
                .to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        }
    }

    // -----------------------------------------------------------------------
    // M3: control-flow lowering
    // -----------------------------------------------------------------------

    /// Lower an `if_statement` to an [`Expr::If`].
    ///
    /// CST (probed): `[Kw("if"), (, expression, ), statement]` with no else,
    /// or `[…, statement, Kw("else"), statement]` with one.  The `then`/
    /// `else` `statement`s are each a block body (a `{ … }` block or a
    /// single statement).  A missing `else` becomes a synthetic empty
    /// nil-valued [`Block`].  An `else if` chain is the grammar nesting
    /// another `if_statement` inside the else `statement`, so it recurses
    /// into a nested `Expr::If` automatically.
    fn lower_if(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, JsLowerError> {
        self.check_stmt_depth(node, depth)?;
        let span = self.span_of(node);

        // Collect the direct child *nodes* in order: [cond_expr, then_stmt]
        // or [cond_expr, then_stmt, else_stmt].  The `if`/`else` keywords,
        // parens, etc. are tokens we skip.
        let nodes = child_nodes(node);
        let cond_node = nodes
            .first()
            .ok_or_else(|| self.unsupported(node, "if (no condition)"))?;
        let then_node = nodes
            .get(1)
            .ok_or_else(|| self.unsupported(node, "if (no then branch)"))?;

        let cond = self.lower_expression(cond_node, 0)?;
        let then_branch = self.lower_body(then_node, depth)?;
        let else_branch = match nodes.get(2) {
            Some(else_node) => self.lower_body(else_node, depth)?,
            // No `else`: an empty, nil-valued block.
            None => Block {
                stmts: Vec::new(),
                value: Expr::NilLit { span: span.clone() },
                span: span.clone(),
            },
        };

        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
            span,
        })
    }

    /// Lower a `while_statement` to a [`Stmt::While`].
    ///
    /// CST: `[Kw("while"), (, expression, ), statement]`.
    fn lower_while(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Stmt, JsLowerError> {
        self.check_stmt_depth(node, depth)?;
        let span = self.span_of(node);
        let nodes = child_nodes(node);
        let cond_node = nodes
            .first()
            .ok_or_else(|| self.unsupported(node, "while (no condition)"))?;
        let body_node = nodes
            .get(1)
            .ok_or_else(|| self.unsupported(node, "while (no body)"))?;

        let cond = self.lower_expression(cond_node, 0)?;
        let body = self.lower_body(body_node, depth)?;
        // The validator observes `Feature::Loops` for every loop statement;
        // declare it so the manifest matches the body exactly.
        self.features_used.add(Feature::Loops);
        Ok(Stmt::While { cond, body, span })
    }

    /// Lower a `for_of_statement` to a [`Stmt::ForEach`].
    ///
    /// CST: `[Kw("for"), (, Kw("let|const|var"), binding_element,
    /// Name("of"), assignment_expression(iter), ), statement]`.  Only the
    /// single-identifier binding (`for (const x of xs)`) is supported;
    /// destructuring (`for (const [a, b] of …)`) is deferred.  The loop
    /// variable `x` is bound into the body scope only.
    fn lower_for_of(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Stmt, JsLowerError> {
        self.check_stmt_depth(node, depth)?;
        let span = self.span_of(node);

        // The binding name lives in the `binding_element`'s single Name
        // token.  Anything else (a destructuring pattern) is deferred.
        let binding = child_node_named(node, "binding_element")
            .ok_or_else(|| self.unsupported(node, "for-of (no binding_element)"))?;
        let var_tok = single_leaf_token(binding)
            .filter(|t| matches!(t.type_, TokenType::Name))
            .ok_or_else(|| JsLowerError {
                message: "for-of destructuring binding is deferred (only `for (const x of …)`)"
                    .to_string(),
                line: span.start_line,
                column: span.start_col,
            })?;
        let var = var_tok.value.clone();

        // The iterable is the `assignment_expression` child — the only
        // expression-shaped node (the `binding_element` is the binding).
        let iter_node = child_node_named(node, "assignment_expression")
            .ok_or_else(|| self.unsupported(node, "for-of (no iterable)"))?;
        let iter = self.lower_expression(iter_node, 0)?;

        let body = self.lower_loop_body_scoped(&var, node, depth)?;
        self.features_used.add(Feature::Loops);
        Ok(Stmt::ForEach {
            var,
            iter,
            body,
            span,
        })
    }

    /// Lower a canonical C-style `for_statement` to a [`Stmt::ForRange`].
    ///
    /// CST: `[Kw("for"), (, Kw("let"), binding_list, ;, expr(cond), ;,
    /// expr(update), ), statement]`.  We accept **only** the canonical
    /// counting shape (see module docs):
    ///
    ///   * init `let i = <start>` (single binding of `i`),
    ///   * cond `i < <stop>` or `i <= <stop>` on the same `i`,
    ///   * update `i = i + <step>`, `i += <step>`, or `i++` (step 1).
    ///
    /// `<=` is rewritten half-open by bumping `stop` to `stop + 1`.  Any
    /// non-canonical shape is a positioned [`JsLowerError`] (deferred).
    fn lower_for(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Stmt, JsLowerError> {
        self.check_stmt_depth(node, depth)?;
        let span = self.span_of(node);

        // The non-canonical bail-out, factored so every rejection carries
        // the loop's position and a uniform "deferred" message.
        let reject = |why: &str| JsLowerError {
            message: format!(
                "non-canonical C-style `for` ({why}) is deferred; only the counting form \
                 `for (let i = <start>; i < <stop>; i++/i += <step>)` is supported"
            ),
            line: span.start_line,
            column: span.start_col,
        };

        // ── init: `let i = <start>` ─────────────────────────────────────
        // The init clause is a `binding_list` (for `let`/`const`) sitting
        // directly under the `for_statement` (the probe shows it is *not*
        // wrapped in a `lexical_declaration` here — the `let` keyword and
        // `binding_list` are direct children).  `var` would surface a
        // `variable_declaration_list` instead; we accept either.
        let (loop_var, start) = self
            .extract_for_init(node)
            .ok_or_else(|| reject("init is not a single `let i = <start>` binding"))?;

        // ── cond: `i < <stop>` / `i <= <stop>` ──────────────────────────
        // The condition is the first `expression` child after the init's
        // terminating `;`.  We need the *branching* relational node.
        let cond_expr = self
            .for_clause_expr(node, 0)
            .ok_or_else(|| reject("missing condition clause"))?;
        let stop = self.extract_for_cond(cond_expr, &loop_var).ok_or_else(|| {
            reject("condition is not `i < <stop>` or `i <= <stop>` on the loop variable")
        })?;

        // ── update: `i = i + <step>` / `i += <step>` / `i++` ────────────
        let update_expr = self
            .for_clause_expr(node, 1)
            .ok_or_else(|| reject("missing update clause"))?;
        let step = self
            .extract_for_step(update_expr, &loop_var)
            .ok_or_else(|| {
                reject("update is not an increment of the loop variable by a constant step")
            })?;

        // ── body (loop variable scoped into it) ─────────────────────────
        let body = self.lower_loop_body_scoped(&loop_var, node, depth)?;

        self.features_used.add(Feature::Loops);
        Ok(Stmt::ForRange {
            var: loop_var,
            start,
            stop,
            step,
            body,
            span,
        })
    }

    /// Extract `(var, start_expr)` from a C-`for` init clause, or `None` if
    /// it is not a single `let|const|var i = <start>` binding.
    fn extract_for_init(&mut self, for_node: &GrammarASTNode) -> Option<(String, Expr)> {
        // `let`/`const` → `binding_list[ lexical_binding[ Name, =, init ] ]`.
        // `var`         → `variable_declaration_list[ variable_declaration ]`.
        let (list_name, binding_name) = if child_node_named(for_node, "binding_list").is_some() {
            ("binding_list", "lexical_binding")
        } else {
            ("variable_declaration_list", "variable_declaration")
        };
        let list = child_node_named(for_node, list_name)?;
        let bindings = children_nodes_named(list, binding_name);
        if bindings.len() != 1 {
            return None; // multi-variable init is non-canonical.
        }
        let binding = bindings[0];
        let name_tok = binding.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
            _ => None,
        })?;
        let init_node = binding.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })?;
        let start = self.lower_expression(init_node, 0).ok()?;
        Some((name_tok.value.clone(), start))
    }

    /// Return the `n`-th `expression` clause node under a `for_statement`
    /// (cond = 0, update = 1).  These are the `expression` rule nodes that
    /// sit between the clause-separating `;`/`)` tokens.
    fn for_clause_expr<'a>(
        &self,
        for_node: &'a GrammarASTNode,
        n: usize,
    ) -> Option<&'a GrammarASTNode> {
        for_node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(node) if node.rule_name == "expression" => Some(node),
                _ => None,
            })
            .nth(n)
    }

    /// Extract the `stop` expression from a canonical loop condition.
    ///
    /// Accepts `i < S` (→ `S`) and `i <= S` (→ half-open `S + 1`), where the
    /// left operand is exactly the loop variable `var`.  Returns `None` for
    /// any other comparison (wrong variable, `>`/`>=`, RHS-anchored, …).
    fn extract_for_cond(&mut self, cond_node: &GrammarASTNode, var: &str) -> Option<Expr> {
        // Peel to the branching `relational_expression`: `[lhs, op, rhs]`.
        let branch = peel_to_branch(cond_node);
        if branch.rule_name != "relational_expression" || branch.children.len() != 3 {
            return None;
        }
        // children = [lhs_node, op_token, rhs_node].
        let lhs = match &branch.children[0] {
            ASTNodeOrToken::Node(n) => n,
            _ => return None,
        };
        let op = match &branch.children[1] {
            ASTNodeOrToken::Token(t) => t.value.as_str(),
            _ => return None,
        };
        let rhs = match &branch.children[2] {
            ASTNodeOrToken::Node(n) => n,
            _ => return None,
        };
        // LHS must be exactly the loop variable.
        let lhs_tok = single_leaf_token(lhs)?;
        if !matches!(lhs_tok.type_, TokenType::Name) || lhs_tok.value != var {
            return None;
        }
        let stop = self.lower_expression(rhs, 0).ok()?;
        match op {
            "<" => Some(stop),
            "<=" => {
                // Half-open rewrite: `i <= S` ⇔ `i < S + 1`.
                let span = stop.span().clone();
                Some(Expr::BuiltinCall {
                    name: "+".to_string(),
                    args: vec![
                        stop,
                        Expr::IntLit {
                            value: 1,
                            span: span.clone(),
                        },
                    ],
                    effects: EffectSet::PURE,
                    span,
                })
            }
            _ => None,
        }
    }

    /// Extract the `step` expression from a canonical loop update clause.
    ///
    /// Accepts (on the loop variable `var`):
    ///
    ///   * `i++`           → `IntLit(1)` (postfix increment),
    ///   * `i += <step>`   → `<step>`,
    ///   * `i = i + <step>`→ `<step>`.
    ///
    /// Returns `None` for decrements, `*=`, a different variable, etc.
    fn extract_for_step(&mut self, update_node: &GrammarASTNode, var: &str) -> Option<Expr> {
        let branch = peel_to_branch(update_node);
        match branch.rule_name.as_str() {
            // ── `i++` : postfix_expression[ lhs, Name("++") ] ───────────
            "postfix_expression" => {
                let nodes = child_nodes(branch);
                let target = nodes.first()?;
                let t = single_leaf_token(target)?;
                if !matches!(t.type_, TokenType::Name) || t.value != var {
                    return None;
                }
                // The operator token must be `++` (reject `i--`).
                let op_ok = branch
                    .children
                    .iter()
                    .any(|c| matches!(c, ASTNodeOrToken::Token(tok) if tok.value == "++"));
                if !op_ok {
                    return None;
                }
                Some(Expr::IntLit {
                    value: 1,
                    span: self.span_of(branch),
                })
            }
            // ── `i += s` or `i = i + s` :
            //    assignment_expression[ lhs, op, rhs ] ─────────────────
            "assignment_expression" if branch.children.len() == 3 => {
                let lhs = match &branch.children[0] {
                    ASTNodeOrToken::Node(n) => n,
                    _ => return None,
                };
                let lhs_tok = single_leaf_token(lhs)?;
                if !matches!(lhs_tok.type_, TokenType::Name) || lhs_tok.value != var {
                    return None;
                }
                // The assignment operator value (`=`, `+=`, …).
                let op = match &branch.children[1] {
                    ASTNodeOrToken::Node(n) => single_leaf_token(n)?.value.clone(),
                    ASTNodeOrToken::Token(t) => t.value.clone(),
                };
                let rhs = match &branch.children[2] {
                    ASTNodeOrToken::Node(n) => n,
                    _ => return None,
                };
                match op.as_str() {
                    // `i += s` → step is `s`.
                    "+=" => self.lower_expression(rhs, 0).ok(),
                    // `i = i + s` → the RHS must be `i + s`; step is `s`.
                    "=" => self.extract_plus_step(rhs, var),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// From an RHS shaped `i + <step>` (an `additive_expression` whose left
    /// operand is the loop variable and whose single operator is `+`),
    /// extract `<step>`.  Returns `None` for anything else (e.g. `i - 1`,
    /// `i * 2`, `s + i`).
    fn extract_plus_step(&mut self, rhs: &GrammarASTNode, var: &str) -> Option<Expr> {
        let branch = peel_to_branch(rhs);
        // `i + s` is one `additive_expression` with children
        // `[i_node, Plus, s_node]`.
        if branch.rule_name != "additive_expression" || branch.children.len() != 3 {
            return None;
        }
        let lhs = match &branch.children[0] {
            ASTNodeOrToken::Node(n) => n,
            _ => return None,
        };
        let op = match &branch.children[1] {
            ASTNodeOrToken::Token(t) => t.value.as_str(),
            _ => return None,
        };
        if op != "+" {
            return None;
        }
        let lhs_tok = single_leaf_token(lhs)?;
        if !matches!(lhs_tok.type_, TokenType::Name) || lhs_tok.value != var {
            return None;
        }
        let step_node = match &branch.children[2] {
            ASTNodeOrToken::Node(n) => n,
            _ => return None,
        };
        self.lower_expression(step_node, 0).ok()
    }

    // -----------------------------------------------------------------------
    // M3: shared body / block helpers
    // -----------------------------------------------------------------------

    /// Lower a control-flow *body* `statement` (an `if`/`while` branch or a
    /// `for` body) into a [`Block`].
    ///
    /// The body is either a `{ … }` block (→ its statement sequence) or a
    /// single statement (→ a one-item block).  Either way names bound inside
    /// it are block-scoped: we snapshot `declared_locals` before lowering
    /// and restore it after, so an inner `let` does not leak outward.
    fn lower_body(
        &mut self,
        body_stmt: &GrammarASTNode,
        depth: usize,
    ) -> Result<Block, JsLowerError> {
        let saved = self.cur().locals.clone();
        let result = self.lower_body_inner(body_stmt, depth);
        // Restore the outer scope regardless of success so a partially
        // mutated set never leaks (defensive; on `Err` we abort anyway).
        self.cur().locals = saved;
        result
    }

    /// Lower a loop body with `loop_var` bound into the body scope only.
    ///
    /// Mirrors the validator, which adds the loop variable to its `LocalEnv`
    /// for the body and rewinds afterwards.  The variable must resolve to a
    /// `Scope::Local` `VarRef` inside the body but be invisible after the
    /// loop.  We add it to `declared_locals` over the body and then restore
    /// the snapshot (which excludes it).
    fn lower_loop_body_scoped(
        &mut self,
        loop_var: &str,
        for_node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Block, JsLowerError> {
        let body_stmt = self
            .loop_body_node(for_node)
            .ok_or_else(|| self.unsupported(for_node, "loop (no body)"))?;
        let saved = self.cur().locals.clone();
        self.cur().locals.insert(loop_var.to_string());
        let result = self.lower_body_inner(body_stmt, depth);
        self.cur().locals = saved;
        result
    }

    /// The body `statement` of a loop is its **last** direct child node
    /// (after the header tokens / clause expressions).
    fn loop_body_node<'a>(&self, for_node: &'a GrammarASTNode) -> Option<&'a GrammarASTNode> {
        child_nodes(for_node).into_iter().next_back()
    }

    /// Inner body-lowering shared by [`lower_body`] and
    /// [`lower_loop_body_scoped`] (which own the scope save/restore).
    fn lower_body_inner(
        &mut self,
        body_stmt: &GrammarASTNode,
        depth: usize,
    ) -> Result<Block, JsLowerError> {
        // Descend the `statement` wrapper to the concrete body node.
        let inner = single_child_node(body_stmt).unwrap_or(body_stmt);
        if inner.rule_name == "block" {
            return self.lower_block(inner, depth);
        }
        // A single (unbraced) statement body, e.g. `if (c) x = 1;`.  Lower
        // the one statement and fold it into a one-element `Block`, reusing
        // the same Stmt/Expr → (stmts, tail) routing as a block.
        let span = self.span_of(body_stmt);
        let mut stmts: Vec<Stmt> = Vec::new();
        let value = match self.lower_source_element(body_stmt, depth + 1)? {
            Lowered::Stmt(s) => {
                stmts.push(*s);
                Expr::NilLit { span: span.clone() }
            }
            Lowered::Expr(e) => e,
        };
        Ok(Block { stmts, value, span })
    }

    /// Lower a `block` (`{ stmt* }`) into a [`Block`].  The `{`/`}` tokens
    /// are skipped by [`lower_stmt_seq`].  Recurses with `depth + 1` so the
    /// nesting guard catches pathological depth.
    fn lower_block(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Block, JsLowerError> {
        self.check_stmt_depth(node, depth)?;
        let span = self.span_of(node);
        self.lower_stmt_seq(&node.children, span, depth + 1)
    }

    /// Enforce the [`MAX_STMT_DEPTH`] nesting bound; error if exceeded.
    fn check_stmt_depth(&self, node: &GrammarASTNode, depth: usize) -> Result<(), JsLowerError> {
        if depth > MAX_STMT_DEPTH {
            return Err(JsLowerError {
                message: format!(
                    "control-flow nests deeper than the supported limit ({MAX_STMT_DEPTH})"
                ),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            });
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // M4: functions, arrows, return, calls
    // -----------------------------------------------------------------------

    /// Lower a `function_declaration`.
    ///
    /// At **top level** (the `main` frame) the function is a user-visible
    /// `Function` with no captures — its body sees only its params, its own
    /// locals, and module functions/globals/builtins.  When **nested**
    /// inside another function, it is lifted to a top-level *synthesised*
    /// `Function` whose free variables are captured, and the declaration
    /// site binds the function's name as a `let*` to a `MakeClosure`
    /// referencing it (so it can be returned / called indirectly and so its
    /// name resolves locally).
    fn lower_function_declaration(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, JsLowerError> {
        self.check_stmt_depth(node, depth)?;
        let name = function_decl_name(node)
            .ok_or_else(|| self.unsupported(node, "function_declaration (no name)"))?;
        let params = self.formal_param_specs(node)?;
        let body_node = child_node_named(node, "function_body");

        // A *nested* declaration (we're inside a function frame deeper than
        // `main`) is a closure: lift + capture, and bind the name locally.
        let nested = self.scopes.len() > 1;

        let (lowered_fn, captures) =
            self.lower_callable_body(&name, &params, body_node, node, depth, nested)?;

        if nested {
            // Synthesised closure body; bind `name` locally to a closure.
            let span = self.span_of(node);
            let make = Expr::MakeClosure {
                fn_name: name.clone(),
                captures,
                span: span.clone(),
            };
            self.synthesised.push(lowered_fn);
            // The name becomes a local of the enclosing frame so later
            // references (`return inner;`) resolve and so a sibling can
            // call it.  First sighting binds; a redeclare re-assigns.
            let stmt = if self.is_current_local(&name) {
                self.features_used.add(Feature::MutableBindings);
                Stmt::Assign {
                    name,
                    scope: Scope::Local,
                    value: make,
                    span,
                }
            } else {
                self.declare_local(&name);
                Stmt::LetStarBinding {
                    name,
                    sir_type: None,
                    value: make,
                    span,
                }
            };
            Ok(Lowered::Stmt(Box::new(stmt)))
        } else {
            // Top-level user function: it lives in the module function
            // table, not in `main`'s body.  It contributes no statement and
            // no tail value — emit a nil expression that the block builder
            // discards (a pure literal is superseded by any real value, and
            // dropped at the end of the block otherwise).
            self.user_functions.push(lowered_fn);
            Ok(Lowered::Expr(Expr::NilLit {
                span: self.span_of(node),
            }))
        }
    }

    /// Lower an `arrow_function` to a [`MakeClosure`] over a synthesised
    /// `__lambda_<N>` `Function`.
    ///
    /// CST: `[arrow_parameters, Name("=>"), concise_body]`.  The
    /// `concise_body` is either an expression (1 child) — the closure body
    /// is a `Block` with no statements and that expression as its value —
    /// or a `{ … }` block (3 children) following the same tail-`return`
    /// rule as a `function` body.
    fn lower_arrow_function(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, JsLowerError> {
        self.check_stmt_depth(node, depth)?;
        let span = self.span_of(node);
        let params = self.arrow_param_specs(node)?;
        let concise = child_node_named(node, "concise_body")
            .ok_or_else(|| self.unsupported(node, "arrow_function (no body)"))?;

        let fn_name = self.fresh_lambda_name();
        let (lowered_fn, captures) =
            self.lower_arrow_callable(&fn_name, &params, concise, node, depth)?;
        self.synthesised.push(lowered_fn);

        Ok(Expr::MakeClosure {
            fn_name,
            captures,
            span,
        })
    }

    /// Lower a `call_expression` (`callee(args)`).
    ///
    /// CST: `[callee, arguments]`.  Dispatch on the callee:
    ///   * a bare identifier that names a module `function` → `DirectCall`;
    ///   * `console.log(x)` → `BuiltinCall("print", [x])`;
    ///   * any *other* dotted member call `recv.method(a, b)` → the SIR
    ///     **method-dispatch convention**
    ///     `BuiltinCall("__method__", [recv, StrLit("method"), a, b])`
    ///     (C3 — collection methods; see below);
    ///   * a bare identifier resolving to a closure *value* (local / param /
    ///     capture) → `IndirectCall` on that value.
    ///
    /// ## Member-method calls → `__method__` dispatch (C3)
    ///
    /// The narrow waist for a method call already exists and needs **no new
    /// IR node**: every frontend packs `recv.meth(arg1, arg2)` as
    ///
    /// ```text
    /// BuiltinCall {
    ///     name:    "__method__",
    ///     args:    [recv_expr, StrLit("meth"), arg1_expr, arg2_expr, …],
    ///     effects, span,
    /// }
    /// ```
    ///
    /// with the **receiver at `args[0]`**, the **method name always a
    /// `StrLit` at `args[1]`**, and the call arguments following.  This is
    /// exactly how the Ruby frontend lowers `recv.meth(…)`
    /// (`ruby-to-semantic-ir`'s `fold_one_dot_call`); a backend routes the
    /// envelope to its OOP runtime (`__SirOop.callMethod` / the JS runtime's
    /// method dispatcher), so `arr.map`/`arr.push`/`str.toUpperCase()` run
    /// end-to-end without any per-method backend change.
    ///
    /// The synthetic `StrLit` method name is the reason we declare
    /// [`Feature::Strings`] — the SIR validator observes `Strings` for every
    /// `StrLit`, so the manifest must match.  We deliberately do **not**
    /// invent a `MethodDispatch` feature here: `__method__` is not gated by a
    /// dedicated flag in v0, and `Strings` is what makes validation pass.
    ///
    /// A callback argument (`arr.map(x => x * 2)`) lowers through the
    /// crate's existing arrow/closure path, so it arrives as a
    /// [`MakeClosure`](Expr::MakeClosure) in the argument list — the JS
    /// higher-order convention (pass a callable, no trailing-block syntax).
    ///
    /// `console.log(...)` keeps its dedicated `print` lowering (below); a
    /// *computed* member call (`obj[expr](…)`) stays deferred.
    fn lower_call_expression(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, JsLowerError> {
        let span = self.span_of(node);

        // ## The flat call chain (learned by probing the parser)
        //
        // A chained call `xs.filter(f).map(g)` is **flat** in the CST: the
        // parser emits a *single* `call_expression` whose children are
        //
        // ```text
        // [ callee, arguments, (Dot, Name, arguments)* ]
        // ```
        //
        // — e.g. `xs.filter(f).map(g)` →
        // `[member_expr(xs.filter), args(f), ., map, args(g)]`.  The first
        // `[callee, arguments]` pair is the base call; each trailing
        // `.name(args)` triple is a further method invocation whose receiver
        // is the accumulated call so far.  We therefore lower the base call,
        // then fold each `.name(args)` step into a `__method__` dispatch with
        // the running expression as `args[0]`.  The fold is iterative (a
        // long chain costs no native stack); every lowered argument still
        // consumes the `MAX_EXPR_DEPTH` budget through `lower_arguments`.
        let children = &node.children;
        let callee = children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .ok_or_else(|| self.unsupported(node, "call_expression (no callee)"))?;
        // The first `arguments` node (and its child index) — the base call's
        // argument list.
        let first_args_idx = children
            .iter()
            .position(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "arguments"))
            .ok_or_else(|| self.unsupported(node, "call_expression (no arguments)"))?;
        let base_args_node = match &children[first_args_idx] {
            ASTNodeOrToken::Node(n) => n,
            ASTNodeOrToken::Token(_) => unreachable!("position found an arguments Node"),
        };
        let base_args = self.lower_arguments(base_args_node, depth)?;

        // Lower the *base* call segment (`callee(base_args)`).
        let mut acc = self.lower_base_call(callee, base_args, span.clone())?;

        // Fold each trailing `.name(args)` method step into a `__method__`
        // dispatch on the running receiver.
        let mut i = first_args_idx + 1;
        while i < children.len() {
            // Expect `Dot`, `Name`, `arguments`.
            let is_dot = matches!(&children[i], ASTNodeOrToken::Token(t) if t.value == ".");
            if !is_dot {
                // A computed step (`…[expr](…)`) or other trailing access is
                // not a named method → deferred.
                return Err(JsLowerError {
                    message: "computed / non-dot call chain step is deferred".to_string(),
                    line: span.start_line,
                    column: span.start_col,
                });
            }
            let method_tok = match children.get(i + 1) {
                Some(ASTNodeOrToken::Token(t)) if matches!(t.type_, TokenType::Name) => t,
                _ => return Err(self.unsupported(node, "call chain (non-name method)")),
            };
            let args_node = match children.get(i + 2) {
                Some(ASTNodeOrToken::Node(n)) if n.rule_name == "arguments" => n,
                _ => return Err(self.unsupported(node, "call chain (method without arguments)")),
            };
            let step_args = self.lower_arguments(args_node, depth)?;
            let name_span = self.span_of_token(method_tok);
            acc = self.make_method_dispatch(
                acc,
                method_tok.value.clone(),
                name_span,
                step_args,
                span.clone(),
            )?;
            i += 3;
        }

        Ok(acc)
    }

    /// Lower the *base* segment of a call chain — `callee(args)` with the
    /// callee already lowered from its CST node.  Dispatches exactly as the
    /// original single-call lowering did:
    ///   * `console.log(...)` → `BuiltinCall("print", …)` (kept intact);
    ///   * any *other* dotted member callee `recv.method` → `__method__`
    ///     dispatch (C3);
    ///   * a bare module-function name → `DirectCall`;
    ///   * a bare value name (a closure handle) → `IndirectCall`.
    ///
    /// A computed / non-identifier callee stays deferred.
    fn lower_base_call(
        &mut self,
        callee: &GrammarASTNode,
        arg_exprs: Vec<Expr>,
        span: Span,
    ) -> Result<Expr, JsLowerError> {
        // `console.log(...)` → builtin print.  Detect the two-segment
        // member callee `member_expression[ console, ., log ]` and keep its
        // dedicated lowering intact (do not route it through `__method__`).
        if let Some((obj, method)) = member_callee_parts(callee) {
            if obj == "console" && method == "log" {
                // `print` may print; mark the effect so backends emit it.
                return Ok(Expr::BuiltinCall {
                    name: "print".to_string(),
                    args: arg_exprs,
                    effects: EffectSet::PURE.with(Effect::MayPrint),
                    span,
                });
            }
        }

        // Any *other* dotted member call `recv.method(args…)` → `__method__`
        // dispatch (C3).  We peel the callee to its branching
        // `member_expression` and require the **final** access to be a
        // `.name` (a dot method); the receiver is everything before it,
        // lowered through the ordinary member-read path (so `obj.inner.push`
        // folds the `obj.inner` map read as the receiver of `.push`).  A
        // computed final access (`obj[expr](…)`) is not a named method and
        // stays deferred.
        if let Some((method_tok, last_access)) = dotted_method_callee(callee) {
            let name_span = self.span_of_token(method_tok);
            let method = method_tok.value.clone();
            let branch = peel_to_branch(callee);
            let recv = self.lower_member_prefix(branch, last_access, &span)?;
            return self.make_method_dispatch(recv, method, name_span, arg_exprs, span);
        }

        // A bare-identifier callee.
        if let Some(tok) = single_leaf_token(callee) {
            if matches!(tok.type_, TokenType::Name) {
                let fname = tok.value.clone();
                // Known module function → DirectCall (recursion / forward
                // reference / mutual recursion all resolve here).
                if self.function_names.contains(&fname) {
                    return Ok(Expr::DirectCall {
                        fn_name: fname,
                        args: arg_exprs,
                        effects: EffectSet::PURE,
                        span,
                    });
                }
                // Otherwise it must resolve to a *value* (a closure handle
                // bound to a local / param / capture) → IndirectCall.
                let target = self.resolve_name(&fname, span.clone(), tok.line, tok.column)?;
                self.features_used.add(Feature::Closures);
                return Ok(Expr::IndirectCall {
                    target: Box::new(target),
                    args: arg_exprs,
                    effects: EffectSet::PURE,
                    span,
                });
            }
        }

        // A computed / non-identifier callee (`(f())(x)`, `xs[0](y)`, …) is
        // deferred — those entail member access / collections (M5).
        Err(JsLowerError {
            message: "call of a non-identifier callee is deferred past M4".to_string(),
            line: span.start_line,
            column: span.start_col,
        })
    }

    /// Build the SIR method-dispatch envelope
    /// `BuiltinCall("__method__", [receiver, StrLit(method), args…])` and
    /// declare [`Feature::Strings`] for the synthetic method-name `StrLit`.
    /// This is the single narrow-waist convention every backend routes to
    /// its OOP runtime (see [`lower_call_expression`](Self::lower_call_expression)).
    ///
    /// # Security — reject dangerous method names at lowering time
    ///
    /// The method name here is *attacker-controlled*: it is a source-level
    /// identifier from an untrusted input program, and it flows verbatim into
    /// a `StrLit` that every backend uses to perform a dynamic property
    /// lookup on the receiver (`recv[name]`).  A handful of JavaScript member
    /// names are *reflective gadgets* that turn that lookup into code
    /// execution — most dangerously `constructor`, which on any function
    /// yields the `Function` constructor and lets a translated program build
    /// and run arbitrary code (`id.constructor("…evil…")` → RCE).  `apply`,
    /// `call`, `bind`, `__proto__`, `prototype`, and the
    /// `__define/lookup*etter__` pair are the other prototype-chain escape
    /// hatches.
    ///
    /// We therefore refuse to *lower* any of those names: the dangerous
    /// `StrLit` never enters SIR, so every backend — not just the JS runtime,
    /// which carries its own allowlist as the tight gate — is protected at the
    /// source.  The frontend stays otherwise permissive (unknown-but-harmless
    /// method names still lower fine); this denylist blocks only the names
    /// that are *never* legitimate collection/String/Number methods.
    fn make_method_dispatch(
        &mut self,
        receiver: Expr,
        method: String,
        name_span: Span,
        args: Vec<Expr>,
        span: Span,
    ) -> Result<Expr, JsLowerError> {
        if is_dangerous_method_name(&method) {
            return Err(JsLowerError {
                message: format!(
                    "method `{method}` is a reflective JavaScript gadget \
                     (a prototype/constructor escape hatch) and is refused at \
                     lowering time to prevent arbitrary-code execution"
                ),
                line: name_span.start_line,
                column: name_span.start_col,
            });
        }
        self.features_used.add(Feature::Strings);
        let mut full_args = Vec::with_capacity(args.len() + 2);
        full_args.push(receiver);
        full_args.push(Expr::StrLit {
            value: method,
            span: name_span,
        });
        full_args.extend(args);
        Ok(Expr::BuiltinCall {
            name: "__method__".to_string(),
            args: full_args,
            effects: EffectSet::PURE,
            span,
        })
    }

    /// Lower the `arguments` node (`( argument_list? )`) into a `Vec<Expr>`.
    fn lower_arguments(
        &mut self,
        args_node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Vec<Expr>, JsLowerError> {
        // No `argument_list` child → a zero-argument call `f()`.
        let list = match child_node_named(args_node, "argument_list") {
            Some(l) => l,
            None => return Ok(Vec::new()),
        };
        let mut out = Vec::new();
        for child in &list.children {
            if let ASTNodeOrToken::Node(n) = child {
                out.push(self.lower_expression(n, depth + 1)?);
            }
            // Comma tokens are skipped.
        }
        Ok(out)
    }

    /// Shared core for lowering a *named* callable (top-level or nested
    /// `function` declaration) to a `Function` plus its `MakeClosure`
    /// captures (empty for a top-level function).
    ///
    /// Pushes a fresh scope frame (params + closure-ness), lowers the body
    /// with the tail-`return` rule, pops the frame, and returns the
    /// synthesised `Function` and the captures discovered while lowering.
    fn lower_callable_body(
        &mut self,
        name: &str,
        params: &[FormalParamSpec],
        body_node: Option<&GrammarASTNode>,
        decl_node: &GrammarASTNode,
        depth: usize,
        is_closure: bool,
    ) -> Result<(Function, Vec<CaptureValue>), JsLowerError> {
        let span = self.span_of(decl_node);
        self.scopes.push(FnScope::new(
            params.iter().map(|p| p.name.clone()).collect(),
            is_closure,
        ));

        // Lower the default initializers **inside** the freshly pushed frame
        // (so a default may reference earlier params), then the body.  We do
        // defaults before the body, but both see the same param frame.
        let params_result = self.lower_param_specs(params, depth, &span);

        // Lower the function body's statement sequence with the tail-return
        // rule.  An absent / empty body yields a nil-valued block.
        let body_children: &[ASTNodeOrToken] =
            body_node.map(|b| b.children.as_slice()).unwrap_or(&[]);
        let body_result = self.lower_function_body(body_children, span.clone(), depth + 1);

        let frame = self.scopes.pop().expect("pushed a frame");
        let lowered_params = params_result?;
        let body = body_result?;

        let function = Function {
            name: name.to_string(),
            params: lowered_params,
            return_type: None,
            captures: frame
                .captures
                .iter()
                .map(|n| Capture {
                    name: n.clone(),
                    sir_type: None,
                })
                .collect(),
            body,
            // A closure body may allocate; a plain top-level function is
            // pure unless its body says otherwise.  We keep it simple and
            // mark closures `MayAllocate` (matching the twig reference).
            effects: if is_closure {
                EffectSet::PURE.with(Effect::MayAllocate)
            } else {
                EffectSet::PURE
            },
            metadata: Metadata::new(),
            span: span.clone(),
        };
        if is_closure {
            self.features_used.add(Feature::Closures);
        }
        Ok((function, frame.capture_values))
    }

    /// Like [`lower_callable_body`] but for an arrow's `concise_body`,
    /// which may be a bare expression (no statements, expression is the
    /// tail value) or a `{ … }` block.
    fn lower_arrow_callable(
        &mut self,
        name: &str,
        params: &[FormalParamSpec],
        concise: &GrammarASTNode,
        arrow_node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Function, Vec<CaptureValue>), JsLowerError> {
        let span = self.span_of(arrow_node);
        self.scopes.push(FnScope::new(
            params.iter().map(|p| p.name.clone()).collect(),
            true,
        ));

        // Lower default initializers inside the frame (may reference earlier
        // params), then the concise body.
        let params_result = self.lower_param_specs(params, depth, &span);

        // Expression body: `concise_body` has a single expression child and
        // no `{`.  Block body: it wraps a `function_body` between braces.
        let result = (|| -> Result<Block, JsLowerError> {
            if let Some(fb) = child_node_named(concise, "function_body") {
                // Block-bodied arrow → same tail-return rule as a function.
                self.lower_function_body(&fb.children, span.clone(), depth + 1)
            } else {
                // Expression-bodied arrow → a statement-free block whose
                // value is the expression.
                let expr_node = concise
                    .children
                    .iter()
                    .find_map(|c| match c {
                        ASTNodeOrToken::Node(n) => Some(n),
                        ASTNodeOrToken::Token(_) => None,
                    })
                    .ok_or_else(|| self.unsupported(concise, "arrow concise body"))?;
                let value = self.lower_expression(expr_node, depth + 1)?;
                Ok(Block {
                    stmts: Vec::new(),
                    value,
                    span: span.clone(),
                })
            }
        })();

        let frame = self.scopes.pop().expect("pushed a frame");
        let lowered_params = params_result?;
        let body = result?;

        let function = Function {
            name: name.to_string(),
            params: lowered_params,
            return_type: None,
            captures: frame
                .captures
                .iter()
                .map(|n| Capture {
                    name: n.clone(),
                    sir_type: None,
                })
                .collect(),
            body,
            effects: EffectSet::PURE.with(Effect::MayAllocate),
            metadata: Metadata::new(),
            span,
        };
        self.features_used.add(Feature::Closures);
        Ok((function, frame.capture_values))
    }

    /// Lower a run of [`FormalParamSpec`]s into [`Param`]s, lowering each
    /// default initializer with the function frame already on the scope
    /// stack.
    ///
    /// **Call-time & param-scope semantics.** In JavaScript a default is
    /// evaluated at call time and may read *earlier* parameters
    /// (`function f(a, b = a + 1)`).  Because the whole param frame is
    /// already pushed by the caller before this runs, lowering the default
    /// as an ordinary expression makes a reference to `a` resolve as a
    /// `Scope::Param` `VarRef` — exactly the SIR model (the IR validator and
    /// every backend treat `Param.default` as evaluated in param scope).
    ///
    /// **Depth bound.** The initializer is an ordinary expression node, so
    /// it is lowered through [`lower_expression`], which is bounded by
    /// [`MAX_EXPR_DEPTH`] (a pathologically deep default becomes a positioned
    /// error, never a native stack overflow — CWE-674).  We start it at
    /// `depth + 1` so a default nested inside a deep function still counts
    /// against the same budget.
    ///
    /// A parameter that carries a default makes the module observe
    /// [`Feature::DefaultParams`]; the validator independently observes the
    /// same feature off the `default = Some(_)` slot, so the declared and
    /// observed manifests agree.
    fn lower_param_specs(
        &mut self,
        params: &[FormalParamSpec],
        depth: usize,
        span: &Span,
    ) -> Result<Vec<Param>, JsLowerError> {
        let mut out = Vec::with_capacity(params.len());
        for spec in params {
            let default = match spec.default {
                None => None,
                Some(expr_node) => {
                    self.features_used.add(Feature::DefaultParams);
                    Some(Box::new(self.lower_expression(expr_node, depth + 1)?))
                }
            };
            out.push(Param {
                name: spec.name.clone(),
                sir_type: None,
                kind: ParamKind::Required,
                default,
                span: span.clone(),
            });
        }
        Ok(out)
    }

    /// Lower a function/closure body's statement sequence into a [`Block`],
    /// applying the **tail-position `return`** rule (SIR19 "Return").
    ///
    /// A `return` is in *tail position* iff it is the body's last statement
    /// — or, recursively, the last statement of a branch of a tail-position
    /// `if`.  This admits the natural guard-via-`if`/`else` recursion shape
    /// (`if (base) { return b; } else { return rec; }` as the body's last
    /// statement) without an early-return node, while still rejecting a
    /// genuine early `return` (one followed by more statements).
    ///
    /// - tail `return expr;` → `block.value = expr`;
    /// - tail bare `return;` → `block.value = NilLit`;
    /// - no `return` → `block.value = NilLit` (or the trailing bare
    ///   expression's value, like the top-level block builder);
    /// - a `return` not in tail position → an "early return"
    ///   [`JsLowerError`].
    fn lower_function_body(
        &mut self,
        children: &[ASTNodeOrToken],
        block_span: Span,
        depth: usize,
    ) -> Result<Block, JsLowerError> {
        // Split into the leading statements and the final statement-bearing
        // node (the only position where a `return` / returning `if` is
        // allowed).
        let node_idxs: Vec<usize> = children
            .iter()
            .enumerate()
            .filter_map(|(i, c)| matches!(c, ASTNodeOrToken::Node(_)).then_some(i))
            .collect();
        let last_idx = node_idxs.last().copied();

        let mut stmts: Vec<Stmt> = Vec::new();
        let mut tail: Option<Expr> = None;

        for (i, child) in children.iter().enumerate() {
            let n = match child {
                ASTNodeOrToken::Node(n) => n,
                ASTNodeOrToken::Token(_) => continue,
            };
            let is_last = Some(i) == last_idx;
            // A non-final statement must not contain a tail return; the
            // only place a `return` is legal is the final node (handled
            // below).  `reject_returns` walks for any stray `return`.
            if !is_last {
                self.reject_returns(n, depth)?;
                match self.lower_source_element(n, depth)? {
                    Lowered::Stmt(s) => {
                        if let Some(prev) = tail.take() {
                            let span = prev.span().clone();
                            stmts.push(Stmt::ExprStmt { expr: prev, span });
                        }
                        stmts.push(*s);
                    }
                    Lowered::Expr(e) => tail = Some(e),
                }
                continue;
            }

            // Final node: lower it in tail position.  A `return` / returning
            // `if` / nested `block` becomes the block value; an ordinary
            // trailing statement is pushed (nil tail), an ordinary trailing
            // expression is the value.
            if let Some(prev) = tail.take() {
                let span = prev.span().clone();
                stmts.push(Stmt::ExprStmt { expr: prev, span });
            }
            let concrete = concrete_statement(n);
            match concrete.rule_name.as_str() {
                "return_statement" => {
                    tail = Some(self.lower_return_value(concrete)?);
                }
                "if_statement" => {
                    tail = Some(self.lower_tail_if(concrete, depth)?);
                }
                "block" => {
                    self.check_stmt_depth(concrete, depth)?;
                    let span = self.span_of(concrete);
                    let saved = self.cur().locals.clone();
                    let nested = self.lower_function_body(&concrete.children, span, depth + 1);
                    self.cur().locals = saved;
                    tail = Some(Expr::Block(Box::new(nested?)));
                }
                _ => match self.lower_source_element(n, depth)? {
                    Lowered::Stmt(s) => stmts.push(*s),
                    Lowered::Expr(e) => tail = Some(e),
                },
            }
        }

        let value = tail.unwrap_or(Expr::NilLit {
            span: block_span.clone(),
        });
        Ok(Block {
            stmts,
            value,
            span: block_span,
        })
    }

    /// Lower a *tail-position* `if_statement` to an [`Expr::If`] whose
    /// branch values come from recursively tail-lowering each branch.  A
    /// missing `else` yields a nil-valued else block.
    fn lower_tail_if(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, JsLowerError> {
        self.check_stmt_depth(node, depth)?;
        let span = self.span_of(node);
        let nodes = child_nodes(node);
        let cond_node = nodes
            .first()
            .ok_or_else(|| self.unsupported(node, "if (no condition)"))?;
        let then_node = nodes
            .get(1)
            .ok_or_else(|| self.unsupported(node, "if (no then branch)"))?;
        let cond = self.lower_expression(cond_node, 0)?;
        let then_branch = self.tail_branch_block(then_node, depth)?;
        let else_branch = match nodes.get(2) {
            Some(else_node) => self.tail_branch_block(else_node, depth)?,
            None => Block {
                stmts: Vec::new(),
                value: Expr::NilLit { span: span.clone() },
                span: span.clone(),
            },
        };
        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
            span,
        })
    }

    /// Lower an `if`-branch statement (a `{ … }` block or a single
    /// statement) as a *tail* body — so a `return` inside the branch
    /// becomes the branch block's value.  Names bound inside are
    /// block-scoped (snapshot/restore the current frame's locals).
    fn tail_branch_block(
        &mut self,
        branch_stmt: &GrammarASTNode,
        depth: usize,
    ) -> Result<Block, JsLowerError> {
        let saved = self.cur().locals.clone();
        let inner = concrete_statement(branch_stmt);
        let span = self.span_of(branch_stmt);
        let result = if inner.rule_name == "block" {
            self.check_stmt_depth(inner, depth)
                .and_then(|()| self.lower_function_body(&inner.children, span, depth + 1))
        } else {
            // A single (unbraced) branch statement, e.g. `if (c) return 1;`.
            // Wrap the one statement as a one-item tail body.
            self.lower_function_body(
                std::slice::from_ref(&ASTNodeOrToken::Node(branch_stmt.clone())),
                span,
                depth + 1,
            )
        };
        self.cur().locals = saved;
        result
    }

    /// Reject any `return_statement` anywhere inside `node` (used for the
    /// non-final statements of a function body: a `return` there is a
    /// genuine early return, unsupported in v0).
    ///
    /// We do **not** descend into a nested `function_declaration` or
    /// `arrow_function`: a `return` inside *those* belongs to the nested
    /// callable (it is checked when that callable's own body is lowered),
    /// not to the enclosing function.
    ///
    /// This is a recursive CST walk run *before* the depth-guarded lowering,
    /// so it carries its **own** [`MAX_STMT_DEPTH`] bound: a pathologically
    /// deep statement subtree turns into a positioned error rather than a
    /// native stack overflow (CWE-674).
    fn reject_returns(&self, node: &GrammarASTNode, depth: usize) -> Result<(), JsLowerError> {
        if depth > MAX_STMT_DEPTH {
            return Err(self.too_deep_error(node));
        }
        if node.rule_name == "return_statement" {
            return Err(self.early_return_error(node));
        }
        if matches!(
            node.rule_name.as_str(),
            "function_declaration" | "arrow_function"
        ) {
            return Ok(());
        }
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                self.reject_returns(n, depth + 1)?;
            }
        }
        Ok(())
    }

    /// The positioned "too deeply nested" error shared by the pre-lowering
    /// recursive CST walks ([`reject_returns`](Self::reject_returns) and
    /// [`collect_function_names`]).  Mirrors the message
    /// [`check_stmt_depth`](Self::check_stmt_depth) emits.
    fn too_deep_error(&self, node: &GrammarASTNode) -> JsLowerError {
        JsLowerError {
            message: format!("input nests deeper than the supported limit ({MAX_STMT_DEPTH})"),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        }
    }

    /// Lower the value of a tail `return_statement`: its `expression` child,
    /// or `NilLit` for a bare `return;`.
    fn lower_return_value(&mut self, ret: &GrammarASTNode) -> Result<Expr, JsLowerError> {
        match child_node_named(ret, "expression") {
            Some(e) => self.lower_expression(e, 0),
            None => Ok(Expr::NilLit {
                span: self.span_of(ret),
            }),
        }
    }

    /// Extract a `function_declaration`'s formal parameters as
    /// [`FormalParamSpec`]s (name + optional default-initialiser CST node).
    /// Rest `...r` / destructuring `{a}`/`[a]` stay deferred.
    fn formal_param_specs<'a>(
        &self,
        decl_node: &'a GrammarASTNode,
    ) -> Result<Vec<FormalParamSpec<'a>>, JsLowerError> {
        match child_node_named(decl_node, "formal_parameters") {
            None => Ok(Vec::new()), // zero-parameter function.
            Some(fp) => self.param_specs(fp, decl_node),
        }
    }

    /// Extract an `arrow_function`'s parameters as [`FormalParamSpec`]s.  The
    /// `arrow_parameters` node is either `[(, formal_parameters?, )]` or a
    /// bare `[Name]` (for `a => …`, which can carry no default).
    fn arrow_param_specs<'a>(
        &self,
        arrow_node: &'a GrammarASTNode,
    ) -> Result<Vec<FormalParamSpec<'a>>, JsLowerError> {
        let ap = child_node_named(arrow_node, "arrow_parameters")
            .ok_or_else(|| self.unsupported(arrow_node, "arrow_function (no parameters)"))?;
        // Bare single-identifier form: `a => …` (no parens, no default).
        if let Some(tok) = ap.token() {
            if matches!(tok.type_, TokenType::Name) {
                return Ok(vec![FormalParamSpec {
                    name: tok.value.clone(),
                    default: None,
                }]);
            }
        }
        match child_node_named(ap, "formal_parameters") {
            None => Ok(Vec::new()), // `() => …`.
            Some(fp) => self.param_specs(fp, arrow_node),
        }
    }

    /// Extract positional parameter specs from a `formal_parameters` node.
    ///
    /// Two `formal_parameter` shapes are accepted (see the probed CST in the
    /// module header):
    ///
    /// - `[Name]` — a plain positional parameter; `default: None`.
    /// - `[Name, "=", <assignment_expression>]` — a **default** parameter;
    ///   `default: Some(<expr node>)`.  The initializer is an ordinary
    ///   expression node, lowered later **inside the function frame** (so it
    ///   may reference earlier params, matching JS call-time semantics) by
    ///   the depth-bounded [`lower_expression`].
    ///
    /// Any other shape — rest `...r`, a destructuring pattern `{a}`/`[a]`
    /// (which introduces a child *node* in the binding position rather than a
    /// leading `Name` token) — stays deferred.
    fn param_specs<'a>(
        &self,
        fp: &'a GrammarASTNode,
        ctx_node: &GrammarASTNode,
    ) -> Result<Vec<FormalParamSpec<'a>>, JsLowerError> {
        let mut specs = Vec::new();
        for param in children_nodes_named(fp, "formal_parameter") {
            // The binding must be a *single leading `Name` token* (rejecting
            // destructuring, whose binding is a node, and rest, whose first
            // token is `...`).
            let name = match param.children.first() {
                Some(ASTNodeOrToken::Token(t)) if matches!(t.type_, TokenType::Name) => {
                    t.value.clone()
                }
                _ => return Err(self.deferred_param(ctx_node)),
            };
            // Classify the remaining children:
            //   - none                      → plain param (`a`)
            //   - [Equals, <expr-node>]     → default param (`a = expr`)
            //   - anything else             → deferred
            let default = match &param.children[1..] {
                [] => None,
                [ASTNodeOrToken::Token(eq), ASTNodeOrToken::Node(expr)] if eq.value == "=" => {
                    Some(expr)
                }
                _ => return Err(self.deferred_param(ctx_node)),
            };
            specs.push(FormalParamSpec { name, default });
        }
        Ok(specs)
    }

    /// The positioned "deferred parameter form" error (rest / destructuring).
    fn deferred_param(&self, ctx_node: &GrammarASTNode) -> JsLowerError {
        JsLowerError {
            message: "rest / destructuring parameters are deferred past M4 \
                      (simple positional and default params supported)"
                .to_string(),
            line: ctx_node.start_line.unwrap_or(0),
            column: ctx_node.start_column.unwrap_or(0),
        }
    }

    /// A fresh synthesised arrow-function name (`__lambda_<N>`).
    fn fresh_lambda_name(&mut self) -> String {
        let name = format!("__lambda_{}", self.lambda_counter);
        self.lambda_counter += 1;
        name
    }

    /// Names of the user's *top-level* `function` declarations, in the
    /// order they were lowered (used to build the module export list).
    fn top_level_function_names_in_order(&self) -> Vec<String> {
        self.user_functions.iter().map(|f| f.name.clone()).collect()
    }

    // -----------------------------------------------------------------------
    // Declarations: let / const / var  →  binding or assignment statement
    // -----------------------------------------------------------------------

    /// Lower a `lexical_declaration` (`let`/`const` form).
    ///
    /// Shape (from the probe):
    /// `lexical_declaration[ Keyword(let|const), binding_list, Semicolon ]`
    /// where `binding_list` holds one or more `lexical_binding`s, each
    /// `lexical_binding[ Name, Equals, assignment_expression ]`.
    ///
    /// M2 supports the common single-binding case; a comma-separated
    /// multi-binding list (`let a = 1, b = 2;`) is rejected (deferred) so
    /// the lossy behaviour is explicit rather than silent.
    fn lower_lexical_declaration(&mut self, node: &GrammarASTNode) -> Result<Stmt, JsLowerError> {
        let list = child_node_named(node, "binding_list")
            .ok_or_else(|| self.unsupported(node, "lexical_declaration (no binding_list)"))?;
        let bindings = children_nodes_named(list, "lexical_binding");
        self.lower_single_binding(node, &bindings, "lexical_binding")
    }

    /// Lower a `variable_statement` (`var` form).
    ///
    /// Shape: `variable_statement[ Keyword(var), variable_declaration_list,
    /// Semicolon ]`, the list holding `variable_declaration`s each shaped
    /// `[ Name, Equals, assignment_expression ]`.  `var` hoisting is NOT
    /// modelled (SIR19 spec "`var` hoisting"): we emit the binding at its
    /// source position, exactly like `let`.
    fn lower_variable_statement(&mut self, node: &GrammarASTNode) -> Result<Stmt, JsLowerError> {
        let list = child_node_named(node, "variable_declaration_list")
            .ok_or_else(|| self.unsupported(node, "variable_statement (no declaration list)"))?;
        let decls = children_nodes_named(list, "variable_declaration");
        self.lower_single_binding(node, &decls, "variable_declaration")
    }

    /// Shared core for `let`/`const`/`var`: lower exactly one binding of
    /// the form `[ Name, Equals, <init expr> ]`.
    fn lower_single_binding(
        &mut self,
        decl_node: &GrammarASTNode,
        bindings: &[&GrammarASTNode],
        what: &str,
    ) -> Result<Stmt, JsLowerError> {
        if bindings.len() != 1 {
            return Err(JsLowerError {
                message: format!(
                    "multi-binding `{what}` (`let a = 1, b = 2;`) is deferred past M2"
                ),
                line: decl_node.start_line.unwrap_or(0),
                column: decl_node.start_column.unwrap_or(0),
            });
        }
        let binding = bindings[0];

        // The binding name is the first `Name` token child.
        let name_tok = binding
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
                _ => None,
            })
            .ok_or_else(|| self.unsupported(binding, &format!("{what} (destructuring/no name)")))?;
        let name = name_tok.value.clone();
        let span = self.span_of(decl_node);

        // The initialiser is the single expression-shaped child node.
        // A declaration with no initialiser (`let x;`) is deferred — the
        // IR has no "uninitialised binding" and inventing a `NilLit`
        // would mask the source's intent.
        let init_node = binding
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .ok_or_else(|| JsLowerError {
                message: format!(
                    "uninitialised binding `{name}` (`{what}` with no `= …`) is deferred past M2"
                ),
                line: span.start_line,
                column: span.start_col,
            })?;
        let value = self.lower_expression(init_node, 0)?;

        // First sighting → `let*` binding; a re-declaration of an
        // already-declared name (legal for `var`, a redeclare error for
        // `let`/`const` in real JS but we don't enforce that) becomes an
        // `Assign` to keep validation honest.
        if self.is_current_local(&name) {
            self.features_used.add(Feature::MutableBindings);
            Ok(Stmt::Assign {
                name,
                scope: Scope::Local,
                value,
                span,
            })
        } else {
            self.declare_local(&name);
            // Sequential `let*` (not parallel `let`): see module docs.
            Ok(Stmt::LetStarBinding {
                name,
                sir_type: None,
                value,
                span,
            })
        }
    }

    // -----------------------------------------------------------------------
    // expression_statement  →  bare expression OR re-assignment statement
    // -----------------------------------------------------------------------

    /// Lower an `expression_statement` (`<expression> ;`).
    ///
    /// Two cases distinguished by the inner expression's shape:
    ///   * a top-level `assignment_expression` with an `=` operator
    ///     (`x = …`) becomes a binding/assignment **statement**;
    ///   * anything else is a value-producing expression returned as
    ///     [`Lowered::Expr`].
    fn lower_expression_statement(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Lowered, JsLowerError> {
        // Children are the `expression` node followed by a `Semicolon`.
        let expr_node = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .ok_or_else(|| JsLowerError {
                message: "empty expression statement".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;

        // Peek for a top-level assignment: descend the single-child
        // spine until something branches; if that something is an
        // `assignment_expression` with three children (`lhs op rhs`),
        // it's a statement-level assignment.
        let branch = peel_to_branch(expr_node);
        if branch.rule_name == "assignment_expression" && branch.children.len() == 3 {
            return self
                .lower_assignment(branch)
                .map(|s| Lowered::Stmt(Box::new(s)));
        }
        self.lower_expression(expr_node, 0).map(Lowered::Expr)
    }

    /// Lower a statement-level `assignment_expression` (`x = expr`,
    /// `xs[i] = expr`, `obj.prop = expr`).
    ///
    /// Shape: `assignment_expression[ left_hand_side_expression,
    /// assignment_operator, assignment_expression ]`.  Only the plain `=`
    /// operator is supported; compound assignment (`+=`, …) is deferred.  The
    /// target may be a bare identifier (→ `Assign`) **or** (M5) a member /
    /// subscript access (→ `SeqSet` / `MapSet`), disambiguated exactly as the
    /// read side ([`lower_member_expression`](Self::lower_member_expression)).
    fn lower_assignment(&mut self, node: &GrammarASTNode) -> Result<Stmt, JsLowerError> {
        let span = self.span_of(node);

        // children[0] = LHS target, children[1] = assignment_operator,
        // children[2] = RHS value.
        let lhs = match &node.children[0] {
            ASTNodeOrToken::Node(n) => n,
            ASTNodeOrToken::Token(_) => {
                return Err(self.unsupported(node, "assignment (token LHS)"))
            }
        };
        let op = &node.children[1];
        // Only the plain `=` operator is supported.
        let op_is_plain_eq = match op {
            ASTNodeOrToken::Node(n) => single_leaf_token(n)
                .map(|t| t.value == "=")
                .unwrap_or(false),
            ASTNodeOrToken::Token(t) => t.value == "=",
        };
        if !op_is_plain_eq {
            return Err(JsLowerError {
                message: "compound assignment (`+=`, `-=`, …) is deferred past M2".to_string(),
                line: span.start_line,
                column: span.start_col,
            });
        }

        // ── M5: member / subscript target → SeqSet / MapSet ─────────────
        // Peel the LHS spine; a *branching* `member_expression` is an
        // indexed/member assignment target (`xs[i] = v`, `obj.prop = v`).
        let lhs_branch = peel_to_branch(lhs);
        if lhs_branch.rule_name == "member_expression" {
            return self.lower_member_assignment(lhs_branch, node, span);
        }

        // The target must be a bare identifier.  Peel the LHS spine to
        // its leaf token; anything that branches (and isn't a member
        // access, handled above) is deferred.
        let target_tok = single_leaf_token(lhs_branch).ok_or_else(|| JsLowerError {
            message: "assignment to a non-identifier target is deferred".to_string(),
            line: span.start_line,
            column: span.start_col,
        })?;
        if !matches!(target_tok.type_, TokenType::Name) {
            return Err(self.unsupported(lhs, "assignment target (not a name)"));
        }
        let name = target_tok.value.clone();

        let rhs = match &node.children[2] {
            ASTNodeOrToken::Node(n) => n,
            ASTNodeOrToken::Token(_) => {
                return Err(self.unsupported(node, "assignment (token RHS)"))
            }
        };
        let value = self.lower_expression(rhs, 0)?;

        // Re-assignment to a name already in scope keeps its scope tag:
        //   * a current-frame local        → `Assign { scope: Local }`
        //   * the current frame's param    → `Assign { scope: Param }`
        //   * a captured outer variable    → `Assign { scope: Capture }`
        // First sighting of a never-declared name via bare `x = …` creates
        // a binding in the current frame (JS implicitly creates a binding
        // on assignment without a declarator).
        if let Some(scope) = self.assign_target_scope(&name, &span) {
            self.features_used.add(Feature::MutableBindings);
            Ok(Stmt::Assign {
                name,
                scope,
                value,
                span,
            })
        } else {
            self.declare_local(&name);
            Ok(Stmt::LetStarBinding {
                name,
                sir_type: None,
                value,
                span,
            })
        }
    }

    /// Lower an indexed / member assignment target (`xs[i] = v`,
    /// `obj.prop = v`, `obj["k"] = v`, and chained `grid[0][1] = v`) to a
    /// [`Stmt::SeqSet`] / [`Stmt::MapSet`].
    ///
    /// `member` is the branching `member_expression` LHS;
    /// `assign_node.children[2]` is the RHS value.  Like the read side, the
    /// chain is **flat** in the CST (`[receiver, access, access, …]`).  The
    /// *final* access is the assignment site; every access *before* it builds
    /// the receiver expression.  We split the children at the last access,
    /// lower the receiver prefix as an ordinary `member_expression` read (so
    /// `grid[0][1] = v` indexes `grid[0]` first), then emit the matching Set.
    ///
    /// The SeqSet-vs-MapSet choice mirrors the read side exactly: a dotted
    /// target is always a map key (`.length` is read-only — assignment is a
    /// resize the IR can't express, so it is deferred); a bracket target with
    /// a string-literal key is a map key, any other bracket key a sequence
    /// index.
    fn lower_member_assignment(
        &mut self,
        member: &GrammarASTNode,
        assign_node: &GrammarASTNode,
        span: Span,
    ) -> Result<Stmt, JsLowerError> {
        // The RHS value (children[2] of the assignment).
        let rhs = match &assign_node.children[2] {
            ASTNodeOrToken::Node(n) => n,
            ASTNodeOrToken::Token(_) => {
                return Err(self.unsupported(assign_node, "assignment (token RHS)"))
            }
        };
        let value = self.lower_expression(rhs, 0)?;

        // Find where the *final* access starts (a `.` or `[` token).  Earlier
        // children form the receiver prefix; the final access is the target.
        let last_access = member
            .children
            .iter()
            .rposition(
                |c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "." || t.value == "["),
            )
            .ok_or_else(|| self.unsupported(member, "assignment target (no access)"))?;

        // The receiver is the member_expression truncated to everything
        // before the final access; if that is a single node, lower it
        // directly, otherwise re-wrap it as a `member_expression` and lower
        // the prefix chain through the read path.
        let recv = self.lower_member_prefix(member, last_access, &span)?;

        match &member.children[last_access] {
            // ── final `.name` access → MapSet (string key) ─────────────
            ASTNodeOrToken::Token(t) if t.value == "." => {
                let prop = match member.children.get(last_access + 1) {
                    Some(ASTNodeOrToken::Token(p)) if matches!(p.type_, TokenType::Name) => {
                        p.value.clone()
                    }
                    _ => return Err(self.unsupported(member, "assignment to non-name property")),
                };
                if prop == "length" {
                    return Err(JsLowerError {
                        message: "assignment to `.length` is deferred past M5 (no resize node)"
                            .to_string(),
                        line: span.start_line,
                        column: span.start_col,
                    });
                }
                self.features_used.add(Feature::Maps);
                self.features_used.add(Feature::Strings);
                Ok(Stmt::MapSet {
                    map: recv,
                    key: Expr::StrLit {
                        value: prop,
                        span: span.clone(),
                    },
                    value,
                    span,
                })
            }
            // ── final `[index]` access → SeqSet / MapSet ───────────────
            ASTNodeOrToken::Token(t) if t.value == "[" => {
                let index_node = match member.children.get(last_access + 1) {
                    Some(ASTNodeOrToken::Node(n)) => n,
                    _ => return Err(self.unsupported(member, "subscript target (no index)")),
                };
                let index = self.lower_expression(index_node, 0)?;
                if matches!(index, Expr::StrLit { .. }) {
                    self.features_used.add(Feature::Maps);
                    Ok(Stmt::MapSet {
                        map: recv,
                        key: index,
                        value,
                        span,
                    })
                } else {
                    self.features_used.add(Feature::Sequences);
                    Ok(Stmt::SeqSet {
                        seq: recv,
                        index,
                        value,
                        span,
                    })
                }
            }
            _ => Err(self.unsupported(member, "assignment target (unsupported access form)")),
        }
    }

    /// Lower the *receiver prefix* of a flat `member_expression` assignment
    /// target: everything before the access at `last_access`.  If the prefix
    /// is just the bare receiver node (a simple `xs[i] = v` / `obj.p = v`),
    /// lower that node directly; otherwise (a chained `grid[0][1] = v`)
    /// re-wrap the prefix children as a `member_expression` and lower it
    /// through the read path so the inner accesses fold correctly.
    fn lower_member_prefix(
        &mut self,
        member: &GrammarASTNode,
        last_access: usize,
        _span: &Span,
    ) -> Result<Expr, JsLowerError> {
        let prefix = &member.children[..last_access];
        // Count access tokens in the prefix; zero means the prefix is the
        // bare receiver node (the common, single-access case).
        let prefix_accesses = prefix
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "." || t.value == "["))
            .count();
        let recv_node = prefix
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .ok_or_else(|| self.unsupported(member, "assignment target (no receiver)"))?;
        if prefix_accesses == 0 {
            self.lower_expression(recv_node, 0)
        } else {
            // Re-wrap the prefix as its own member_expression and lower it as
            // a read (folds the inner `grid[0]` of `grid[0][1] = v`).
            let inner = GrammarASTNode {
                rule_name: "member_expression".to_string(),
                children: prefix.to_vec(),
                start_line: member.start_line,
                start_column: member.start_column,
                end_line: member.end_line,
                end_column: member.end_column,
            };
            self.lower_member_expression(&inner, 0)
        }
    }

    /// The [`Scope`] tag for re-assigning an *already in-scope* name, or
    /// `None` if the name is not yet bound anywhere on the stack (so the
    /// assignment should create a fresh binding).  Resolving the name also
    /// records any capture needed to reach an enclosing binding.
    fn assign_target_scope(&mut self, name: &str, span: &Span) -> Option<Scope> {
        match self.resolve_local_chain(name, span) {
            Some(Expr::VarRef { scope, .. }) => Some(scope),
            _ => None,
        }
    }

    // -----------------------------------------------------------------------
    // expression → Expr  (M2: literals, var refs, unary/binary operators)
    // -----------------------------------------------------------------------

    /// Lower a JS `expression` to a SIR [`Expr`].
    ///
    /// The CST spine from `expression` down to the leaf is a chain of
    /// single-child precedence wrappers (see module docs).  We walk that
    /// spine to its bottom iteratively.  Two outcomes:
    ///   * the bottom is a single leaf token → a literal or variable
    ///     reference;
    ///   * we hit a node that *branches* (an operator with operands) →
    ///     dispatch on the rule name to build the matching SIR node.
    ///
    /// `depth` bounds the *operand* recursion (each branch lowers its
    /// children with `depth + 1`); the iterative spine-peel itself does
    /// not consume the budget.
    fn lower_expression(
        &mut self,
        expr: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, JsLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(JsLowerError {
                message: format!(
                    "expression nests deeper than the supported limit ({MAX_EXPR_DEPTH})"
                ),
                line: expr.start_line.unwrap_or(0),
                column: expr.start_column.unwrap_or(0),
            });
        }

        // Descend through single-child wrapper nodes.
        let mut cur = expr;
        loop {
            // A leaf node (exactly one child, a token) is a literal or a
            // variable reference — classify and emit.
            if let Some(tok) = cur.token() {
                return self.lower_leaf_token(tok);
            }
            match single_child_node(cur) {
                Some(next) => cur = next,
                None => {
                    // A branching node — an operator (or, in M2's
                    // still-unsupported set, a call/member access).
                    return self.lower_branch(cur, depth);
                }
            }
        }
    }

    /// Dispatch a *branching* precedence node to its operator handler.
    fn lower_branch(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, JsLowerError> {
        match node.rule_name.as_str() {
            // ── flat left-associative binary chains ─────────────────
            // children = [lhs, op, rhs, op, rhs, …]
            "additive_expression"
            | "multiplicative_expression"
            | "relational_expression"
            | "equality_expression" => self.lower_binary_chain(node, depth),

            // ── short-circuit logical chains ────────────────────────
            "logical_and_expression" => self.lower_logical_chain(node, depth, true),
            "logical_or_expression" => self.lower_logical_chain(node, depth, false),

            // ── prefix unary ────────────────────────────────────────
            // children = [op_token, operand]
            "unary_expression" => self.lower_unary(node, depth),

            // ── M4: arrow functions and calls ───────────────────────
            "arrow_function" => self.lower_arrow_function(node, depth),
            "call_expression" => self.lower_call_expression(node, depth),

            // ── M5: collections ─────────────────────────────────────
            // Array / object literals and member / subscript access.
            "array_literal" => self.lower_array_literal(node, depth),
            "object_literal" => self.lower_object_literal(node, depth),
            "member_expression" => self.lower_member_expression(node, depth),

            // ── M5: parenthesised expression ────────────────────────
            // `( expr )` — the parser wraps a grouping in a branching
            // `primary_expression[ (, expression, ) ]`.  This is how an
            // object literal reaches value position at statement start
            // (`({a: 1})`), and grouping in general.  Peel to the inner
            // expression and lower it.
            "primary_expression" if is_parenthesised(node) => {
                let inner = child_node_named(node, "expression")
                    .ok_or_else(|| self.unsupported(node, "parenthesised (no inner expr)"))?;
                self.lower_expression(inner, depth + 1)
            }

            // ── still unsupported (classes, `this`, `new`, …) ───────
            other => Err(self.unsupported(node, other)),
        }
    }

    // -----------------------------------------------------------------------
    // M5: collections — array / object literals, member / subscript access
    // -----------------------------------------------------------------------

    /// Lower an `array_literal` (`[ e0, e1, … ]`) to an [`Expr::SeqLit`].
    ///
    /// CST: `array_literal[ LBracket, element_list, RBracket ]`, where
    /// `element_list` holds `assignment_expression` element nodes separated
    /// by `Comma` tokens (empty for `[]`).  Each element is lowered with the
    /// depth-bounded expression lowerer, so a nested `[[[…]]]` tower past
    /// [`MAX_EXPR_DEPTH`] is a positioned error, not a stack overflow.
    ///
    /// Spread elements (`[...xs]`) and elisions (`[1, , 3]`) are deferred:
    /// the former adds a `spread_element` child we don't model, the latter
    /// a hole we'd have to invent a value for.
    fn lower_array_literal(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, JsLowerError> {
        let span = self.span_of(node);
        let mut items = Vec::new();
        // The element nodes live under the `element_list` child (absent for
        // some grammars when empty; the probe shows an empty `element_list`).
        if let Some(list) = child_node_named(node, "element_list") {
            for child in &list.children {
                match child {
                    ASTNodeOrToken::Node(n) => {
                        // A `spread_element` (`...xs`) is the one element node
                        // we cannot model as a plain item; reject it.
                        if peel_to_branch(n).rule_name == "spread_element" {
                            return Err(JsLowerError {
                                message: "array spread (`[...xs]`) is deferred past M5".to_string(),
                                line: span.start_line,
                                column: span.start_col,
                            });
                        }
                        items.push(self.lower_expression(n, depth + 1)?);
                    }
                    // Comma separators carry no element.
                    ASTNodeOrToken::Token(_) => {}
                }
            }
        }
        self.features_used.add(Feature::Sequences);
        Ok(Expr::SeqLit { items, span })
    }

    /// Lower an `object_literal` (`{ k0: v0, k1: v1, … }`) to an
    /// [`Expr::MapLit`].
    ///
    /// CST: `object_literal[ LBrace, property_definition*, (Comma), RBrace ]`,
    /// each `property_definition[ property_name, Colon, assignment_expression ]`
    /// where `property_name` wraps a single `Name` token (identifier key
    /// `a:`) or a `String` token (string key `"k":`).  Both lower to a
    /// **string** map key (`StrLit`); the JS distinction between an
    /// identifier and a quoted key is purely syntactic.
    ///
    /// Computed keys (`{ [e]: v }`), shorthand (`{ x }`), methods
    /// (`{ f() {} }`), getters/setters, and spread (`{ ...o }`) are deferred —
    /// each surfaces a `property_definition` shape we don't model.  Values are
    /// lowered with the depth-bounded expression lowerer, so a nested
    /// `{a:{b:{…}}}` tower past [`MAX_EXPR_DEPTH`] is a positioned error.
    fn lower_object_literal(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, JsLowerError> {
        let span = self.span_of(node);
        let mut entries = Vec::new();
        for prop in children_nodes_named(node, "property_definition") {
            // A simple `key: value` property has exactly a `property_name`,
            // a `Colon`, and one expression value node.  Anything else
            // (shorthand `{x}`, method `{f(){}}`, spread `{...o}`, computed
            // `{[e]:v}`) is deferred.
            let key = self.object_key(prop, &span)?;
            let value_node = prop
                .children
                .iter()
                .rev()
                .find_map(|c| match c {
                    ASTNodeOrToken::Node(n) => Some(n),
                    ASTNodeOrToken::Token(_) => None,
                })
                .ok_or_else(|| JsLowerError {
                    message: "object property without a value (shorthand / method) \
                              is deferred past M5"
                        .to_string(),
                    line: span.start_line,
                    column: span.start_col,
                })?;
            let value = self.lower_expression(value_node, depth + 1)?;
            entries.push(semantic_ir::nodes::MapEntry { key, value });
        }
        self.features_used.add(Feature::Maps);
        Ok(Expr::MapLit { entries, span })
    }

    /// Extract the string key of a `property_definition`, rejecting the
    /// deferred key forms (computed `[e]`, shorthand, method, numeric).
    fn object_key(&mut self, prop: &GrammarASTNode, span: &Span) -> Result<Expr, JsLowerError> {
        let name_node = child_node_named(prop, "property_name").ok_or_else(|| JsLowerError {
            message: "object property key form (computed / shorthand / method) \
                      is deferred past M5"
                .to_string(),
            line: span.start_line,
            column: span.start_col,
        })?;
        let tok = single_leaf_token(name_node).ok_or_else(|| JsLowerError {
            message: "computed object key (`{ [e]: v }`) is deferred past M5".to_string(),
            line: span.start_line,
            column: span.start_col,
        })?;
        match tok.type_ {
            // Identifier key `a:` and quoted key `"k":` both become a string
            // key — the JS-level distinction is syntactic only.
            TokenType::Name | TokenType::String => {
                self.features_used.add(Feature::Strings);
                Ok(Expr::StrLit {
                    value: tok.value.clone(),
                    span: self.span_of_token(tok),
                })
            }
            // Numeric keys (`{ 0: v }`) would need integer-keyed maps we
            // don't model in v0.
            other => Err(JsLowerError {
                message: format!(
                    "object key token {other:?} is deferred past M5 (string / identifier keys only)"
                ),
                line: tok.line,
                column: tok.column,
            }),
        }
    }

    /// Lower a *branching* `member_expression` — a dot member or a bracket
    /// subscript — to the spec's collection node.
    ///
    /// The disambiguation (SIR19 collections table) is **by access shape and
    /// key kind**, not by any value-type inference (the IR is untyped here):
    ///
    /// | JS access      | CST shape                                  | SIR node                         |
    /// |----------------|--------------------------------------------|----------------------------------|
    /// | `xs.length`    | `[obj, Dot, Name("length")]`               | `SeqLen { seq: obj }`            |
    /// | `obj.prop`     | `[obj, Dot, Name(prop)]`                   | `MapGet { map: obj, key: "prop"}`|
    /// | `obj["k"]`     | `[obj, LBracket, expr(String), RBracket]`  | `MapGet { map: obj, key: "k" }`  |
    /// | `xs[i]`        | `[obj, LBracket, expr(non-string), RBracket]` | `SeqIndex { seq: obj, index: i}` |
    ///
    /// So: **dot access → `MapGet`** (string key), with the single special
    /// case `.length` → `SeqLen`; **bracket access with a string-literal key
    /// → `MapGet`** (mirroring dot's string key), and **bracket access with
    /// any other key → `SeqIndex`**.  This default is spec-sanctioned (the
    /// table fixes `xs[i]`→`SeqIndex`, `d[k]`/`d.k`→`MapGet`,
    /// `xs.length`→`SeqLen`); the only genuinely ambiguous source form,
    /// `x[<string-literal>]`, is routed to `MapGet` by treating a quoted key
    /// the same as a dotted one.
    ///
    /// A multi-segment chain (`a.b.c`, `grid[0][1]`) is **left-associative**
    /// and, crucially, **flat in the CST**: the parser emits a *single*
    /// `member_expression` whose children are `[receiver, access, access, …]`
    /// — e.g. `grid[0][1]` → `[grid, [, 0, ], [, 1, ]]` (7 children) and
    /// `a.b.c` → `[a, ., b, ., c]` (5 children).  We therefore fold the
    /// accesses left over the child sequence (no CST recursion), lowering only
    /// the receiver and each index through the depth-bounded expression
    /// lowerer.  The *fold* is iterative, so a long chain costs no native
    /// stack; each lowered index/receiver still consumes the `MAX_EXPR_DEPTH`
    /// budget, so a `a[b[c[…]]]` index tower is a positioned error.
    fn lower_member_expression(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, JsLowerError> {
        let span = self.span_of(node);
        // The receiver is the first child *node*; the trailing children are
        // a flat sequence of `.name` or `[index]` accesses applied left to
        // right.  We walk the children with a small cursor so both access
        // shapes (and chains of them) fold uniformly.
        let children = &node.children;
        let first_idx = children
            .iter()
            .position(|c| matches!(c, ASTNodeOrToken::Node(_)))
            .ok_or_else(|| self.unsupported(node, "member_expression (no receiver)"))?;
        let recv_node = match &children[first_idx] {
            ASTNodeOrToken::Node(n) => n,
            ASTNodeOrToken::Token(_) => unreachable!("position found a Node"),
        };
        let mut acc = self.lower_expression(recv_node, depth + 1)?;

        let mut i = first_idx + 1;
        while i < children.len() {
            match &children[i] {
                // ── `.name` dot access ──────────────────────────────────
                ASTNodeOrToken::Token(t) if t.value == "." => {
                    // The next child is the property `Name` token.
                    let prop = match children.get(i + 1) {
                        Some(ASTNodeOrToken::Token(p)) if matches!(p.type_, TokenType::Name) => {
                            p.value.clone()
                        }
                        _ => {
                            return Err(self.unsupported(node, "member access (non-name property)"))
                        }
                    };
                    i += 2;
                    acc = if prop == "length" {
                        // The one dotted property that is a sequence op.
                        self.features_used.add(Feature::Sequences);
                        Expr::SeqLen {
                            seq: Box::new(acc),
                            span: span.clone(),
                        }
                    } else {
                        self.features_used.add(Feature::Maps);
                        self.features_used.add(Feature::Strings);
                        Expr::MapGet {
                            map: Box::new(acc),
                            key: Box::new(Expr::StrLit {
                                value: prop,
                                span: span.clone(),
                            }),
                            span: span.clone(),
                        }
                    };
                }
                // ── `[index]` bracket access ────────────────────────────
                ASTNodeOrToken::Token(t) if t.value == "[" => {
                    // The next child is the index node (an `expression` for a
                    // standalone subscript, or a bare precedence node in a
                    // flat chain); the child after that is the `]` token.
                    let index_node = match children.get(i + 1) {
                        Some(ASTNodeOrToken::Node(n)) => n,
                        _ => return Err(self.unsupported(node, "subscript (no index)")),
                    };
                    let index = self.lower_expression(index_node, depth + 1)?;
                    i += 3; // skip `[`, index, `]`.
                            // A *string-literal* key reads like object access → MapGet
                            // (mirrors `obj.prop`); any other index → sequence index.
                    acc = if matches!(index, Expr::StrLit { .. }) {
                        self.features_used.add(Feature::Maps);
                        Expr::MapGet {
                            map: Box::new(acc),
                            key: Box::new(index),
                            span: span.clone(),
                        }
                    } else {
                        self.features_used.add(Feature::Sequences);
                        Expr::SeqIndex {
                            seq: Box::new(acc),
                            index: Box::new(index),
                            span: span.clone(),
                        }
                    };
                }
                // Optional chaining `?.`, tagged templates, etc. are deferred.
                _ => return Err(self.unsupported(node, "member_expression (unsupported access)")),
            }
        }

        Ok(acc)
    }

    /// Lower a flat, left-associative binary chain to nested
    /// `BuiltinCall`s.  `a + b - c` (one `additive_expression` with
    /// children `[a, +, b, -, c]`) folds left into
    /// `BuiltinCall("-", [BuiltinCall("+", [a, b]), c])`.
    fn lower_binary_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, JsLowerError> {
        // Split children into operand nodes and operator tokens.  The
        // grammar guarantees the alternating shape `[n, t, n, t, n, …]`.
        let mut acc: Option<Expr> = None;
        let mut pending_op: Option<&Token> = None;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) => {
                    let operand = self.lower_expression(n, depth + 1)?;
                    match (acc.take(), pending_op.take()) {
                        (None, _) => acc = Some(operand),
                        (Some(lhs), Some(op)) => {
                            acc = Some(self.build_binary_op(op, lhs, operand)?);
                        }
                        (Some(_), None) => {
                            // Two operands with no operator between —
                            // shouldn't happen for a well-formed CST.
                            return Err(self.unsupported(node, &node.rule_name));
                        }
                    }
                }
                ASTNodeOrToken::Token(t) => pending_op = Some(t),
            }
        }

        acc.ok_or_else(|| self.unsupported(node, &node.rule_name))
    }

    /// Build one binary `BuiltinCall` from an operator token and its two
    /// already-lowered operands, applying equality normalisation.
    fn build_binary_op(&mut self, op: &Token, lhs: Expr, rhs: Expr) -> Result<Expr, JsLowerError> {
        // Normalise the operator spelling to the IR builtin name.  Both
        // loose and strict equality collapse to the strict-shaped IR
        // comparison — a deliberate semantic change (see module docs).
        let builtin = match op.value.as_str() {
            "+" | "-" | "*" | "/" | "%" | "<" | ">" | "<=" | ">=" => op.value.as_str(),
            "==" | "===" => "=",
            "!=" | "!==" => "!=",
            other => {
                return Err(JsLowerError {
                    message: format!("unsupported binary operator `{other}`"),
                    line: op.line,
                    column: op.column,
                })
            }
        };
        let span = self.span_of_token(op);
        Ok(Expr::BuiltinCall {
            name: builtin.to_string(),
            args: vec![lhs, rhs],
            // Arithmetic/comparison builtins are pure.
            effects: EffectSet::PURE,
            span,
        })
    }

    /// Lower a logical chain (`&&` / `||`) to nested short-circuit nodes.
    /// `a && b && c` folds left into `And(And(a, b), c)`.  These are
    /// **not** builtins: `LogicalAnd`/`LogicalOr` carry short-circuit
    /// semantics the validator records as `Feature::ShortCircuit`.
    fn lower_logical_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        is_and: bool,
    ) -> Result<Expr, JsLowerError> {
        // Short-circuit nodes are observed by the validator as
        // `Feature::ShortCircuit`; declare it so the manifest matches.
        self.features_used.add(Feature::ShortCircuit);
        let mut acc: Option<Expr> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) => {
                    let operand = self.lower_expression(n, depth + 1)?;
                    acc = Some(match acc.take() {
                        None => operand,
                        Some(lhs) => {
                            let span = lhs.span().clone();
                            if is_and {
                                Expr::LogicalAnd {
                                    lhs: Box::new(lhs),
                                    rhs: Box::new(operand),
                                    span,
                                }
                            } else {
                                Expr::LogicalOr {
                                    lhs: Box::new(lhs),
                                    rhs: Box::new(operand),
                                    span,
                                }
                            }
                        }
                    });
                }
                // The `&&` / `||` operator token carries no operand.
                ASTNodeOrToken::Token(_) => {}
            }
        }
        acc.ok_or_else(|| self.unsupported(node, &node.rule_name))
    }

    /// Lower a prefix `unary_expression` (`!x`, `-x`).
    ///
    /// Children are `[op_token, operand]`.  `!` → `BuiltinCall("not")`,
    /// `-` → `BuiltinCall("neg")` — except `-<numeric literal>` is
    /// constant-folded into a negative literal (see module docs).  Other
    /// prefix operators (`+`, `~`, `typeof`, `void`, `delete`) are
    /// deferred.
    fn lower_unary(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, JsLowerError> {
        // The operator is the leading token; the operand is the node.
        let op = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) => Some(t),
                ASTNodeOrToken::Node(_) => None,
            })
            .ok_or_else(|| self.unsupported(node, "unary_expression (no operator)"))?;
        let operand_node = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .ok_or_else(|| self.unsupported(node, "unary_expression (no operand)"))?;

        match op.value.as_str() {
            "!" => {
                let operand = self.lower_expression(operand_node, depth + 1)?;
                Ok(Expr::BuiltinCall {
                    name: "not".to_string(),
                    args: vec![operand],
                    effects: EffectSet::PURE,
                    span: self.span_of_token(op),
                })
            }
            "-" => {
                // Constant-fold `-<numeric literal>`: peel the operand
                // spine; if it bottoms out at a single Number token, emit
                // a negative literal directly (keeps the spec's `-7 →
                // IntLit` row exact).
                if let Some(tok) = single_leaf_token(peel_to_branch(operand_node)) {
                    if matches!(tok.type_, TokenType::Number) {
                        return self
                            .lower_number(&format!("-{}", tok.value), self.span_of_token(op));
                    }
                }
                let operand = self.lower_expression(operand_node, depth + 1)?;
                Ok(Expr::BuiltinCall {
                    name: "neg".to_string(),
                    args: vec![operand],
                    effects: EffectSet::PURE,
                    span: self.span_of_token(op),
                })
            }
            other => Err(JsLowerError {
                message: format!(
                    "unary operator `{other}` is deferred past M2 (only `!` and `-` supported)"
                ),
                line: op.line,
                column: op.column,
            }),
        }
    }

    /// Classify a leaf token and build the matching SIR atom.
    ///
    /// Covers M1 literals plus M2 variable references.  See the truth
    /// tables in the module docs.
    fn lower_leaf_token(&mut self, tok: &Token) -> Result<Expr, JsLowerError> {
        let span = self.span_of_token(tok);
        match tok.type_ {
            // ── number ──────────────────────────────────────────────
            TokenType::Number => self.lower_number(&tok.value, span),

            // ── keyword literals: true / false / null ───────────────
            TokenType::Keyword => match tok.value.as_str() {
                "true" => Ok(Expr::BoolLit { value: true, span }),
                "false" => Ok(Expr::BoolLit { value: false, span }),
                "null" => Ok(Expr::NilLit { span }),
                other => Err(JsLowerError {
                    message: format!("keyword `{other}` is not a value expression supported in M2"),
                    line: tok.line,
                    column: tok.column,
                }),
            },

            // ── string ──────────────────────────────────────────────
            TokenType::String => {
                self.features_used.add(Feature::Strings);
                Ok(Expr::StrLit {
                    value: tok.value.clone(),
                    span,
                })
            }

            // ── identifier: undefined / variable reference ──────────
            // `undefined` is a global identifier, not a keyword, so it
            // arrives as a `Name` token; collapse it to `NilLit` (the
            // JS null/undefined distinction is intentionally lost in v0).
            TokenType::Name if tok.value == "undefined" => Ok(Expr::NilLit { span }),
            // Any other identifier is a variable reference.  Resolve it
            // against the scope chain (local/param/capture), then against
            // module functions (a `function f` used as a *value* — e.g.
            // `return inner;` — is a `Scope::Global` reference the
            // validator accepts).  An undeclared name is a positioned
            // "unresolved name" error (SIR19 "Error model").
            TokenType::Name => self.resolve_name(&tok.value, span, tok.line, tok.column),

            other => Err(JsLowerError {
                message: format!("unsupported token {other:?} in expression position"),
                line: tok.line,
                column: tok.column,
            }),
        }
    }

    /// Lower a numeric literal's *text* into `IntLit` or `FloatLit`.
    ///
    /// A literal is treated as an integer iff it parses as an `i64` and
    /// its text contains neither a decimal point nor an exponent marker
    /// (`e`/`E`).  Otherwise it's a float.  Hex/octal/binary integer
    /// forms (`0x…`, `0o…`, `0b…`) and `BigInt` (`10n`) are deferred.
    ///
    /// A leading `-` (from a constant-folded unary minus) is permitted
    /// and parsed as part of the literal.
    fn lower_number(&mut self, text: &str, span: Span) -> Result<Expr, JsLowerError> {
        let looks_float = text.contains('.') || text.contains('e') || text.contains('E');
        // Detect the non-decimal integer forms after any leading sign.
        let digits = text.strip_prefix('-').unwrap_or(text);
        let non_decimal = digits.len() > 1
            && digits.starts_with('0')
            && matches!(
                digits.as_bytes()[1],
                b'x' | b'X' | b'o' | b'O' | b'b' | b'B'
            );
        if non_decimal || text.ends_with('n') {
            return Err(JsLowerError {
                message: format!(
                    "numeric literal `{text}` form (hex/octal/binary/BigInt) is deferred past M2"
                ),
                line: span.start_line,
                column: span.start_col,
            });
        }

        if !looks_float {
            if let Ok(value) = text.parse::<i64>() {
                return Ok(Expr::IntLit { value, span });
            }
            // Integer-shaped but doesn't fit i64.  Fall through to float
            // so we don't lose the program; JS holds it as a double.
        }

        match text.parse::<f64>() {
            Ok(value) => {
                self.features_used.add(Feature::Floats);
                Ok(Expr::FloatLit { value, span })
            }
            Err(_) => Err(JsLowerError {
                message: format!("could not parse numeric literal `{text}`"),
                line: span.start_line,
                column: span.start_col,
            }),
        }
    }

    /// Build the standard "out of scope" error for a node.
    fn unsupported(&self, node: &GrammarASTNode, what: &str) -> JsLowerError {
        JsLowerError {
            message: format!(
                "`{what}` is out of scope for this milestone; deferred to a later one"
            ),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        }
    }
}

// ---------------------------------------------------------------------------
// Lowered — a top-level item is either a statement or a tail expression
// ---------------------------------------------------------------------------

/// The result of lowering one top-level `source_element`.
///
/// Bindings and assignments are [`Stmt`]s that accumulate in the block;
/// a bare expression is a candidate tail value.  Keeping the distinction
/// explicit (rather than always wrapping in `ExprStmt`) lets
/// [`Lowerer::lower_program`] route the final expression into the block's
/// `value` slot, matching the SIR "statements then a value" block shape.
///
/// `Stmt` is boxed: it is substantially larger than `Expr` (it embeds
/// loop/class/try variants), so an unboxed enum would size every
/// `Lowered` to the largest variant — clippy's `large_enum_variant`.
enum Lowered {
    Stmt(Box<Stmt>),
    Expr(Expr),
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

/// Conservatively decide whether evaluating `expr` may have an observable
/// side effect (printing, mutation, allocation, or *any* call whose callee
/// could).  Used by the block builder to decide whether a bare-expression
/// statement that is *superseded* as a tail value must still be flushed as
/// an `ExprStmt` (it may run for effect) or can be dropped (it's pure).
///
/// This is deliberately **conservative**: when in doubt we answer `true`
/// (keep the statement).  Dropping a side-effecting expression is a
/// correctness bug; keeping a pure one is at most a harmless dead `ExprStmt`
/// that later passes can elide.
///
/// Pure (droppable) forms are the literals, a variable reference, and the
/// pure operators over pure operands.  Everything that *calls* (a
/// `DirectCall` / `IndirectCall` / `BuiltinCall` such as `print`) or builds
/// a closure is treated as potentially effectful.
fn expr_may_have_effects(expr: &Expr) -> bool {
    match expr {
        // SIR26 conversion (not currently emitted here): pure iff its value is.
        Expr::Convert { value, .. } => expr_may_have_effects(value),
        // Provably pure leaves.
        Expr::IntLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::NilLit { .. }
        | Expr::SymLit { .. }
        | Expr::StrLit { .. }
        | Expr::VarRef { .. } => false,

        // Pure iff every operand is pure.
        Expr::StrConcat { parts, .. } => parts.iter().any(expr_may_have_effects),
        Expr::SeqLit { items, .. } => items.iter().any(expr_may_have_effects),
        Expr::SeqLen { seq, .. } => expr_may_have_effects(seq),
        Expr::SeqIndex { seq, index, .. } => {
            expr_may_have_effects(seq) || expr_may_have_effects(index)
        }
        Expr::MapGet { map, key, .. } => expr_may_have_effects(map) || expr_may_have_effects(key),
        Expr::MapLit { entries, .. } => entries
            .iter()
            .any(|e| expr_may_have_effects(&e.key) || expr_may_have_effects(&e.value)),
        Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
            expr_may_have_effects(lhs) || expr_may_have_effects(rhs)
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            expr_may_have_effects(cond)
                || block_may_have_effects(then_branch)
                || block_may_have_effects(else_branch)
        }
        Expr::Block(b) => block_may_have_effects(b),

        // KW1 compile-compat stub: a `KeywordArg`'s effect is exactly its
        // inner `value`'s effect (the `name` is a static label) — recurse
        // faithfully so effect inference stays conservative-correct.  Real
        // support pending KW2–KW8.
        Expr::KeywordArg { value, .. } => expr_may_have_effects(value),

        // SIR22 array/matrix nodes: the spec's "Effects" section marks every
        // one of these `Pure`, so — like `SeqLit`/`MapLit`/etc. above — they
        // are pure iff every operand is pure.
        Expr::ArrayLit { rows, .. } => rows.iter().any(|row| row.iter().any(expr_may_have_effects)),
        Expr::Range {
            start, step, stop, ..
        } => {
            expr_may_have_effects(start)
                || step.as_deref().is_some_and(expr_may_have_effects)
                || expr_may_have_effects(stop)
        }
        Expr::MatMul { lhs, rhs, .. } => expr_may_have_effects(lhs) || expr_may_have_effects(rhs),
        Expr::ElementwiseOp { lhs, rhs, .. } => {
            expr_may_have_effects(lhs) || expr_may_have_effects(rhs)
        }
        Expr::Transpose { target, .. } => expr_may_have_effects(target),
        Expr::IndexGet {
            target, indices, ..
        } => {
            expr_may_have_effects(target)
                || indices.iter().any(|idx| match idx {
                    IndexArg::Scalar(e) | IndexArg::Range(e) => expr_may_have_effects(e),
                    IndexArg::Whole => false,
                })
        }

        // Calls, closures, and intrinsics may do anything → keep.
        Expr::DirectCall { .. }
        | Expr::IndirectCall { .. }
        | Expr::BuiltinCall { .. }
        | Expr::MakeClosure { .. }
        | Expr::Intrinsic { .. } => true,

        // SIR23 symbolic-expression/pattern nodes: the spec's "Effects"
        // section marks every one of these `Pure` too — same treatment as
        // the SIR22 array/matrix nodes above.
        Expr::SymSymbol { .. } | Expr::SymRational { .. } => false,
        Expr::SymApply { head, args, .. } => {
            expr_may_have_effects(head) || args.iter().any(expr_may_have_effects)
        }
        Expr::SymPatternBlank { head, .. } => {
            head.as_deref().is_some_and(expr_may_have_effects)
        }
        Expr::SymPatternNamed { pattern, .. } => expr_may_have_effects(pattern),
        Expr::SymRule { lhs, rhs, .. } => {
            expr_may_have_effects(lhs) || expr_may_have_effects(rhs)
        }
        Expr::SymReplaceAll { expr, rules, .. } => {
            expr_may_have_effects(expr) || rules.iter().any(expr_may_have_effects)
        }
    }
}

/// A block may have an effect if any of its statements does, or if its tail
/// value expression does.  Any statement at all (`let*`, `assign`, an
/// `ExprStmt` we only keep *because* it had an effect, a loop, …) is treated
/// as effectful — the block builder never emits a statement for a provably
/// pure dropped value.
fn block_may_have_effects(block: &Block) -> bool {
    !block.stmts.is_empty() || expr_may_have_effects(&block.value)
}

/// If `node` has exactly one child and that child is a nested node,
/// return it.  This is the workhorse for descending the CST's precedence
/// spine: each precedence layer that wasn't "used" appears as a wrapper
/// with a single node child.  Returns `None` when the node branches
/// (multiple children) or when its single child is a token (a leaf).
fn single_child_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    if node.children.len() == 1 {
        match &node.children[0] {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        }
    } else {
        None
    }
}

/// Peel a node's single-child precedence spine down to the first node
/// that *branches* (more than one child) or is a leaf (single token
/// child).  Returns that node.  Unlike [`single_child_node`] this keeps
/// descending; it's used to classify an expression's "real" shape
/// without lowering it (e.g. is the LHS of an `expression_statement` an
/// assignment?).
fn peel_to_branch(node: &GrammarASTNode) -> &GrammarASTNode {
    let mut cur = node;
    while let Some(next) = single_child_node(cur) {
        cur = next;
    }
    cur
}

/// If `node` is a leaf wrapper bottoming out at a single token, return
/// that token.  Peels the precedence spine first, so
/// `single_leaf_token(primary_expression-wrapping-`x`)` yields the `x`
/// token.  Returns `None` if the bottom node branches.
fn single_leaf_token(node: &GrammarASTNode) -> Option<&Token> {
    peel_to_branch(node).token()
}

/// Return every direct child of `node` that is a *node* (dropping the
/// interleaved tokens), in source order.  Used by the control-flow lowerers
/// to read a statement's operand nodes positionally — e.g. an
/// `if_statement`'s `[cond, then, else]` or a loop's trailing body node —
/// without having to thread past the keyword/paren tokens.
fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// Is `node` a parenthesised grouping `( expression )` — the branching
/// `primary_expression[ LParen, expression, RParen ]` the parser emits for
/// `(e)`?  Used (M5) to peel a grouping that brought an object literal into
/// value position at statement start (`({a: 1})`).
fn is_parenthesised(node: &GrammarASTNode) -> bool {
    node.rule_name == "primary_expression"
        && node
            .children
            .first()
            .map(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "("))
            .unwrap_or(false)
}

/// Return the first direct child node of `node` whose `rule_name` is
/// `name`, if any.
fn child_node_named<'a>(node: &'a GrammarASTNode, name: &str) -> Option<&'a GrammarASTNode> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == name => Some(n),
        _ => None,
    })
}

/// Return every direct child node of `node` whose `rule_name` is `name`.
fn children_nodes_named<'a>(node: &'a GrammarASTNode, name: &str) -> Vec<&'a GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == name => Some(n),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// M4 free helpers
// ---------------------------------------------------------------------------

/// Descend `source_element` / `statement` single-child wrappers to the
/// concrete statement node (the one whose `rule_name` is a real statement
/// kind like `return_statement` / `if_statement` / `block`).
fn concrete_statement(node: &GrammarASTNode) -> &GrammarASTNode {
    let inner = single_child_node(node).unwrap_or(node);
    if inner.rule_name == "statement" || inner.rule_name == "source_element" {
        concrete_statement(inner)
    } else {
        inner
    }
}

/// The declared name of a `function_declaration`: its first `Name` token
/// (the one right after the `function` keyword).
fn function_decl_name(node: &GrammarASTNode) -> Option<String> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t.value.clone()),
        _ => None,
    })
}

/// Pass-1 collector: gather **every** `function_declaration` name in the
/// program — top-level *and* nested — into `out`.
///
/// Both top-level and nested declarations end up as module-level
/// `Function`s (nested ones are lifted), so every name must be globally
/// resolvable for `DirectCall`s and (mutual) recursion.  We recurse through
/// the whole CST so a declaration buried inside a block / loop / another
/// function is still discovered.
///
/// This walk runs in [`compile`] *before* the depth-guarded lowering, so it
/// carries its **own** [`MAX_STMT_DEPTH`] bound: a pathologically deep CST
/// (thousands of nested blocks / functions) turns into a positioned error
/// rather than a native stack overflow (CWE-674).
fn collect_function_names(
    node: &GrammarASTNode,
    out: &mut HashSet<String>,
    depth: usize,
) -> Result<(), JsLowerError> {
    if depth > MAX_STMT_DEPTH {
        return Err(JsLowerError {
            message: format!("input nests deeper than the supported limit ({MAX_STMT_DEPTH})"),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        });
    }
    if node.rule_name == "function_declaration" {
        if let Some(name) = function_decl_name(node) {
            out.insert(name);
        }
    }
    // Note: we deliberately do *not* descend into `arrow_function` bodies to
    // collect names — arrows have no declaration name, and a `function`
    // declaration nested inside an arrow body is still a declaration we want
    // to lift, so we keep descending into every child uniformly.
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            collect_function_names(n, out, depth + 1)?;
        }
    }
    Ok(())
}

/// If `callee` is a two-segment member access `obj.method` (the CST shape
/// `member_expression[ primary(obj), Dot, Name(method) ]`), return
/// `(obj, method)`.  Returns `None` for a bare identifier or a deeper /
/// computed member chain.
fn member_callee_parts(callee: &GrammarASTNode) -> Option<(String, String)> {
    // Peel precedence wrappers down to the branching node; a member access
    // branches at `member_expression` with `[obj, Dot, Name]`.
    let branch = peel_to_branch(callee);
    if branch.rule_name != "member_expression" {
        return None;
    }
    // children = [obj_node, Dot, Name(method)].
    if branch.children.len() != 3 {
        return None;
    }
    let obj_node = match &branch.children[0] {
        ASTNodeOrToken::Node(n) => n,
        _ => return None,
    };
    let is_dot = matches!(&branch.children[1], ASTNodeOrToken::Token(t) if t.value == ".");
    if !is_dot {
        return None;
    }
    let method = match &branch.children[2] {
        ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => t.value.clone(),
        _ => return None,
    };
    let obj = single_leaf_token(obj_node)
        .filter(|t| matches!(t.type_, TokenType::Name))?
        .value
        .clone();
    Some((obj, method))
}

/// Method names that must never be lowered into a `__method__` dispatch
/// because they are reflective JavaScript gadgets — prototype-chain and
/// constructor escape hatches that turn an ordinary `recv[name]` property
/// lookup into arbitrary-code execution.
///
/// The headline gadget is `constructor`: on any function value it resolves to
/// the global `Function` constructor, so a translated untrusted program of the
/// form `id.constructor("return …")` synthesises and runs attacker code (an
/// RCE — a native higher-order method such as `Array.prototype.map` will then
/// happily invoke the resulting function).  `apply`/`call`/`bind` re-bind
/// `this`, and `__proto__`/`prototype`/`__define*etter__`/`__lookup*etter__`
/// walk or mutate the prototype chain.  None of these is ever a legitimate
/// collection / String / Number method, so we reject them outright here —
/// this is defense in depth in front of the JS runtime's own allowlist.
fn is_dangerous_method_name(name: &str) -> bool {
    matches!(
        name,
        "constructor"
            | "__proto__"
            | "prototype"
            | "apply"
            | "call"
            | "bind"
            | "__defineGetter__"
            | "__defineSetter__"
            | "__lookupGetter__"
            | "__lookupSetter__"
    )
}

/// If `callee` peels to a `member_expression` whose **final** access is a
/// dot-name (`recv.method`), return the method-name `Name` token together
/// with the child index of that final `.` token (its access position in the
/// flat member chain).  The caller uses the index to split off the receiver
/// prefix via [`lower_member_prefix`](Lowerer::lower_member_prefix).
///
/// The receiver may be *any* expression — a bare identifier (`arr.map`) or a
/// deeper member chain (`obj.inner.push`, where the receiver `obj.inner` is a
/// map read folded before the `.push` dispatch).  We therefore do **not**
/// constrain the receiver's shape here (unlike [`member_callee_parts`], which
/// only matches a bare-identifier object for the `console.log` special case).
/// A *chained* call (`xs.filter(f).map(g)`) is folded one step at a time by
/// [`lower_call_expression`](Lowerer::lower_call_expression) — each step's
/// base callee reaching this helper is a plain `member_expression`.
///
/// Returns `None` for a bare identifier (not a member access) or when the
/// final access is a *computed* subscript (`obj[expr](…)`) — those are not
/// named methods and stay deferred.
fn dotted_method_callee(callee: &GrammarASTNode) -> Option<(&Token, usize)> {
    let branch = peel_to_branch(callee);
    if branch.rule_name != "member_expression" {
        return None;
    }
    // Locate the *final* access token (`.` or `[`) in the flat chain.
    let last_access = branch
        .children
        .iter()
        .rposition(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "." || t.value == "["))?;
    // The final access must be a dot — a named method, not a computed
    // subscript — followed by a `Name` token (the method name).
    let is_dot =
        matches!(&branch.children[last_access], ASTNodeOrToken::Token(t) if t.value == ".");
    if !is_dot {
        return None;
    }
    let method_tok = match branch.children.get(last_access + 1) {
        Some(ASTNodeOrToken::Token(t)) if matches!(t.type_, TokenType::Name) => t,
        _ => return None,
    };
    Some((method_tok, last_access))
}

/// Detect *genuine* mutual recursion: a cycle of length ≥ 2 in the static
/// call graph of the module's functions.
///
/// The SIR validator never *observes* `Feature::MutualRecursion` (there is
/// no node for it), so a frontend that over-declares it triggers a
/// "declared but unused" warning.  We therefore declare it only when a real
/// cycle exists — `f` calls `g` and `g` (transitively) calls `f`.  A
/// function calling only itself (direct self-recursion) is **not** mutual
/// recursion and is excluded.
fn has_mutual_recursion(functions: &[Function]) -> bool {
    use std::collections::HashMap;
    // Map each function name to the set of function names it directly
    // `DirectCall`s (ignoring self-calls for the cycle search below — a
    // self-loop is single-function recursion, not mutual).
    let names: HashSet<&str> = functions.iter().map(|f| f.name.as_str()).collect();
    let mut edges: HashMap<&str, HashSet<String>> = HashMap::new();
    for f in functions {
        let mut callees = HashSet::new();
        collect_direct_callees(&f.body, &names, &f.name, &mut callees);
        edges.insert(f.name.as_str(), callees);
    }
    // A mutual-recursion cycle exists iff some pair `a → b` and `b →* a`.
    // We do a DFS from each node and look for a back-edge to a *different*
    // start node that can also reach back.  Simpler: detect any cycle of
    // length ≥ 2 via reachability — `a` reaches `b` and `b` reaches `a`
    // with `a != b`.
    let node_list: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();
    let reaches = |start: &str| -> HashSet<String> {
        let mut seen = HashSet::new();
        let mut stack = vec![start.to_string()];
        while let Some(cur) = stack.pop() {
            if let Some(cs) = edges.get(cur.as_str()) {
                for c in cs {
                    if seen.insert(c.clone()) {
                        stack.push(c.clone());
                    }
                }
            }
        }
        seen
    };
    for a in &node_list {
        let ra = reaches(a);
        for b in &ra {
            if b.as_str() != *a && reaches(b).contains(*a) {
                return true;
            }
        }
    }
    false
}

/// Collect the names this block `DirectCall`s (excluding `self_name`), into
/// `out`.  Walks every nested expression/statement.
fn collect_direct_callees(
    block: &Block,
    names: &HashSet<&str>,
    self_name: &str,
    out: &mut HashSet<String>,
) {
    for s in &block.stmts {
        collect_stmt_callees(s, names, self_name, out);
    }
    collect_expr_callees(&block.value, names, self_name, out);
}

fn collect_stmt_callees(
    stmt: &Stmt,
    names: &HashSet<&str>,
    self_name: &str,
    out: &mut HashSet<String>,
) {
    match stmt {
        Stmt::LetBinding { value, .. }
        | Stmt::LetStarBinding { value, .. }
        | Stmt::Assign { value, .. }
        | Stmt::ExprStmt { expr: value, .. } => collect_expr_callees(value, names, self_name, out),
        Stmt::While { cond, body, .. } => {
            collect_expr_callees(cond, names, self_name, out);
            collect_direct_callees(body, names, self_name, out);
        }
        Stmt::ForRange {
            start,
            stop,
            step,
            body,
            ..
        } => {
            collect_expr_callees(start, names, self_name, out);
            collect_expr_callees(stop, names, self_name, out);
            collect_expr_callees(step, names, self_name, out);
            collect_direct_callees(body, names, self_name, out);
        }
        Stmt::ForEach { iter, body, .. } => {
            collect_expr_callees(iter, names, self_name, out);
            collect_direct_callees(body, names, self_name, out);
        }
        // Other statement kinds carry no DirectCall the JS frontend emits.
        _ => {}
    }
}

fn collect_expr_callees(
    expr: &Expr,
    names: &HashSet<&str>,
    self_name: &str,
    out: &mut HashSet<String>,
) {
    match expr {
        Expr::DirectCall { fn_name, args, .. } => {
            if fn_name != self_name && names.contains(fn_name.as_str()) {
                out.insert(fn_name.clone());
            }
            for a in args {
                collect_expr_callees(a, names, self_name, out);
            }
        }
        Expr::IndirectCall { target, args, .. } => {
            collect_expr_callees(target, names, self_name, out);
            for a in args {
                collect_expr_callees(a, names, self_name, out);
            }
        }
        Expr::BuiltinCall { args, .. } => {
            for a in args {
                collect_expr_callees(a, names, self_name, out);
            }
        }
        Expr::MakeClosure { captures, .. } => {
            for c in captures {
                collect_expr_callees(&c.value, names, self_name, out);
            }
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_expr_callees(cond, names, self_name, out);
            collect_direct_callees(then_branch, names, self_name, out);
            collect_direct_callees(else_branch, names, self_name, out);
        }
        Expr::Block(b) => collect_direct_callees(b, names, self_name, out),
        Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
            collect_expr_callees(lhs, names, self_name, out);
            collect_expr_callees(rhs, names, self_name, out);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Tests — depth bounds on the pre-lowering recursive CST walks (CWE-674)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic node with a rule name and children, spans stamped.
    fn node(rule: &str, children: Vec<ASTNodeOrToken>) -> GrammarASTNode {
        GrammarASTNode {
            rule_name: rule.to_string(),
            children,
            start_line: Some(1),
            start_column: Some(1),
            end_line: Some(1),
            end_column: Some(1),
        }
    }

    /// A `block`-chain nested `n` levels deep, bottoming out at a childless
    /// terminal node.  The depth guards trip while descending, so no leaf
    /// token is needed.
    fn nest_blocks(n: usize) -> GrammarASTNode {
        let mut cur = node("primary_expression", Vec::new());
        for _ in 0..n {
            cur = node("block", vec![ASTNodeOrToken::Node(cur)]);
        }
        cur
    }

    /// A bare `Lowerer` for exercising its private walks directly.
    fn lowerer() -> Lowerer {
        Lowerer {
            file_name: "test".to_string(),
            features_used: FeatureManifest::new(),
            scopes: Vec::new(),
            function_names: HashSet::new(),
            user_functions: Vec::new(),
            synthesised: Vec::new(),
            lambda_counter: 0,
        }
    }

    #[test]
    fn collect_function_names_is_depth_bounded() {
        // Pass-1 name collection over a tower far deeper than MAX_STMT_DEPTH
        // must return a positioned error, not overflow the stack.
        let deep = nest_blocks(MAX_STMT_DEPTH + 64);
        let mut names = HashSet::new();
        let err = collect_function_names(&deep, &mut names, 0)
            .expect_err("deep tower must trip the pass-1 guard");
        assert!(
            err.message.contains("deeper than the supported limit"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn collect_function_names_accepts_shallow_input() {
        // A shallow tree well within the bound resolves normally.
        let shallow = node(
            "function_declaration",
            vec![ASTNodeOrToken::Token(Token {
                type_: TokenType::Name,
                value: "f".to_string(),
                line: 1,
                column: 1,
                type_name: None,
                flags: None,
                cv: None,
            })],
        );
        let mut names = HashSet::new();
        collect_function_names(&shallow, &mut names, 0).expect("shallow input is fine");
        assert!(names.contains("f"));
    }

    #[test]
    fn reject_returns_is_depth_bounded() {
        // The early-return detection walk over a tower far deeper than
        // MAX_STMT_DEPTH must return a positioned error, not overflow the
        // stack — isolated from pass-1 by calling it directly.
        let deep = nest_blocks(MAX_STMT_DEPTH + 64);
        let lw = lowerer();
        let err = lw
            .reject_returns(&deep, 0)
            .expect_err("deep tower must trip the reject_returns guard");
        assert!(
            err.message.contains("deeper than the supported limit"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn reject_returns_finds_a_shallow_early_return() {
        // Sanity: within the depth bound, a stray `return` is still rejected
        // as an early return (not a depth error).
        let ret = node("return_statement", Vec::new());
        let wrapper = node("block", vec![ASTNodeOrToken::Node(ret)]);
        let lw = lowerer();
        let err = lw
            .reject_returns(&wrapper, 0)
            .expect_err("a nested return is an early return");
        assert!(err.message.contains("early return"), "got: {}", err.message);
    }
}
