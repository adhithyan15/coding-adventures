//! The lowering pass from `coding_adventures_java_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.11.0 (task #54:
//! `Expr::IndirectCall`, invoking a lambda-valued local)**.
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
//! **Supported (M3b, new):**
//! - Lambda expressions (`(int x) -> x + 1`, `(int a, int b) -> { return
//!   a + b; }`) → [`Expr::MakeClosure`], hoisting the body to a
//!   synthesized top-level `Function` (`__lambda_N`, mirroring how `main`
//!   itself is already synthesized). Every parameter must be explicitly
//!   typed — the untyped-inferred forms (a bare `x -> ...` with no
//!   parentheses, an untyped `(x) -> ...`, and `var`-inferred parameters)
//!   are rejected: Java infers an untyped/`var` lambda parameter's type
//!   from the lambda's own *target functional-interface type* (the
//!   abstract method it implements), and this frontend has no visibility
//!   into that at all (no functional-interface declarations exist yet —
//!   that's a later SIR29 milestone), so guessing would be a real
//!   mis-lowering, not a convenience.
//! - Captures, discovered *on-resolve* (mirrors
//!   `javascript-to-semantic-ir`'s identically-reasoned approach, adapted
//!   from that crate's one-scope-frame-per-function design to this
//!   crate's own one-frame-per-*block* `locals` stack — see
//!   `resolve_name`'s own doc comment for the full mechanism): a bare
//!   name referenced from inside a lambda body that isn't declared by
//!   the lambda itself is captured from the enclosing scope, however many
//!   lambda boundaries deep that enclosing declaration actually is.
//!   Assigning to (or incrementing/decrementing) a captured name is
//!   rejected — Java requires captured locals to be effectively final.
//! - Both `lambda_body` shapes: an expression body (the lambda's value
//!   directly) and a block body (`{ ... }`, using the *same*
//!   "`return` only in tail position" rule method bodies use, but with no
//!   declared return type to validate the returned value's kind against —
//!   a lambda's own return kind is simply *inferred*, not checked,
//!   mirroring how there is no declared parameter type to check argument
//!   kinds against for the same underlying reason).
//! - A new `Kind::Closure` classification (a lambda's own result kind) —
//!   lets a lambda be the initializer of a `var`-inferred local (`var f =
//!   (int x) -> x + 1;`, inferring `f: Closure`) or a bare expression
//!   statement, without this frontend needing any real functional-
//!   interface type system.
//!
//! **Supported (M4a, new):**
//! - Single-dimensional array types (`int[]`, `String[]`, …) — a new
//!   `Kind::Array(ArrayElemKind)` classification, where `ArrayElemKind`
//!   is a small flat `Copy` enum (not a boxed, recursive `Kind`, which
//!   would force `Kind` itself to drop `Copy` and ripple `.clone()`
//!   calls through the hundreds of call sites that thread it by value —
//!   see `Kind::Array`'s own doc comment). Multi-dimensional arrays
//!   (`int[][]`) are rejected.
//! - Array-literal declaration initializers (`int[] xs = {1, 2, 3};`,
//!   `var xs = {1, 2, 3};`) → [`Expr::SeqLit`]. Uses SIR16's `Expr::
//!   SeqLit`/`Feature::Sequences` — a flat, homogeneous 1-D sequence
//!   (`items: Vec<Expr>`) — rather than SIR22's `Expr::ArrayLit`/
//!   `Feature::NDArrays` (`rows: Vec<Vec<Expr>>`, row-major-matrix-
//!   shaped, built for MATLAB/Octave's true N-dimensional arrays, a
//!   meaningfully different domain a Java array isn't). Only the bare
//!   `{ ... }` initializer form is supported — the `new int[5]` (sized,
//!   uninitialized) and `new int[]{1,2,3}` array-creation-*expression*
//!   forms are deferred.
//! - Array indexing reads (`xs[i]`) → [`Expr::SeqIndex`], and `.length`
//!   → [`Expr::SeqLen`] — together enabling the realistic `for (int i =
//!   0; i < xs.length; i++) { ...xs[i]... }` pattern this milestone
//!   exists to unlock.
//!
//! **Supported (M4b, new):**
//! - Plain indexed assignment (`xs[i] = v;`) → [`Stmt::SeqSet`]. Detected
//!   by a new `indexed_assign_target` check, run *ahead of*
//!   `extract_bare_name` in `lower_expr_statement`'s own assignment-target
//!   dispatch, so a plain-name target (`x = v;`, unchanged since M1) and
//!   an indexed target (`xs[i] = v;`, new) are told apart before either is
//!   lowered — every other assignment-target shape (a field target, a
//!   qualified target) still falls through to `extract_bare_name`'s
//!   existing "reject rather than mis-lower" catch-all, unchanged.
//! - **Narrowed during implementation, mirroring the M2→M2a/M2b and
//!   M3→M3a/M3b splits**: compound assignment or increment/decrement on
//!   an indexed target (`xs[i] += v;`, `xs[i]++;`) was *not* supported
//!   this milestone, deferred instead — naively lowering it would
//!   evaluate the index expression twice (once to read the current
//!   element, once to write the new one), silently double-evaluating any
//!   side effect a non-constant index expression carries (e.g. a method
//!   call as the index). This is exactly the kind of double-evaluation
//!   bug this crate's own `/security-review` history has caught before
//!   (see `CHANGELOG.md`'s do-while/for-update entries) — deferred rather
//!   than risking it, tracked as its own follow-up task rather than
//!   shipped unsound. **Resolved as task #59** — see
//!   `lower_indexed_compound_assignment`/`lower_indexed_incdec`'s own doc
//!   comments for the once-only-evaluation fix.
//!
//! **Supported (M4c, new):**
//! - `new` array-creation expressions, both grammar shapes (confirmed via
//!   direct CST inspection, not assumed): `"new" array_creation_type
//!   array_dimension_exprs {LBRACKET RBRACKET}` (`new int[5]`, sized/
//!   uninitialized) and `"new" array_creation_type {LBRACKET RBRACKET}
//!   array_initializer` (`new int[]{1, 2, 3}`, explicit-type initializer
//!   — delegates directly to the same `lower_array_initializer` M4a
//!   built, since it's semantically identical to the bare `{1, 2, 3}`
//!   declarator-initializer form, just `new`-prefixed with an
//!   always-explicit element type).
//! - **`new T[N]` only when `N` is a compile-time-constant, non-negative
//!   integer literal, capped at [`MAX_SIZED_ARRAY_LEN`] elements**: SIR16
//!   has no repeat/fill primitive at all (confirmed by an exhaustive grep
//!   of every `Seq*` node — only `SeqLit`/`SeqIndex`/`SeqLen`/`SeqSet`
//!   exist), so a non-constant size (`new int[n]` for a variable `n`)
//!   genuinely cannot be represented without a new SIR primitive that
//!   doesn't exist yet, and is rejected rather than attempted. The
//!   element-count cap is a CWE-400/770-style resource-exhaustion guard
//!   (see that constant's own doc comment) — the same DoS discipline this
//!   crate's other milestones have already needed. `new T[N]` is also
//!   only supported for numeric/boolean element kinds — a sized
//!   reference-typed array (`new String[N]`) is deferred, since real Java
//!   fills it with `null`, which this frontend's exact element-kind-match
//!   invariant (every `Expr::SeqLit` item's `Kind` equals the array's own
//!   declared element `Kind`) doesn't cleanly represent yet.
//!
//! **Supported (M4d, new):**
//! - Real multi-dimensional array *types* (`int[][]`, `int[][][]`, …),
//!   capped at [`MAX_ARRAY_DIMS`] dimensions — [`Kind::Array`] gained a
//!   dimension count (`u8`, alongside the existing element kind)
//!   *without* becoming a boxed, recursive type: a multi-dimensional
//!   array is representationally just a nested sequence of sequences (a
//!   `SeqLit` of `SeqLit`s), so a flat dimension count is enough — see
//!   `Kind::Array`'s own doc comment for the full reasoning.
//! - Multi-dimensional array *literals* with an **explicit** declared
//!   type (`int[][] grid = {{1, 2}, {3, 4}};`), including genuinely
//!   ragged rows (`{{1, 2, 3}, {4}}` — real, independent inner arrays,
//!   not a rectangular matrix). `lower_array_initializer` recurses one
//!   dimension at a time, requiring each element be itself a nested
//!   `array_initializer` until the base (single-dimension) case is
//!   reached — `var`-inferred multi-dimensional array literals remain
//!   deferred (inferring dims from a literal's own, possibly ragged,
//!   nesting depth is real added complexity this milestone doesn't need;
//!   an explicit declared type sidesteps it entirely).
//! - Chained index *reads* (`grid[i][j]`, `cube[i][j][k]`) via a new
//!   `lower_chained_index`, reached only when *every* suffix in a
//!   2+-suffix `primary_expression` is `[...]`-shaped
//!   (`is_index_only_suffix`) — a mix of `[` and `.`/`(` suffixes (e.g.
//!   `grid[i].length`) still fell through to the pre-existing
//!   multi-suffix rejection at the time, a real, disclosed, narrower gap
//!   than full suffix-chain generalization; **resolved as task #60** —
//!   see that entry below and `lower_chained_index_then_length`'s own
//!   doc comment. `Kind::index_once` peels exactly one dimension per
//!   suffix, shared by the single-suffix (`lower_index_get`) and chained
//!   paths alike, so `xs[i]` on a 1-D array is unchanged from M4a.
//! - `.length` and plain indexed assignment (M4b) already generalize for
//!   free: `.length` was already dims-agnostic; `grid[i] = v;` (a whole
//!   sub-array assignment, single suffix) now requires `v` match the
//!   peeled-once result kind rather than always the flat element kind.
//!   A *chained* assignment target (`grid[i][j] = v;`) is **not**
//!   reachable at all — `indexed_assign_target`'s own fixed single-suffix
//!   match arm doesn't recognize a multi-suffix lvalue, so it falls
//!   through to the pre-existing bare-name-only rejection, still
//!   deferred (a separate, still-open gap from compound-assignment/
//!   increment-decrement on a *single*-suffix indexed target, resolved
//!   as task #59).
//!
//! **Supported (task #54, new):**
//! - *Invoking* a lambda-valued local or parameter (`var f = (int x) ->
//!   x + 1; f(5);`) → [`Expr::IndirectCall`]. `lower_call_expression`
//!   checks `resolve_name` on the bare callee *before* falling back to
//!   `method_signatures`, mirroring real Java's own name-resolution
//!   priority: a functional-interface-typed local in scope is invoked
//!   directly through that binding, and a same-named top-level method is
//!   not reachable through this call syntax while such a local exists.
//! - [`Kind::Closure`] gained a `u32` index into a new `Lowerer::
//!   closure_signatures` side table (each lambda's own param kinds +
//!   return kind, interned when the lambda is lowered) — needed so an
//!   indirect call can type-check its arguments and pick the right
//!   result `Kind`, without embedding the signature inline on `Kind`
//!   itself (which would force it to drop `Copy`, the same concern
//!   `Kind::Array` already navigates by staying flat).
//! - **Reassigning a `Closure`-kinded local is rejected** (`var f = (int
//!   x) -> x + 1; f = g;`), found by `/security-review`: this crate only
//!   tracks a local's `Kind` at *declaration* time, and a plain `=`
//!   reassignment never re-checks or re-records it — harmless for every
//!   other `Kind`, but `Kind::Closure(idx)`'s own `idx` is load-bearing,
//!   so an unrejected reassignment would leave a later call site
//!   type-checking against the *original* signature, not whatever the
//!   variable was actually reassigned to.
//! - A local/parameter that resolves but isn't `Closure`-kinded (`int x
//!   = 1; x();`) is rejected with a clear error rather than silently
//!   falling through to a same-named method — matching real Java, which
//!   would also reject this rather than reinterpreting `x` as a method
//!   reference.
//!
//! **Deliberately out of scope for v0.11.0** (each rejected with an
//! explicit [`JavaLowerError`], tracked in
//! [JV02](../../../specs/JV02-java-to-semantic-ir.md)'s own milestone
//! table): `switch` (SIR has no `Switch`/`Match`/`Case` IR node at all —
//! confirmed by a repo-wide grep, not assumed — so this needs its own
//! spec-level design decision before any frontend can target it, tracked
//! as a separate backlog item, not silently dropped), qualified/method-
//! reference calls (including array/`String` method-call surface other
//! than the special-cased `.length` field access), method overloading
//! (only one method per name is supported — this frontend has no
//! type-based overload resolution), varargs parameters, fields,
//! constructors, static/instance initializers, nested types, an early or
//! branched `return` (in a method *or* a lambda), untyped or `var`-
//! inferred lambda parameters, calling a lambda-valued *method
//! parameter* (this frontend has no way to declare one — no functional-
//! interface parameter type exists — a boundary of what's expressible,
//! not a gap in invocation itself), multi-dimensional `new` array-
//! creation forms (`new
//! int[2][3]`, `new int[][]{{1,2}}` — M4c's own two shapes stay
//! single-dimension only), a *chained* indexed-assignment target
//! (`grid[i][j] = v;` — a single-suffix indexed target's own compound-
//! assignment/increment-decrement, `xs[i] += v;`/`xs[i]++;`, is
//! supported as of task #59, see that entry above), a mixed index-
//! then-`(` primary-suffix chain (`grid[i].foo()`, a qualified method
//! call — no such method-call surface exists on an array at all, so this
//! remains unreachable regardless; a mixed index-then-`.length` chain,
//! `grid[i].length`, is supported as of task #60, see that entry above),
//! `var`-inferred multi-dimensional array literals, a non-constant or
//! reference-typed
//! `new T[N]` (see M4c's own entry above for why), `List`/`Map`
//! collection literals, field/array *field* access beyond `.length`,
//! casts, `instanceof`, the ternary conditional, bitwise operators
//! (`& | ^ ~ << >> >>>`), increment/decrement or compound assignment
//! used as a *value* rather than a bare statement, `break`/`continue`
//! (SIR has no IR primitive for either — every loop body this milestone
//! lowers must not contain one, checked structurally, not merely
//! "happens not to occur in the test corpus" — this also means a bare
//! `for (;;)` loop genuinely cannot terminate via any construct this
//! milestone can lower, a real and permanent limitation until `break`
//! exists), multiple comma-separated expressions in one `for` init/update
//! clause, `var` as an enhanced-`for` element type, uninitialized
//! declarations, multiple declarators per statement, C-style array-
//! bracket declarators (on a variable, a method parameter, or a method's
//! own return type), and reference types other than `String`.
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
    walk_stmt_default, Block, Capture, CaptureValue, EffectSet, Expr, Feature, FeatureManifest,
    Function, Metadata, Module, Param, ParamKind, Scope, Span, Stmt, Visitor,
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

/// Maximum element count `new T[N]` (M4c's sized-but-uninitialized array
/// creation) may materialize as a literal `Expr::SeqLit`. Not a real
/// language limit — a CWE-400/770-style resource-exhaustion guard: since
/// `N` must already be a compile-time-constant non-negative integer
/// literal for this milestone to lower it at all (a non-constant `N`
/// needs a real repeat/fill IR primitive SIR16 doesn't have yet, and is
/// rejected outright, not attempted), an attacker (or just an honest
/// typo) supplying a huge literal like `new int[2_000_000_000]` would
/// otherwise blow up `O(N)` source bytes into `O(N)` emitted IR nodes —
/// this crate's own established DoS class (see the do-while
/// exponential-blowup finding in `CHANGELOG.md`'s `[0.3.0]` entry; this
/// one is linear, not exponential, but still unbounded without a cap).
const MAX_SIZED_ARRAY_LEN: i64 = 10_000;

