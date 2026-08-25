//! The lowering pass from `coding_adventures_java_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.5.0 (JV02
//! milestone M3a)**.
//!
//! # Scope
//!
//! Java requires an explicit `class`/`main`-method wrapper at the source
//! level — this crate recognizes exactly one shape and returns a clean
//! [`JavaLowerError`] for anything else, rather than silently
//! mis-lowering:
//!
//! **Supported (M0, unchanged):**
//! - Exactly one top-level `class` declaration, containing a
//!   `public static void main(String[] args) { ... }` method (M3a lifts
//!   the original "exactly one method total" restriction — see below).
//! - Literal expressions: integer (`42`), floating-point (`3.14`), boolean
//!   (`true`/`false`), `null`, and string (`"str"`) literals.
//!
//! **Supported (M1, unchanged):**
//! - Local variable declarations with an explicit primitive type
//!   (`int`/`long`/`short`/`byte`/`char`/`float`/`double`/`boolean`) or
//!   `String`, each requiring an initializer (`int x = 1;`).
//! - `var` type inference (Java 10+ — see "The `var` ambiguity" below),
//!   inferring the declared kind from the initializer.
//! - Re-assignment of an already-declared local (`x = 2;`), plain `=`
//!   only.
//! - Arithmetic (`+ - * / %`), relational (`< > <= >=`), equality
//!   (`== !=`), and logical (`&& || !`) operators, plus unary `+`/`-`.
//! - String concatenation via `+` when either operand is `String`
//!   (lowers to [`Expr::StrConcat`], which auto-stringifies non-string
//!   parts — see that node's own doc comment — matching Java's own `+`
//!   semantics for mixed-type concatenation, e.g. `"n=" + 5`).
//! - Parenthesized sub-expressions.
//!
//! **Supported (M2a, unchanged):**
//! - `if`/`else` (an absent `else` becomes a synthetic empty, nil-valued
//!   block — matching the established `javascript-to-semantic-ir`/
//!   `ruby-to-semantic-ir` precedent for the same "the IR's `If` is an
//!   expression with two non-optional branches" shape).
//! - `while` and `do`/`while` (the latter desugars to a synthetic flag-
//!   guarded pretest loop, lowering the body exactly once — see
//!   `lower_do_while_statement`'s own doc comment for why, and for the
//!   exact desugared shape).
//! - Compound assignment (`+= -= *= /= %=`) and increment/decrement
//!   (`++`/`--`, prefix and postfix) — but **only as a bare statement**
//!   (`i++;`, `x += 1;`), desugaring to `Stmt::Assign` by reusing M1's own
//!   `combine_additive`/`combine_multiplicative` op-selection. Using
//!   either as a *value* (`y = i++;`) remains out of scope — that needs
//!   pre-increment-value capture semantics this milestone doesn't build.
//!
//! **Supported (M2b, new):**
//! - Classic `for (init; cond; update) body` — SIR's `Stmt::ForRange` is a
//!   canonical `for var in range(start, stop, step)` counting loop, too
//!   narrow to represent Java's fully general three-clause `for` (an
//!   arbitrary init/cond/update, not necessarily a simple increasing
//!   counter), so this desugars to `{ init; while (cond) { body; update }
//!   }` instead (mirrors `c-to-semantic-ir`'s own identically-reasoned
//!   precedent for C's own equally general `for` — see
//!   `lower_for_statement`'s own doc comment). Each clause may be a
//!   declaration (`for (int i = 0; ...)`), a single expression
//!   (`for (i = 0; ...)`, reusing an already-declared variable), or
//!   entirely absent (`for (;;)`, defaulting the condition to `true`).
//! - Enhanced `for (T x : xs) body` → `Stmt::ForEach` directly (SIR
//!   already has exactly this shape, no desugaring needed). `var` as the
//!   element type is rejected: M1/M2 have no array/collection `Kind` or
//!   construction syntax at all yet (that's JV02 M4), so there's no way
//!   to infer the element type from the iterable the way real Java would.
//!
//! Every block (an `if`/`while`/`do`-`while`/`for`/enhanced-`for` body) is
//! its own lexical scope, mirroring the SIR validator's own `Block`-scoped
//! `LocalEnv` mark/rewind discipline exactly (a local declared inside an
//! `if` body is not visible after it, in both Java and the validator's own
//! contract) — a classic `for`'s own init-declared variable additionally
//! spans its condition/update/body (but not beyond the loop), matching
//! Java's own for-loop scoping exactly.
//!
//! **Supported (M3a, new):**
//! - Every `method_declaration` in the class body — static or instance
//!   (both lower identically to a flat top-level [`Function`]; there is
//!   no real object/receiver model until a later milestone, so "instance"
//!   here just means "lacks the `static` modifier") — with a typed
//!   parameter list (`int add(int a, int b)`), lowered in a first pass
//!   that registers every method's *name* and call signature before any
//!   body is lowered, so forward references and mutual recursion between
//!   methods resolve regardless of textual order (mirrors
//!   `python-to-semantic-ir`'s/`javascript-to-semantic-ir`'s own two-pass
//!   precedent).
//! - Bare unqualified calls, `foo(a, b)`, to a method declared elsewhere
//!   in the same class (including the calling method itself — plain and
//!   mutual recursion both work) → [`Expr::DirectCall`]. A *qualified*
//!   call (`x.foo(...)`) remains out of scope — there is no receiver/
//!   object model yet.
//! - `return`, but **only** as the literal last top-level statement of a
//!   method body (SIR has no `Stmt::Return` primitive at all — a
//!   function's value is always its own body `Block`'s trailing `.value`
//!   — confirmed by an exhaustive grep of the `Stmt` enum) — an early or
//!   branched `return` is a clean, disclosed rejection, not a silent
//!   mis-lowering. `void` methods may end with a bare `return;` or simply
//!   fall off the end (both become an empty, nil-valued body tail).
//!
//! **Deliberately out of scope for v0.5.0** (each rejected with an
//! explicit [`JavaLowerError`], tracked in
//! [JV02](../../../specs/JV02-java-to-semantic-ir.md)'s own milestone
//! table): `switch` (SIR has no `Switch`/`Match`/`Case` IR node at all —
//! confirmed by a repo-wide grep, not assumed — so this needs its own
//! spec-level design decision before any frontend can target it, tracked
//! as a separate backlog item, not silently dropped), qualified/method-
//! reference calls, method overloading (only one method per name is
//! supported — this frontend has no type-based overload resolution),
//! varargs parameters, fields, constructors, static/instance
//! initializers, nested types, an early or branched `return`, lambdas
//! (JV02 M3b), field/array access, casts, `instanceof`, the ternary
//! conditional, bitwise operators (`& | ^ ~ << >> >>>`), increment/
//! decrement or compound assignment used as a *value* rather than a bare
//! statement, `break`/`continue` (SIR has no IR primitive for either —
//! every loop body this milestone lowers must not contain one, checked
//! structurally, not merely "happens not to occur in the test corpus" —
//! this also means a bare `for (;;)` loop genuinely cannot terminate via
//! any construct this milestone can lower, a real and permanent
//! limitation until `break` exists), multiple comma-separated expressions
//! in one `for` init/update clause, `var` as an enhanced-`for` element
//! type, uninitialized declarations, multiple declarators per statement,
//! C-style array-bracket declarators (on a variable, a method parameter,
//! or a method's own return type), array initializers, and reference
//! types other than `String`.
//!
//! ## The `var` ambiguity
//!
//! `local_var_type = type | "var"` is an ordered PEG choice with `type`
//! tried first. Since `type` can itself resolve to a bare `class_type`
//! (`qualified_name` of one segment), the grammar parses `var x = 1;` as
//! `type -> class_type -> qualified_name -> NAME "var"` — the literal
//! `"var"` alternative is *never actually reached* for real source
//! (confirmed by direct inspection of the parser's own output, not
//! assumed from reading the grammar). This lowerer therefore detects
//! `var` by its resolved shape (a single-segment class type literally
//! named `var`) rather than by which grammar alternative matched. This is
//! not a heuristic: the JLS reserves `var` as a type name, so no real
//! Java source can ever declare a class actually named `var` — the two
//! cases are truly unambiguous, just not distinguished by which grammar
//! rule fired.

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Param, ParamKind,
    Scope, Span, Stmt,
};
use std::collections::{HashMap, HashSet};

/// Maximum descent depth through the Java grammar's expression-precedence
/// chain (`assignment_expression` → `conditional_expression` → … →
/// `literal`, one grammar rule per precedence level — see this module's
/// own doc comment and `java-parser`'s own `MAX_RULE_DEPTH`). Mirrors
/// every other SIR frontend's identically-named, identically-justified
/// guard: turns pathologically deep (but parseable) input into a clean
/// [`JavaLowerError`] instead of a native (uncatchable) stack overflow.
/// The parser's own `MAX_RULE_DEPTH` (180) already bounds this in
/// practice; this is this frontend's own independent guard, not a
/// reliance on that upstream cap. Every mutually-recursive expression
/// lowering helper in this module (`lower_expr` and its callees) takes
/// and threads a `depth` parameter checked against this constant, exactly
/// like M0's `descend_to_literal` did.
const MAX_EXPR_DEPTH: usize = 64;

/// Maximum recursion depth for [`collect_bounded`]'s own raw-CST tree
/// walk (finding the top-level `class_declaration`) — a separate budget
/// from [`MAX_EXPR_DEPTH`] (that one bounds the expression-precedence
/// chain specifically; this one bounds an arbitrary tree walk, a
/// conceptually different traversal even though both currently use the
/// same numeric value). Exists for the same reason: `compile()` is a
/// public entry point accepting a raw `GrammarASTNode`, not guaranteed to
/// have come from a depth-capped parser. (`collect_class_methods`, M3a's
/// replacement for the old `find_main_method`, needs no such guard of its
/// own — see that function's own doc comment for why.)
const MAX_TREE_DEPTH: usize = 64;

/// Maximum nesting depth through the *statement*/block-lowering chain
/// (`lower_statement` → `lower_if_statement`/`lower_while_statement`/
/// `lower_do_while_statement` → `lower_body` → `lower_block_node` →
/// `lower_block_statement` → `lower_statement` → …) — a third,
/// conceptually distinct recursion budget from [`MAX_EXPR_DEPTH`]
/// (expression-precedence chain) and [`MAX_TREE_DEPTH`] (raw CST
/// tree-walking in `collect_bounded`). Real Java source
/// can nest `if`/`while`/`do`-`while` bodies arbitrarily deep
/// (`if (a) if (b) if (c) …`), and `compile()` is a public entry point
/// accepting a raw `GrammarASTNode`, not only one produced by
/// `parse_java`'s own depth-capped parser — without this guard, a deeply
/// nested control-flow tree handed straight to `compile()` would be a
/// CWE-674 uncontrolled-recursion DoS, the same class of bug this crate's
/// other two depth guards already exist to prevent.
const MAX_STMT_DEPTH: usize = 64;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<java>";

/// A lightweight, lowering-time-only classification of a Java local
/// variable's/expression's static type — just enough to select the
/// correct SIR operator (`div_trunc` vs `div_true`, `StrConcat` vs
/// numeric `+`) and to reject nonsensical operand combinations (`"a" -
/// "b"`, `1 && 2`). This is *not* a real type checker: it assumes the
/// input is already valid, type-correct Java (as every other SIR
/// frontend assumes about its own input) and exists purely to recover
/// the handful of type-directed decisions Java's own compiler would make
/// implicitly. `Null` exists only transiently, as the kind of a bare
/// `null` literal — a variable's own tracked kind is always its
/// *declared* kind (`Str` for `String x = null;`), never `Null` itself
/// (see `lower_local_var_decl`'s handling of the `var x = null;` case,
/// which Java itself also rejects).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Int,
    Float,
    Bool,
    Str,
    Null,
    /// The "kind" of a call to a `void` method, or of a bare `return;` in
    /// one — M3a's addition. Not a real value kind (Java itself forbids
    /// using a void call as a value: `int x = voidMethod();` is a compile
    /// error) — this exists purely so `lower_expr`'s uniform `(Expr,
    /// Kind)` return shape has *something* to produce for a void call
    /// used as a bare statement (`voidMethod();`, whose result `Kind` is
    /// simply discarded by `lower_expr_statement`'s fallback path). Any
    /// attempt to use it as a real operand falls through to an ordinary
    /// "operands must be ..." rejection at whichever operator tried to
    /// consume it — the same "reject rather than mis-lower" discipline
    /// this crate uses everywhere else, not a dedicated special case.
    Void,
}

/// An error encountered during Java → SIR lowering.
///
/// Mirrors `MatlabLowerError`/`PythonLowerError`/`TwigLowerError`'s shape
/// exactly (`message` + 1-based `line`/`column`) so tooling can treat
/// every SIR frontend uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// Lower a parsed Java `program` CST into a [`Module`] named
/// `module_name`. See this module's own doc comment for the exact
/// supported subset (JV02 milestone M1).
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, JavaLowerError> {
    Lowerer::new(module_name).lower_program(tree)
}