/// Maximum dimension count a Java array *type* (`int[][][]…`) or a
/// nested array *literal* (`{{{1}}}`) may declare — M4d's own
/// CWE-674-adjacent guard, mirroring [`MAX_EXPR_DEPTH`]'s reasoning:
/// `kind_of_type_node` and `lower_array_initializer` both recurse (the
/// latter genuinely, once per nesting level of a real literal; the
/// former only counts bracket-pair tokens, so it isn't itself a stack-
/// depth risk, but the *value* it produces feeds `Kind::Array`'s own
/// `u8` dimension field, which must stay boundable regardless). Java
/// itself caps array dimensionality at 255 (the JVM's own `arraycount`
/// limit); this frontend's own cap is far smaller since no real program
/// needs more than a handful of dimensions, and a smaller cap keeps the
/// nested-literal nesting depth this milestone's own recursion has to
/// handle correspondingly small.
const MAX_ARRAY_DIMS: usize = 8;

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
    /// The kind of an `Expr::MakeClosure` (a lowered lambda expression)
    /// — M3b's addition. Exists so a lambda can be the initializer of a
    /// `var`-inferred local (`var f = (int x) -> x + 1;`, inferring `f`
    /// as `Closure`) or a bare expression statement, without needing a
    /// real functional-interface type system (out of scope — this
    /// frontend has no notion of `Runnable`/`Function<T,R>`/etc. as a
    /// declarable type). Like `Void`, no operator recognizes `Closure`
    /// as a valid operand, so any other use falls through to an ordinary
    /// "wrong kind" rejection.
    ///
    /// Carries a `u32` index into `Lowerer::closure_signatures` (a small
    /// `Copy` handle, not the signature inline) — added when `Expr::
    /// IndirectCall` was wired up: invoking a closure-typed local needs
    /// its param/return kinds to type-check the call and pick the right
    /// result `Kind`, and a plain flat `Closure` (M3b's original shape)
    /// carried no way to recover which lambda's signature a given local
    /// actually holds. A `Vec<Kind>`/`Box<MethodSig>` field directly on
    /// this variant would force `Kind` to drop its `Copy` derive (the
    /// same concern `Kind::Array` already navigates by staying flat) —
    /// interning the signature in a side table and storing only its
    /// index keeps `Kind` itself exactly as small and `Copy` as before.
    Closure(u32),
    /// A Java array's own kind -- M4a's addition, extended to real
    /// multi-dimensional arrays in M4d. Carries the *element* kind
    /// (`ArrayElemKind`, a small flat `Copy` enum -- not a boxed,
    /// recursive `Kind`) and a dimension count (`u8`, always `>= 1`,
    /// capped at `MAX_ARRAY_DIMS`) -- deliberately *not* a recursive
    /// `Kind::Array(Box<Kind>)`: `Kind` itself derives `Copy` and is
    /// threaded by value through hundreds of call sites across this
    /// file, so a boxed field would force `Kind` to drop `Copy` and
    /// ripple `.clone()` calls through nearly every one of them. A
    /// multi-dimensional Java array is representationally a *nested
    /// sequence of sequences* (`int[][] grid = {{1,2},{3,4}};` lowers to
    /// a `SeqLit` of `SeqLit`s, exactly the shape SIR16 already supports
    /// with no new IR node needed) -- so the dimension count alone,
    /// alongside the always-scalar `ArrayElemKind`, is enough to
    /// represent it without `Kind` ever needing to nest. Indexing peels
    /// one dimension at a time (`grid[i]` has kind `Array(elem, dims -
    /// 1)` when `dims > 1`, or plain `elem.as_kind()` once `dims == 1`
    /// exactly like M4a's original single-dimensional behavior) -- see
    /// `lower_primary_expression`'s own doc comment for how a chained
    /// `grid[i][j]` reaches this.
    Array(ArrayElemKind, u8),
}

/// The element kind of a single-dimensional Java array (see `Kind::
/// Array`'s own doc comment for why this is a separate, non-recursive
/// type rather than `Box<Kind>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArrayElemKind {
    Int,
    Float,
    Bool,
    Str,
}

impl ArrayElemKind {
    /// The `Kind` of one *fully-indexed* element of an array of this
    /// element kind (e.g. `xs[i]`'s own result kind, for a single-
    /// dimensional `xs: Array(Int, 1)`). For a multi-dimensional array,
    /// indexing once peels only one dimension — see `Kind::index_once`,
    /// which calls this only once `dims` has reached `1`.
    fn as_kind(self) -> Kind {
        match self {
            ArrayElemKind::Int => Kind::Int,
            ArrayElemKind::Float => Kind::Float,
            ArrayElemKind::Bool => Kind::Bool,
            ArrayElemKind::Str => Kind::Str,
        }
    }

    /// The inverse of [`ArrayElemKind::as_kind`]: `None` for any `Kind`
    /// that isn't itself a valid array element kind (`Kind::Null`/
    /// `Void`/`Closure`/`Array(_)` — an array-of-arrays' own *element*
    /// kind is still always a scalar `ArrayElemKind`, since `Kind::
    /// Array`'s own dimension count already carries the nesting; the
    /// others are non-value placeholder kinds that can't be an array
    /// element at all). Shared by every call site that resolves a scalar
    /// `Kind` into an array's own element kind (`kind_of_type_node`,
    /// `lower_array_initializer`, and M4c's `lower_new_sized_array`).
    fn from_kind(kind: Kind) -> Option<ArrayElemKind> {
        match kind {
            Kind::Int => Some(ArrayElemKind::Int),
            Kind::Float => Some(ArrayElemKind::Float),
            Kind::Bool => Some(ArrayElemKind::Bool),
            Kind::Str => Some(ArrayElemKind::Str),
            Kind::Null | Kind::Void | Kind::Closure(_) | Kind::Array(_, _) => None,
        }
    }
}

impl Kind {
    /// The `Kind` produced by indexing *once* into a value of this kind
    /// — `xs[i]`'s own result kind, for `xs: Kind::Array(elem, dims)`.
    /// Peels exactly one dimension: `dims > 1` still leaves an array
    /// (`Kind::Array(elem, dims - 1)`, itself indexable again — this is
    /// what lets `grid[i][j]` chain, one `index_once` call per `[...]`
    /// suffix in `lower_primary_expression`'s own chained-index fold);
    /// `dims == 1` bottoms out at the plain element kind
    /// ([`ArrayElemKind::as_kind`]). `None` for any non-array kind — the
    /// caller must reject "indexing is only supported on an array-typed
    /// value" itself, mirroring every other kind-mismatch rejection in
    /// this crate.
    fn index_once(self) -> Option<Kind> {
        match self {
            Kind::Array(elem, dims) if dims > 1 => Some(Kind::Array(elem, dims - 1)),
            Kind::Array(elem, _) => Some(elem.as_kind()),
            _ => None,
        }
    }
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
    /// `lookup_local_with_frame` below are this crate's own mirror of that stack,
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
    /// Counter for the synthetic "have we run the update clause yet"
    /// flag a classic `for` loop with a non-empty update clause lowers
    /// to (`__for_first_0`, `__for_first_1`, …) — see
    /// `lower_for_statement_inner`'s own doc comment. Mirrors
    /// `do_while_counter`'s uniqueness role for a different synthetic
    /// name; never consulted by name lookup.
    for_counter: usize,
    /// Depth of syntactically-enclosing Java loops (`while`/`do`-`while`/
    /// classic or enhanced `for`) around the statement currently being
    /// lowered — `0` means "not inside a loop at all". `lower_break_
    /// statement`/`lower_continue_statement` consult this to reject a
    /// `break`/`continue` outside any loop with a Java-flavored error,
    /// mirroring `javac`'s own diagnostic rather than relying solely on
    /// the shared `semantic-ir` validator's more generic one. Saved to
    /// `0` and restored (never just incremented past) around a lambda
    /// body's own lowering (`lower_lambda_expression`) and a method
    /// body's own lowering (`lower_method_declaration`) — real Java
    /// forbids `break`/`continue` from reaching an outer loop across
    /// either of those boundaries, so a bare `break;` written directly
    /// inside a lambda passed to, say, `list.forEach(x -> { break; })`
    /// must be rejected even though that lambda happens to be lexically
    /// nested inside an enclosing loop.
    loop_depth: usize,
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
    /// Stack of currently-open lambda bodies, innermost last — M3b's
    /// addition, empty except while lowering a `lambda_expression`'s own
    /// body. Drives capture discovery: see `resolve_name`'s own doc
    /// comment for the full design (mirrors `javascript-to-semantic-ir`'s
    /// "on-resolve" approach, adapted from that crate's one-frame-per-
    /// function scope stack to this crate's own one-frame-per-*block*
    /// `locals` stack).
    closure_stack: Vec<ClosureFrame>,
    /// Counter for the synthesized `__lambda_N` function name each
    /// lambda expression lowers to — mirrors `do_while_counter`'s own
    /// monotonic-uniqueness role, just for a different synthetic name.
    lambda_counter: usize,
    /// Every lambda body lowered so far, as a synthesized top-level
    /// `Function` (`__lambda_N`), accumulated while lowering method
    /// bodies and appended to `Module.functions` once at the end of
    /// `lower_program` — mirrors `javascript-to-semantic-ir`'s own
    /// `synthesised` list.
    synthesized_functions: Vec<Function>,
    /// Every lambda's own call signature (param kinds + return kind),
    /// indexed by the `u32` a `Kind::Closure(idx)` carries — the side
    /// table `Kind::Closure`'s own doc comment explains why the
    /// signature is interned here rather than stored inline. Appended to
    /// (never removed from) each time a `lambda_expression` is lowered;
    /// `lower_call_expression`'s own indirect-call path looks a callee's
    /// signature up here once `resolve_name` reports it resolved to a
    /// `Closure`-kinded local rather than a real top-level method name.
    closure_signatures: Vec<MethodSig>,
    /// Counter for the synthetic `__idx_seq_N`/`__idx_at_N` temp-variable
    /// pair each compound-assignment or increment/decrement on an
    /// *indexed* target lowers to (`xs[i] += v;`, `xs[i]++;`) — see
    /// `lower_indexed_compound_assignment`/`lower_indexed_incdec`'s own
    /// doc comments for why the temps exist at all. Mirrors
    /// `do_while_counter`'s monotonic-uniqueness role, just for a
    /// different synthetic name; never consulted by name lookup.
    indexed_temp_counter: usize,
    /// The single top-level class's own name, captured once in
    /// `lower_program` — M5's own addition (task #67), previously parsed
    /// (via `collect_bounded(..., "class_declaration", ...)`) but never
    /// captured, since nothing before M5 needed it. Consulted only by
    /// `lower_static_method_call` to recognize `ClassName.staticMethod(
    /// args)` as a *self*-reference to this same compilation unit's own
    /// class -- an external/JDK static (`Math.PI`, `System.out`) is a
    /// different `class_ref` and stays rejected.
    class_name: String,
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
    /// Whether the method declared the `static` modifier — M5's own
    /// addition, unused before task #67. `main` is always `true` (real
    /// Java requires it). Consulted only by `lower_static_method_call`,
    /// to reject `ClassName.instanceMethod()` the same way real
    /// `javac` does, rather than silently allowing it just because
    /// this frontend has no receiver/`this` model to actually enforce
    /// instance semantics with yet.
    is_static: bool,
}