struct Lowerer {
    module_name: String,
    /// Features observed while lowering, used to build the manifest so it
    /// declares *exactly* what the module emits (mirrors every other SIR
    /// frontend's own `observed` accumulator).
    observed: FeatureManifest,
    /// A stack of scope frames, innermost last. M2a introduces real block
    /// scoping (`if`/`while`/`do`-`while` bodies) — mirrors the SIR
    /// validator's own `LocalEnv` mark/rewind discipline exactly (see
    /// `semantic_ir::validator`'s `check_block`, which pushes a mark
    /// before a `Block`'s statements and rewinds it after): a name
    /// declared inside a nested block is visible only within that block
    /// and any blocks nested inside *it*, never after it ends nor to a
    /// sibling block. `push_scope`/`pop_scope`/`declare_local`/
    /// `lookup_local` below are this crate's own mirror of that stack,
    /// not merely a leak-prevention bookkeeping list — a Java program can
    /// shadow an outer local with the same name in a nested block-scoped
    /// language (in fact Java forbids that specific case, but this
    /// frontend is not a full type-checker — see [`Kind`]'s own doc
    /// comment — so a real lookup, not just presence-tracking, is what
    /// this needs).
    ///
    /// Each entry also carries the declaration's `Scope` tag (`Local` for
    /// an ordinary let/loop-variable binding, `Param` for a method
    /// parameter -- added in M3a) alongside its `Kind`: the SIR
    /// validator's own `check_varref` distinguishes the two
    /// (`Scope::Local` checks only against `let`-bound names,
    /// `Scope::Param` only against the function's own parameter list),
    /// so every `VarRef`/`Assign` this crate emits must carry the scope
    /// tag matching how the name was actually declared, not a blanket
    /// `Scope::Local`.
    locals: Vec<HashMap<String, (Kind, Scope)>>,
    /// Counter for the synthetic flag variable each `do`-`while` lowers
    /// (`__do_while_0`, `__do_while_1`, …) — see `lower_do_while_statement`'s
    /// own doc comment. Guarantees uniqueness across sibling do-while
    /// statements in the same function; never consulted by name lookup.
    do_while_counter: usize,
    /// Every method's resolved call signature (parameter kinds + return
    /// kind), computed in a first pass over the class body before *any*
    /// method body is lowered — mirrors `python-to-semantic-ir`'s/
    /// `javascript-to-semantic-ir`'s own two-pass precedent, so a call to
    /// a method declared later in the source (or to the enclosing method
    /// itself, i.e. recursion) resolves regardless of textual order.
    /// Keyed by method name; JV02 M3a supports at most one method per
    /// name (see `lower_program`'s own duplicate-name rejection —
    /// overload resolution is out of scope).
    method_signatures: HashMap<String, MethodSig>,
    /// Name of the method currently being lowered. Consulted only by
    /// `lower_call_expression` to record a `call_graph` edge — never used
    /// for scoping or name resolution.
    current_method: String,
    /// Call graph among top-level methods (`caller -> callees` by name),
    /// accumulated while lowering method bodies. Used once, after every
    /// method has been lowered, to detect `Feature::MutualRecursion` (a
    /// cycle of length ≥ 2) via `has_mutual_recursion` below.
    ///
    /// `HashMap`, not `Vec<(String, HashSet<String>)>`: an earlier
    /// version used a `Vec` and inserted a new call-graph edge with a
    /// linear `iter_mut().find(...)` scan over every method on *every*
    /// lowered call expression, making graph *construction* itself
    /// `O(V·E)` — reintroducing, in a different spot, the same class of
    /// algorithmic-complexity blowup `has_mutual_recursion`'s own
    /// `O(V+E)` DFS was written to eliminate (found by a second round of
    /// `/security-review`, on the first round's own fix). A `HashMap`
    /// gives `O(1)`-average insertion instead.
    call_graph: HashMap<String, HashSet<String>>,
}

/// A method's call-site-relevant signature: what `lower_call_expression`
/// needs to type-check a call and select its result `Kind`, computed
/// before any method body is lowered (see `method_signatures`'s own doc
/// comment). Deliberately *not* the same shape as a lowered `Function`'s
/// own `params: Vec<Param>` — this only needs each parameter's `Kind`,
/// not its name or span.
#[derive(Debug, Clone)]
struct MethodSig {
    param_kinds: Vec<Kind>,
    /// `Kind::Void` for a method with no return value.
    return_kind: Kind,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
            locals: Vec::new(),
            do_while_counter: 0,
            method_signatures: HashMap::new(),
            current_method: String::new(),
            call_graph: HashMap::new(),
        }
    }

    /// Enter a new innermost lexical scope. Every `Block` this crate
    /// lowers (`main`'s own top-level body, and every `if`/`while`/`do`-
    /// `while` body) gets exactly one push/pop pair around its own
    /// statement-lowering — see `lower_block_node`/`lower_body`.
    fn push_scope(&mut self) {
        self.locals.push(HashMap::new());
    }

    /// Leave the innermost scope; its declared names go out of scope.
    fn pop_scope(&mut self) {
        self.locals.pop();
    }

    /// Declare `name` in the *innermost* currently-active scope.
    fn declare_local(&mut self, name: String, kind: Kind) {
        self.locals
            .last_mut()
            .expect("declare_local called with no active scope (push_scope was not called)")
            .insert(name, (kind, Scope::Local));
    }

    /// Like `declare_local`, but tags the entry `Scope::Param` — used
    /// only by `lower_formal_parameters` (M3a). A method's parameters
    /// live in the very same scope frame as its body's own top-level
    /// locals (see `lower_method_declaration`'s own doc comment for why
    /// that is the *correct* shape, not a shortcut), so this shares
    /// `declare_local`'s storage; only the recorded `Scope` differs.
    fn declare_param(&mut self, name: String, kind: Kind) {
        self.locals
            .last_mut()
            .expect("declare_param called with no active scope (push_scope was not called)")
            .insert(name, (kind, Scope::Param));
    }

    /// Look up `name`, searching from the innermost scope outward —
    /// exactly the lexical-shadowing order a real name lookup needs.
    /// Returns the declaration's `Kind` *and* its `Scope` tag (`Local` or
    /// `Param`) — see the `locals` field's own doc comment for why both
    /// matter to every caller that goes on to build a `VarRef`/`Assign`.
    fn lookup_local(&self, name: &str) -> Option<(Kind, Scope)> {
        self.locals
            .iter()
            .rev()
            .find_map(|frame| frame.get(name).copied())
    }

    fn lower_program(&mut self, program: &GrammarASTNode) -> Result<Module, JavaLowerError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        let mut class_decls = Vec::new();
        collect_bounded(program, "class_declaration", 0, self, &mut class_decls)?;
        let class_decl = match class_decls.as_slice() {
            [only] => *only,
            [] => return Err(self.err_at(program, "expected one top-level class declaration, found none (JV02 M0 supports exactly one)".to_string())),
            _ => return Err(self.err_at(program, format!("expected exactly one top-level class declaration, found {} (JV02 M0 supports exactly one)", class_decls.len()))),
        };

        let method_decls = self.collect_class_methods(class_decl)?;

        // Pass 1: register every method's name + call signature before
        // any method body is lowered, so a call to a method declared
        // later in the source (or to the enclosing method itself, i.e.
        // recursion) resolves regardless of textual order — mirrors
        // python-to-semantic-ir's/javascript-to-semantic-ir's own
        // two-pass precedent.
        let mut order: Vec<(String, &GrammarASTNode)> = Vec::with_capacity(method_decls.len());
        for decl in &method_decls {
            let name = self.method_name(decl).ok_or_else(|| {
                self.err_at(
                    decl,
                    "malformed method declaration (missing name)".to_string(),
                )
            })?;
            if self.method_signatures.contains_key(&name) {
                return Err(self.err_at(
                    decl,
                    format!("duplicate method name `{name}` (JV02 M3a does not support overloading — only one method per name is supported so far)"),
                ));
            }
            let sig = self.compute_method_signature(&name, decl)?;
            self.method_signatures.insert(name.clone(), sig);
            order.push((name, decl));
        }
        if !self.method_signatures.contains_key("main") {
            return Err(self.err_at(
                class_decl,
                "expected a `main` method (JV02 M0 requires `public static void main(String[] args)`)"
                    .to_string(),
            ));
        }

        // Pass 2: lower each method's body into a `Function`, now that
        // every method's signature is already known.
        let mut functions = Vec::with_capacity(order.len());
        for (name, decl) in &order {
            functions.push(self.lower_method_declaration(name, decl)?);
        }

        if self.has_mutual_recursion() {
            self.observed.add(Feature::MutualRecursion);
        }

        let span = Span::point(FILE, 1, 1);
        let metadata = Metadata::new()
            .with_source_language("java")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION);

        Ok(Module {
            name: self.module_name.clone(),
            manifest: self.observed.clone(),
            imports: vec![],
            exports: vec![],
            functions,
            globals: vec![],
            metadata,
            span,
        })
    }

    /// Collect every `method_declaration` directly inside `class_decl`'s
    /// own `class_body`, rejecting any other class-member shape (field,
    /// constructor, static/instance initializer, nested type) with a
    /// clear, explicit error rather than silently skipping it — JV02
    /// M3a lowers methods only, and this crate's own standing discipline
    /// is to reject unsupported input loudly rather than mis-lower or
    /// drop it (see this module's own doc comment).
    ///
    /// Unlike M0's now-removed `find_main_method`, this needs no depth
    /// guard of its own: `class_body`'s grammar production (`LBRACE {
    /// class_body_declaration } RBRACE`) makes every relevant node a
    /// *direct* child of `class_body`, and `class_body_declaration`
    /// itself is a flat, single-level PEG alternation (`static_initializer
    /// | ... | method_declaration | ... | SEMICOLON`) — so this walk is
    /// two levels deep by construction, not by a runtime check. (A
    /// method's own *body* can of course still nest arbitrarily —
    /// `lower_method_body_block`'s `MAX_STMT_DEPTH` guard covers that,
    /// exactly as it already does for `main`.)
    fn collect_class_methods<'a>(
        &self,
        class_decl: &'a GrammarASTNode,
    ) -> Result<Vec<&'a GrammarASTNode>, JavaLowerError> {
        let class_body = self
            .first_child_named(class_decl, "class_body")
            .ok_or_else(|| {
                self.err_at(
                    class_decl,
                    "malformed class declaration (missing body)".to_string(),
                )
            })?;
        let mut methods = Vec::new();
        for cbd in child_nodes(class_body) {
            if let Some(m) = self.first_child_named(cbd, "method_declaration") {
                methods.push(m);
                continue;
            }
            let is_bare_semicolon =
                matches!(cbd.children.as_slice(), [ASTNodeOrToken::Token(t)] if t.value == ";");
            if is_bare_semicolon {
                continue;
            }
            return Err(self.err_at(
                cbd,
                "only method declarations are supported inside a class body so far (fields, constructors, static/instance initializers, and nested types are deferred to later JV02 milestones)".to_string(),
            ));
        }
        Ok(methods)
    }

    fn method_name(&self, method_decl: &GrammarASTNode) -> Option<String> {
        let declarator = self.first_child_named(method_decl, "method_declarator")?;
        for child in &declarator.children {
            if let ASTNodeOrToken::Token(t) = child {
                if t.type_ == lexer::token::TokenType::Name {
                    return Some(t.value.clone());
                }
            }
        }
        None
    }

    /// Compute `name`'s call-site signature (parameter kinds + return
    /// kind) without lowering (or scoping) anything — called from pass 1,
    /// before any method body exists to reference it. `main` is special-
    /// cased: its own `String[] args` parameter is never lowered as a
    /// real SIR parameter (M0's own established convention — array types
    /// are out of scope, and nothing in this milestone can call `main`
    /// meaningfully anyway), so its signature is simply "zero parameters,
    /// void".
    fn compute_method_signature(
        &self,
        name: &str,
        decl: &GrammarASTNode,
    ) -> Result<MethodSig, JavaLowerError> {
        if name == "main" {
            return Ok(MethodSig {
                param_kinds: vec![],
                return_kind: Kind::Void,
            });
        }
        let declarator = self
            .first_child_named(decl, "method_declarator")
            .ok_or_else(|| {
                self.err_at(
                    decl,
                    "malformed method declaration (missing declarator)".to_string(),
                )
            })?;
        let param_kinds = self
            .formal_parameter_kind_name_pairs(declarator)?
            .into_iter()
            .map(|(_, kind)| kind)
            .collect();
        let return_kind = self.method_return_kind(decl)?;
        Ok(MethodSig {
            param_kinds,
            return_kind,
        })
    }

    /// Resolve `method_declaration`'s own `result_type` (`"void" |
    /// type`) to a [`Kind`], reusing `kind_of_type_node` for the non-void
    /// case (identical rejection rules: no array types, only `String`
    /// among reference types).
    fn method_return_kind(&self, decl: &GrammarASTNode) -> Result<Kind, JavaLowerError> {
        let result_type = self.first_child_named(decl, "result_type").ok_or_else(|| {
            self.err_at(
                decl,
                "malformed method declaration (missing result type)".to_string(),
            )
        })?;
        let is_void = result_type
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "void"));
        if is_void {
            return Ok(Kind::Void);
        }
        let ty = self
            .first_child_named(result_type, "type")
            .ok_or_else(|| self.err_at(result_type, "malformed result type".to_string()))?;
        self.kind_of_type_node(ty)
    }

    /// Extract each parameter's `(name, Kind)` pair from a
    /// `method_declarator`'s optional `formal_parameter_list`, without
    /// declaring anything in scope — the read-only half shared by
    /// `compute_method_signature` (pass 1, kinds only) and
    /// `lower_formal_parameters` (pass 2, which additionally calls
    /// `declare_local` and builds real `Param`s). Rejects varargs and
    /// C-style array-bracket parameter declarators (`int x[]`) — both
    /// deferred, matching this crate's existing array-type scope
    /// boundary.
    fn formal_parameter_kind_name_pairs(
        &self,
        declarator: &GrammarASTNode,
    ) -> Result<Vec<(String, Kind)>, JavaLowerError> {
        let Some(list) = self.first_child_named(declarator, "formal_parameter_list") else {
            return Ok(vec![]);
        };
        if child_nodes(list)
            .into_iter()
            .any(|n| n.rule_name == "varargs_parameter")
        {
            return Err(self.err_at(
                list,
                "varargs parameters (`...`) are not supported yet (deferred to a later JV02 milestone)".to_string(),
            ));
        }
        let mut out = Vec::new();
        for fp in child_nodes(list)
            .into_iter()
            .filter(|n| n.rule_name == "formal_parameter")
        {
            let has_array_brackets = fp
                .children
                .iter()
                .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "["));
            if has_array_brackets {
                return Err(self.err_at(
                    fp,
                    "C-style array parameter brackets (`int x[]`) are not supported yet (deferred to JV02 M4)".to_string(),
                ));
            }
            let ty = self
                .first_child_named(fp, "type")
                .ok_or_else(|| self.err_at(fp, "malformed parameter (missing type)".to_string()))?;
            let kind = self.kind_of_type_node(ty)?;
            let name_tok = fp
                .children
                .iter()
                .find_map(|c| match c {
                    ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Name => Some(t),
                    _ => None,
                })
                .ok_or_else(|| self.err_at(fp, "malformed parameter (missing name)".to_string()))?;
            out.push((name_tok.value.clone(), kind));
        }
        Ok(out)
    }

    /// Lower one `method_declaration` (already registered in
    /// `method_signatures` by pass 1) into a [`Function`]. Params and the
    /// body share one flat scope (params declared, then the body's own
    /// top-level statements lowered into the *same* frame — not a nested
    /// scope of their own) because that is Java's actual rule: a method
    /// body may not redeclare a parameter name, so there is no shadowing
    /// case for a separate frame to model.
    fn lower_method_declaration(
        &mut self,
        name: &str,
        decl: &GrammarASTNode,
    ) -> Result<Function, JavaLowerError> {
        let span = self.span_of(decl);
        let is_main = name == "main";
        let declarator = self
            .first_child_named(decl, "method_declarator")
            .ok_or_else(|| {
                self.err_at(
                    decl,
                    "malformed method declaration (missing declarator)".to_string(),
                )
            })?;
        let has_trailing_array_brackets = declarator
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "["));
        if has_trailing_array_brackets {
            return Err(self.err_at(
                declarator,
                "C-style array-return-type declarators (`int foo()[]`) are not supported yet (deferred to JV02 M4)".to_string(),
            ));
        }

        self.current_method = name.to_string();
        self.call_graph.insert(name.to_string(), HashSet::new());

        self.push_scope();
        let params = if is_main {
            vec![]
        } else {
            match self.lower_formal_parameters(declarator) {
                Ok(p) => p,
                Err(e) => {
                    self.pop_scope();
                    return Err(e);
                }
            }
        };

        let method_body = self.first_child_named(decl, "method_body").ok_or_else(|| {
            self.err_at(
                decl,
                "malformed method declaration (missing body)".to_string(),
            )
        })?;
        let block = match self.first_child_named(method_body, "block") {
            Some(b) => b,
            None => {
                self.pop_scope();
                return Err(self.err_at(
                    method_body,
                    format!(
                        "method `{name}` has no body (abstract/native methods are not supported)"
                    ),
                ));
            }
        };
        let return_kind = self
            .method_signatures
            .get(name)
            .expect("signature already computed in pass 1")
            .return_kind;
        let body = match self.lower_method_body_block(block, return_kind, 0) {
            Ok(b) => b,
            Err(e) => {
                self.pop_scope();
                return Err(e);
            }
        };
        self.pop_scope();

        Ok(Function {
            name: name.to_string(),
            params,
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span,
        })
    }

    /// Lower a `method_declarator`'s own `formal_parameter_list` into
    /// real [`Param`]s, declaring each one in the *current* (already-
    /// pushed, by the caller) innermost scope as it goes — the mutating
    /// counterpart of `formal_parameter_kind_name_pairs`.
    fn lower_formal_parameters(
        &mut self,
        declarator: &GrammarASTNode,
    ) -> Result<Vec<Param>, JavaLowerError> {
        let pairs = self.formal_parameter_kind_name_pairs(declarator)?;
        let span = self.span_of(declarator);
        let mut params = Vec::with_capacity(pairs.len());
        for (name, kind) in pairs {
            self.declare_param(name.clone(), kind);
            self.observed.add(Feature::DynamicTyping);
            params.push(Param {
                name,
                sir_type: None,
                kind: ParamKind::Required,
                default: None,
                span: span.clone(),
            });
        }
        Ok(params)
    }

    /// Lower a method body's `block` node directly into that method's
    /// own [`Block`] value (unlike `lower_block_node`, this does *not*
    /// push its own scope — see `lower_method_declaration`'s own doc
    /// comment for why params and the body share one flat frame), while
    /// also handling `return`: SIR has no `Stmt::Return` primitive at all
    /// (a function's value is always its own body `Block.value` —
    /// confirmed by an exhaustive grep of the `Stmt` enum) so a Java
    /// `return` is accepted *only* as the literal last top-level
    /// statement of the method body, becoming that `Block`'s own
    /// `value`. A `return` appearing anywhere else — nested inside an
    /// `if`/`while`/`for`/etc. body, or textually followed by more
    /// statements — falls straight through to `lower_statement`'s own
    /// existing "unsupported statement kind" rejection (a `return` is
    /// simply not one of the alternatives that dispatcher recognizes),
    /// cleanly rejecting genuine early/branched returns as a real,
    /// disclosed JV02 M3a scope limit rather than silently mis-lowering
    /// them.
    fn lower_method_body_block(
        &mut self,
        block: &GrammarASTNode,
        return_kind: Kind,
        depth: usize,
    ) -> Result<Block, JavaLowerError> {
        if depth >= MAX_STMT_DEPTH {
            return Err(self.err_at(
                block,
                format!("statement/block nesting exceeds {MAX_STMT_DEPTH} levels"),
            ));
        }
        let span = self.span_of(block);
        let block_stmts: Vec<&GrammarASTNode> = child_nodes(block)
            .into_iter()
            .filter(|n| n.rule_name == "block_statement")
            .collect();
        let mut stmts = Vec::with_capacity(block_stmts.len());
        let mut value = Expr::NilLit { span: span.clone() };
        for (i, block_stmt) in block_stmts.iter().enumerate() {
            let is_last = i + 1 == block_stmts.len();
            if let Some(ret) = self.find_return_statement_direct(block_stmt) {
                if !is_last {
                    return Err(self.err_at(
                        ret,
                        "`return` is only supported as the last statement of a method body (an early or branched return is deferred to a later JV02 milestone)".to_string(),
                    ));
                }
                let ret_expr_node = self.first_child_named(ret, "expression");
                match (ret_expr_node, return_kind) {
                    (Some(_), Kind::Void) => {
                        return Err(self.err_at(
                            ret,
                            "`return <expr>;` is not supported in a `void` method".to_string(),
                        ));
                    }
                    (None, Kind::Void) => {}
                    (None, _) => {
                        return Err(self.err_at(
                            ret,
                            "`return;` requires an expression in a non-`void` method".to_string(),
                        ));
                    }
                    (Some(expr_node), expected_kind) => {
                        let (expr, kind) = self.lower_expr(expr_node, 0)?;
                        if kind != expected_kind {
                            return Err(self.err_at(
                                expr_node,
                                "returned expression's kind does not match the method's declared return type".to_string(),
                            ));
                        }
                        value = expr;
                    }
                }
                break;
            }
            stmts.push(self.lower_block_statement(block_stmt, depth + 1)?);
        }
        Ok(Block { stmts, value, span })
    }

    /// If `block_stmt` (a `block_statement`) directly wraps a `statement
    /// -> return_statement`, return that `return_statement` node.
    /// Deliberately shallow (does not look inside a nested `if`/`while`/
    /// etc. body) — `lower_method_body_block` only ever calls this on a
    /// method body's own *top-level* statements, by construction.
    fn find_return_statement_direct<'a>(
        &self,
        block_stmt: &'a GrammarASTNode,
    ) -> Option<&'a GrammarASTNode> {
        let statement = self.first_child_named(block_stmt, "statement")?;
        self.first_child_named(statement, "return_statement")
    }

    /// Does the call graph contain a cycle of length ≥ 2 (two or more
    /// methods that call each other, however indirectly)? Plain self-
    /// recursion (`f` calling `f` directly) does not count.
    ///
    /// A single linear-time (`O(V+E)`) three-color DFS, not the naive
    /// "probe every edge with its own reachability search" approach
    /// (`O(E·(V+E))`, which `python-to-semantic-ir`'s otherwise-
    /// identically-purposed `has_mutual_recursion`/`reaches` pair uses):
    /// found by `/security-review` as a real algorithmic-complexity DoS
    /// risk (CWE-407) on a large, densely-interconnected call graph (many
    /// methods each calling many others) — unlike this crate's other
    /// guarded traversals, nothing bounds the number of *sibling* methods
    /// in one class body, so the naive approach's edge-count-squared
    /// blowup was reachable from ordinary (if very large) valid Java
    /// source, not just an adversarial hand-built tree. A back edge to a
    /// node still `Gray` (on the current DFS path) is exactly a cycle;
    /// skipping an edge from a node to itself is what excludes plain
    /// self-recursion. Implemented with an explicit work-stack, not real
    /// recursion — the number of methods in one class is not otherwise
    /// bounded, so a real call stack here would reintroduce the same
    /// class of uncontrolled-recursion risk this crate's other depth
    /// guards exist to prevent.
    fn has_mutual_recursion(&self) -> bool {
        #[derive(Clone, Copy, PartialEq)]
        enum Color {
            White,
            Gray,
            Black,
        }

        let graph: HashMap<&str, Vec<&str>> = self
            .call_graph
            .iter()
            .map(|(n, callees)| (n.as_str(), callees.iter().map(|c| c.as_str()).collect()))
            .collect();
        let mut color: HashMap<&str, Color> = graph.keys().map(|k| (*k, Color::White)).collect();

        for &start in graph.keys() {
            if color[start] != Color::White {
                continue;
            }
            let mut stack: Vec<(&str, usize)> = vec![(start, 0)];
            color.insert(start, Color::Gray);
            while let Some(&(node, idx)) = stack.last() {
                let children = &graph[node];
                if idx < children.len() {
                    let child = children[idx];
                    stack.last_mut().expect("just matched Some above").1 += 1;
                    if child == node {
                        continue; // self-loop -- plain self-recursion, not mutual
                    }
                    match color.get(child).copied().unwrap_or(Color::White) {
                        Color::White => {
                            color.insert(child, Color::Gray);
                            stack.push((child, 0));
                        }
                        Color::Gray => return true, // back edge to a distinct on-stack node -> real cycle
                        Color::Black => {}
                    }
                } else {
                    color.insert(node, Color::Black);
                    stack.pop();
                }
            }
        }
        false
    }

    // ── statement/block-level lowering ──────────────────────────────

    /// Lower a `block` node (`LBRACE { block_statement } RBRACE`) into a
    /// [`Block`], pushing a fresh scope for its own statements and
    /// popping it before returning — the scope boundary a Java `{ }`
    /// itself introduces, mirroring the SIR validator's own `check_block`
    /// mark/rewind exactly (see the `locals` field's own doc comment).
    fn lower_block_node(
        &mut self,
        block_node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Block, JavaLowerError> {
        if depth >= MAX_STMT_DEPTH {
            return Err(self.err_at(
                block_node,
                format!("statement/block nesting exceeds {MAX_STMT_DEPTH} levels"),
            ));
        }
        let span = self.span_of(block_node);
        self.push_scope();
        let mut stmts = Vec::new();
        for block_stmt in child_nodes(block_node) {
            if block_stmt.rule_name != "block_statement" {
                continue;
            }
            match self.lower_block_statement(block_stmt, depth + 1) {
                Ok(s) => stmts.push(s),
                Err(e) => {
                    self.pop_scope();
                    return Err(e);
                }
            }
        }
        self.pop_scope();
        Ok(Block {
            stmts,
            value: Expr::NilLit { span: span.clone() },
            span,
        })
    }

    /// Lower a `statement` node used in a *body position* (the sole
    /// statement after an `if`/`while`/`do` with no braces, e.g. `if (x)
    /// foo();`) into a [`Block`]. If the statement is itself a brace-
    /// delimited `block`, delegates straight to `lower_block_node`
    /// (avoiding a redundant extra scope layer); otherwise wraps the one
    /// lowered statement in a fresh scope of its own — Java gives even a
    /// brace-less body its own scope for any (rare, since M1 requires an
    /// initializer and a bare non-declaration statement can't introduce a
    /// name) declaration it might contain.
    fn lower_body(
        &mut self,
        stmt_node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Block, JavaLowerError> {
        if depth >= MAX_STMT_DEPTH {
            return Err(self.err_at(
                stmt_node,
                format!("statement/block nesting exceeds {MAX_STMT_DEPTH} levels"),
            ));
        }
        if let Some(block_node) = self.first_child_named(stmt_node, "block") {
            return self.lower_block_node(block_node, depth + 1);
        }
        let span = self.span_of(stmt_node);
        self.push_scope();
        let stmt = self.lower_statement(stmt_node, depth + 1);
        self.pop_scope();
        Ok(Block {
            stmts: vec![stmt?],
            value: Expr::NilLit { span: span.clone() },
            span,
        })
    }

    /// Lower one `block_statement`. `block_statement = var_declaration |
    /// class_declaration | statement`, and (a grammar quirk) `statement`
    /// itself *also* lists `var_declaration` as one of its own
    /// alternatives — both positions are checked so a local variable
    /// declaration is recognized regardless of which alternative the
    /// parser actually took.
    fn lower_block_statement(
        &mut self,
        block_stmt: &GrammarASTNode,
        depth: usize,
    ) -> Result<Stmt, JavaLowerError> {
        if let Some(var_decl) = self.first_child_named(block_stmt, "var_declaration") {
            return self.lower_var_declaration_node(var_decl);
        }
        let statement = self
            .first_child_named(block_stmt, "statement")
            .ok_or_else(|| {
                self.err_at(
                    block_stmt,
                    "unsupported statement kind (JV02 M2a does not lower this yet)".to_string(),
                )
            })?;
        self.lower_statement(statement, depth)
    }

    /// Lower a `statement` node (dispatching on which alternative of
    /// `statement = block | var_declaration | ... | if_statement | ...`
    /// the parser took) into a single [`Stmt`]. Shared by
    /// `lower_block_statement` (a statement inside a `{ }`) and
    /// `lower_body` (a brace-less single-statement body).
    fn lower_statement(
        &mut self,
        statement: &GrammarASTNode,
        depth: usize,
    ) -> Result<Stmt, JavaLowerError> {
        if depth >= MAX_STMT_DEPTH {
            return Err(self.err_at(
                statement,
                format!("statement/block nesting exceeds {MAX_STMT_DEPTH} levels"),
            ));
        }
        if let Some(var_decl) = self.first_child_named(statement, "var_declaration") {
            return self.lower_var_declaration_node(var_decl);
        }
        if let Some(if_stmt) = self.first_child_named(statement, "if_statement") {
            return self.lower_if_statement(if_stmt, depth);
        }
        if let Some(while_stmt) = self.first_child_named(statement, "while_statement") {
            return self.lower_while_statement(while_stmt, depth);
        }
        if let Some(do_while_stmt) = self.first_child_named(statement, "do_while_statement") {
            return self.lower_do_while_statement(do_while_stmt, depth);
        }
        if let Some(for_stmt) = self.first_child_named(statement, "for_statement") {
            return self.lower_for_statement(for_stmt, depth);
        }
        if let Some(enhanced_for_stmt) = self.first_child_named(statement, "enhanced_for_statement")
        {
            return self.lower_enhanced_for_statement(enhanced_for_stmt, depth);
        }
        if let Some(expr_stmt) = self.first_child_named(statement, "expression_statement") {
            let expression = self
                .first_child_named(expr_stmt, "expression")
                .ok_or_else(|| {
                    self.err_at(
                        expr_stmt,
                        "expression statement has no expression".to_string(),
                    )
                })?;
            return self.lower_expr_statement(expression);
        }
        Err(self.err_at(
            statement,
            "unsupported statement kind (JV02 M2b supports variable declarations, assignment, if/while/do-while/for/enhanced-for, and bare expression statements — switch/break/continue have no SIR IR yet, everything else is deferred further)"
                .to_string(),
        ))
    }

    /// `if_statement = "if" LPAREN expression RPAREN statement [ "else"
    /// statement ] ;`. Lowers to `Stmt::ExprStmt` wrapping `Expr::If` —
    /// the IR's conditional is an expression with no statement-level
    /// counterpart (see `Expr::If`'s own doc comment), the same
    /// convention `javascript-to-semantic-ir`/`ruby-to-semantic-ir` use.
    /// An absent `else` becomes a synthetic empty, `NilLit`-valued block.
    fn lower_if_statement(
        &mut self,
        if_stmt: &GrammarASTNode,
        depth: usize,
    ) -> Result<Stmt, JavaLowerError> {
        let span = self.span_of(if_stmt);
        let cond_node = self
            .first_child_named(if_stmt, "expression")
            .ok_or_else(|| {
                self.err_at(if_stmt, "malformed `if` (missing condition)".to_string())
            })?;
        let (cond, cond_kind) = self.lower_expr(cond_node, 0)?;
        if cond_kind != Kind::Bool {
            return Err(self.err_at(cond_node, "`if` condition must be boolean".to_string()));
        }
        let branches: Vec<&GrammarASTNode> = child_nodes(if_stmt)
            .into_iter()
            .filter(|n| n.rule_name == "statement")
            .collect();
        let (then_stmt, else_stmt) = match branches.as_slice() {
            [then] => (*then, None),
            [then, els] => (*then, Some(*els)),
            _ => {
                return Err(self.err_at(
                    if_stmt,
                    "malformed `if` (unexpected statement count)".to_string(),
                ))
            }
        };
        let then_branch = self.lower_body(then_stmt, depth + 1)?;
        let else_branch = match else_stmt {
            Some(e) => self.lower_body(e, depth + 1)?,
            None => Block {
                stmts: vec![],
                value: Expr::NilLit { span: span.clone() },
                span: span.clone(),
            },
        };
        Ok(Stmt::ExprStmt {
            expr: Expr::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
                span: span.clone(),
            },
            span,
        })
    }

    /// `while_statement = "while" LPAREN expression RPAREN statement ;`
    fn lower_while_statement(
        &mut self,
        while_stmt: &GrammarASTNode,
        depth: usize,
    ) -> Result<Stmt, JavaLowerError> {
        let span = self.span_of(while_stmt);
        let cond_node = self
            .first_child_named(while_stmt, "expression")
            .ok_or_else(|| {
                self.err_at(
                    while_stmt,
                    "malformed `while` (missing condition)".to_string(),
                )
            })?;
        let (cond, cond_kind) = self.lower_expr(cond_node, 0)?;
        if cond_kind != Kind::Bool {
            return Err(self.err_at(cond_node, "`while` condition must be boolean".to_string()));
        }
        let body_stmt = self
            .first_child_named(while_stmt, "statement")
            .ok_or_else(|| {
                self.err_at(while_stmt, "malformed `while` (missing body)".to_string())
            })?;
        let body = self.lower_body(body_stmt, depth + 1)?;
        self.observed.add(Feature::Loops);
        Ok(Stmt::While { cond, body, span })
    }

    /// `do_while_statement = "do" statement "while" LPAREN expression
    /// RPAREN SEMICOLON ;`. SIR's `Stmt::While` is pretest-only (there is
    /// no do-while primitive), so this desugars `do S while (C);` to a
    /// synthetic flag-guarded pretest loop — `boolean __do_while_N =
    /// true; while (__do_while_N || C) { S; __do_while_N = false; }` —
    /// lowering `S` exactly **once**.
    ///
    /// An earlier version instead built the more literal `{ S; while (C)
    /// S }` shape by lowering `S` once and *cloning* its already-lowered
    /// `Block.stmts` for the second copy. `/security-review` caught that
    /// as a real resource-exhaustion DoS: cloning duplicates whatever
    /// nested `do`/`while` structure `S` *itself* already contains, so
    /// `N` levels of nested `do`/`while` — ordinary, valid, brace-less
    /// Java source, no adversarial tree needed — produced `O(2^N)`
    /// emitted IR nodes from `O(N)` source bytes (the same amplification
    /// shape as XML "billion laughs"), invisible to `MAX_STMT_DEPTH`
    /// since the blowup happens on each stack frame's *return* (the
    /// clone), not from call-stack recursion depth. The flag-guarded
    /// rewrite here has no clone at all, so emitted IR size is always
    /// linear in source size.
    ///
    /// `__do_while_N`'s uniqueness comes from `do_while_counter` (a
    /// monotonic per-`Lowerer` counter — two sibling do-while statements
    /// in the same function must not share a flag) *and* a collision
    /// check against every currently-visible name via `lookup_local`:
    /// the flag lives in the enclosing scope for the duration of the
    /// synthetic `Expr::Block` (it is not itself scope-pushed), so it
    /// must not collide with any real Java local already in scope at
    /// this point — `__do_while_0` is a legal Java identifier, so a
    /// program that happens to declare a variable by that exact name is
    /// a real, reachable case, not a hypothetical one.
    fn lower_do_while_statement(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Stmt, JavaLowerError> {
        let span = self.span_of(node);
        let body_stmt = self.first_child_named(node, "statement").ok_or_else(|| {
            self.err_at(node, "malformed `do`/`while` (missing body)".to_string())
        })?;
        let mut body = self.lower_body(body_stmt, depth + 1)?;
        let cond_node = self.first_child_named(node, "expression").ok_or_else(|| {
            self.err_at(
                node,
                "malformed `do`/`while` (missing condition)".to_string(),
            )
        })?;
        let (cond, cond_kind) = self.lower_expr(cond_node, 0)?;
        if cond_kind != Kind::Bool {
            return Err(self.err_at(
                cond_node,
                "`do`/`while` condition must be boolean".to_string(),
            ));
        }
        self.observed.add(Feature::Loops);
        self.observed.add(Feature::ShortCircuit);
        self.observed.add(Feature::MutableBindings);

        // Desugar `do S while (C);` to a flag-guarded pretest loop, NOT a
        // literal "run S once, then while (C) S" duplication of S's own
        // already-lowered IR. An earlier version of this desugaring did
        // clone the lowered body for the once-executed copy — caught by
        // `/security-review` as a real resource-exhaustion DoS: cloning
        // duplicates whatever nested do-while structure S *itself*
        // already contains, so N levels of nested do-while (valid,
        // ordinary, brace-less Java source — no adversarial tree needed)
        // produce O(2^N) emitted IR nodes from O(N) source bytes, the
        // same amplification-attack shape as XML "billion laughs". The
        // `MAX_STMT_DEPTH` guard does not catch this: the blowup happens
        // on each stack frame's *return* (the clone), not from the call-
        // stack depth itself, which stays correctly bounded throughout.
        //
        // This rewrite lowers S exactly once — no cloning — so emitted
        // IR size is always linear in source size, regardless of
        // nesting depth:
        //
        //   boolean __do_while_N = true;
        //   while (__do_while_N || C) { S; __do_while_N = false; }
        //
        // `__do_while_N`'s uniqueness comes from a per-`Lowerer` counter
        // (`do_while_counter`), not just its `__`-prefix: two sibling
        // do-while statements in the same function must not share a
        // flag. It is never *declared* via `declare_local` (real Java
        // source can never look this name up), but it still must not
        // *collide* with a real Java local visible at either of the two
        // points it's referenced — `__do_while_0` is a legal Java
        // identifier, so a program that happens to declare a variable by
        // that exact name is a real, reachable case, checked against
        // two different scopes for two different reasons (both caught
        // by `/security-review`, in two separate rounds — an earlier
        // version checked neither):
        //  - `lookup_local` — the *ambient* scope active right here,
        //    before `S` is lowered — covers a same-named local declared
        //    in an *enclosing* scope (the flag declaration and its
        //    `flag || C` reference both live in that ambient scope).
        //  - `body_declares_name` — `S`'s own *top-level* statements
        //    (already lowered into `body.stmts` above) — covers a
        //    same-named local `S` declares directly (not nested inside
        //    a further sub-block of `S`, which would be a distinct,
        //    already-popped inner scope of its own): the appended
        //    `__do_while_N = false;` flag-clear lives at exactly that
        //    top level, so it's the one place body-local collisions can
        //    actually reach it. `lookup_local` alone can't see this: by
        //    the time this code runs, `lower_body`'s own scope for `S`
        //    has already been pushed *and popped* (that's the correct,
        //    real Java scope boundary — a `do`/`while` body's own locals
        //    must not leak past it), so a name `S` declares is already
        //    gone from `self.locals` again. Missing this check would
        //    leave the flag-clear assignment resolving to `S`'s own
        //    shadowing local under any backend with real block scoping
        //    (unlike this crate's own Python execution-proof harness,
        //    whose flat function-level scoping happens not to manifest
        //    the bug for the cases tried) — silently leaving the outer
        //    flag never cleared, so `flag || C` never goes false: an
        //    infinite loop, the same DoS-by-nontermination class as the
        //    exponential-blowup finding this desugaring already fixed
        //    once, reached a different way.
        let mut flag_name = format!("__do_while_{}", self.do_while_counter);
        self.do_while_counter += 1;
        while self.lookup_local(&flag_name).is_some() || body_declares_name(&body, &flag_name) {
            flag_name = format!("__do_while_{}", self.do_while_counter);
            self.do_while_counter += 1;
        }

        let flag_decl = Stmt::LetStarBinding {
            name: flag_name.clone(),
            sir_type: None,
            value: Expr::BoolLit {
                value: true,
                span: span.clone(),
            },
            span: span.clone(),
        };
        let flag_ref = Expr::VarRef {
            name: flag_name.clone(),
            scope: Scope::Local,
            span: span.clone(),
        };
        let loop_cond = Expr::LogicalOr {
            lhs: Box::new(flag_ref),
            rhs: Box::new(cond),
            span: span.clone(),
        };
        body.stmts.push(Stmt::Assign {
            name: flag_name,
            scope: Scope::Local,
            value: Expr::BoolLit {
                value: false,
                span: span.clone(),
            },
            span: span.clone(),
        });

        Ok(Stmt::ExprStmt {
            expr: Expr::Block(Box::new(Block {
                stmts: vec![
                    flag_decl,
                    Stmt::While {
                        cond: loop_cond,
                        body,
                        span: span.clone(),
                    },
                ],
                value: Expr::NilLit { span: span.clone() },
                span: span.clone(),
            })),
            span,
        })
    }

    /// `for_statement = "for" LPAREN for_init SEMICOLON [ expression ]
    /// SEMICOLON [ for_update ] RPAREN statement ;`. SIR's `Stmt::ForRange`
    /// is a canonical `for var in range(start, stop, step)` counting loop
    /// — too narrow a shape to represent Java's fully general three-clause
    /// `for` (an arbitrary init/cond/update, not necessarily a simple
    /// increasing counter). Desugars to `Stmt::While` instead, mirroring
    /// `c-to-semantic-ir`'s own precedent for C's identically-general
    /// `for` (chosen over `javascript-to-semantic-ir`'s stricter
    /// canonical-`ForRange`-only-else-reject approach, since Java's
    /// classic `for` is highly variable in shape):
    ///
    ///   { init; while (cond) { S; update; } }
    ///
    /// wrapped in one synthetic `Expr::Block` — matching `do`/`while`'s
    /// own established wrapping pattern — so `init`'s own scope (the loop
    /// variable, if it's a declaration) spans the whole construct but
    /// ends exactly where Java's own `for` scope does, not the
    /// surrounding function. Delegates the "run once, wrapped correctly"
    /// scope bookkeeping to `lower_for_statement_inner`, which this
    /// wrapper always calls with the scope pushed and popped around it —
    /// pushed *before* `init` (`init`'s own declared variable must be
    /// visible in `cond`, `update`, and the body, all of which are
    /// lowered inside `_inner`) and popped only after everything below
    /// has finished, including on an error return.
    fn lower_for_statement(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Stmt, JavaLowerError> {
        self.push_scope();
        let result = self.lower_for_statement_inner(node, depth);
        self.pop_scope();
        result
    }

    fn lower_for_statement_inner(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Stmt, JavaLowerError> {
        let span = self.span_of(node);
        let for_init = self.first_child_named(node, "for_init").ok_or_else(|| {
            self.err_at(node, "malformed `for` (missing init clause)".to_string())
        })?;
        let init_stmt = self.lower_for_init(for_init)?;

        let cond = match self.first_child_named(node, "expression") {
            Some(cond_node) => {
                let (c, k) = self.lower_expr(cond_node, 0)?;
                if k != Kind::Bool {
                    return Err(
                        self.err_at(cond_node, "`for` condition must be boolean".to_string())
                    );
                }
                c
            }
            // An absent condition means "loop forever" (`for (;;)`),
            // exactly like `c-to-semantic-ir`'s own identically-reasoned
            // handling of C's own absent-condition `for`.
            None => Expr::BoolLit {
                value: true,
                span: span.clone(),
            },
        };

        let update_stmt = self
            .first_child_named(node, "for_update")
            .map(|fu| self.lower_for_update(fu))
            .transpose()?;

        let body_stmt = self
            .first_child_named(node, "statement")
            .ok_or_else(|| self.err_at(node, "malformed `for` (missing body)".to_string()))?;
        let mut body = self.lower_body(body_stmt, depth + 1)?;
        if let Some(u) = update_stmt {
            // `update` is spliced onto the *end* of `body.stmts`, sharing
            // one flat scope with whatever `body` itself already declared
            // at its own top level — by the time we're here, `lower_body`
            // has already pushed and popped `body`'s own scope (the
            // correct real Java scope boundary), so `self.lookup_local`
            // can no longer see anything `body` declared. If `body`
            // redeclares the exact name `update` assigns to (e.g. `for
            // (int i = 0; i < 3; i++) { int i = 999; ... }`), that
            // redeclaration would shadow the real loop control variable
            // for the appended update under any backend with real block
            // scoping — the update would silently mutate the *body's own*
            // local instead, leaving the real loop variable permanently
            // unincremented (an infinite loop) — caught by
            // `/security-review`, the same "collision checked before the
            // colliding scope existed" bug class `lower_do_while_statement`
            // already needed two rounds of fixes for. Real Java rejects
            // this exact source outright (`variable i is already
            // defined`), so rejecting it here loses no real program's
            // ability to compile; `Stmt::Assign` is the only shape
            // `update_stmt` collision-checking needs to cover, since
            // that's the only shape `lower_for_update`
            // (`lower_expr_statement`) ever produces for an assignment/
            // compound-assignment/increment-decrement update clause — an
            // update clause that's just a bare value expression
            // (`Stmt::ExprStmt`, e.g. a pointless `for (...; ...; 5)`)
            // assigns to no name at all, so there is nothing to collide.
            if let Stmt::Assign { name, .. } = &u {
                if body_declares_name(&body, name) {
                    return Err(self.err_at(
                        node,
                        format!(
                            "the `for` loop's own update clause target `{name}` is shadowed by a variable the loop body declares directly — rename one of them"
                        ),
                    ));
                }
            }
            body.stmts.push(u);
        }
        self.observed.add(Feature::Loops);

        let while_stmt = Stmt::While {
            cond,
            body,
            span: span.clone(),
        };
        let mut outer_stmts = Vec::new();
        if let Some(i) = init_stmt {
            outer_stmts.push(i);
        }
        outer_stmts.push(while_stmt);
        Ok(Stmt::ExprStmt {
            expr: Expr::Block(Box::new(Block {
                stmts: outer_stmts,
                value: Expr::NilLit { span: span.clone() },
                span: span.clone(),
            })),
            span,
        })
    }

    /// `for_init = {annotation} [final] local_var_type
    /// variable_declarators | [expression_list] ;` — the empty
    /// alternative (`for (;;)`) is `for_init` with zero children; the
    /// bare-expression alternative (`for (i = 0; ...)`, no declaration)
    /// wraps its `expression_list` directly (no `local_var_type`
    /// sibling); the declaration alternative has `local_var_type` and
    /// `variable_declarators` as direct children, structurally identical
    /// to `local_variable_declaration_statement`'s own pair minus the
    /// wrapping node and trailing `SEMICOLON` — `lower_variable_declarator`
    /// (shared with `lower_local_var_decl`) handles the actual lowering
    /// either way.
    fn lower_for_init(
        &mut self,
        for_init: &GrammarASTNode,
    ) -> Result<Option<Stmt>, JavaLowerError> {
        if for_init.children.is_empty() {
            return Ok(None);
        }
        if let Some(lvt) = self.first_child_named(for_init, "local_var_type") {
            let declared_kind = self.declared_kind_of_local_var_type(lvt)?;
            let declarators = self
                .first_child_named(for_init, "variable_declarators")
                .ok_or_else(|| {
                    self.err_at(
                        for_init,
                        "malformed `for` init (missing declarators)".to_string(),
                    )
                })?;
            let declarator = self.single_variable_declarator(declarators)?;
            return self
                .lower_variable_declarator(declared_kind, declarator, for_init)
                .map(Some);
        }
        if let Some(expr_list) = self.first_child_named(for_init, "expression_list") {
            let expr = self.single_expression_in_list(expr_list)?;
            return self.lower_expr_statement(expr).map(Some);
        }
        Err(self.err_at(for_init, "malformed `for` init".to_string()))
    }

    /// `for_update = expression_list ;`. Reuses `lower_expr_statement` —
    /// each item is an ordinary `expression` node, structurally identical
    /// to an expression-statement's own child, so the same assignment/
    /// compound-assignment/increment-decrement handling applies unchanged.
    fn lower_for_update(&mut self, for_update: &GrammarASTNode) -> Result<Stmt, JavaLowerError> {
        let expr_list = self
            .first_child_named(for_update, "expression_list")
            .ok_or_else(|| {
                self.err_at(
                    for_update,
                    "malformed `for` update (missing expression list)".to_string(),
                )
            })?;
        let expr = self.single_expression_in_list(expr_list)?;
        self.lower_expr_statement(expr)
    }

    /// `expression_list = expression { COMMA expression } ;` — M2b
    /// supports exactly one expression per `for` init/update clause
    /// (`for (int i = 0, j = 0; ...)`-style multi-clause forms are
    /// deferred, mirroring `lower_local_var_decl`'s own single-declarator
    /// restriction).
    fn single_expression_in_list<'a>(
        &self,
        expr_list: &'a GrammarASTNode,
    ) -> Result<&'a GrammarASTNode, JavaLowerError> {
        let exprs: Vec<&GrammarASTNode> = child_nodes(expr_list)
            .into_iter()
            .filter(|n| n.rule_name == "expression")
            .collect();
        match exprs.as_slice() {
            [only] => Ok(only),
            _ => Err(self.err_at(
                expr_list,
                "multiple comma-separated expressions in one `for` clause are not supported yet (deferred; use only one)".to_string(),
            )),
        }
    }

    /// `enhanced_for_statement = "for" LPAREN {annotation} [final]
    /// local_var_type NAME COLON expression RPAREN statement ;` → lowers
    /// directly to `Stmt::ForEach` (no desugaring needed — SIR already has
    /// exactly this shape). `var` is rejected: M1/M2 have no array/
    /// collection `Kind` or construction syntax at all yet (that's JV02
    /// M4), so there is no way to infer the loop variable's element type
    /// from the iterable expression the way real Java type inference
    /// would — an explicit element type is required for now.
    fn lower_enhanced_for_statement(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Stmt, JavaLowerError> {
        let span = self.span_of(node);
        let lvt = self
            .first_child_named(node, "local_var_type")
            .ok_or_else(|| {
                self.err_at(node, "malformed enhanced `for` (missing type)".to_string())
            })?;
        let declared_kind = self.declared_kind_of_local_var_type(lvt)?;
        let var_kind = declared_kind.ok_or_else(|| {
            self.err_at(
                node,
                "`var` in an enhanced `for` is not supported yet (deferred; the element type can't be inferred without array/collection support)".to_string(),
            )
        })?;
        let var_name_tok = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Name => Some(t),
                _ => None,
            })
            .ok_or_else(|| {
                self.err_at(
                    node,
                    "malformed enhanced `for` (missing loop variable name)".to_string(),
                )
            })?;
        let var_name = var_name_tok.value.clone();
        let iter_node = self.first_child_named(node, "expression").ok_or_else(|| {
            self.err_at(
                node,
                "malformed enhanced `for` (missing iterable expression)".to_string(),
            )
        })?;
        let (iter, _iter_kind) = self.lower_expr(iter_node, 0)?;
        let body_stmt = self.first_child_named(node, "statement").ok_or_else(|| {
            self.err_at(node, "malformed enhanced `for` (missing body)".to_string())
        })?;

        self.push_scope();
        self.declare_local(var_name.clone(), var_kind);
        let body = self.lower_body(body_stmt, depth + 1);
        self.pop_scope();
        let body = body?;

        self.observed.add(Feature::Loops);
        Ok(Stmt::ForEach {
            var: var_name,
            iter,
            body,
            span,
        })
    }

    fn lower_var_declaration_node(
        &mut self,
        var_decl: &GrammarASTNode,
    ) -> Result<Stmt, JavaLowerError> {
        let lvds = self
            .first_child_named(var_decl, "local_variable_declaration_statement")
            .ok_or_else(|| self.err_at(var_decl, "malformed variable declaration".to_string()))?;
        self.lower_local_var_decl(lvds)
    }

    /// Lower `local_variable_declaration_statement` (`{annotation}
    /// ["final"] local_var_type variable_declarators SEMICOLON`) into a
    /// `Stmt::LetBinding`. See this module's own doc comment for the
    /// exact supported subset (single declarator, initializer required,
    /// no array-bracket declarator suffix).
    fn lower_local_var_decl(&mut self, lvds: &GrammarASTNode) -> Result<Stmt, JavaLowerError> {
        let lvt = self
            .first_child_named(lvds, "local_var_type")
            .ok_or_else(|| {
                self.err_at(
                    lvds,
                    "malformed local variable declaration (missing type)".to_string(),
                )
            })?;
        let declared_kind = self.declared_kind_of_local_var_type(lvt)?;
        let declarators = self
            .first_child_named(lvds, "variable_declarators")
            .ok_or_else(|| {
                self.err_at(
                    lvds,
                    "malformed local variable declaration (missing declarators)".to_string(),
                )
            })?;
        let declarator = self.single_variable_declarator(declarators)?;
        self.lower_variable_declarator(declared_kind, declarator, lvds)
    }

    /// Extract the single `variable_declarator` from a `variable_declarators`
    /// node, rejecting the (deferred) multi-declarator case with a clear
    /// error. Shared by `lower_local_var_decl` and the classic `for`
    /// loop's own declaration-form init clause (`lower_for_init`), whose
    /// `variable_declarators` node has the identical shape either way.
    fn single_variable_declarator<'a>(
        &self,
        declarators: &'a GrammarASTNode,
    ) -> Result<&'a GrammarASTNode, JavaLowerError> {
        let decls: Vec<&GrammarASTNode> = child_nodes(declarators)
            .into_iter()
            .filter(|n| n.rule_name == "variable_declarator")
            .collect();
        match decls.as_slice() {
            [only] => Ok(only),
            _ => Err(self.err_at(
                declarators,
                "multiple variable declarators in one statement are not supported yet (deferred; declare each variable in its own statement)".to_string(),
            )),
        }
    }

    /// Lower one `variable_declarator` (`NAME {LBRACKET RBRACKET} [EQUALS
    /// variable_initializer]`) given its already-resolved declared kind
    /// (`None` for `var`) into a `Stmt::LetStarBinding`, declaring the
    /// name in the current innermost scope. `span_node` supplies the
    /// emitted statement's span — the enclosing construct (a standalone
    /// declaration statement, or a classic `for`'s own init clause),
    /// since `variable_declarator` itself doesn't carry a span typical of
    /// the whole declaration.
    fn lower_variable_declarator(
        &mut self,
        declared_kind: Option<Kind>,
        declarator: &GrammarASTNode,
        span_node: &GrammarASTNode,
    ) -> Result<Stmt, JavaLowerError> {
        let has_array_brackets = declarator
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "["));
        if has_array_brackets {
            return Err(self.err_at(
                declarator,
                "C-style array declarator brackets (`int x[]`) are not supported yet (deferred to JV02 M4)".to_string(),
            ));
        }
        let name_tok = declarator
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Name => Some(t),
                _ => None,
            })
            .ok_or_else(|| {
                self.err_at(
                    declarator,
                    "malformed variable declarator (missing name)".to_string(),
                )
            })?;
        let name = name_tok.value.clone();

        let initializer = self.first_child_named(declarator, "variable_initializer").ok_or_else(|| {
            self.err_at(
                declarator,
                "uninitialized local variable declarations are not supported yet (an initializer is required)".to_string(),
            )
        })?;
        let init_expr_node = match initializer.children.as_slice() {
            [ASTNodeOrToken::Node(n)] if n.rule_name == "expression" => n,
            _ => {
                return Err(self.err_at(
                    initializer,
                    "array initializers are not supported yet (deferred to JV02 M4)".to_string(),
                ))
            }
        };
        let (value, value_kind) = self.lower_expr(init_expr_node, 0)?;

        let kind = match declared_kind {
            Some(k) => k,
            None => {
                if value_kind == Kind::Null {
                    return Err(self.err_at(
                        init_expr_node,
                        "cannot infer `var`'s type from a `null` initializer".to_string(),
                    ));
                }
                value_kind
            }
        };
        self.declare_local(name.clone(), kind);

        // `Stmt::LetStarBinding`, not `LetBinding`: Java's local
        // declarations are strictly sequential — `int x = 1; int y = x +
        // 1;` requires `y`'s initializer to see `x`. `LetBinding` has
        // *parallel*-let semantics instead (consecutive bindings evaluate
        // outside each other's scope — see that variant's own doc
        // comment), which would make every declaration but the first
        // reference an "unknown name" per `semantic_ir::validate()`.
        let span = self.span_of(span_node);
        Ok(Stmt::LetStarBinding {
            name,
            sir_type: None,
            value,
            span,
        })
    }

    /// Resolve `local_var_type`'s declared kind, or `None` for `var`
    /// (type inferred from the initializer by the caller). See this
    /// module's own doc comment ("The `var` ambiguity") for why `var` is
    /// detected by resolved shape rather than by grammar alternative.
    fn declared_kind_of_local_var_type(
        &self,
        lvt: &GrammarASTNode,
    ) -> Result<Option<Kind>, JavaLowerError> {
        match lvt.children.as_slice() {
            // The literal `"var"` grammar alternative — dead in practice
            // (see the module doc comment) but handled defensively in
            // case a future grammar revision changes the ordering.
            [ASTNodeOrToken::Token(t)] if t.value == "var" => Ok(None),
            [ASTNodeOrToken::Node(type_node)] => {
                if single_segment_class_type_name(type_node) == Some("var") {
                    return Ok(None);
                }
                self.kind_of_type_node(type_node).map(Some)
            }
            _ => Err(self.err_at(lvt, "malformed local variable type".to_string())),
        }
    }

    /// Resolve a `type` node (`{annotation} primitive_type
    /// {LBRACKET RBRACKET} | {annotation} class_type {LBRACKET RBRACKET}`)
    /// to a [`Kind`]. Only `String` is accepted among reference types —
    /// every other class type (including any user-defined class, since
    /// M1 has no class-declaration lowering yet) is out of scope.
    fn kind_of_type_node(&self, type_node: &GrammarASTNode) -> Result<Kind, JavaLowerError> {
        let has_array_brackets = type_node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "["));
        if has_array_brackets {
            return Err(self.err_at(
                type_node,
                "array types are not supported yet (deferred to JV02 M4)".to_string(),
            ));
        }
        if let Some(prim) = self.first_child_named(type_node, "primitive_type") {
            let tok = prim
                .children
                .iter()
                .find_map(|c| match c {
                    ASTNodeOrToken::Token(t) => Some(t),
                    ASTNodeOrToken::Node(_) => None,
                })
                .ok_or_else(|| self.err_at(prim, "malformed primitive type".to_string()))?;
            return match tok.value.as_str() {
                "boolean" => Ok(Kind::Bool),
                "byte" | "short" | "int" | "long" | "char" => Ok(Kind::Int),
                "float" | "double" => Ok(Kind::Float),
                other => Err(self.err_at(prim, format!("unsupported primitive type `{other}`"))),
            };
        }
        if let Some(class_type) = self.first_child_named(type_node, "class_type") {
            return match single_segment_class_type_name(type_node) {
                Some("String") => Ok(Kind::Str),
                _ => {
                    let name =
                        qualified_name_text(class_type).unwrap_or_else(|| "<unknown>".to_string());
                    Err(self.err_at(
                        class_type,
                        format!("unsupported reference type `{name}` (JV02 M1 supports only `String` and primitive types)"),
                    ))
                }
            };
        }
        Err(self.err_at(type_node, "malformed type node".to_string()))
    }

    /// Lower a full expression-statement's `expression` node. Handled in
    /// order: a bare-name-target plain or compound assignment (`x = ...`,
    /// `x += ...`, etc. → `Stmt::Assign`); a bare increment/decrement
    /// (`i++;`, `--i;` → `Stmt::Assign`, desugared the same way compound
    /// assignment is); or an ordinary value expression evaluated for
    /// effect (→ `Stmt::ExprStmt`, matching M0's existing behavior for
    /// e.g. `42;`).
    fn lower_expr_statement(
        &mut self,
        expression: &GrammarASTNode,
    ) -> Result<Stmt, JavaLowerError> {
        let inner = match expression.children.as_slice() {
            [ASTNodeOrToken::Node(n)] => n,
            _ => return Err(self.err_at(expression, "malformed `expression` node".to_string())),
        };
        if inner.rule_name == "lambda_expression" {
            return Err(self.err_at(
                inner,
                "lambda expressions are not supported yet (deferred to JV02 M3b)".to_string(),
            ));
        }
        if inner.rule_name == "assignment_expression" {
            if let [ASTNodeOrToken::Node(lvalue_node), ASTNodeOrToken::Node(op_node), ASTNodeOrToken::Node(rhs_node)] =
                inner.children.as_slice()
            {
                let op_tok = op_node
                    .children
                    .iter()
                    .find_map(|c| match c {
                        ASTNodeOrToken::Token(t) => Some(t),
                        ASTNodeOrToken::Node(_) => None,
                    })
                    .ok_or_else(|| {
                        self.err_at(op_node, "malformed assignment operator".to_string())
                    })?;
                let name = self.extract_bare_name(lvalue_node, 0)?;
                let (declared_kind, declared_scope) =
                    self.lookup_local(&name).ok_or_else(|| {
                        self.err_at(
                            lvalue_node,
                            format!("assignment to undeclared local variable `{name}`"),
                        )
                    })?;
                let span = self.span_of(inner);
                let value = match op_tok.value.as_str() {
                    "=" => self.lower_expr(rhs_node, 0)?.0,
                    "+=" | "-=" | "*=" | "/=" | "%=" => {
                        let (rhs, rhs_kind) = self.lower_expr(rhs_node, 0)?;
                        let lhs_span = self.span_of(lvalue_node);
                        let lhs_expr = Expr::VarRef { name: name.clone(), scope: declared_scope, span: lhs_span };
                        let op_char = op_tok.value.chars().next().expect("non-empty operator token");
                        match op_char {
                            '+' | '-' => self.combine_additive(lhs_expr, declared_kind, rhs, rhs_kind, op_char, op_node)?.0,
                            '*' | '/' | '%' => {
                                self.combine_multiplicative(lhs_expr, declared_kind, rhs, rhs_kind, op_char, op_node)?.0
                            }
                            _ => unreachable!("compound assignment operator token was matched but its leading char isn't one of + - * / %"),
                        }
                    }
                    other => {
                        return Err(self.err_at(
                            op_node,
                            format!("unsupported assignment operator `{other}` (deferred to a later JV02 milestone)"),
                        ))
                    }
                };
                self.observed.add(Feature::MutableBindings);
                return Ok(Stmt::Assign {
                    name,
                    scope: declared_scope,
                    value,
                    span,
                });
            }
        }
        if let Some((target_node, op)) = self.bare_incdec_target(inner, 0)? {
            let name = self.extract_bare_name(target_node, 0)?;
            let (declared_kind, declared_scope) = self.lookup_local(&name).ok_or_else(|| {
                self.err_at(
                    target_node,
                    format!("`{op}{op}` on undeclared local variable `{name}`"),
                )
            })?;
            if !matches!(declared_kind, Kind::Int | Kind::Float) {
                return Err(self.err_at(
                    target_node,
                    format!("`{op}{op}` requires a numeric operand"),
                ));
            }
            let span = self.span_of(inner);
            let lhs_expr = Expr::VarRef {
                name: name.clone(),
                scope: declared_scope,
                span: span.clone(),
            };
            let one = if declared_kind == Kind::Float {
                Expr::FloatLit {
                    value: 1.0,
                    span: span.clone(),
                }
            } else {
                Expr::IntLit {
                    value: 1,
                    span: span.clone(),
                }
            };
            let (value, _) =
                self.combine_additive(lhs_expr, declared_kind, one, declared_kind, op, inner)?;
            self.observed.add(Feature::MutableBindings);
            return Ok(Stmt::Assign {
                name,
                scope: declared_scope,
                value,
                span,
            });
        }
        let (expr, _kind) = self.lower_expr(inner, 0)?;
        let span = self.span_of(expression);
        Ok(Stmt::ExprStmt { expr, span })
    }

    /// Walks the same single-child expression-precedence chain
    /// `lower_expr` does, looking for a *bare* `i++`/`i--`/`++i`/`--i`
    /// shape with no other real operator present at any level above it —
    /// i.e. the entire statement is exactly an increment/decrement, not
    /// one nested inside a larger expression (`y = i++` is a different,
    /// still-unsupported shape — see `lower_unary`/`lower_postfix`'s own
    /// rejection of increment/decrement in *value* position, which this
    /// helper does not change). Returns `None` — not an error — for any
    /// other expression shape, so the caller can fall through to ordinary
    /// value-expression lowering. `'+'`/`'-'` in the returned tuple mean
    /// `++`/`--` respectively, matching `combine_additive`'s own operator
    /// convention.
    fn bare_incdec_target<'a>(
        &self,
        node: &'a GrammarASTNode,
        depth: usize,
    ) -> Result<Option<(&'a GrammarASTNode, char)>, JavaLowerError> {
        if depth >= MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting exceeds {MAX_EXPR_DEPTH} levels"),
            ));
        }
        match node.rule_name.as_str() {
            "unary_expression" => match node.children.as_slice() {
                [ASTNodeOrToken::Token(t), ASTNodeOrToken::Node(inner)]
                    if t.value == "++" || t.value == "--" =>
                {
                    Ok(Some((
                        inner,
                        t.value.chars().next().expect("non-empty operator token"),
                    )))
                }
                [ASTNodeOrToken::Node(only)] => self.bare_incdec_target(only, depth + 1),
                _ => Ok(None),
            },
            "postfix_expression" => {
                let op = node.children.iter().skip(1).find_map(|c| match c {
                    ASTNodeOrToken::Token(t) if t.value == "++" || t.value == "--" => {
                        Some(t.value.chars().next().expect("non-empty operator token"))
                    }
                    _ => None,
                });
                match (node.children.first(), op) {
                    (Some(ASTNodeOrToken::Node(target)), Some(op)) => Ok(Some((target, op))),
                    _ => Ok(None),
                }
            }
            "expression"
            | "assignment_expression"
            | "conditional_expression"
            | "logical_or_expression"
            | "logical_and_expression"
            | "bitwise_or_expression"
            | "bitwise_xor_expression"
            | "bitwise_and_expression"
            | "equality_expression"
            | "relational_expression"
            | "shift_expression"
            | "additive_expression"
            | "multiplicative_expression"
            | "unary_expression_not_plus_minus" => match node.children.as_slice() {
                [ASTNodeOrToken::Node(only)] => self.bare_incdec_target(only, depth + 1),
                _ => Ok(None),
            },
            _ => Ok(None),
        }
    }

    /// Walk an assignment target's `unary_expression` chain down to its
    /// `primary`, requiring it to be a bare `NAME` — `foo.bar = x`,
    /// `arr[0] = x`, and any other non-simple target are out of scope for
    /// M1/M2a (rejected here rather than mis-lowered).
    fn extract_bare_name(
        &self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<String, JavaLowerError> {
        if depth >= MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting exceeds {MAX_EXPR_DEPTH} levels"),
            ));
        }
        if node.rule_name == "primary" {
            return match node.children.as_slice() {
                [ASTNodeOrToken::Token(t)] if t.type_ == lexer::token::TokenType::Name => Ok(t.value.clone()),
                _ => Err(self.err_at(
                    node,
                    "assignment target must be a simple local variable (field or indexed assignment targets are not supported yet)".to_string(),
                )),
            };
        }
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.extract_bare_name(only, depth + 1),
            _ => Err(self.err_at(
                node,
                "assignment target must be a simple local variable (field or indexed assignment targets are not supported yet)".to_string(),
            )),
        }
    }

    // ── expression-level lowering ───────────────────────────────────
    //
    // Each Java grammar precedence level gets its own dispatch arm below,
    // mirroring the grammar's own explicit rule-per-level structure (see
    // `java21.grammar`'s "Assignment" through "Primary Expressions"
    // sections). Every level that has no real operator present in a
    // given tree is a single-child wrapper — that case always just
    // recurses into the one child. A level with more than one child means
    // a real operator is present, which is either lowered (if in M1's
    // scope) or rejected with a clear "deferred" error.

    /// Dispatch on `node.rule_name` to the right precedence-level lowering
    /// helper. Returns the lowered [`Expr`] together with its inferred
    /// [`Kind`] (needed by callers up the chain to pick the right SIR
    /// operator — `div_trunc` vs `div_true`, `StrConcat` vs numeric `+`
    /// — and to reject ill-typed operand combinations).
    fn lower_expr(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        if depth >= MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting exceeds {MAX_EXPR_DEPTH} levels"),
            ));
        }
        match node.rule_name.as_str() {
            "expression" => self.lower_expression_rule(node, depth),
            "assignment_expression" => self.lower_assignment_expression_as_value(node, depth),
            "conditional_expression" => self.lower_conditional_expression(node, depth),
            "logical_or_expression" => self.lower_logical_chain(node, depth, "||", false),
            "logical_and_expression" => self.lower_logical_chain(node, depth, "&&", true),
            "bitwise_or_expression" | "bitwise_xor_expression" | "bitwise_and_expression" => {
                self.lower_single_child_only(node, depth, "bitwise operators")
            }
            "equality_expression" => self.lower_equality(node, depth),
            "relational_expression" => self.lower_relational(node, depth),
            "shift_expression" => self.lower_single_child_only(node, depth, "shift operators"),
            "additive_expression" => self.lower_additive(node, depth),
            "multiplicative_expression" => self.lower_multiplicative(node, depth),
            "unary_expression" => self.lower_unary(node, depth),
            "unary_expression_not_plus_minus" => self.lower_unary_not_plus_minus(node, depth),
            "postfix_expression" => self.lower_postfix(node, depth),
            "primary_expression" => self.lower_primary_expression(node, depth),
            "primary" => self.lower_primary(node, depth),
            other => Err(self.err_at(
                node,
                format!(
                    "unsupported expression construct `{other}` (JV02 M1 does not lower this yet)"
                ),
            )),
        }
    }

    /// `expression = lambda_expression | assignment_expression ;`
    fn lower_expression_rule(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] if only.rule_name == "lambda_expression" => Err(self
                .err_at(
                    only,
                    "lambda expressions are not supported yet (deferred to JV02 M3b)".to_string(),
                )),
            [ASTNodeOrToken::Node(only)] => self.lower_expr(only, depth + 1),
            _ => Err(self.err_at(node, "malformed `expression` node".to_string())),
        }
    }

    /// `assignment_expression = unary_expression assignment_operator
    /// assignment_expression | conditional_expression ;` — reached here
    /// only for a *value* position (statement-top assignment is peeled
    /// off earlier by `lower_expr_statement`), so the 3-child real-
    /// assignment shape means a *nested* assignment expression, which
    /// M1 does not support (SIR's `Assign` is a statement, not an
    /// expression — see `Stmt::Assign`'s own doc comment).
    fn lower_assignment_expression_as_value(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.lower_expr(only, depth + 1),
            [ASTNodeOrToken::Node(_), ASTNodeOrToken::Node(_), ASTNodeOrToken::Node(_)] => Err(self.err_at(
                node,
                "nested assignment expressions are not supported (JV02 M1 supports assignment only as a full statement)".to_string(),
            )),
            _ => Err(self.err_at(node, "malformed `assignment_expression` node".to_string())),
        }
    }

    /// `conditional_expression = logical_or_expression [ QUESTION
    /// assignment_expression COLON assignment_expression ] ;`
    fn lower_conditional_expression(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.lower_expr(only, depth + 1),
            _ => Err(self.err_at(
                node,
                "the ternary conditional operator (`?:`) is not supported yet (deferred to a later JV02 milestone)".to_string(),
            )),
        }
    }

    /// Shared fold for `logical_or_expression` (`{ OR_OR
    /// logical_and_expression }`) and `logical_and_expression` (`{
    /// AND_AND bitwise_or_expression }`) — both require every operand to
    /// be `Kind::Bool` and produce `Kind::Bool`.
    fn lower_logical_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        op_value: &str,
        is_and: bool,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let mut acc: Option<(Expr, Kind)> = None;
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                let (expr, kind) = self.lower_expr(n, depth + 1)?;
                acc = Some(match acc.take() {
                    // Pure passthrough (no real operator at this level) —
                    // do NOT validate `kind` here. Every expression flows
                    // through this precedence level regardless of its
                    // type; only an *actual* `&&`/`||` combination
                    // requires boolean operands.
                    None => (expr, kind),
                    Some((lhs, lhs_kind)) => {
                        if lhs_kind != Kind::Bool || kind != Kind::Bool {
                            return Err(
                                self.err_at(n, format!("`{op_value}` requires boolean operands"))
                            );
                        }
                        let span = lhs.span().clone();
                        let combined = if is_and {
                            Expr::LogicalAnd {
                                lhs: Box::new(lhs),
                                rhs: Box::new(expr),
                                span,
                            }
                        } else {
                            Expr::LogicalOr {
                                lhs: Box::new(lhs),
                                rhs: Box::new(expr),
                                span,
                            }
                        };
                        self.observed.add(Feature::ShortCircuit);
                        (combined, Kind::Bool)
                    }
                });
            }
        }
        acc.ok_or_else(|| self.err_at(node, format!("empty `{}` expression", node.rule_name)))
    }

    /// A precedence level M1 does not touch (bitwise/shift): pass through
    /// when the grammar produced no real operator (single child), reject
    /// with a clear "deferred" error otherwise.
    fn lower_single_child_only(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        what: &str,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.lower_expr(only, depth + 1),
            _ => Err(self.err_at(
                node,
                format!("{what} are not supported yet (deferred to a later JV02 milestone)"),
            )),
        }
    }

    /// `equality_expression = relational_expression { (EQUALS_EQUALS |
    /// NOT_EQUALS) relational_expression } ;`. Restricted to numeric/
    /// boolean operands — Java's `==`/`!=` on `String` is *reference*
    /// equality (a well-known Java gotcha, since it silently diverges
    /// from `.equals()`), a fundamentally different operation from every
    /// other SIR frontend's `=`/`!=` builtin (value equality); lowering
    /// it as value equality would be a silent correctness bug, so it is
    /// rejected instead (string equality needs `.equals()`, which is
    /// method-call surface — JV02 M4+).
    fn lower_equality(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let mut acc: Option<(Expr, Kind)> = None;
        let mut pending_op: Option<&'static str> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if t.value == "==" || t.value == "!=" => {
                    pending_op = Some(if t.value == "==" { "=" } else { "!=" });
                }
                ASTNodeOrToken::Node(n) => {
                    let (expr, kind) = self.lower_expr(n, depth + 1)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
                        // Pure passthrough (no real `==`/`!=` at this
                        // level) — do NOT validate `kind` here; only an
                        // actual comparison requires numeric/boolean
                        // operands.
                        (None, _) => (expr, kind),
                        (Some((lhs, lhs_kind)), Some(op)) => {
                            if kind == Kind::Str || lhs_kind == Kind::Str {
                                return Err(self.err_at(
                                    n,
                                    "`==`/`!=` on `String` is Java reference equality, not value equality, and is not supported (use `.equals()` — deferred to a later JV02 milestone)".to_string(),
                                ));
                            }
                            if !kinds_compatible_for_compare(lhs_kind, kind) {
                                return Err(self.err_at(
                                    node,
                                    "equality comparison requires both operands to be the same general kind (both numeric, or both boolean)".to_string(),
                                ));
                            }
                            let span = lhs.span().clone();
                            (
                                Expr::BuiltinCall {
                                    name: op.to_string(),
                                    args: vec![lhs, expr],
                                    effects: EffectSet::PURE,
                                    span,
                                },
                                Kind::Bool,
                            )
                        }
                        (Some(_), None) => {
                            return Err(
                                self.err_at(node, "malformed equality expression".to_string())
                            )
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        acc.ok_or_else(|| self.err_at(node, "empty equality expression".to_string()))
    }

    /// `relational_expression = shift_expression { (LESS_THAN |
    /// GREATER_THAN | LESS_EQUALS | GREATER_EQUALS) shift_expression |
    /// "instanceof" instanceof_target } ;`
    fn lower_relational(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let has_instanceof = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "instanceof"));
        if has_instanceof {
            return Err(self.err_at(
                node,
                "`instanceof` is not supported yet (deferred to a later JV02 milestone)"
                    .to_string(),
            ));
        }
        let mut acc: Option<(Expr, Kind)> = None;
        let mut pending_op: Option<&'static str> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if matches!(t.value.as_str(), "<" | ">" | "<=" | ">=") => {
                    pending_op = Some(match t.value.as_str() {
                        "<" => "<",
                        ">" => ">",
                        "<=" => "<=",
                        ">=" => ">=",
                        _ => unreachable!(),
                    });
                }
                ASTNodeOrToken::Node(n) => {
                    let (expr, kind) = self.lower_expr(n, depth + 1)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
                        // Pure passthrough (no real relational operator at
                        // this level) — do NOT validate `kind` here; only
                        // an actual comparison requires numeric operands.
                        (None, _) => (expr, kind),
                        (Some((lhs, lhs_kind)), Some(op)) => {
                            if !matches!(lhs_kind, Kind::Int | Kind::Float)
                                || !matches!(kind, Kind::Int | Kind::Float)
                            {
                                return Err(self.err_at(
                                    n,
                                    "relational comparison requires numeric operands".to_string(),
                                ));
                            }
                            let span = lhs.span().clone();
                            (
                                Expr::BuiltinCall {
                                    name: op.to_string(),
                                    args: vec![lhs, expr],
                                    effects: EffectSet::PURE,
                                    span,
                                },
                                Kind::Bool,
                            )
                        }
                        (Some(_), None) => {
                            return Err(
                                self.err_at(node, "malformed relational expression".to_string())
                            )
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        acc.ok_or_else(|| self.err_at(node, "empty relational expression".to_string()))
    }

    /// `additive_expression = multiplicative_expression { (PLUS | MINUS)
    /// multiplicative_expression } ;`. `+` routes to string concatenation
    /// when either operand is `Kind::Str` (see `combine_additive`);
    /// everything else requires numeric operands.
    fn lower_additive(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let mut acc: Option<(Expr, Kind)> = None;
        let mut pending_op: Option<char> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if t.value == "+" || t.value == "-" => {
                    pending_op = Some(t.value.chars().next().expect("non-empty operator token"));
                }
                ASTNodeOrToken::Node(n) => {
                    let (rhs, rhs_kind) = self.lower_expr(n, depth + 1)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
                        (None, _) => (rhs, rhs_kind),
                        (Some((lhs, lhs_kind)), Some(op)) => {
                            self.combine_additive(lhs, lhs_kind, rhs, rhs_kind, op, node)?
                        }
                        (Some(_), None) => {
                            return Err(
                                self.err_at(node, "malformed additive expression".to_string())
                            )
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        acc.ok_or_else(|| self.err_at(node, "empty additive expression".to_string()))
    }

    fn combine_additive(
        &mut self,
        lhs: Expr,
        lhs_kind: Kind,
        rhs: Expr,
        rhs_kind: Kind,
        op: char,
        node: &GrammarASTNode,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        if op == '+' && (lhs_kind == Kind::Str || rhs_kind == Kind::Str) {
            self.observed.add(Feature::StringInterpolation);
            let span = lhs.span().clone();
            let parts = match lhs {
                Expr::StrConcat { parts, .. } => {
                    let mut parts = parts;
                    parts.push(rhs);
                    parts
                }
                other => vec![other, rhs],
            };
            return Ok((Expr::StrConcat { parts, span }, Kind::Str));
        }
        if !matches!(lhs_kind, Kind::Int | Kind::Float)
            || !matches!(rhs_kind, Kind::Int | Kind::Float)
        {
            return Err(self.err_at(
                node,
                format!(
                    "`{op}` requires numeric operands (or, for `+`, at least one `String` operand)"
                ),
            ));
        }
        let result_kind = if lhs_kind == Kind::Float || rhs_kind == Kind::Float {
            Kind::Float
        } else {
            Kind::Int
        };
        let span = lhs.span().clone();
        Ok((
            Expr::BuiltinCall {
                name: op.to_string(),
                args: vec![lhs, rhs],
                effects: EffectSet::PURE,
                span,
            },
            result_kind,
        ))
    }

    /// `multiplicative_expression = unary_expression { (STAR | SLASH |
    /// PERCENT) unary_expression } ;`. `/` selects `div_trunc` (both
    /// operands integral — Java truncates toward zero, same as Rust/C)
    /// or `div_true` (either operand `float`/`double`) per SIR21 T3b-2's
    /// op-name convention (see `c-to-semantic-ir`'s identically-reasoned
    /// selection). Java's primitive numeric types are all signed, so
    /// `udiv_trunc` never applies here.
    fn lower_multiplicative(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let mut acc: Option<(Expr, Kind)> = None;
        let mut pending_op: Option<char> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if matches!(t.value.as_str(), "*" | "/" | "%") => {
                    pending_op = Some(t.value.chars().next().expect("non-empty operator token"));
                }
                ASTNodeOrToken::Node(n) => {
                    let (rhs, rhs_kind) = self.lower_expr(n, depth + 1)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
                        (None, _) => (rhs, rhs_kind),
                        (Some((lhs, lhs_kind)), Some(op)) => {
                            self.combine_multiplicative(lhs, lhs_kind, rhs, rhs_kind, op, node)?
                        }
                        (Some(_), None) => {
                            return Err(self
                                .err_at(node, "malformed multiplicative expression".to_string()))
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        acc.ok_or_else(|| self.err_at(node, "empty multiplicative expression".to_string()))
    }

    fn combine_multiplicative(
        &mut self,
        lhs: Expr,
        lhs_kind: Kind,
        rhs: Expr,
        rhs_kind: Kind,
        op: char,
        node: &GrammarASTNode,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        if !matches!(lhs_kind, Kind::Int | Kind::Float)
            || !matches!(rhs_kind, Kind::Int | Kind::Float)
        {
            return Err(self.err_at(node, format!("`{op}` requires numeric operands")));
        }
        let is_float = lhs_kind == Kind::Float || rhs_kind == Kind::Float;
        let result_kind = if is_float { Kind::Float } else { Kind::Int };
        let name = match op {
            '*' => "*".to_string(),
            '%' => "%".to_string(),
            '/' => {
                if is_float {
                    "div_true".to_string()
                } else {
                    "div_trunc".to_string()
                }
            }
            _ => unreachable!("combine_multiplicative called with an unrecognized operator"),
        };
        let span = lhs.span().clone();
        Ok((
            Expr::BuiltinCall {
                name,
                args: vec![lhs, rhs],
                effects: EffectSet::PURE,
                span,
            },
            result_kind,
        ))
    }

    /// `unary_expression = PLUS_PLUS unary_expression | MINUS_MINUS
    /// unary_expression | PLUS unary_expression | MINUS unary_expression
    /// | unary_expression_not_plus_minus ;`
    fn lower_unary(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.lower_expr(only, depth + 1),
            [ASTNodeOrToken::Token(t), ASTNodeOrToken::Node(inner)] => match t.value.as_str() {
                "++" | "--" => Err(self.err_at(
                    node,
                    "prefix increment/decrement operators are not supported yet (deferred to a later JV02 milestone)".to_string(),
                )),
                "+" => {
                    let (expr, kind) = self.lower_expr(inner, depth + 1)?;
                    if !matches!(kind, Kind::Int | Kind::Float) {
                        return Err(self.err_at(inner, "unary `+` requires a numeric operand".to_string()));
                    }
                    Ok((expr, kind))
                }
                "-" => {
                    let (expr, kind) = self.lower_expr(inner, depth + 1)?;
                    if !matches!(kind, Kind::Int | Kind::Float) {
                        return Err(self.err_at(inner, "unary `-` requires a numeric operand".to_string()));
                    }
                    let negated = match expr {
                        Expr::IntLit { value, span } => Expr::IntLit {
                            value: value.wrapping_neg(),
                            span,
                        },
                        Expr::FloatLit { value, span } => Expr::FloatLit { value: -value, span },
                        other => {
                            let span = other.span().clone();
                            Expr::BuiltinCall {
                                name: "neg".to_string(),
                                args: vec![other],
                                effects: EffectSet::PURE,
                                span,
                            }
                        }
                    };
                    Ok((negated, kind))
                }
                other => Err(self.err_at(node, format!("unsupported unary operator `{other}`"))),
            },
            _ => Err(self.err_at(node, "malformed `unary_expression` node".to_string())),
        }
    }

    /// `unary_expression_not_plus_minus = TILDE unary_expression | BANG
    /// unary_expression | cast_expression | postfix_expression ;`. The
    /// single-child case covers *both* remaining alternatives —
    /// `cast_expression` is naturally rejected by `lower_expr`'s own
    /// catch-all (it has no dispatch arm), so no special-case is needed
    /// here to distinguish it from `postfix_expression`.
    fn lower_unary_not_plus_minus(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.lower_expr(only, depth + 1),
            [ASTNodeOrToken::Token(t), ASTNodeOrToken::Node(inner)] => match t.value.as_str() {
                "~" => Err(self.err_at(
                    node,
                    "bitwise complement (`~`) is not supported yet (deferred to a later JV02 milestone)".to_string(),
                )),
                "!" => {
                    let (expr, kind) = self.lower_expr(inner, depth + 1)?;
                    if kind != Kind::Bool {
                        return Err(self.err_at(inner, "unary `!` requires a boolean operand".to_string()));
                    }
                    let span = expr.span().clone();
                    Ok((
                        Expr::BuiltinCall {
                            name: "not".to_string(),
                            args: vec![expr],
                            effects: EffectSet::PURE,
                            span,
                        },
                        Kind::Bool,
                    ))
                }
                other => Err(self.err_at(node, format!("unsupported unary operator `{other}`"))),
            },
            _ => Err(self.err_at(node, "malformed `unary_expression_not_plus_minus` node".to_string())),
        }
    }

    /// `postfix_expression = primary_expression { PLUS_PLUS | MINUS_MINUS
    /// } ;`
    fn lower_postfix(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let has_incr_decr =
            node.children.iter().skip(1).any(
                |c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "++" || t.value == "--"),
            );
        if has_incr_decr {
            return Err(self.err_at(
                node,
                "postfix increment/decrement operators are not supported yet (deferred to a later JV02 milestone)".to_string(),
            ));
        }
        match node.children.first() {
            Some(ASTNodeOrToken::Node(primary_expr)) => self.lower_expr(primary_expr, depth + 1),
            _ => Err(self.err_at(node, "malformed `postfix_expression` node".to_string())),
        }
    }

    /// `primary_expression = primary { primary_suffix } ;` — M3a adds
    /// exactly one new shape: a *bare* unqualified call, `NAME(args)`,
    /// which parses as `primary_expression(primary=NAME, primary_suffix=
    /// LPAREN [argument_list] RPAREN)` — a `primary` that is a single bare
    /// `NAME` token followed by exactly *one* suffix, itself starting
    /// with `(` (confirmed by direct CST inspection, not assumed from the
    /// grammar text alone). Every other suffix shape — field access
    /// (`.field`), a *qualified* call (`x.foo(...)`, which chains a
    /// `.foo` suffix *then* a separate `(...)` suffix — i.e. two
    /// suffixes, not one), `::` method references, and so on — remains
    /// out of scope, rejected exactly as before.
    fn lower_primary_expression(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(primary)] => self.lower_expr(primary, depth + 1),
            [ASTNodeOrToken::Node(primary), ASTNodeOrToken::Node(suffix)]
                if suffix.rule_name == "primary_suffix" =>
            {
                self.lower_call_expression(primary, suffix, node, depth)
            }
            _ => Err(self.err_at(
                node,
                "field access, method calls with more than one suffix, and other primary suffixes are not supported yet (deferred to a later JV02 milestone)".to_string(),
            )),
        }
    }

    /// Lower a bare unqualified call `NAME(args)` — see
    /// `lower_primary_expression`'s own doc comment for the exact CST
    /// shape this expects. `primary` must be a single bare `NAME` token
    /// (a *qualified* callee, e.g. `x.foo`, never reaches this function —
    /// it fails `lower_primary_expression`'s own suffix-count match arm
    /// first, since a qualified call chains two suffixes); `suffix` must
    /// itself start with `(` (an *unparenthesized* suffix, e.g. `.field`
    /// with no call, is rejected here rather than silently mis-lowered as
    /// a call).
    fn lower_call_expression(
        &mut self,
        primary: &GrammarASTNode,
        suffix: &GrammarASTNode,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let is_call_suffix =
            matches!(suffix.children.first(), Some(ASTNodeOrToken::Token(t)) if t.value == "(");
        if !is_call_suffix {
            return Err(self.err_at(
                node,
                "field access and other primary suffixes are not supported yet (deferred to a later JV02 milestone)".to_string(),
            ));
        }
        let callee = match primary.children.as_slice() {
            [ASTNodeOrToken::Token(t)] if t.type_ == lexer::token::TokenType::Name => {
                t.value.clone()
            }
            _ => {
                return Err(self.err_at(
                    node,
                    "method calls are only supported on a bare method name so far (a qualified receiver, e.g. `x.foo(...)`, is deferred to a later JV02 milestone)".to_string(),
                ))
            }
        };
        let sig = self.method_signatures.get(&callee).cloned().ok_or_else(|| {
            self.err_at(
                node,
                format!("call to unknown method `{callee}` (JV02 M3a can only call a method declared in the same class)"),
            )
        })?;

        let arg_nodes: Vec<&GrammarASTNode> = match self.first_child_named(suffix, "argument_list")
        {
            Some(al) => child_nodes(al)
                .into_iter()
                .filter(|n| n.rule_name == "expression")
                .collect(),
            None => vec![],
        };
        if arg_nodes.len() != sig.param_kinds.len() {
            return Err(self.err_at(
                node,
                format!(
                    "`{callee}` expects {} argument(s), found {}",
                    sig.param_kinds.len(),
                    arg_nodes.len()
                ),
            ));
        }
        let mut args = Vec::with_capacity(arg_nodes.len());
        for (arg_node, expected_kind) in arg_nodes.iter().zip(sig.param_kinds.iter()) {
            let (arg_expr, arg_kind) = self.lower_expr(arg_node, depth + 1)?;
            if arg_kind != *expected_kind {
                return Err(self.err_at(
                    arg_node,
                    format!("argument to `{callee}` has the wrong kind"),
                ));
            }
            args.push(arg_expr);
        }

        if let Some(callees) = self.call_graph.get_mut(&self.current_method) {
            callees.insert(callee.clone());
        }

        let span = self.span_of(node);
        Ok((
            Expr::DirectCall {
                fn_name: callee,
                args,
                effects: EffectSet::PURE,
                span,
            },
            sig.return_kind,
        ))
    }

    /// `primary = literal | "this" | ... | LPAREN expression RPAREN |
    /// NAME ;` — M1 supports exactly three of these alternatives:
    /// literals, parenthesized sub-expressions, and bare variable
    /// references. Everything else (`this`, `super`, `switch`
    /// expressions, object/array construction) is out of scope.
    fn lower_primary(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(n)] if n.rule_name == "literal" => {
                let expr = self.lower_literal(n)?;
                let kind = kind_of_literal_expr(&expr);
                Ok((expr, kind))
            }
            // Bare `NAME` — a variable reference. This position is
            // reached only when the grammar matched the `NAME` terminal
            // specifically (not an operator token — the shared,
            // cross-language `TokenType` enum tags several operators
            // without their own dedicated variant as `Name` too, but
            // `primary`'s own grammar production never places one of
            // those there), so the token is always a genuine identifier
            // lexeme here.
            [ASTNodeOrToken::Token(t)] if t.type_ == lexer::token::TokenType::Name => {
                let name = t.value.clone();
                let (kind, scope) = self.lookup_local(&name).ok_or_else(|| {
                    self.err_at(node, format!("reference to undeclared local variable `{name}`"))
                })?;
                let span = self.span_of(node);
                Ok((Expr::VarRef { name, scope, span }, kind))
            }
            [ASTNodeOrToken::Token(open), ASTNodeOrToken::Node(inner), ASTNodeOrToken::Token(_close)]
                if open.value == "(" =>
            {
                self.lower_expr(inner, depth + 1)
            }
            _ => Err(self.err_at(
                node,
                "unsupported primary expression (JV02 M1 supports only literals, bare variable references, and parenthesized expressions)".to_string(),
            )),
        }
    }

    /// Lower a `literal` node's single token child to an [`Expr`].
    fn lower_literal(&mut self, literal: &GrammarASTNode) -> Result<Expr, JavaLowerError> {
        let span = self.span_of(literal);
        let tok = match literal.children.as_slice() {
            [ASTNodeOrToken::Token(t)] => t,
            _ => return Err(self.err_at(literal, "malformed literal node".to_string())),
        };
        match (tok.type_, tok.value.as_str()) {
            (_, "true") => Ok(Expr::BoolLit { value: true, span }),
            (_, "false") => Ok(Expr::BoolLit { value: false, span }),
            (_, "null") => Ok(Expr::NilLit { span }),
            (lexer::token::TokenType::Number, text) => Ok(self.number_literal_expr(text, span)),
            (lexer::token::TokenType::String, text) => {
                self.observed.add(Feature::Strings);
                Ok(Expr::StrLit {
                    value: text.to_string(),
                    span,
                })
            }
            _ => Err(self.err_at(
                literal,
                format!(
                    "unsupported literal token `{}` (`{}`)",
                    tok.value, tok.type_
                ),
            )),
        }
    }

    /// A Java `NUMBER` lexeme is a float if it has a decimal point or
    /// exponent (or an `f`/`F`/`d`/`D` suffix, stripped before parsing —
    /// M0 does not distinguish Java's `float` vs. `double`, both lower to
    /// `Expr::FloatLit`), otherwise an int; an integer lexeme too large
    /// for `i64` falls back to a float rather than silently truncating or
    /// erroring. Mirrors `matlab-to-semantic-ir`'s identically-reasoned
    /// `number_literal_expr` — including its own hard-won lesson that
    /// `Feature::Floats` must be observed on every `FloatLit` branch, not
    /// just the "has a dot" one, or a float-literal module fails
    /// `semantic_ir::validate()`.
    fn number_literal_expr(&mut self, text: &str, span: Span) -> Expr {
        let trimmed = text.trim_end_matches(['f', 'F', 'd', 'D']);
        if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
            self.observed.add(Feature::Floats);
            Expr::FloatLit {
                value: trimmed.parse::<f64>().unwrap_or(0.0),
                span,
            }
        } else {
            match trimmed.parse::<i64>() {
                Ok(v) => Expr::IntLit { value: v, span },
                Err(_) => {
                    self.observed.add(Feature::Floats);
                    Expr::FloatLit {
                        value: trimmed.parse::<f64>().unwrap_or(0.0),
                        span,
                    }
                }
            }
        }
    }

    fn first_child_named<'a>(
        &self,
        node: &'a GrammarASTNode,
        kind: &str,
    ) -> Option<&'a GrammarASTNode> {
        child_nodes(node).into_iter().find(|n| n.rule_name == kind)
    }

    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::point(
            FILE,
            node.start_line.unwrap_or(1),
            node.start_column.unwrap_or(1),
        )
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> JavaLowerError {
        JavaLowerError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }
}