/// One entry per currently-open lambda body (see `Lowerer::closure_stack`).
/// Tracks where its own scope begins in the shared `locals` stack and
/// accumulates the captures discovered while lowering its body.
struct ClosureFrame {
    /// `self.locals.len()` at the moment this lambda's own first scope
    /// frame was pushed (immediately before its own parameters were
    /// declared). Any `locals` frame at an index *below* this mark
    /// belongs to an enclosing scope (the containing method, or an outer
    /// lambda); any frame at index *at or above* this mark belongs to
    /// this lambda's own body. See `resolve_name`'s own doc comment for
    /// how this mark is used to detect a capture.
    locals_mark: usize,
    /// The lambda expression's own span — used as the span of every
    /// `CaptureValue`'s `value` expression, since a capture's value is
    /// conceptually read at the point the closure literal itself is
    /// constructed, not at any particular reference inside its body.
    span: Span,
    /// Captures discovered so far, in first-reference order, deduplicated
    /// by name (each name is captured at most once per lambda, however
    /// many times its body references it). Becomes the synthesized
    /// `Function`'s own `captures` field.
    captures: Vec<Capture>,
    /// Parallel to `captures`: for each captured name, the expression
    /// (evaluated in the *enclosing* scope) supplying its value at the
    /// point this lambda literal is constructed. Becomes `Expr::
    /// MakeClosure`'s own `captures: Vec<CaptureValue>`.
    capture_values: Vec<CaptureValue>,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
            locals: Vec::new(),
            do_while_counter: 0,
            for_counter: 0,
            loop_depth: 0,
            method_signatures: HashMap::new(),
            current_method: String::new(),
            call_graph: HashMap::new(),
            closure_stack: Vec::new(),
            lambda_counter: 0,
            synthesized_functions: Vec::new(),
            closure_signatures: Vec::new(),
            indexed_temp_counter: 0,
            class_name: String::new(),
        }
    }

    /// Pick a synthetic local name of the form `{prefix}_{N}` guaranteed
    /// not to collide with any name currently in scope — used by
    /// `lower_indexed_compound_assignment`/`lower_indexed_incdec` to name
    /// the temp bindings that hold an indexed target's `seq`/index
    /// expressions, evaluated exactly once. Simpler than `fresh_flag_name`
    /// (no `DeclaredNameCollector` scan of a `body`): those temps live
    /// only inside a synthetic `Expr::Block` this crate builds itself,
    /// containing nothing but the `LetStarBinding`s and the one `SeqSet`
    /// it constructs — there is no arbitrary user-authored body for a
    /// later declaration to shadow the name from within, unlike the
    /// do-while flag (which shares scope with the loop body itself). Only
    /// ambient-scope collision (an already-in-scope real local happening
    /// to share the candidate name) is possible, so a single
    /// `lookup_local_with_frame` check is enough.
    fn fresh_temp_name(&mut self, prefix: &str) -> String {
        loop {
            let candidate = format!("{prefix}_{}", self.indexed_temp_counter);
            self.indexed_temp_counter += 1;
            if self.lookup_local_with_frame(&candidate).is_none() {
                return candidate;
            }
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

    /// Look up `name` *and* the `locals` frame index it was found in,
    /// searching from the innermost scope outward — exactly the lexical-
    /// shadowing order a real name lookup needs, and the frame index
    /// `resolve_name` needs to decide whether a reference crosses a
    /// lambda's own `ClosureFrame::locals_mark` (i.e. is a genuine
    /// capture) or is simply local/param to whichever function is
    /// currently being lowered. Returns the declaration's `Kind` *and*
    /// its `Scope` tag (`Local` or `Param`) — see the `locals` field's
    /// own doc comment for why both matter to every caller that goes on
    /// to build a `VarRef`/`Assign`.
    fn lookup_local_with_frame(&self, name: &str) -> Option<(Kind, Scope, usize)> {
        self.locals
            .iter()
            .enumerate()
            .rev()
            .find_map(|(i, frame)| frame.get(name).map(|&(k, s)| (k, s, i)))
    }

    /// Picks a synthetic guard-flag name of the form `{prefix}_{N}`
    /// (starting at `*counter`, incrementing `*counter` past every
    /// candidate tried, including the one finally chosen) that collides
    /// with **neither**:
    ///
    /// 1. any name currently visible in the *ambient* scope at the call
    ///    site (an outer local, a parameter, ...) — checked via `self.
    ///    lookup_local_with_frame`, the same lookup every ordinary name
    ///    reference goes through; nor
    /// 2. any name `body` itself *declares*, anywhere within it (inside a
    ///    nested `if`, `while`, `for`, ...) — checked via
    ///    `DeclaredNameCollector`, since those declarations live in a
    ///    scope frame this crate has already popped by the time the
    ///    caller (`lower_do_while_statement`/`lower_for_statement_inner`)
    ///    picks the flag name, so `lookup_local_with_frame` alone can't
    ///    see them.
    ///
    /// Both checks are necessary: (1) alone misses a body-declared local
    /// shadowing the flag (a `do`/`while`/`for` body scope has already
    /// closed by the time this runs); (2) alone misses an *outer* local
    /// of the same name that the body only reads/writes, never
    /// redeclares (`do { flag_name = flag_name + 1; } while (...)`) —
    /// still a real collision, since the flag lives in the very same
    /// synthetic `Expr::Block` as the loop, which several backends
    /// (`semantic-ir-to-python`, `semantic-ir-to-ruby`) compile as one
    /// flat scope shared with everything the loop references from
    /// outside it too.
    ///
    /// # Why a name-uniqueness check, not an unforgeable-character trick
    ///
    /// An earlier version of this fix used `#` (illegal in a Java
    /// identifier per JLS §3.8) in the flag's own name, reasoning that no
    /// real Java source could ever spell it. A second `/security-review`
    /// round proved that reasoning false: every backend's `sanitize_ident`
    /// (e.g. `semantic-ir-to-python::sanitize_ident`) exists precisely to
    /// turn an *arbitrary* string into a legal identifier in its target
    /// language by escaping illegal characters — so `sanitize_ident("__do_
    /// while#0")` produces an ordinary, `#`-free string (e.g.
    /// `___do_while_230` under Python's hex-escape scheme) that a real
    /// Java program CAN declare directly. Since `sanitize_ident` is
    /// idempotent on names that are already legal (the overwhelming
    /// majority of real Java identifiers, which use only ASCII
    /// letters/digits/underscore — a subset of both Python's and Ruby's
    /// own identifier alphabets), a plain, escape-free candidate name
    /// collides with a real Java local if and only if that local's raw
    /// source name is *exactly* the candidate string — no backend-
    /// specific escaping knowledge is needed to detect that. This method
    /// checks the candidate directly against real Java names instead,
    /// retrying with the next counter value on a hit, which closes the
    /// hole completely rather than relying on a name no real Java source
    /// is expected to spell.
    ///
    /// Takes the starting counter *by value* and returns `(name,
    /// next_counter)` rather than a `&mut usize` in/out parameter: the
    /// caller reads its own `do_while_counter`/`for_counter` field
    /// (a plain `Copy` read, not a borrow) before calling, so this
    /// `&self` method and the caller's later `self.do_while_counter =
    /// next` assignment never overlap as simultaneous borrows of `self`.
    fn fresh_flag_name(&self, prefix: &str, start_counter: usize, body: &Block) -> (String, usize) {
        let mut collector = DeclaredNameCollector {
            names: HashSet::new(),
        };
        collector.visit_block(body, 0);
        let mut counter = start_counter;
        loop {
            let candidate = format!("{prefix}_{counter}");
            counter += 1;
            if self.lookup_local_with_frame(&candidate).is_none()
                && !collector.names.contains(&candidate)
            {
                return (candidate, counter);
            }
        }
    }

    /// Resolve a bare name for use as a `VarRef`/assignment target,
    /// capture-aware — what every *use* of a resolved name should call.
    ///
    /// Mirrors `javascript-to-semantic-ir`'s "on-resolve" capture
    /// discovery: a capture is never pre-scanned for, it simply falls
    /// out of ordinary name resolution the first time a lambda body
    /// references a name it doesn't itself declare. Adapted from that
    /// crate's one-`FnScope`-per-*function* design to this crate's own
    /// one-frame-per-*block* `locals` stack via `closure_stack`: each
    /// open lambda records the `locals.len()` at the moment its own
    /// scope began (`ClosureFrame::locals_mark`), so "does this
    /// reference cross a lambda boundary" is just "did `lookup_local`
    /// find the name at a frame index below that mark".
    ///
    /// A reference crossing *more than one* nested lambda boundary (a
    /// lambda inside a lambda capturing from the outermost enclosing
    /// method) is threaded through every intermediate boundary in turn —
    /// each records its own capture of the name, using the *previous*
    /// boundary's own capture as its value once one exists — exactly
    /// mirroring `javascript-to-semantic-ir`'s identically-reasoned
    /// `resolve_local_chain`.
    ///
    /// Idempotent: once a name is captured by a given lambda, `record_capture`
    /// installs a `Scope::Capture` entry directly into that lambda's own
    /// first scope frame, so a *second* reference to the same name from
    /// within the same lambda body resolves directly via the ordinary
    /// `lookup_local_with_frame` walk (frame index at-or-above that
    /// lambda's own mark) without re-crossing the boundary — a name
    /// referenced many times in one lambda is captured only once.
    ///
    /// When `closure_stack` is empty (not currently lowering a lambda
    /// body), behaves identically to `lookup_local`.
    fn resolve_name(&mut self, name: &str) -> Option<(Kind, Scope)> {
        let (kind, scope, frame_idx) = self.lookup_local_with_frame(name)?;
        if self.closure_stack.is_empty() {
            return Some((kind, scope));
        }
        let crossed: Vec<usize> = (0..self.closure_stack.len())
            .filter(|&k| frame_idx < self.closure_stack[k].locals_mark)
            .collect();
        let mut current_scope = scope;
        for k in crossed {
            current_scope = self.record_capture(k, name, kind, current_scope);
        }
        Some((kind, current_scope))
    }

    /// Record `name` (already known to have kind `kind`, currently
    /// reachable as `value_scope` one level out from lambda boundary
    /// `k`) as one of that lambda's own captures, if not already
    /// recorded. Returns `Scope::Capture` — what every reference to
    /// `name` from *within* this lambda (or a lambda nested further
    /// inside it) must use from this point on.
    fn record_capture(&mut self, k: usize, name: &str, kind: Kind, value_scope: Scope) -> Scope {
        let mark = self.closure_stack[k].locals_mark;
        if !self.locals[mark].contains_key(name) {
            let span = self.closure_stack[k].span.clone();
            let value = Expr::VarRef {
                name: name.to_string(),
                scope: value_scope,
                span,
            };
            self.closure_stack[k].captures.push(Capture {
                name: name.to_string(),
                sir_type: None,
            });
            self.closure_stack[k].capture_values.push(CaptureValue {
                name: name.to_string(),
                value,
            });
            self.locals[mark].insert(name.to_string(), (kind, Scope::Capture));
        }
        Scope::Capture
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

        self.class_name = self.class_name_of(class_decl).ok_or_else(|| {
            self.err_at(
                class_decl,
                "malformed class declaration (missing name)".to_string(),
            )
        })?;

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
        // Every lambda expression lowered while lowering method bodies
        // above hoisted its own body into `synthesized_functions`
        // (M3b) -- append them now. Order relative to the methods above
        // is cosmetic (the validator resolves names against the whole
        // set, and a synthesized `__lambda_N` is never called by a
        // `DirectCall` from method-name text anyway), so simple
        // append-after mirrors this crate's own preference for the
        // least surprising, most direct code over a more elaborate
        // ordering convention.
        functions.append(&mut self.synthesized_functions);

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

    /// Extract `class_decl`'s own name -- M5's own addition (task #67).
    /// `class_declaration`'s grammar production is `{class_modifier}
    /// "class" NAME [extends class_type] [implements interface_type_
    /// list] class_body`: the class's own name is the *only* bare `NAME`
    /// token appearing as a direct child (an `extends`/`implements`
    /// clause, if present, nests its own type names one level deeper
    /// inside a `class_type`/`interface_type_list` node, never as a
    /// direct child here) -- mirrors `method_name`'s identical "first
    /// direct-child `NAME` token" technique for `method_declarator`.
    fn class_name_of(&self, class_decl: &GrammarASTNode) -> Option<String> {
        for child in &class_decl.children {
            if let ASTNodeOrToken::Token(t) = child {
                if t.type_ == lexer::token::TokenType::Name {
                    return Some(t.value.clone());
                }
            }
        }
        None
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
                is_static: true,
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
            is_static: self.method_is_static(decl),
        })
    }

    /// Whether `decl` (a `method_declaration`) carries the `static`
    /// modifier — M5's own addition. `method_declaration`'s own grammar
    /// production is `{method_modifier} result_type method_declarator
    /// ...`, so every modifier is a direct child `method_modifier` node
    /// wrapping exactly one keyword token; this just scans those for
    /// `"static"`.
    fn method_is_static(&self, decl: &GrammarASTNode) -> bool {
        decl.children.iter().any(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "method_modifier" => n
                .children
                .iter()
                .any(|mc| matches!(mc, ASTNodeOrToken::Token(t) if t.value == "static")),
            _ => false,
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
            self.reject_dollar_sign_identifier(&name_tok.value, fp)?;
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
        // A method's own body is its own statement-flow boundary — same
        // reasoning as `lower_lambda_expression`'s identical save/
        // restore, applied here defensively: `self` is shared across
        // every method in the module (see `lower_program`'s own
        // sequential-methods loop), and every loop arm already
        // increments/decrements `loop_depth` in balanced pairs, so in
        // practice this is always already `0` on entry here. The
        // explicit reset makes that an enforced invariant rather than an
        // implicit one a future refactor could quietly break.
        let saved_loop_depth = std::mem::take(&mut self.loop_depth);
        let body = match self.lower_method_body_block(block, return_kind, 0) {
            Ok(b) => b,
            Err(e) => {
                self.loop_depth = saved_loop_depth;
                self.pop_scope();
                return Err(e);
            }
        };
        self.loop_depth = saved_loop_depth;
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
        if let Some(break_stmt) = self.first_child_named(statement, "break_statement") {
            return self.lower_break_statement(break_stmt);
        }
        if let Some(continue_stmt) = self.first_child_named(statement, "continue_statement") {
            return self.lower_continue_statement(continue_stmt);
        }
        Err(self.err_at(
            statement,
            "unsupported statement kind (JV02 supports variable declarations, assignment, if/while/do-while/for/enhanced-for, bare break/continue, and bare expression statements — switch still has no SIR IR at all, everything else is deferred further)"
                .to_string(),
        ))
    }

    /// `break_statement = "break" [ NAME ] SEMICOLON ;`. Lowers to
    /// `Stmt::Break` — bare (unlabeled) only, matching SIR's own
    /// bare-only `Stmt::Break`/`Stmt::Continue` (see the SIR16 addendum's
    /// "Loop control" section: SIR v0 has no loop-label vocabulary at
    /// all). A labeled `break foo;` is rejected cleanly rather than
    /// mis-targeting the wrong enclosing loop. Rejects a bare `break;`
    /// outside any loop with a Java-flavored diagnostic — the shared
    /// `semantic-ir` validator would also catch this (its own `loop_
    /// stack` tracking independently enforces the same rule), but
    /// `self.loop_depth` lets this frontend give a clearer error before
    /// ever reaching that shared, more generic check.
    fn lower_break_statement(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Stmt, JavaLowerError> {
        let span = self.span_of(node);
        if let Some(label) = label_token(node) {
            return Err(self.err_at(
                node,
                format!(
                    "labeled `break {label};` is not supported yet (deferred — SIR has no loop-label vocabulary; only a bare `break;` targeting the nearest enclosing loop is supported)"
                ),
            ));
        }
        if self.loop_depth == 0 {
            return Err(self.err_at(node, "`break` outside a loop".to_string()));
        }
        self.observed.add(Feature::LoopControl);
        Ok(Stmt::Break { span })
    }

    /// `continue_statement = "continue" [ NAME ] SEMICOLON ;`. Mirrors
    /// `lower_break_statement` exactly — see its own doc comment for the
    /// bare-only and outside-a-loop rejection rationale, both identical
    /// here.
    fn lower_continue_statement(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Stmt, JavaLowerError> {
        let span = self.span_of(node);
        if let Some(label) = label_token(node) {
            return Err(self.err_at(
                node,
                format!(
                    "labeled `continue {label};` is not supported yet (deferred — SIR has no loop-label vocabulary; only a bare `continue;` targeting the nearest enclosing loop is supported)"
                ),
            ));
        }
        if self.loop_depth == 0 {
            return Err(self.err_at(node, "`continue` outside a loop".to_string()));
        }
        self.observed.add(Feature::LoopControl);
        Ok(Stmt::Continue { span })
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
        self.loop_depth += 1;
        let body = self.lower_body(body_stmt, depth + 1);
        self.loop_depth -= 1;
        let body = body?;
        self.observed.add(Feature::Loops);
        Ok(Stmt::While { cond, body, span })
    }

    /// `do_while_statement = "do" statement "while" LPAREN expression
    /// RPAREN SEMICOLON ;`. SIR's `Stmt::While` is pretest-only (there is
    /// no do-while primitive), so this desugars `do S while (C);` to a
    /// synthetic flag-guarded pretest loop — `boolean __do_while_N =
    /// true; while (__do_while_N ? ({ __do_while_N = false; true }) : (C))
    /// { S }` — lowering `S` exactly **once**.
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
    /// **The flag-clear lives inside the loop *condition*, not appended
    /// to the body** — a second correctness bug, distinct from the
    /// cloning DoS above, found while wiring `Stmt::Continue` support
    /// (task #64): an earlier version of this same rewrite instead used
    /// `while (__do_while_N || C) { S; __do_while_N = false; }`, with the
    /// flag-clear appended as `S`'s own trailing statement. SIR's
    /// `Stmt::Continue` jumps straight back to re-evaluating a `While`'s
    /// own `cond` — so a `continue` anywhere inside `S` (before reaching
    /// that trailing statement) would skip the flag-clear entirely,
    /// leaving `__do_while_N` permanently `true`. Since `flag || C`
    /// short-circuits once `flag` is `true`, `C` would then never even be
    /// evaluated again — an unconditional infinite loop regardless of
    /// `C`'s real value, on the very first `continue` a real Java
    /// `do`/`while` executes. This was inert (unreachable) before this
    /// crate had any way to lower `continue` at all; wiring that support
    /// is exactly what would have made it live. Embedding the flag-clear
    /// in the condition itself — the one place a `continue` can never
    /// skip — closes this the same way `lower_for_statement_inner`'s own
    /// analogous update-clause fix does (see that function's doc
    /// comment for the general pattern this mirrors).
    ///
    /// `__do_while_N`'s uniqueness comes from `do_while_counter` (a
    /// monotonic per-`Lowerer` counter — two sibling do-while statements
    /// in the same function must not share a flag) *and* [`fresh_flag_
    /// name`]'s collision check against every name `body` itself declares
    /// (at any nesting depth): the flag's own reference lives inside the
    /// loop's *condition*, which several backends (`semantic-ir-to-
    /// python`, `semantic-ir-to-ruby`) compile with FLAT scoping relative
    /// to the body — no new scope opens for either — so a body-declared
    /// local sharing the flag's exact name would re-arm it to `true` on
    /// every iteration, making the loop **run forever** regardless of the
    /// real condition. `__do_while_0` is a legal Java identifier, so a
    /// program that happens to declare a body-local by that exact name is
    /// a real, reachable case, not a hypothetical one — see `fresh_flag_
    /// name`'s own doc comment for why a plain, escape-free name plus a
    /// direct collision check is the fix, not an attempted "unforgeable"
    /// character.
    fn lower_do_while_statement(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Stmt, JavaLowerError> {
        let span = self.span_of(node);
        let body_stmt = self.first_child_named(node, "statement").ok_or_else(|| {
            self.err_at(node, "malformed `do`/`while` (missing body)".to_string())
        })?;
        self.loop_depth += 1;
        let body = self.lower_body(body_stmt, depth + 1);
        self.loop_depth -= 1;
        let body = body?;
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
        self.observed.add(Feature::MutableBindings);

        // See `fresh_flag_name`'s own doc comment for why this checks
        // both ambient scope and every name `body` declares (at any
        // nesting depth), not an attempt to pick a name no real Java
        // source could spell.
        let (flag_name, next_counter) =
            self.fresh_flag_name("__do_while", self.do_while_counter, &body);
        self.do_while_counter = next_counter;

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
        // `flag ? ({ flag = false; true }) : (C)` — on the first
        // condition check, unconditionally clear the flag and enter the
        // body (do-while always runs `S` at least once); every
        // subsequent check (including one reached via `continue`, which
        // jumps straight here) evaluates the real condition `C`.
        let loop_cond = Expr::If {
            cond: Box::new(flag_ref),
            then_branch: Box::new(Block {
                stmts: vec![Stmt::Assign {
                    name: flag_name,
                    scope: Scope::Local,
                    value: Expr::BoolLit {
                        value: false,
                        span: span.clone(),
                    },
                    span: span.clone(),
                }],
                value: Expr::BoolLit {
                    value: true,
                    span: span.clone(),
                },
                span: span.clone(),
            }),
            else_branch: Box::new(Block {
                stmts: vec![],
                value: cond,
                span: span.clone(),
            }),
            span: span.clone(),
        };

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
    /// classic `for` is highly variable in shape).
    ///
    /// When there is no update clause (`for (init; cond;) S`, or `for
    /// (;;)`), the shape is the obvious `{ init; while (cond) { S } }` —
    /// nothing for a `continue` inside `S` to skip, since SIR's own
    /// `Stmt::Continue` already jumps straight back to re-evaluating
    /// `cond`, exactly matching Java's own `continue` target here.
    ///
    /// **When there IS an update clause**, the naive `{ init; while
    /// (cond) { S; update; } }` shape (this crate's own earlier version)
    /// is wrong the moment `S` can contain a `continue`: SIR's `Stmt::
    /// Continue` jumps to re-evaluating `cond`, which would skip the
    /// appended `update` entirely — an inert bug before this crate had
    /// any way to lower `continue` at all (task #64 is what first makes
    /// it live), but a real one now. Java's own `continue` inside a
    /// classic `for`'s body jumps to `update`, THEN re-checks `cond` — a
    /// different target than a bare `while`'s `continue` altogether, so
    /// this needs its own rewrite, not just "append `update`, like
    /// before":
    ///
    ///   init;
    ///   boolean __for_first_N = true;
    ///   while (__for_first_N ? ({ __for_first_N = false; cond })
    ///                         : ({ update; cond })) {
    ///     S
    ///   }
    ///
    /// mirroring `lower_do_while_statement`'s own analogous flag-guard
    /// fix (see that function's doc comment for the shared reasoning):
    /// embedding `update` inside the loop *condition* itself puts it at
    /// the one position a `continue` can never skip, since that's
    /// exactly where `continue` re-enters. The flag suppresses `update`
    /// on the very first check only (Java's `for` never runs `update`
    /// before the first `cond` test) and is cleared unconditionally the
    /// first time through — same one-shot-flag idiom `do`/`while`'s own
    /// fix uses, applied to gate `update` instead of the loop-entry
    /// pretest. Applied unconditionally whenever an update clause is
    /// present, not only when `S` is known to contain a `continue` —
    /// mirrors `do`/`while`'s own flag guard, which is likewise applied
    /// to every `do`/`while` regardless of whether it happens to need
    /// it, rather than adding a "does this body contain a `continue`
    /// targeting *this* loop" scanner whose own correctness would be a
    /// new thing to get right.
    ///
    /// A useful side effect of moving `update` out of `S`'s own lowered
    /// `Block.stmts` and into this separate wrapped-condition `Expr::
    /// Block`: `update`'s target name can no longer collide with a local
    /// `S` declares directly (real Java's own `for`-header scope was
    /// never inside `S`'s scope to begin with — this rewrite is actually
    /// *more* faithful to Java's real scoping than the old "append to
    /// body" shape was), so the update-target/body-local collision check
    /// the no-update-clause-needed-none codepath still had to run is no
    /// longer needed at all.
    ///
    /// Wrapped in one synthetic `Expr::Block` — matching `do`/`while`'s
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
        self.loop_depth += 1;
        let body = self.lower_body(body_stmt, depth + 1);
        self.loop_depth -= 1;
        let body = body?;
        self.observed.add(Feature::Loops);

        // No update clause: SIR's own `Stmt::Continue` already jumps
        // straight to re-evaluating `cond`, exactly matching Java's own
        // `continue` target here — nothing further needed, no flag.
        //
        // An update clause: wrap it into the condition itself, gated by
        // a one-shot "have we run the first check yet" flag — see this
        // function's own doc comment (on `lower_for_statement`) for why
        // appending `update` to `body.stmts` (this crate's earlier
        // approach) is wrong the moment `body` can contain a `continue`,
        // and why embedding it in the condition fixes that the same way
        // `lower_do_while_statement`'s own analogous fix does.
        let (flag_decl, while_cond) = match update_stmt {
            None => (None, cond),
            Some(update) => {
                // See `fresh_flag_name`'s own doc comment (and `lower_do_
                // while_statement`'s identically-reasoned flag-name
                // comment) for why this is a direct collision check
                // against every name `body` declares, not an attempt to
                // pick a name no real Java source could spell: the flag
                // reference lives inside the loop's own *condition*,
                // which several backends compile with flat scoping
                // relative to the body (no new scope opened for either),
                // so an unchecked body-declared local sharing the flag's
                // name would re-arm it every iteration and skip `update`
                // forever — an infinite loop.
                let (flag_name, next_counter) =
                    self.fresh_flag_name("__for_first", self.for_counter, &body);
                self.for_counter = next_counter;
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
                let wrapped_cond = Expr::If {
                    cond: Box::new(flag_ref),
                    then_branch: Box::new(Block {
                        stmts: vec![Stmt::Assign {
                            name: flag_name,
                            scope: Scope::Local,
                            value: Expr::BoolLit {
                                value: false,
                                span: span.clone(),
                            },
                            span: span.clone(),
                        }],
                        value: cond.clone(),
                        span: span.clone(),
                    }),
                    else_branch: Box::new(Block {
                        stmts: vec![update],
                        value: cond,
                        span: span.clone(),
                    }),
                    span: span.clone(),
                };
                (Some(flag_decl), wrapped_cond)
            }
        };

        let while_stmt = Stmt::While {
            cond: while_cond,
            body,
            span: span.clone(),
        };
        let mut outer_stmts = Vec::new();
        if let Some(i) = init_stmt {
            outer_stmts.push(i);
        }
        if let Some(f) = flag_decl {
            outer_stmts.push(f);
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
        self.reject_dollar_sign_identifier(&var_name, node)?;
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
        self.loop_depth += 1;
        let body = self.lower_body(body_stmt, depth + 1);
        self.loop_depth -= 1;
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
        self.reject_dollar_sign_identifier(&name, declarator)?;

        let initializer = self.first_child_named(declarator, "variable_initializer").ok_or_else(|| {
            self.err_at(
                declarator,
                "uninitialized local variable declarations are not supported yet (an initializer is required)".to_string(),
            )
        })?;
        let (value, value_kind) = match initializer.children.as_slice() {
            [ASTNodeOrToken::Node(n)] if n.rule_name == "expression" => self.lower_expr(n, 0)?,
            [ASTNodeOrToken::Node(n)] if n.rule_name == "array_initializer" => {
                let declared = match declared_kind {
                    Some(Kind::Array(e, d)) => Some((e, d)),
                    Some(_) => {
                        return Err(self.err_at(
                            n,
                            "an array initializer (`{ ... }`) cannot initialize a non-array-typed declaration".to_string(),
                        ))
                    }
                    None => None, // `var` -- infer the element kind from the elements themselves.
                };
                self.lower_array_initializer(n, declared, 0)?
            }
            _ => return Err(self.err_at(initializer, "malformed variable initializer".to_string())),
        };

        let kind = match declared_kind {
            Some(k) => k,
            None => {
                if value_kind == Kind::Null {
                    return Err(self.err_at(
                        initializer,
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

    /// Lower an `array_initializer` (`LBRACE [variable_initializer
    /// {COMMA variable_initializer}] [COMMA] RBRACE`) into an `Expr::
    /// SeqLit` — the `{1, 2, 3}` shorthand form used directly as a
    /// variable declarator's own initializer (M4a). Each element must
    /// itself be a bare `expression`, not a nested `array_initializer` —
    /// multi-dimensional array literals are out of scope, matching
    /// `kind_of_type_node`'s own single-dimension restriction (deferred
    /// to task #56).
    ///
    /// SIR16's `Expr::SeqLit`/`Feature::Sequences`, not SIR22's `Expr::
    /// ArrayLit`/`Feature::NDArrays`: a Java array is a flat, homogeneous
    /// 1-D sequence, exactly `SeqLit`'s own shape (`items: Vec<Expr>`) —
    /// `ArrayLit`'s own `rows: Vec<Vec<Expr>>` is row-major-matrix-
    /// shaped, built for MATLAB/Octave's true N-dimensional arrays, a
    /// meaningfully different domain. `Feature::Sequences` is also the
    /// older, more foundational SIR16 layer every backend (including
    /// Python, this crate's own execution-proof harness) already
    /// supports, unlike SIR22's newer, narrower-adoption array/matrix
    /// family.
    ///
    /// `declared_elem_kind`: `Some(k)` when the declared type is
    /// explicitly `T[]` (every element must lower to kind `k`); `None`
    /// for `var` (every element must share one common kind, inferred
    /// from the first — an empty `var`-inferred array literal is
    /// rejected, matching real `javac`'s own identical rejection, since
    /// there's nothing to infer from).
    fn lower_array_initializer(
        &mut self,
        array_init: &GrammarASTNode,
        declared: Option<(ArrayElemKind, u8)>,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        if depth >= MAX_EXPR_DEPTH {
            return Err(self.err_at(
                array_init,
                format!("expression nesting exceeds {MAX_EXPR_DEPTH} levels"),
            ));
        }
        // M4d: an explicitly-declared multi-dimensional array type (`dims
        // > 1`) requires every element to itself be a *nested*
        // `array_initializer`, one dimension shallower -- recurses down
        // to the `dims == 1` base case below, which is exactly M4a's
        // original flat logic, untouched. `var`-inferred arrays
        // (`declared: None`) stay restricted to a single dimension this
        // milestone -- inferring a *nested* literal's own dimension count
        // from potentially inconsistent nesting adds real complexity this
        // milestone doesn't need; `int[][] grid = {{1,2}};` (explicit
        // type) works, `var grid = {{1,2}};` remains deferred.
        if let Some((elem, dims)) = declared {
            if dims > 1 {
                let span = self.span_of(array_init);
                let mut items = Vec::new();
                for vi in child_nodes(array_init)
                    .into_iter()
                    .filter(|n| n.rule_name == "variable_initializer")
                {
                    let inner = match vi.children.as_slice() {
                        [ASTNodeOrToken::Node(n)] if n.rule_name == "array_initializer" => n,
                        [ASTNodeOrToken::Node(n)] if n.rule_name == "expression" => {
                            return Err(self.err_at(
                                n,
                                format!("expected a nested array initializer (this array declares {dims} dimensions), found a plain value"),
                            ));
                        }
                        _ => return Err(self.err_at(vi, "malformed array element".to_string())),
                    };
                    let (expr, _kind) =
                        self.lower_array_initializer(inner, Some((elem, dims - 1)), depth + 1)?;
                    items.push(expr);
                }
                self.observed.add(Feature::Sequences);
                return Ok((Expr::SeqLit { items, span }, Kind::Array(elem, dims)));
            }
        }
        // `dims == 1` (explicit single-dimensional type) or `None` (`var`
        // inference) -- M4a's original flat logic, unchanged.
        let span = self.span_of(array_init);
        let mut items = Vec::new();
        let mut elem_kind = declared.map(|(e, _)| e);
        for vi in child_nodes(array_init)
            .into_iter()
            .filter(|n| n.rule_name == "variable_initializer")
        {
            let expr_node = match vi.children.as_slice() {
                [ASTNodeOrToken::Node(n)] if n.rule_name == "expression" => n,
                [ASTNodeOrToken::Node(n)] if n.rule_name == "array_initializer" => {
                    return Err(self.err_at(
                        n,
                        "a nested array literal here needs an explicit, multi-dimensional declared array type (`var`-inferred multi-dimensional array literals are not supported yet)".to_string(),
                    ));
                }
                _ => return Err(self.err_at(vi, "malformed array element".to_string())),
            };
            let (expr, kind) = self.lower_expr(expr_node, depth + 1)?;
            let this_elem_kind = ArrayElemKind::from_kind(kind).ok_or_else(|| {
                self.err_at(
                    expr_node,
                    "array elements must be a primitive or `String` value".to_string(),
                )
            })?;
            match elem_kind {
                Some(k) if k == this_elem_kind => {}
                Some(_) => {
                    return Err(self.err_at(
                        expr_node,
                        "array element's kind does not match the array's own declared or already-inferred element type".to_string(),
                    ));
                }
                None => elem_kind = Some(this_elem_kind),
            }
            items.push(expr);
        }
        let elem_kind = elem_kind.ok_or_else(|| {
            self.err_at(
                array_init,
                "cannot infer an empty array literal's element type without an explicit declared array type".to_string(),
            )
        })?;
        self.observed.add(Feature::Sequences);
        Ok((Expr::SeqLit { items, span }, Kind::Array(elem_kind, 1)))
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
        let bracket_pairs = type_node
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "["))
            .count();
        if bracket_pairs == 0 {
            return self.scalar_kind_of_type_node(type_node);
        }
        if bracket_pairs > MAX_ARRAY_DIMS {
            return Err(self.err_at(
                type_node,
                format!("array type declares more than {MAX_ARRAY_DIMS} dimensions"),
            ));
        }
        let scalar = self.scalar_kind_of_type_node(type_node)?;
        let elem = ArrayElemKind::from_kind(scalar).ok_or_else(|| {
            self.err_at(
                type_node,
                format!("unsupported array element kind `{scalar:?}`"),
            )
        })?;
        Ok(Kind::Array(elem, bracket_pairs as u8))
    }

    /// Resolve `type_node`'s own base (non-array) kind — the shared core
    /// `kind_of_type_node` uses both for a plain scalar type and, after
    /// stripping the array-bracket check, for an array type's own
    /// element kind. `type_node`'s own trailing `{LBRACKET RBRACKET}`
    /// tokens (if any — see `kind_of_type_node`'s own caller) are
    /// ignored here; this function only ever inspects the `primitive_
    /// type`/`class_type` child.
    fn scalar_kind_of_type_node(&self, type_node: &GrammarASTNode) -> Result<Kind, JavaLowerError> {
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
                if let Some((primary, suffixes)) = self.indexed_assign_target(lvalue_node, 0)? {
                    if op_tok.value == "=" {
                        return self.lower_indexed_assignment(primary, suffixes, rhs_node, inner);
                    }
                    return self.lower_indexed_compound_assignment(
                        primary, suffixes, op_tok, op_node, rhs_node, inner,
                    );
                }
                let name = self.extract_bare_name(lvalue_node, 0)?;
                let (declared_kind, declared_scope) =
                    self.resolve_name(&name).ok_or_else(|| {
                        self.err_at(
                            lvalue_node,
                            format!("assignment to undeclared local variable `{name}`"),
                        )
                    })?;
                if declared_scope == Scope::Capture {
                    return Err(self.err_at(
                        lvalue_node,
                        format!("cannot assign to `{name}`: local variables referenced from a lambda body must be effectively final"),
                    ));
                }
                // Caught by `/security-review`: this crate tracks each
                // local's `Kind` only at declaration time -- a plain `=`
                // reassignment lowers the RHS but never re-checks or
                // re-records the declared `Kind` against it (a real,
                // pre-existing gap for every `Kind`, harmless for every
                // *other* variant since none of them carry state that a
                // later expression depends on). `Kind::Closure(idx)` is
                // the one exception: `idx` is load-bearing -- a later
                // `lower_call_expression` trusts it verbatim to look up
                // `closure_signatures[idx]` and type-check the call.
                // Reassigning a closure-typed local would leave that
                // index silently stale (`var f = (int x) -> x+1; f = ()
                // -> 42; f();` would still type-check `f`'s call against
                // its *original* 1-parameter signature, not the 0-
                // parameter closure `f` now actually holds) -- rejected
                // outright rather than mis-tracked, since correctly
                // updating the recorded `Kind` in place would require
                // rewriting the scope frame the name was originally
                // declared in, not just the innermost one (`declare_local`
                // only ever inserts into `self.locals.last_mut()`, which
                // is wrong whenever the assignment happens inside a
                // nested block relative to the declaration).
                if matches!(declared_kind, Kind::Closure(_)) {
                    return Err(self.err_at(
                        lvalue_node,
                        format!("cannot reassign `{name}`: reassigning a lambda-valued variable is not supported yet (deferred to a later JV02 milestone)"),
                    ));
                }
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
            if let Some((primary, suffixes)) = self.indexed_assign_target(target_node, 0)? {
                return self.lower_indexed_incdec(primary, suffixes, op, inner);
            }
            let name = self.extract_bare_name(target_node, 0)?;
            let (declared_kind, declared_scope) = self.resolve_name(&name).ok_or_else(|| {
                self.err_at(
                    target_node,
                    format!("`{op}{op}` on undeclared local variable `{name}`"),
                )
            })?;
            if declared_scope == Scope::Capture {
                return Err(self.err_at(
                    target_node,
                    format!("cannot assign to `{name}`: local variables referenced from a lambda body must be effectively final"),
                ));
            }
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
    /// `primary`, requiring it to be a bare `NAME` — `foo.bar = x` and any
    /// other non-simple target remain out of scope (rejected here rather
    /// than mis-lowered). Reached only after `indexed_assign_target` (M4b)
    /// has already ruled out the other supported target shape (`xs[i] =
    /// v`, `xs[i] += v`, `xs[i]++` — every assignment/incdec operator, not
    /// just `"="`, since task #59 resolved compound-assignment/increment-
    /// decrement on an indexed target too) — this function's own error
    /// message covers every genuinely unsupported case: field targets and
    /// other qualified targets.
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
                    "assignment target must be a simple local variable or an indexed array element (`xs[i] = v`, `xs[i] += v`, `xs[i]++`) -- field targets and other qualified targets are not supported yet".to_string(),
                )),
            };
        }
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.extract_bare_name(only, depth + 1),
            _ => Err(self.err_at(
                node,
                "assignment target must be a simple local variable or an indexed array element (`xs[i] = v`, `xs[i] += v`, `xs[i]++`) -- field targets and other qualified targets are not supported yet".to_string(),
            )),
        }
    }

    /// Walk an assignment target's `unary_expression` chain down to either
    /// a bare `primary` (a simple local-variable target, handled by
    /// `extract_bare_name`) or a `primary_expression` matching M4b's
    /// indexed-assignment shape, generalized to a *chained* target
    /// (`xs[i]`, `grid[i][j]`, … -- one or more `[...]` suffixes, every
    /// one index-only) -- returns `Some((primary, suffixes))` only for
    /// the latter, so `lower_expr_statement` can distinguish "plain name
    /// assignment" (unchanged) from "indexed assignment" (new) before
    /// falling through to `extract_bare_name`'s existing rejection for
    /// every other shape. `suffixes` is always non-empty when `Some`;
    /// callers treat its *last* element as the write index and any
    /// leading elements as read-only peels down to the write target,
    /// mirroring `lower_primary_expression`'s own `is_index_only_suffix`
    /// guard for the value-position chained-read case. A suffix chain
    /// mixing in a `.`/`(` suffix anywhere (a qualified call, field
    /// access, etc.) still falls through to `Ok(None)` below, the same
    /// "reject rather than mis-lower" outcome `extract_bare_name` already
    /// gives every other unsupported target shape.
    fn indexed_assign_target<'a>(
        &self,
        node: &'a GrammarASTNode,
        depth: usize,
    ) -> Result<Option<(&'a GrammarASTNode, &'a [ASTNodeOrToken])>, JavaLowerError> {
        if depth >= MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting exceeds {MAX_EXPR_DEPTH} levels"),
            ));
        }
        if node.rule_name == "primary_expression" {
            if let [ASTNodeOrToken::Node(primary), rest @ ..] = node.children.as_slice() {
                if !rest.is_empty() && rest.iter().all(is_index_only_suffix) {
                    return Ok(Some((primary, rest)));
                }
            }
            return Ok(None);
        }
        if node.rule_name == "primary" {
            return Ok(None);
        }
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(only)] => self.indexed_assign_target(only, depth + 1),
            _ => Ok(None),
        }
    }

    /// Hoist a (possibly chained) indexed-assignment target's own
    /// `primary` and every suffix's index expression into fresh
    /// once-only-evaluated local temps, then rebuild the read-position
    /// target chain (`seq_ref`/`idx_ref`) from those temps' `VarRef`s --
    /// shared by `lower_indexed_compound_assignment`/
    /// `lower_indexed_incdec`, both of which read the current element and
    /// write it back, so every evaluation on the way there (`primary`,
    /// and each `[...]` suffix's own index expression) must happen
    /// exactly once, not once per read/write use (the same
    /// double-evaluation hazard task #59's own single-suffix version of
    /// this hoisting already guards against -- generalized here to N
    /// suffixes instead of exactly one). `lower_indexed_assignment`
    /// (plain `=`) does *not* use this helper: a plain assignment target
    /// is only ever built once, so embedding each lowered sub-expression
    /// directly (no hoisting) is already single-evaluation-safe.
    ///
    /// `suffixes` must be non-empty (guaranteed by every caller reaching
    /// this via `indexed_assign_target`'s own non-empty return). Returns
    /// `(let_bindings, seq_ref, idx_ref, kind_before_final_index)` --
    /// `let_bindings` must be spliced in before whatever statement reads
    /// `seq_ref`/`idx_ref`; `kind_before_final_index` is the peeled kind
    /// *before* applying the last suffix's own index (i.e. `primary`'s
    /// kind after `suffixes.len() - 1` peels) -- each caller applies its
    /// own final `Kind::index_once` with its own error message, since
    /// `lower_indexed_compound_assignment` and `lower_indexed_incdec`
    /// word that rejection differently.
    fn hoist_indexed_target(
        &mut self,
        primary: &GrammarASTNode,
        suffixes: &[ASTNodeOrToken],
        span: &Span,
    ) -> Result<(Vec<Stmt>, Expr, Expr, Kind), JavaLowerError> {
        let (seq_expr, mut kind) = self.lower_expr(primary, 0)?;
        let seq_tmp = self.fresh_temp_name("__idx_seq");
        let mut let_bindings = vec![Stmt::LetStarBinding {
            name: seq_tmp.clone(),
            sir_type: None,
            value: seq_expr,
            span: span.clone(),
        }];
        let mut seq_ref = Expr::VarRef {
            name: seq_tmp,
            scope: Scope::Local,
            span: span.clone(),
        };
        let mut idx_ref: Option<Expr> = None;
        // `suffixes` must be non-empty -- guaranteed by every caller
        // reaching this via `indexed_assign_target`'s own non-empty
        // return (see this function's own doc comment). Guarded
        // defensively so a future edit that loosens that guarantee fails
        // a debug assertion rather than silently underflowing here.
        debug_assert!(
            !suffixes.is_empty(),
            "hoist_indexed_target requires a non-empty suffix chain"
        );
        let last = suffixes.len() - 1;
        for (i, suffix) in suffixes.iter().enumerate() {
            let suffix = match suffix {
                ASTNodeOrToken::Node(s) => s,
                ASTNodeOrToken::Token(_) => unreachable!(
                    "indexed_assign_target only ever returns index-only suffixes, which `is_index_only_suffix` guarantees are Nodes"
                ),
            };
            let index_node = self
                .first_child_named(suffix, "expression")
                .ok_or_else(|| self.err_at(suffix, "malformed array index".to_string()))?;
            let (index_expr, index_kind) = self.lower_expr(index_node, 0)?;
            if index_kind != Kind::Int {
                return Err(self.err_at(index_node, "an array index must be an `int`".to_string()));
            }
            let idx_tmp = self.fresh_temp_name("__idx_at");
            let_bindings.push(Stmt::LetStarBinding {
                name: idx_tmp.clone(),
                sir_type: None,
                value: index_expr,
                span: span.clone(),
            });
            let this_idx_ref = Expr::VarRef {
                name: idx_tmp,
                scope: Scope::Local,
                span: span.clone(),
            };
            if i < last {
                let peeled = kind.index_once().ok_or_else(|| {
                    self.err_at(
                        primary,
                        "indexing (`[...]`) is only supported on an array-typed value".to_string(),
                    )
                })?;
                seq_ref = Expr::SeqIndex {
                    seq: Box::new(seq_ref),
                    index: Box::new(this_idx_ref),
                    span: span.clone(),
                };
                kind = peeled;
            } else {
                idx_ref = Some(this_idx_ref);
            }
        }
        self.observed.add(Feature::Sequences);
        Ok((
            let_bindings,
            seq_ref,
            idx_ref.expect("suffixes is non-empty, so the loop's last iteration always sets idx_ref"),
            kind,
        ))
    }

    /// Lower `xs[i] = v;`/`grid[i][j] = v;` (`primary` indexed by one or
    /// more chained `suffixes`, assigned `rhs`) into `Stmt::SeqSet` --
    /// M4b, generalized in M4d via `Kind::index_once` the same way
    /// `lower_index_get` is: on a multi-dimensional array, `grid[i] = v;`
    /// assigns a whole sub-array (`v` must itself be `Kind::Array(elem,
    /// dims - 1)`), matching real Java's own "an array of arrays"
    /// semantics. `primary` must resolve to an array-typed value; every
    /// index expression must resolve to `Kind::Int`; `rhs` must resolve
    /// to exactly the fully-peeled result kind (no implicit widening --
    /// matches this crate's existing "reject rather than mis-lower"
    /// discipline for every other kind-mismatch case). A *chained* target
    /// (`suffixes.len() > 1`) peels every suffix but the last via
    /// `lower_chained_index` (unchanged from its own value-position use)
    /// and writes through the last suffix's own index -- no temp-hoisting
    /// needed here, unlike `lower_indexed_compound_assignment`/
    /// `lower_indexed_incdec`: a plain assignment target is built exactly
    /// once, so embedding each lowered sub-expression directly is already
    /// single-evaluation-safe.
    ///
    /// `context` is the enclosing `assignment_expression` node, used only
    /// for this statement's own span.
    fn lower_indexed_assignment(
        &mut self,
        primary: &GrammarASTNode,
        suffixes: &[ASTNodeOrToken],
        rhs: &GrammarASTNode,
        context: &GrammarASTNode,
    ) -> Result<Stmt, JavaLowerError> {
        // `suffixes` must be non-empty -- guaranteed by every caller
        // reaching this via `indexed_assign_target`'s own non-empty
        // return. Guarded defensively so a future edit that loosens that
        // guarantee fails a debug assertion rather than silently
        // underflowing the `suffixes.len() - 1` slice bound below.
        debug_assert!(
            !suffixes.is_empty(),
            "lower_indexed_assignment requires a non-empty suffix chain"
        );
        let (seq, seq_kind) = if suffixes.len() == 1 {
            self.lower_expr(primary, 0)?
        } else {
            self.lower_chained_index(primary, &suffixes[..suffixes.len() - 1], primary, 0)?
        };
        let result_kind = seq_kind.index_once().ok_or_else(|| {
            self.err_at(
                primary,
                "indexed assignment (`[...] = ...`) is only supported on an array-typed value"
                    .to_string(),
            )
        })?;
        let last_suffix = match suffixes.last() {
            Some(ASTNodeOrToken::Node(s)) => s,
            _ => unreachable!(
                "indexed_assign_target only ever returns index-only suffixes, which `is_index_only_suffix` guarantees are Nodes"
            ),
        };
        let index_node = self
            .first_child_named(last_suffix, "expression")
            .ok_or_else(|| self.err_at(last_suffix, "malformed array index".to_string()))?;
        let (index, index_kind) = self.lower_expr(index_node, 0)?;
        if index_kind != Kind::Int {
            return Err(self.err_at(index_node, "an array index must be an `int`".to_string()));
        }
        let (value, value_kind) = self.lower_expr(rhs, 0)?;
        if value_kind != result_kind {
            return Err(self.err_at(
                rhs,
                "the assigned value's kind does not match the array's own element kind".to_string(),
            ));
        }
        self.observed.add(Feature::Sequences);
        let span = self.span_of(context);
        Ok(Stmt::SeqSet {
            seq,
            index,
            value,
            span,
        })
    }

    /// Lower `xs[i] += v;`/`grid[i][j] -= v;`/etc. (a compound-assignment
    /// operator on a possibly-chained indexed target) — closes the gap
    /// `lower_indexed_assignment`'s own doc comment names as deferred: a
    /// compound assignment reads the current element *and* writes it
    /// back, so `primary` and every suffix's own index expression must
    /// each be evaluated exactly **once**, not once per read/write use —
    /// naively re-lowering (or even cloning an already-lowered `Expr` and
    /// embedding it twice) would make the *emitted* target-language code
    /// evaluate a non-constant index expression (e.g. `xs[next()] +=
    /// v;`) twice, silently double-evaluating any side effect it carries.
    /// Fixed via `hoist_indexed_target`'s shared temp-hoisting (task #59's
    /// original single-suffix version, generalized to N suffixes) —
    /// wrapped in one synthetic `Expr::Block` so this still returns
    /// exactly one `Stmt` to `lower_expr_statement`'s caller.
    ///
    /// `op_tok`/`op_node` are the already-matched assignment-operator
    /// token/node from `lower_expr_statement`'s own dispatch — reused
    /// here rather than re-extracted. Only `+= -= *= /= %=` are
    /// supported, mirroring the plain-name compound-assignment path's own
    /// operator set exactly (`&= |= ^= <<= >>= >>>=` fall through to the
    /// same "deferred" rejection that path already gives).
    fn lower_indexed_compound_assignment(
        &mut self,
        primary: &GrammarASTNode,
        suffixes: &[ASTNodeOrToken],
        op_tok: &lexer::token::Token,
        op_node: &GrammarASTNode,
        rhs: &GrammarASTNode,
        context: &GrammarASTNode,
    ) -> Result<Stmt, JavaLowerError> {
        let op_char = match op_tok.value.as_str() {
            "+=" | "-=" | "*=" | "/=" | "%=" => {
                op_tok.value.chars().next().expect("non-empty operator token")
            }
            other => {
                return Err(self.err_at(
                    op_node,
                    format!("unsupported assignment operator `{other}` (deferred to a later JV02 milestone)"),
                ))
            }
        };
        let span = self.span_of(context);
        let (mut stmts, seq_ref, idx_ref, kind_before_final_index) =
            self.hoist_indexed_target(primary, suffixes, &span)?;
        let result_kind = kind_before_final_index.index_once().ok_or_else(|| {
            self.err_at(
                primary,
                "indexed assignment (`[...] = ...`) is only supported on an array-typed value"
                    .to_string(),
            )
        })?;
        let (rhs_expr, rhs_kind) = self.lower_expr(rhs, 0)?;
        let lhs_read = Expr::SeqIndex {
            seq: Box::new(seq_ref.clone()),
            index: Box::new(idx_ref.clone()),
            span: span.clone(),
        };
        let value = match op_char {
            '+' | '-' => {
                self.combine_additive(lhs_read, result_kind, rhs_expr, rhs_kind, op_char, op_node)?.0
            }
            '*' | '/' | '%' => {
                self.combine_multiplicative(lhs_read, result_kind, rhs_expr, rhs_kind, op_char, op_node)?.0
            }
            _ => unreachable!("compound assignment operator token was matched but its leading char isn't one of + - * / %"),
        };
        stmts.push(Stmt::SeqSet {
            seq: seq_ref,
            index: idx_ref,
            value,
            span: span.clone(),
        });
        Ok(Stmt::ExprStmt {
            expr: Expr::Block(Box::new(Block {
                stmts,
                value: Expr::NilLit { span: span.clone() },
                span: span.clone(),
            })),
            span,
        })
    }

    /// Lower `xs[i]++;`/`grid[i][j]--;`/`++xs[i];`/`--grid[i][j];`
    /// (increment or decrement on a possibly-chained indexed target) —
    /// the other half of the gap `lower_indexed_assignment`'s own doc
    /// comment names as deferred. Desugars to `xs[i] += 1;`/`xs[i] -=
    /// 1;` exactly like the bare-name incdec path already does (see
    /// `lower_expr_statement`'s own handling just above), reusing the
    /// identical `hoist_indexed_target` once-only-evaluation temp-binding
    /// shape `lower_indexed_compound_assignment` uses, for the identical
    /// reason (a non-constant index expression, e.g. `xs[next()]++;`,
    /// must not be evaluated twice).
    fn lower_indexed_incdec(
        &mut self,
        primary: &GrammarASTNode,
        suffixes: &[ASTNodeOrToken],
        op: char,
        context: &GrammarASTNode,
    ) -> Result<Stmt, JavaLowerError> {
        let span = self.span_of(context);
        let (mut stmts, seq_ref, idx_ref, kind_before_final_index) =
            self.hoist_indexed_target(primary, suffixes, &span)?;
        let result_kind = kind_before_final_index.index_once().ok_or_else(|| {
            self.err_at(
                primary,
                "indexing (`[...]`) is only supported on an array-typed value".to_string(),
            )
        })?;
        if !matches!(result_kind, Kind::Int | Kind::Float) {
            let last_suffix = match suffixes.last() {
                Some(ASTNodeOrToken::Node(s)) => s,
                _ => unreachable!(
                    "indexed_assign_target only ever returns index-only suffixes, which `is_index_only_suffix` guarantees are Nodes"
                ),
            };
            return Err(self.err_at(
                last_suffix,
                format!("`{op}{op}` requires a numeric array element"),
            ));
        }
        let lhs_read = Expr::SeqIndex {
            seq: Box::new(seq_ref.clone()),
            index: Box::new(idx_ref.clone()),
            span: span.clone(),
        };
        let one = if result_kind == Kind::Float {
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
            self.combine_additive(lhs_read, result_kind, one, result_kind, op, context)?;
        stmts.push(Stmt::SeqSet {
            seq: seq_ref,
            index: idx_ref,
            value,
            span: span.clone(),
        });
        Ok(Stmt::ExprStmt {
            expr: Expr::Block(Box::new(Block {
                stmts,
                value: Expr::NilLit { span: span.clone() },
                span: span.clone(),
            })),
            span,
        })
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
            "lambda_expression" => self.lower_lambda_expression(node, depth),
            other => Err(self.err_at(
                node,
                format!(
                    "unsupported expression construct `{other}` (JV02 M1 does not lower this yet)"
                ),
            )),
        }
    }

    /// `expression = lambda_expression | assignment_expression ;` — both
    /// alternatives are single-child wrappers, so this just recurses
    /// through `lower_expr`'s own dispatch (which recognizes
    /// `lambda_expression` directly, M3b).
    fn lower_expression_rule(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        match node.children.as_slice() {
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

    /// `primary_expression = primary { primary_suffix } ;` — M3a adds one
    /// shape (a *bare* unqualified call, `NAME(args)`) and M4a adds two
    /// more (array indexing, `xs[i]`; and `.length`), reached when there
    /// is *exactly one* suffix. M4d adds a fourth: two-or-more suffixes
    /// where *every* one is a `[...]` index (`grid[i][j]`, chained
    /// indexing into a multi-dimensional array) — see
    /// `lower_chained_index`'s own doc comment. Task #60 adds a fifth:
    /// two-or-more suffixes where every *leading* one is a `[...]` index
    /// and the *trailing* one is `.length` (`grid[i].length`,
    /// `cube[i][j].length`) — see `lower_chained_index_then_length`'s own
    /// doc comment. Any other multi-suffix shape (a *qualified* call
    /// `x.foo(...)`, which chains a `.foo` suffix *then* a separate
    /// `(...)` suffix, `.length` anywhere but the trailing position,
    /// `::` method references, and so on) remains out of scope, rejected
    /// as before — both chain guards require *every* non-final suffix be
    /// `[...]`, so a chain mixing in a `.`/`(` suffix anywhere else still
    /// falls through to the final catch-all unchanged. Which shape
    /// applies is decided by the suffix's own leading token (and, for the
    /// two chain guards, the trailing suffix's own shape) — confirmed by
    /// direct CST inspection, not assumed from the grammar text alone.
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
                match suffix.children.first() {
                    Some(ASTNodeOrToken::Token(t)) if t.value == "(" => {
                        self.lower_call_expression(primary, suffix, node, depth)
                    }
                    Some(ASTNodeOrToken::Token(t)) if t.value == "[" => {
                        self.lower_index_get(primary, suffix, node, depth)
                    }
                    Some(ASTNodeOrToken::Token(t)) if t.value == "." => {
                        self.lower_dot_suffix(primary, suffix, node, depth)
                    }
                    _ => Err(self.err_at(
                        node,
                        "this primary suffix is not supported yet (deferred to a later JV02 milestone)".to_string(),
                    )),
                }
            }
            [ASTNodeOrToken::Node(primary), rest @ ..]
                if rest.len() >= 2 && rest.iter().all(is_index_only_suffix) =>
            {
                self.lower_chained_index(primary, rest, node, depth)
            }
            [ASTNodeOrToken::Node(primary), rest @ ..]
                if rest.len() >= 2
                    && is_length_suffix(&rest[rest.len() - 1])
                    && rest[..rest.len() - 1].iter().all(is_index_only_suffix) =>
            {
                let length_suffix = match &rest[rest.len() - 1] {
                    ASTNodeOrToken::Node(s) => s,
                    ASTNodeOrToken::Token(_) => {
                        return Err(self.err_at(node, "malformed primary suffix chain".to_string()))
                    }
                };
                self.lower_chained_index_then_length(
                    primary,
                    &rest[..rest.len() - 1],
                    length_suffix,
                    node,
                    depth,
                )
            }
            [ASTNodeOrToken::Node(primary), ASTNodeOrToken::Node(dot_suffix), ASTNodeOrToken::Node(call_suffix)]
                if matches!(primary.children.as_slice(), [ASTNodeOrToken::Token(t)] if t.type_ == lexer::token::TokenType::Name)
                    && dot_suffix.rule_name == "primary_suffix"
                    && matches!(dot_suffix.children.first(), Some(ASTNodeOrToken::Token(t)) if t.value == ".")
                    && call_suffix.rule_name == "primary_suffix"
                    && matches!(call_suffix.children.first(), Some(ASTNodeOrToken::Token(t)) if t.value == "(") =>
            {
                self.lower_static_method_call(primary, dot_suffix, call_suffix, node, depth)
            }
            _ => Err(self.err_at(
                node,
                "field access, method calls with more than one suffix, and other primary suffixes are not supported yet (deferred to a later JV02 milestone)".to_string(),
            )),
        }
    }

    /// Lower `ClassName.staticMethod(args)` -- M5 (task #67). `semantic-
    /// ir`'s own `Expr::VirtualCall` doc comment is explicit that a
    /// *static* call needs no new node at all: it's an ordinary
    /// `Expr::DirectCall` against a mangled top-level identity. Since
    /// this frontend has no receiver/object model until M6 (M3a already
    /// lowers every method -- static or instance -- identically, flat
    /// top-level), `ClassName.staticMethod(args)` on the *one* class
    /// this frontend itself is compiling is semantically identical to
    /// the bare call `staticMethod(args)` M3a already handles -- same
    /// `method_signatures` table, same `Expr::DirectCall`, just reached
    /// through a qualified suffix chain instead of a bare name.
    ///
    /// Two things this function checks that a bare call doesn't need to:
    /// `class_ref` must literally be this compilation unit's own class
    /// (`self.class_name`) -- any other name is rejected outright, since
    /// this frontend has no import/library-catalog concept at all and
    /// cannot resolve an *external* static (`Math.PI`, `System.out`,
    /// another user class) to anything; and the resolved method must
    /// itself be declared `static` -- `MethodSig::is_static`, M5's own
    /// new field -- since real Java rejects `ClassName.instanceMethod()`
    /// too, and this frontend has no reason to be looser about a
    /// construct it can already fully type-check.
    fn lower_static_method_call(
        &mut self,
        primary: &GrammarASTNode,
        dot_suffix: &GrammarASTNode,
        call_suffix: &GrammarASTNode,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let class_ref = match primary.children.as_slice() {
            [ASTNodeOrToken::Token(t)] if t.type_ == lexer::token::TokenType::Name => {
                t.value.as_str()
            }
            _ => unreachable!(
                "lower_primary_expression's own guard already confirmed primary is a bare NAME token"
            ),
        };
        if class_ref != self.class_name {
            return Err(self.err_at(
                primary,
                format!(
                    "`{class_ref}.` is not supported yet -- only a static call on `{}` itself (this compilation unit's own class) is supported so far (an external class or JDK type like `Math`/`System` is deferred to a later JV02 milestone)",
                    self.class_name
                ),
            ));
        }
        let method_name = dot_suffix
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Name => {
                    Some(t.value.as_str())
                }
                _ => None,
            })
            .ok_or_else(|| self.err_at(dot_suffix, "malformed qualified method reference".to_string()))?;
        let sig = self.method_signatures.get(method_name).cloned().ok_or_else(|| {
            self.err_at(
                node,
                format!("call to unknown method `{method_name}` (JV02 M3a/M5 can only call a method declared in the same class)"),
            )
        })?;
        if !sig.is_static {
            return Err(self.err_at(
                node,
                format!("`{method_name}` is not `static` -- a qualified call (`{class_ref}.{method_name}(...)`) requires a static method (instance method calls are deferred to a later JV02 milestone)"),
            ));
        }
        let args = self.lower_call_arguments(call_suffix, method_name, &sig.param_kinds, depth)?;
        if let Some(callees) = self.call_graph.get_mut(&self.current_method) {
            callees.insert(method_name.to_string());
        }
        let span = self.span_of(node);
        Ok((
            Expr::DirectCall {
                fn_name: method_name.to_string(),
                args,
                effects: EffectSet::PURE,
                span,
            },
            sig.return_kind,
        ))
    }

    /// Lower an array-index-read suffix, `xs[i]` (`primary_suffix =
    /// LBRACKET expression RBRACKET`) into an `Expr::SeqIndex` — M4a,
    /// generalized in M4d to a multi-dimensional array via `Kind::
    /// index_once` (peels exactly one dimension; `xs[i]` on a 1-D array
    /// still gives the plain element kind exactly as before). `primary`
    /// must resolve to an array-typed value; the index expression must
    /// resolve to `Kind::Int`. Uses SIR16's `Expr::SeqIndex` (not SIR22's
    /// `Expr::IndexGet`) for the same reason `lower_array_initializer`
    /// uses `SeqLit` over `ArrayLit` — see that function's own doc
    /// comment. Reached only for a *single* index suffix — `grid[i][j]`
    /// (2+ chained index suffixes) is `lower_chained_index`'s own case.
    fn lower_index_get(
        &mut self,
        primary: &GrammarASTNode,
        suffix: &GrammarASTNode,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let (target, target_kind) = self.lower_expr(primary, depth + 1)?;
        let result_kind = target_kind.index_once().ok_or_else(|| {
            self.err_at(
                node,
                "indexing (`[...]`) is only supported on an array-typed value".to_string(),
            )
        })?;
        let index_node = self
            .first_child_named(suffix, "expression")
            .ok_or_else(|| self.err_at(suffix, "malformed array index".to_string()))?;
        let (index, index_kind) = self.lower_expr(index_node, depth + 1)?;
        if index_kind != Kind::Int {
            return Err(self.err_at(index_node, "an array index must be an `int`".to_string()));
        }
        self.observed.add(Feature::Sequences);
        let span = self.span_of(node);
        Ok((
            Expr::SeqIndex {
                seq: Box::new(target),
                index: Box::new(index),
                span,
            },
            result_kind,
        ))
    }

    /// Lower a *chained* index-suffix sequence, `grid[i][j]` (a `primary`
    /// followed by two-or-more suffixes, every one already confirmed
    /// `[...]`-shaped by `lower_primary_expression`'s own guard) — M4d.
    /// Applies `Kind::index_once` once per suffix, left to right, each
    /// producing a new `Expr::SeqIndex` wrapping the previous one and
    /// peeling exactly one array dimension — `grid[i]` alone (a single
    /// suffix) never reaches this function; that's `lower_index_get`'s
    /// own unchanged case. A chain longer than the target's own
    /// dimension count (e.g. `xs[i][j]` on a 1-D `xs`) fails naturally at
    /// the first suffix whose `index_once` call finds a non-array kind,
    /// with the same "indexing is only supported on an array-typed
    /// value" rejection `lower_index_get` already gives — no separate
    /// bounds check needed.
    fn lower_chained_index(
        &mut self,
        primary: &GrammarASTNode,
        suffixes: &[ASTNodeOrToken],
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let (mut target, mut target_kind) = self.lower_expr(primary, depth + 1)?;
        for suffix in suffixes {
            let suffix = match suffix {
                ASTNodeOrToken::Node(s) => s,
                ASTNodeOrToken::Token(_) => {
                    return Err(self.err_at(node, "malformed primary suffix chain".to_string()))
                }
            };
            let result_kind = target_kind.index_once().ok_or_else(|| {
                self.err_at(
                    node,
                    "indexing (`[...]`) is only supported on an array-typed value".to_string(),
                )
            })?;
            let index_node = self
                .first_child_named(suffix, "expression")
                .ok_or_else(|| self.err_at(suffix, "malformed array index".to_string()))?;
            let (index, index_kind) = self.lower_expr(index_node, depth + 1)?;
            if index_kind != Kind::Int {
                return Err(self.err_at(index_node, "an array index must be an `int`".to_string()));
            }
            let span = self.span_of(suffix);
            target = Expr::SeqIndex {
                seq: Box::new(target),
                index: Box::new(index),
                span,
            };
            target_kind = result_kind;
        }
        self.observed.add(Feature::Sequences);
        Ok((target, target_kind))
    }

    /// Lower a *mixed* index-then-`.length` chain, `grid[i].length`
    /// (`cube[i][j].length`, …) — task #60, the gap `lower_chained_index`'s
    /// own all-`[...]` requirement and `lower_dot_suffix`'s own
    /// single-suffix requirement each left unreached: neither function's
    /// guard recognizes a chain that mixes suffix kinds, even though
    /// nothing about `.length` actually needs the target to be a bare
    /// `primary` — it only needs *some* array-typed expression, and
    /// `lower_chained_index` already knows how to produce exactly that
    /// from one-or-more leading `[...]` suffixes. Delegates the leading
    /// index suffixes straight to `lower_chained_index` unchanged
    /// (`index_suffixes` may be as short as one element — that function's
    /// own loop works fine for a single suffix even though its only
    /// other caller always hands it two-or-more), then applies the exact
    /// same `.length` handling `lower_dot_suffix` does: confirm the
    /// trailing suffix really is `.length` (not some other dotted name
    /// that merely *looks* chain-shaped to `is_length_suffix`'s own
    /// pre-check), confirm the peeled-down target is still array-typed,
    /// and wrap it in `Expr::SeqLen`. A trailing suffix that peels a
    /// *scalar* element down to something whose own `.length` doesn't
    /// exist (`xs[i].length` on a 1-D `int[] xs`) is rejected with the
    /// same "only supported on an array-typed value" message
    /// `lower_dot_suffix` already gives for the un-indexed case.
    fn lower_chained_index_then_length(
        &mut self,
        primary: &GrammarASTNode,
        index_suffixes: &[ASTNodeOrToken],
        length_suffix: &GrammarASTNode,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let is_length = length_suffix.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Name => {
                Some(t.value.as_str())
            }
            _ => None,
        }) == Some("length");
        if !is_length {
            return Err(self.err_at(
                node,
                "field access and qualified method calls are not supported yet, except `.length` on an array (deferred to a later JV02 milestone)".to_string(),
            ));
        }
        let (target, target_kind) =
            self.lower_chained_index(primary, index_suffixes, node, depth)?;
        if !matches!(target_kind, Kind::Array(_, _)) {
            return Err(self.err_at(
                node,
                "`.length` is only supported on an array-typed value".to_string(),
            ));
        }
        let span = self.span_of(node);
        Ok((
            Expr::SeqLen {
                seq: Box::new(target),
                span,
            },
            Kind::Int,
        ))
    }

    /// Lower a `DOT NAME` suffix — this milestone supports exactly one
    /// case, `.length` on an array-typed value (`Expr::SeqLen`, M4a).
    /// Every other dotted suffix (a field, or the first half of a
    /// qualified method call — which always chains a *second* suffix
    /// anyway, already rejected by `lower_primary_expression`'s own
    /// suffix-count check before this function is ever reached) remains
    /// out of scope.
    fn lower_dot_suffix(
        &mut self,
        primary: &GrammarASTNode,
        suffix: &GrammarASTNode,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let is_length = suffix.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Name => {
                Some(t.value.as_str())
            }
            _ => None,
        }) == Some("length");
        if !is_length {
            return Err(self.err_at(
                node,
                "field access and qualified method calls are not supported yet, except `.length` on an array (deferred to a later JV02 milestone)".to_string(),
            ));
        }
        let (target, target_kind) = self.lower_expr(primary, depth + 1)?;
        if !matches!(target_kind, Kind::Array(_, _)) {
            return Err(self.err_at(
                node,
                "`.length` is only supported on an array-typed value".to_string(),
            ));
        }
        self.observed.add(Feature::Sequences);
        let span = self.span_of(node);
        Ok((
            Expr::SeqLen {
                seq: Box::new(target),
                span,
            },
            Kind::Int,
        ))
    }

    /// Lower a bare unqualified call `NAME(args)` — reached only when
    /// `lower_primary_expression`'s own dispatch has already confirmed
    /// the suffix starts with `(`. `primary` must be a single bare
    /// `NAME` token (a *qualified* callee, e.g. `x.foo`, never reaches
    /// this function — it fails `lower_primary_expression`'s own
    /// suffix-count match arm first, since a qualified call chains two
    /// suffixes).
    fn lower_call_expression(
        &mut self,
        primary: &GrammarASTNode,
        suffix: &GrammarASTNode,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
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

        // A local variable or parameter holding a lambda value takes
        // priority over a same-named top-level method, mirroring real
        // Java's own name resolution for a call expression (a local of a
        // functional-interface type in scope is invoked directly through
        // that binding; a same-named method is not reachable through
        // this call syntax at all while such a local is in scope) --
        // this is the `Expr::IndirectCall` entry point (task #54): a
        // lambda could only ever be *created* and passed around before
        // this, never actually invoked.
        if let Some((kind, scope)) = self.resolve_name(&callee) {
            let closure_idx = match kind {
                Kind::Closure(idx) => idx,
                _ => {
                    return Err(self.err_at(
                        node,
                        format!(
                            "`{callee}` is a local variable, not a lambda, and cannot be called"
                        ),
                    ))
                }
            };
            let sig = self.closure_signatures[closure_idx as usize].clone();
            let args = self.lower_call_arguments(suffix, &callee, &sig.param_kinds, depth)?;
            let target_span = self.span_of(primary);
            let span = self.span_of(node);
            return Ok((
                Expr::IndirectCall {
                    target: Box::new(Expr::VarRef {
                        name: callee,
                        scope,
                        span: target_span,
                    }),
                    args,
                    effects: EffectSet::PURE,
                    span,
                },
                sig.return_kind,
            ));
        }

        let sig = self.method_signatures.get(&callee).cloned().ok_or_else(|| {
            self.err_at(
                node,
                format!("call to unknown method `{callee}` (JV02 M3a can only call a method declared in the same class)"),
            )
        })?;
        let args = self.lower_call_arguments(suffix, &callee, &sig.param_kinds, depth)?;

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

    /// Lower a call's own `argument_list` against an already-resolved
    /// `param_kinds` (a real top-level method's signature, or a
    /// closure's own interned one) — shared by both `lower_call_
    /// expression`'s direct- and indirect-call paths, since argument
    /// count/kind checking is identical either way; only how the callee
    /// itself resolves (and what `Expr` variant wraps the result)
    /// differs between them.
    fn lower_call_arguments(
        &mut self,
        suffix: &GrammarASTNode,
        callee: &str,
        param_kinds: &[Kind],
        depth: usize,
    ) -> Result<Vec<Expr>, JavaLowerError> {
        let arg_nodes: Vec<&GrammarASTNode> = match self.first_child_named(suffix, "argument_list")
        {
            Some(al) => child_nodes(al)
                .into_iter()
                .filter(|n| n.rule_name == "expression")
                .collect(),
            None => vec![],
        };
        if arg_nodes.len() != param_kinds.len() {
            return Err(self.err_at(
                suffix,
                format!(
                    "`{callee}` expects {} argument(s), found {}",
                    param_kinds.len(),
                    arg_nodes.len()
                ),
            ));
        }
        let mut args = Vec::with_capacity(arg_nodes.len());
        for (arg_node, expected_kind) in arg_nodes.iter().zip(param_kinds.iter()) {
            let (arg_expr, arg_kind) = self.lower_expr(arg_node, depth + 1)?;
            if arg_kind != *expected_kind {
                return Err(self.err_at(
                    arg_node,
                    format!("argument to `{callee}` has the wrong kind"),
                ));
            }
            args.push(arg_expr);
        }
        Ok(args)
    }

    /// `primary = literal | "this" | ... | "new" array_creation_type
    /// array_dimension_exprs {LBRACKET RBRACKET} | "new" array_creation_
    /// type {LBRACKET RBRACKET} array_initializer | LPAREN expression
    /// RPAREN | NAME ;` — M1 supports literals, parenthesized
    /// sub-expressions, and bare variable references; M4c adds the two
    /// `new`-based array-creation-expression shapes (confirmed via direct
    /// CST inspection, not assumed from the grammar text alone — see
    /// `lower_new_sized_array`/`lower_new_array_with_initializer`'s own
    /// doc comments for the exact children shape each expects).
    /// Everything else (`this`, `super`, `switch` expressions, object
    /// construction) remains out of scope.
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
                let (kind, scope) = self.resolve_name(&name).ok_or_else(|| {
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
            [ASTNodeOrToken::Token(new_tok), ASTNodeOrToken::Node(act), ASTNodeOrToken::Node(dims)]
                if new_tok.value == "new"
                    && act.rule_name == "array_creation_type"
                    && dims.rule_name == "array_dimension_exprs" =>
            {
                self.lower_new_sized_array(act, dims, node, depth)
            }
            [ASTNodeOrToken::Token(new_tok), ASTNodeOrToken::Node(act), rest @ ..]
                if new_tok.value == "new" && act.rule_name == "array_creation_type" =>
            {
                self.lower_new_array_with_initializer(act, rest, node, depth)
            }
            _ => Err(self.err_at(
                node,
                "unsupported primary expression (JV02 M1 supports only literals, bare variable references, and parenthesized expressions)".to_string(),
            )),
        }
    }

    /// Lower `"new" array_creation_type array_dimension_exprs` — a sized,
    /// uninitialized array creation (`new int[5]`) — into an `Expr::
    /// SeqLit` of `N` zero-valued elements, M4c. `dims` is the
    /// `array_dimension_exprs` node (`LBRACKET expression RBRACKET
    /// {LBRACKET expression RBRACKET}` — confirmed via direct CST
    /// inspection that a *single*-dimension `[5]` produces exactly one
    /// `expression` child here, with any further dims as additional
    /// sibling `expression` children of the *same* node, and any jagged
    /// trailing `[]` dims living as extra tokens on `primary` itself
    /// (which this function's own caller's fixed 3-child match arm
    /// already excludes, so only a real single dimension ever reaches
    /// here).
    ///
    /// SIR16 has no repeat/fill primitive (confirmed by an exhaustive
    /// grep of every `Seq*` node — only `SeqLit`/`SeqIndex`/`SeqLen`/
    /// `SeqSet` exist), so this can only be lowered when the size
    /// expression is a compile-time-constant, non-negative integer
    /// literal — a non-constant size (`new int[n]` for a variable `n`)
    /// genuinely cannot be represented without a new SIR primitive, and
    /// is rejected with a clear error rather than attempted (deferred,
    /// tracked as its own follow-up task). Sized creation of a
    /// reference-typed array (`new String[n]`) is *also* deferred: real
    /// Java fills it with `null`, which this frontend's exact
    /// element-kind-match invariant (every `Expr::SeqLit` item's `Kind`
    /// equals the array's own declared element `Kind`, established in
    /// M4a's `lower_array_initializer`) doesn't cleanly represent yet —
    /// only the numeric/boolean element kinds, whose zero-value *is* a
    /// same-kind value, are supported this milestone.
    fn lower_new_sized_array(
        &mut self,
        act: &GrammarASTNode,
        dims: &GrammarASTNode,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let dim_exprs: Vec<&GrammarASTNode> = child_nodes(dims)
            .into_iter()
            .filter(|n| n.rule_name == "expression")
            .collect();
        if dim_exprs.len() != 1 {
            return Err(self.err_at(
                dims,
                "multi-dimensional array creation is not supported yet (deferred to a later JV02 milestone)".to_string(),
            ));
        }
        let size_node = dim_exprs[0];
        let (size_expr, size_kind) = self.lower_expr(size_node, depth + 1)?;
        if size_kind != Kind::Int {
            return Err(self.err_at(size_node, "an array size must be an `int`".to_string()));
        }
        let n = match size_expr {
            Expr::IntLit { value, .. } => value,
            _ => {
                return Err(self.err_at(
                    size_node,
                    "a sized array's size must be a compile-time-constant integer literal (a non-constant size needs a repeat/fill IR primitive this frontend does not have yet -- deferred to a later JV02 milestone)".to_string(),
                ))
            }
        };
        if n < 0 {
            return Err(self.err_at(size_node, "an array size must not be negative".to_string()));
        }
        if n > MAX_SIZED_ARRAY_LEN {
            return Err(self.err_at(
                size_node,
                format!("a sized array creation must not exceed {MAX_SIZED_ARRAY_LEN} elements"),
            ));
        }
        let scalar = self.scalar_kind_of_type_node(act)?;
        let elem_kind = ArrayElemKind::from_kind(scalar).ok_or_else(|| {
            self.err_at(act, format!("unsupported array element kind `{scalar:?}`"))
        })?;
        let zero_span = self.span_of(size_node);
        let zero_value = match elem_kind {
            ArrayElemKind::Int => Expr::IntLit { value: 0, span: zero_span },
            ArrayElemKind::Float => {
                self.observed.add(Feature::Floats);
                Expr::FloatLit { value: 0.0, span: zero_span }
            }
            ArrayElemKind::Bool => Expr::BoolLit { value: false, span: zero_span },
            ArrayElemKind::Str => {
                return Err(self.err_at(
                    act,
                    "sized creation of a reference-typed array (e.g. `new String[n]`) is not supported yet -- only numeric/boolean element kinds are (deferred to a later JV02 milestone)".to_string(),
                ))
            }
        };
        let items = vec![zero_value; n as usize];
        self.observed.add(Feature::Sequences);
        let span = self.span_of(node);
        Ok((Expr::SeqLit { items, span }, Kind::Array(elem_kind, 1)))
    }

    /// Lower `"new" array_creation_type {LBRACKET RBRACKET}
    /// array_initializer` — `new int[]{1, 2, 3}` — M4c. `rest` is
    /// `primary`'s own remaining children after `"new"` and
    /// `array_creation_type` (everything from `lower_primary`'s own
    /// catch-all match arm), expected to be zero-or-more `[`/`]` token
    /// pairs followed by exactly one `array_initializer` node at the
    /// end; any other shape (including a jagged/malformed sized-array
    /// form that fell through `lower_new_sized_array`'s own fixed
    /// 3-child match arm) is rejected here rather than mis-lowered.
    ///
    /// Semantically identical to the bare `{1, 2, 3}` declarator-
    /// initializer form M4a already supports, just `new`-prefixed with
    /// its own always-explicit element type (never `var`-inferred, so
    /// this delegates straight to the same `lower_array_initializer`
    /// M4a built, passing `Some(elem_kind)` unconditionally).
    fn lower_new_array_with_initializer(
        &mut self,
        act: &GrammarASTNode,
        rest: &[ASTNodeOrToken],
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Expr, Kind), JavaLowerError> {
        let (last, brackets) = rest.split_last().ok_or_else(|| {
            self.err_at(
                node,
                "this `new`-based array-creation form is not supported yet (deferred to a later JV02 milestone)".to_string(),
            )
        })?;
        let array_init = match last {
            ASTNodeOrToken::Node(n) if n.rule_name == "array_initializer" => n,
            _ => {
                return Err(self.err_at(
                    node,
                    "this `new`-based array-creation form is not supported yet (deferred to a later JV02 milestone)".to_string(),
                ))
            }
        };
        let bracket_pairs = brackets
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "["))
            .count();
        if bracket_pairs != 1 {
            return Err(self.err_at(
                node,
                "multi-dimensional array creation is not supported yet (deferred to a later JV02 milestone)".to_string(),
            ));
        }
        let scalar = self.scalar_kind_of_type_node(act)?;
        let elem_kind = ArrayElemKind::from_kind(scalar).ok_or_else(|| {
            self.err_at(act, format!("unsupported array element kind `{scalar:?}`"))
        })?;
        self.lower_array_initializer(array_init, Some((elem_kind, 1)), depth)
    }

    /// Lower a `lambda_expression` (`lambda_parameters ARROW lambda_body`)
    /// into an `Expr::MakeClosure`, hoisting its body to a synthesized
    /// top-level `Function` (`__lambda_N`, mirroring how `main` itself is
    /// already synthesized) — the JV02 M3b entry point. Every parameter
    /// must be explicitly typed (`lambda_parameter_kind_name_pairs`'s own
    /// doc comment explains why the untyped/`var`-inferred forms are
    /// rejected); captures are discovered on-resolve while the body is
    /// lowered (`resolve_name`'s own doc comment has the full design).
    ///
    /// Threads `depth` (not a fresh `0`) into every recursive call this
    /// makes, including the lambda body's own — deliberately *not*
    /// mirroring `lower_method_declaration`'s own "reset to 0" pattern
    /// for its method body. Method declarations can never nest inside
    /// each other at the source level (a `class_body`'s own
    /// `method_declaration`s are always flat siblings), so resetting the
    /// depth budget once per method body is safe. Lambda *expressions*
    /// can nest arbitrarily inside each other via ordinary expression or
    /// statement syntax (`x -> (y -> (z -> ...))`, or a block-bodied
    /// lambda's own tail `return` producing another lambda), so if this
    /// function reset the depth counter at its own boundary the way a
    /// method body does, nested lambdas could bypass `MAX_EXPR_DEPTH`/
    /// `MAX_STMT_DEPTH` entirely — an attacker gets a *fresh* budget at
    /// every lambda boundary instead of one shared, bounded budget. This
    /// was caught during design, before writing any code, by asking
    /// specifically whether the M3a method-body precedent's own
    /// depth-reset was safe to copy here — it wasn't, given lambdas
    /// (unlike methods) are a genuinely recursive source-level construct.
    fn lower_lambda_expression(
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
        let params_node = self
            .first_child_named(node, "lambda_parameters")
            .ok_or_else(|| {
                self.err_at(
                    node,
                    "malformed lambda expression (missing parameters)".to_string(),
                )
            })?;
        let body_node = self.first_child_named(node, "lambda_body").ok_or_else(|| {
            self.err_at(
                node,
                "malformed lambda expression (missing body)".to_string(),
            )
        })?;
        let span = self.span_of(node);
        let pairs = self.lambda_parameter_kind_name_pairs(params_node)?;

        self.closure_stack.push(ClosureFrame {
            locals_mark: self.locals.len(),
            span: span.clone(),
            captures: vec![],
            capture_values: vec![],
        });
        self.push_scope();
        let mut params = Vec::with_capacity(pairs.len());
        let mut param_kinds = Vec::with_capacity(pairs.len());
        for (name, kind) in pairs {
            self.declare_param(name.clone(), kind);
            self.observed.add(Feature::DynamicTyping);
            param_kinds.push(kind);
            params.push(Param {
                name,
                sir_type: None,
                kind: ParamKind::Required,
                default: None,
                span: span.clone(),
            });
        }

        // A lambda body is its own statement-flow boundary: real Java
        // forbids a `break`/`continue` inside a lambda from targeting a
        // loop the lambda literal merely happens to be lexically nested
        // in (e.g. `list.forEach(x -> { break; })` inside an enclosing
        // `while` is a `javac` compile error, not a jump to that outer
        // loop). Save/restore `loop_depth` to `0` around the body
        // lowering so `lower_break_statement`/`lower_continue_statement`
        // correctly reject a bare `break`/`continue` written directly in
        // this lambda's own body, regardless of how deeply the lambda
        // *literal* itself is nested inside real Java loops.
        let saved_loop_depth = std::mem::take(&mut self.loop_depth);
        let body_result = self.lower_lambda_body(body_node, depth + 1);
        self.loop_depth = saved_loop_depth;
        self.pop_scope();
        let closure_frame = self.closure_stack.pop().expect("just pushed above");
        let (body, body_kind) = body_result?;

        // Collision-check the synthetic name against every real,
        // user-declared method name (`method_signatures` is fully
        // populated before any body -- and thus any lambda -- is
        // lowered, so this sees every method regardless of textual
        // order) before committing to it -- mirrors `lower_do_while_
        // statement`'s own identically-reasoned `__do_while_N` collision
        // probe. `__lambda_0` is a legal Java method name, so a source
        // file declaring a real method by that exact name is a real,
        // reachable case, not a hypothetical one: without this check,
        // `Module.functions` could end up with two entries sharing one
        // name (the user's real method and this synthesized closure
        // body), which `compile()` itself would not catch (only a
        // separate `semantic_ir::validate()` call would, and only if the
        // caller makes it) -- found by `/security-review`.
        let mut fn_name = format!("__lambda_{}", self.lambda_counter);
        self.lambda_counter += 1;
        while self.method_signatures.contains_key(&fn_name) {
            fn_name = format!("__lambda_{}", self.lambda_counter);
            self.lambda_counter += 1;
        }
        self.observed.add(Feature::Closures);

        self.synthesized_functions.push(Function {
            name: fn_name.clone(),
            params,
            return_type: None,
            captures: closure_frame.captures,
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: span.clone(),
        });

        // Intern this lambda's own call signature so a later `Expr::
        // IndirectCall` through a local holding this closure value can
        // type-check its arguments and pick the right result `Kind` --
        // see `Kind::Closure`'s own doc comment for why this lives in a
        // side table rather than inline on the `Kind` variant itself.
        let closure_idx = self.closure_signatures.len() as u32;
        self.closure_signatures.push(MethodSig {
            param_kinds,
            return_kind: body_kind,
            // `is_static` is meaningless for a lambda's own interned
            // signature -- `lower_static_method_call` (M5) only ever
            // reads it from `method_signatures`, never from
            // `closure_signatures`.
            is_static: false,
        });

        Ok((
            Expr::MakeClosure {
                fn_name,
                captures: closure_frame.capture_values,
                span,
            },
            Kind::Closure(closure_idx),
        ))
    }

    /// Lower a `lambda_body` (`expression | block`) into the synthesized
    /// function's own value/tail-return `Block`, and the `Kind` that
    /// value naturally has (there is no *declared* return type to
    /// validate against — see `lower_lambda_block_body`'s own doc
    /// comment).
    fn lower_lambda_body(
        &mut self,
        body_node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Block, Kind), JavaLowerError> {
        match body_node.children.as_slice() {
            [ASTNodeOrToken::Node(n)] if n.rule_name == "expression" => {
                let span = self.span_of(body_node);
                let (value, kind) = self.lower_expr(n, depth)?;
                Ok((
                    Block {
                        stmts: vec![],
                        value,
                        span,
                    },
                    kind,
                ))
            }
            [ASTNodeOrToken::Node(n)] if n.rule_name == "block" => {
                self.lower_lambda_block_body(n, depth)
            }
            _ => Err(self.err_at(body_node, "malformed lambda body".to_string())),
        }
    }

    /// Lower a block-bodied lambda's own `block` node — a variant of
    /// `lower_method_body_block` for the one place they genuinely
    /// differ: a lambda has no *declared* return type to validate
    /// against (Java infers it from the lambda's target functional-
    /// interface's own abstract method, which this frontend has no
    /// visibility into — no functional-interface declarations exist at
    /// all yet, that's a later SIR29 milestone), so whatever `Kind` the
    /// tail-position `return`'s expression happens to produce (or
    /// `Kind::Void` for a bare `return;`, or for falling off the end
    /// with no `return` at all — a legal "statement lambda" shape, e.g.
    /// `Runnable`-shaped) is simply *returned to the caller*, not
    /// checked against anything. The "`return` only in tail position"
    /// rule itself is unconditional regardless of whether there's a
    /// declared type to check against — SIR still has no `Stmt::Return`
    /// primitive at all — so that half of `lower_method_body_block`'s
    /// own logic is unchanged here.
    fn lower_lambda_block_body(
        &mut self,
        block: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Block, Kind), JavaLowerError> {
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
        let mut value_kind = Kind::Void;
        for (i, block_stmt) in block_stmts.iter().enumerate() {
            let is_last = i + 1 == block_stmts.len();
            if let Some(ret) = self.find_return_statement_direct(block_stmt) {
                if !is_last {
                    return Err(self.err_at(
                        ret,
                        "`return` is only supported as the last statement of a lambda body (an early or branched return is deferred to a later JV02 milestone)".to_string(),
                    ));
                }
                if let Some(expr_node) = self.first_child_named(ret, "expression") {
                    let (expr, kind) = self.lower_expr(expr_node, depth + 1)?;
                    value = expr;
                    value_kind = kind;
                }
                break;
            }
            stmts.push(self.lower_block_statement(block_stmt, depth + 1)?);
        }
        Ok((Block { stmts, value, span }, value_kind))
    }

    /// Extract each lambda parameter's `(name, Kind)` pair from a
    /// `lambda_parameters` node — the lambda counterpart of
    /// `formal_parameter_kind_name_pairs`. Only the fully-explicit-type
    /// shape is supported: `lambda_parameter = type NAME | "var" NAME |
    /// NAME` (JLS), and only the *first* alternative resolves a `Kind`
    /// this frontend can actually know — both `"var" NAME` and a bare
    /// `NAME` rely on inferring the parameter's type from the lambda's
    /// own target functional-interface type (the abstract method it
    /// implements), which this frontend has no visibility into (no
    /// functional-interface declarations exist at all yet — interfaces
    /// are a later SIR29 milestone). Rejecting the untyped/`var` forms
    /// here, rather than guessing, is this crate's own established
    /// "reject rather than mis-lower" discipline — the same reasoning
    /// M1 already applies to `int`/`String` vs. every other reference
    /// type.
    fn lambda_parameter_kind_name_pairs(
        &self,
        params_node: &GrammarASTNode,
    ) -> Result<Vec<(String, Kind)>, JavaLowerError> {
        match params_node.children.as_slice() {
            // `lambda_parameters = NAME` -- a single untyped parameter
            // with no parentheses at all (`x -> ...`). Always untyped by
            // this shape's own grammar production, so always rejected.
            [ASTNodeOrToken::Token(t)] if t.type_ == lexer::token::TokenType::Name => {
                Err(self.err_at(
                    params_node,
                    format!("lambda parameter `{}` has no explicit type, and its type cannot be inferred without a target functional-interface declaration (deferred to a later JV02 milestone — every lambda parameter must be explicitly typed for now)", t.value),
                ))
            }
            _ => {
                let Some(list) = self.first_child_named(params_node, "lambda_parameter_list")
                else {
                    return Ok(vec![]); // `() -> ...` -- zero parameters.
                };
                let mut out = Vec::with_capacity(list.children.len());
                for lp in child_nodes(list)
                    .into_iter()
                    .filter(|n| n.rule_name == "lambda_parameter")
                {
                    out.push(self.lambda_parameter_kind_name(lp)?);
                }
                Ok(out)
            }
        }
    }

    /// Resolve one `lambda_parameter` (`{annotation} ["final"] type NAME
    /// | {annotation} ["final"] "var" NAME | {annotation} ["final"]
    /// NAME`) — see `lambda_parameter_kind_name_pairs`'s own doc comment
    /// for why only the first alternative is supported. Handles both
    /// possible parse shapes for `var x` defensively (the literal `"var"
    /// NAME` grammar alternative, and — mirroring the same PEG-ordering
    /// ambiguity this module's own doc comment documents for top-level
    /// `var` declarations — `var` absorbed into the `type` alternative
    /// as a single-segment class type literally named `var`), rather
    /// than assuming which one the parser actually produces.
    fn lambda_parameter_kind_name(
        &self,
        lp: &GrammarASTNode,
    ) -> Result<(String, Kind), JavaLowerError> {
        let name_tok = lp
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Name => Some(t),
                _ => None,
            })
            .ok_or_else(|| {
                self.err_at(lp, "malformed lambda parameter (missing name)".to_string())
            })?;
        match self.first_child_named(lp, "type") {
            Some(ty) if single_segment_class_type_name(ty) != Some("var") => {
                self.reject_dollar_sign_identifier(&name_tok.value, lp)?;
                Ok((name_tok.value.clone(), self.kind_of_type_node(ty)?))
            }
            _ => Err(self.err_at(
                lp,
                format!("lambda parameter `{}` has no explicit type (`var` and untyped lambda parameters infer their type from a target functional-interface declaration, which this frontend has no visibility into — deferred to a later JV02 milestone)", name_tok.value),
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

    /// Reject a declared local/parameter/loop-variable name containing
    /// `$` — legal in a real Java identifier (JLS §3.8's `NAME` grammar,
    /// which this crate's own lexer accepts: `[a-zA-Z_$][a-zA-Z0-9_$]*`)
    /// but rejected here, at every point this crate turns a Java `NAME`
    /// token into a declared SIR local's name (see this method's four
    /// call sites: `formal_parameter_kind_name_pairs`, `lambda_parameter_
    /// kind_name`, `lower_enhanced_for_statement`, and the local-variable-
    /// declarator path in `lower_var_declaration_node`'s callee).
    ///
    /// # Why this exists — round 3 of `/security-review`
    ///
    /// `fresh_flag_name` (see its own doc comment) picks a loop-control
    /// guard-flag name and checks it directly against every real Java
    /// local's *raw source spelling*, on the stated assumption that two
    /// different raw Java identifiers can never collide once a backend
    /// emits them — true only if every backend's `sanitize_ident` is the
    /// identity function on both names being compared. That holds for
    /// plain `[A-Za-z0-9_]` names (what `fresh_flag_name` itself always
    /// produces), but a *third* `/security-review` round proved it false
    /// for a raw Java name containing `$`: `semantic-ir-to-python::
    /// sanitize_ident` escapes `$` (not part of Python's own identifier
    /// alphabet) to `_24` (its hex code point), so a Java local named
    /// e.g. `_do_while$` sanitizes to `_do_while_24` — a string with no
    /// resemblance to the raw name `fresh_flag_name`'s collision checks
    /// actually compared against, but one that can coincide *exactly*
    /// with a `__do_while_N` candidate for some `N`, reintroducing the
    /// identical flat-scoping collision the last three rounds fixed —
    /// confirmed by actually executing the emitted Python and observing
    /// a hang.
    ///
    /// Rather than teach this backend-agnostic frontend every backend's
    /// own escaping scheme (which would need updating every time a new
    /// backend or a new escaping rule is added), this closes the gap at
    /// the source: no Java identifier containing `$` can be declared as
    /// a SIR local at all, which restores the invariant `fresh_flag_
    /// name`'s own design actually needs — every declarable name lives
    /// in `[A-Za-z0-9_]`, the one alphabet every backend's `sanitize_
    /// ident` treats as its own identity — by construction rather than
    /// by continuing to chase individual backends' escaping quirks.
    /// `$`-containing identifiers are vanishingly rare in hand-written
    /// Java (real-world use is almost exclusively compiler-generated
    /// synthetic names, e.g. inner-class-accessor methods), so this is a
    /// narrow, disclosed scope boundary in the same spirit as this
    /// crate's many other "not supported yet" rejections — not a
    /// meaningful loss of real-world Java coverage.
    fn reject_dollar_sign_identifier(
        &self,
        name: &str,
        node: &GrammarASTNode,
    ) -> Result<(), JavaLowerError> {
        if name.contains('$') {
            return Err(self.err_at(
                node,
                format!(
                    "identifier `{name}` contains `$`, which is not supported yet (deferred — \
                     see `reject_dollar_sign_identifier`'s own doc comment for why this is a \
                     security-motivated restriction, not an oversight)"
                ),
            ));
        }
        Ok(())
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

/// `true` iff `c` is a `primary_suffix` node whose own leading token is
/// `[` — i.e. an array-index suffix (`[expr]`), not a call, field
/// access, or method reference. Used by `lower_primary_expression`'s own
/// chained-index guard: a 2+-suffix `primary_expression` only reaches
/// `lower_chained_index` when *every* suffix passes this check — a chain
/// mixing in a `.`/`(` suffix anywhere (a qualified call, field access,
/// etc.) still falls through to that function's own unchanged rejection.
fn is_index_only_suffix(c: &ASTNodeOrToken) -> bool {
    matches!(
        c,
        ASTNodeOrToken::Node(s)
            if s.rule_name == "primary_suffix"
                && matches!(s.children.first(), Some(ASTNodeOrToken::Token(t)) if t.value == "[")
    )
}

/// `true` iff `c` is a `primary_suffix` node shaped exactly like
/// `lower_dot_suffix`'s own single-suffix `.length` case: a leading `.`
/// token followed by a `NAME` token spelling `length`. Used by
/// `lower_primary_expression`'s own mixed-chain guard (task #60) to
/// recognize a *trailing* `.length` on an otherwise all-index chain
/// (`grid[i].length`) without duplicating `lower_dot_suffix`'s own
/// recognition logic inline — `lower_chained_index_then_length` still
/// re-derives the same `is_length` boolean itself (from the suffix it's
/// actually handed, not from calling this predicate again) since it also
/// needs the real error path when the trailing suffix looks
/// dot-suffix-shaped but isn't `.length`.
fn is_length_suffix(c: &ASTNodeOrToken) -> bool {
    matches!(
        c,
        ASTNodeOrToken::Node(s)
            if s.rule_name == "primary_suffix"
                && matches!(s.children.first(), Some(ASTNodeOrToken::Token(t)) if t.value == ".")
                && s.children.iter().any(|child| matches!(
                    child,
                    ASTNodeOrToken::Token(t)
                        if t.type_ == lexer::token::TokenType::Name && t.value == "length"
                ))
    )
}

/// `break_statement`/`continue_statement`'s own optional trailing `NAME`
/// — the Java label a labeled `break foo;`/`continue foo;` targets, if
/// present. `None` for the bare (unlabeled) form. Used by
/// `lower_break_statement`/`lower_continue_statement` to reject the
/// labeled form cleanly (SIR has no loop-label vocabulary).
fn label_token(node: &GrammarASTNode) -> Option<&str> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Name => {
            Some(t.value.as_str())
        }
        _ => None,
    })
}

/// Collects every name a lowered loop body declares as a local — via
/// `Stmt::LetBinding`/`LetStarBinding` directly, or a `ForRange`/`ForEach`
/// loop variable — at any nesting depth (inside a nested `if`, `while`,
/// `for`, etc.). Used by [`fresh_flag_name`] to guarantee a synthetic
/// guard-flag name can never collide with a real Java local.
///
/// Rides `semantic_ir::Visitor`'s shared, already-depth-guarded traversal
/// (`walker.rs`) rather than a bespoke recursive walk over `Stmt`/`Expr`:
/// this crate's own `Expr::If`/`Expr::Block` (how a bare Java `if`
/// statement and a nested block both lower — see `lower_statement`'s
/// `if`/`else` handling) already nest further declarations the walker
/// must see, and any future SIR node this crate starts emitting inside a
/// loop body is covered automatically, without a second traversal to keep
/// in sync with `nodes.rs`.
struct DeclaredNameCollector {
    names: HashSet<String>,
}

impl Visitor for DeclaredNameCollector {
    fn visit_stmt(&mut self, s: &Stmt, depth: usize) {
        match s {
            Stmt::LetBinding { name, .. } | Stmt::LetStarBinding { name, .. } => {
                self.names.insert(name.clone());
            }
            Stmt::ForRange { var, .. } | Stmt::ForEach { var, .. } => {
                self.names.insert(var.clone());
            }
            _ => {}
        }
        walk_stmt_default(self, s, depth);
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