/// Two kinds are compatible operands for `==`/`!=` if they're both
/// numeric (Java allows mixed `int`/`double` comparison via numeric
/// promotion) or both boolean — never one of each.
fn kinds_compatible_for_compare(a: Kind, b: Kind) -> bool {
    matches!(
        (a, b),
        (Kind::Bool, Kind::Bool) | (Kind::Int | Kind::Float, Kind::Int | Kind::Float)
    )
}

/// Does `block`'s own *top-level* statement list declare a local named
/// `name`? Deliberately shallow — does not recurse into a nested
/// sub-block's own statements, since those live in a distinct,
/// already-scope-popped-by-the-time-this-runs frame of their own (see
/// `lower_do_while_statement`'s own doc comment for why only the
/// top level matters for its particular collision check).
fn body_declares_name(block: &Block, name: &str) -> bool {
    block.stmts.iter().any(|s| match s {
        Stmt::LetBinding { name: n, .. } | Stmt::LetStarBinding { name: n, .. } => n == name,
        _ => false,
    })
}

/// The [`Kind`] of an `Expr` freshly produced by `lower_literal` — only
/// ever one of these five variants, since that is the entirety of what
/// `lower_literal` can construct.
fn kind_of_literal_expr(expr: &Expr) -> Kind {
    match expr {
        Expr::IntLit { .. } => Kind::Int,
        Expr::FloatLit { .. } => Kind::Float,
        Expr::BoolLit { .. } => Kind::Bool,
        Expr::StrLit { .. } => Kind::Str,
        Expr::NilLit { .. } => Kind::Null,
        other => unreachable!("lower_literal produced an unexpected expr shape: {other:?}"),
    }
}

/// If `type_node` is `class_type { LBRACKET RBRACKET }` with no array
/// brackets and a single-segment `qualified_name`, return that one
/// segment's text (e.g. `"String"`, or `"var"` — see this module's own
/// doc comment on the `var` ambiguity). Returns `None` for a primitive
/// type, a multi-segment qualified name (`java.lang.String`), or an
/// array type — none of those are the shape this helper exists to detect.
fn single_segment_class_type_name(type_node: &GrammarASTNode) -> Option<&str> {
    let class_type = type_node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == "class_type" => Some(n),
        _ => None,
    })?;
    let qualified = class_type.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == "qualified_name" => Some(n),
        _ => None,
    })?;
    let names: Vec<&str> = qualified
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Name => {
                Some(t.value.as_str())
            }
            _ => None,
        })
        .collect();
    match names.as_slice() {
        [only] => Some(only),
        _ => None,
    }
}

/// Render a `class_type` node's `qualified_name` back to dotted text, for
/// error messages only (e.g. `"java.util.List"`).
fn qualified_name_text(class_type: &GrammarASTNode) -> Option<String> {
    let qualified = class_type.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == "qualified_name" => Some(n),
        _ => None,
    })?;
    let names: Vec<&str> = qualified
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Name => {
                Some(t.value.as_str())
            }
            _ => None,
        })
        .collect();
    if names.is_empty() {
        None
    } else {
        Some(names.join("."))
    }
}

fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// Depth-guarded pre-order collection of every node named `rule_name`
/// under `node` (inclusive). Deliberately hand-written rather than using
/// the shared `parser::grammar_parser::find_nodes` helper: that function
/// has no depth cap of its own, and `compile()` — the caller that
/// ultimately reaches this — is a public entry point accepting a raw
/// `GrammarASTNode` directly, not only one produced by `parse_java`'s own
/// `MAX_RULE_DEPTH`-capped parser. Calling the unguarded shared helper on
/// a possibly-adversarial tree would reintroduce the exact CWE-674
/// uncontrolled-recursion DoS this crate's own `MAX_TREE_DEPTH` guard
/// exists to prevent — found by `/security-review` before this crate
/// shipped.
fn collect_bounded<'a>(
    node: &'a GrammarASTNode,
    rule_name: &str,
    depth: usize,
    lowerer: &Lowerer,
    out: &mut Vec<&'a GrammarASTNode>,
) -> Result<(), JavaLowerError> {
    if depth >= MAX_TREE_DEPTH {
        return Err(lowerer.err_at(
            node,
            format!("tree nesting exceeds {MAX_TREE_DEPTH} levels"),
        ));
    }
    if node.rule_name == rule_name {
        out.push(node);
    }
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            collect_bounded(n, rule_name, depth + 1, lowerer, out)?;
        }
    }
    Ok(())
}
