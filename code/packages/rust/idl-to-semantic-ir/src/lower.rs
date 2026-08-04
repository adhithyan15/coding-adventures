//! The lowering pass from `coding_adventures_idl_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.1.0** (MA-12e).
//!
//! Structurally this file follows `scilab-to-semantic-ir`'s own shape (the
//! closest array-family precedent, per this task's own brief), but IDL's
//! *grammar* is an Algol/Fortran-family imperative one (MA12 §5), not a
//! MATLAB-derived array grammar — statements, `PRO`/`FUNCTION` definitions,
//! `IF`/`FOR`/`WHILE`/`REPEAT` blocks, an infix precedence cascade with word
//! operators (`EQ`/`AND`/...). So while the *scaffolding* (two-pass name
//! collection, hoisted branch-introduced locals, depth guards, the
//! `Lowered` enum) mirrors Scilab's, the expression/statement dispatch
//! below is written directly against `idl.grammar`'s own rule names,
//! verified against `idl-parser`'s README and `idl-runtime::eval`'s own
//! navigation code (the ground truth for what each rule's children actually
//! are) rather than assumed by analogy.
//!
//! # Scope (v0.1.0)
//!
//! **Supported** (MA12 §4's in-scope surface, the subset that maps cleanly
//! onto SIR10/SIR16/SIR22/KW1's *existing* vocabulary — this crate adds no
//! new `semantic_ir` core variant):
//! - Literals: `NUMBER` (int- or float-shaped by lexeme, mirroring
//!   `scilab_to_semantic_ir::number_literal_expr`), `STRING` (single- or
//!   double-quoted — one underlying `StrLit`, MA12 §2), array literals
//!   `[1, 2, 3]` (always rank-1, MA12 §2 — see "Array literals" below).
//! - Variables (`NAME`, case-folded — see "Case folding" below), assignment
//!   (`x = expr`; first occurrence → `LetStarBinding`, later re-assignment →
//!   `Assign`), subscripted assignment (`Stmt::IndexSet`).
//! - Arithmetic `+ - * /`, `^` (power, left-associative — see "Power is
//!   left-associative" below), matrix product `#`/`##` (see "`#` vs `##`"
//!   below), unary `+`/`-`.
//! - Comparisons `EQ NE LT LE GT GE` → the shared `=`/`!=`/`<`/`<=`/`>`/`>=`
//!   `BuiltinCall`s every other array-family frontend already uses.
//! - Control flow `IF...THEN...ELSE`, `FOR v=a,b[,step] DO`, `WHILE...DO`,
//!   `REPEAT...UNTIL`, a bare `BEGIN...END` block (flattened inline — IDL
//!   has no block-level scoping, MA12 §4, so a bare block introduces no SIR
//!   scope of its own), both the single-statement and `BEGIN...ENDxxx`
//!   block forms.
//! - `PRO`/`FUNCTION` definitions and calls, with keyword arguments and the
//!   `/KEYWORD` boolean shorthand (see "Keyword arguments" and "Two
//!   namespaces" below) — the headline MA12 §3 feature.
//! - Subscripting: plain (`a[i]`), 2-D (`a[i, j]`), ranged (`a[s0:s1]`),
//!   strided (`a[s0:s1:n]`), wildcard `a[*]` — see "Subscripting" below for
//!   the full account, including the two genuinely unresolvable forms
//!   (negative-from-end, wildcard-range-end) this cut rejects rather than
//!   mis-lowers.
//! - `PRINT` (one positional argument only, mirroring
//!   `scilab_to_semantic_ir`'s identical `disp` restriction) → the shared
//!   `print` `BuiltinCall`. `TRANSPOSE(a)` → `Expr::Transpose`.
//!   `INDGEN`/`FINDGEN`/`DINDGEN`/`LINDGEN(n)` → `Expr::Range` (see
//!   "Builtins" below for why these four map cleanly and everything else in
//!   MA12 §4's builtin list does not, in this cut).
//!
//! **Deliberately out of scope for v0.1.0** (each rejected with a clean,
//! explicit [`IdlLowerError`], never silently mis-lowered):
//! - **`AND`/`OR`/`XOR`/`NOT`** (bitwise, MA12 §4) — see "Bitwise operators:
//!   a genuine, disclosed gap" below.
//! - **`SIN COS TAN SQRT ABS EXP ALOG ALOG10`, `TOTAL MIN MAX`,
//!   `N_ELEMENTS`, `SIZE`, `INTARR FLTARR DBLARR LONARR`** — see "Builtins"
//!   below for exactly why each has no clean existing-SIR-vocabulary home
//!   in this cut.
//! - **`BREAK`/`CONTINUE`** — `semantic-ir` has no early-exit control-flow
//!   node at all (confirmed: no `Break`/`Continue` variant in
//!   `semantic-ir/src/nodes.rs`), the identical whole-IR gap
//!   `scilab-to-semantic-ir`'s own module doc documents for the same
//!   reason.
//! - **`RETURN` outside tail position** — see "RETURN" below.
//! - **Negative-from-end and wildcard-range-end subscripts** — see
//!   "Subscripting" below.
//! - **3-D-or-higher subscripting** — `idl-runtime`'s own
//!   `resolve_subscripts` and the JS backend's own `indexGet`/`indexSet`
//!   both cap at rank 2; mirrored here.
//! - Structures, pointers, objects, `LIST`/`HASH`, `COMMON` blocks,
//!   `CASE`/`SWITCH`/`FOREACH`, `_EXTRA`/`_REF_EXTRA` — all deferred whole
//!   by MA12 §4 itself, unchanged here.
//! - Chained assignment, nested `PRO`/`FUNCTION` definitions — IDL has
//!   neither (confirmed: `idl.grammar` reaches `pro_def`/`func_def` only
//!   from `top_level_item`, never from `statement`).
//!
//! # Case folding: yes, uppercase at lowering time
//!
//! `idl-runtime::eval`'s own module doc comment settles this exact question
//! for the runtime layer ("this evaluator decides yes... every identifier
//! this evaluator binds or looks up... is folded via `fold_case`
//! (`str::to_uppercase`) at the point it is first read off a `NAME`
//! token"), citing NV5 Geospatial documentation that real IDL folds
//! variable/routine names to uppercase internally. This frontend makes the
//! **same** decision, for the **same** reason the task asks about: so a SIR
//! module downstream of this lowering sees the identical canonical spelling
//! `idl-runtime` would compute for the identical program — `myVar`/`MYVAR`/
//! `MyVar` all lower to one `VarRef { name: "MYVAR", .. }`, exactly as they
//! resolve to one binding at runtime. Every `NAME` token this file reads —
//! variable, `PRO`/`FUNCTION` name, parameter name, keyword name — goes
//! through [`fold_case`] at the point it is first read off the CST, mirroring
//! `idl-runtime::eval::fold_case`'s own placement discipline exactly. This
//! is the first `-to-semantic-ir` frontend in this repo to make this call at
//! all (neither `scilab-to-semantic-ir` nor `q-to-semantic-ir` fold case,
//! since Scilab/Q are both case-sensitive — confirmed by grepping both
//! crates for any case-folding logic and finding none), so there is no
//! frontend precedent to follow here beyond `idl-runtime`'s own decision.
//!
//! # Subscripting
//!
//! IDL is 0-based (MA12 §2) — **unlike** MATLAB/Scilab's 1-based surface,
//! this frontend applies **no** index-base shift at all. Confirmed directly
//! against `idl-runtime::eval::resolve_index`, which treats a subscript
//! value as an ordinary 0-based position (translating only a *negative*
//! value by adding the axis length) — there is no `- 1` anywhere in that
//! function or in `subscript_positions`.
//!
//! ## 2-D subscript order: IDL's `[column, row]` vs. SIR's `[row, col]` —
//! **the two subscripts must be swapped**
//!
//! This is the single most important, easiest-to-get-backwards decision in
//! this whole file, and it was gotten wrong on a first pass and fixed
//! before landing — flagged here exactly as the task asked, alongside `#`
//! vs `##` below.
//!
//! MA12 §2 note 1 documents that IDL reads a 2-D subscript as
//! `a[column, row]` (the first subscript is the column), transposed from
//! MATLAB's `[row, column]`. `idl-runtime::eval::resolve_subscripts` commits
//! to a concrete mapping onto `array-runtime`'s own `(row, col)` addressing
//! (flagged there as a judgment call, but this frontend's job is to match
//! that ALREADY-CHOSEN runtime behavior, not re-derive a different one):
//!
//! ```text
//! // idl-runtime::eval::resolve_subscripts, 2-subscript arm:
//! let cols = self.subscript_positions(subs[0], arr.ncols())?;  // subs[0] (FIRST written) -> COLUMN axis
//! let rows = self.subscript_positions(subs[1], arr.nrows())?;  // subs[1] (SECOND written) -> ROW axis
//! ...
//! arr.get(rows[0], cols[0])   // array-runtime::Array::get(row, col)
//! ```
//!
//! So for source `a[i, j]`: `i` (written first) selects the **column**,
//! `j` (written second) selects the **row** — and the final read is
//! `get(row=j, col=i)`.
//!
//! `Expr::IndexGet`/`Stmt::IndexSet`'s own `indices: Vec<IndexArg>`, by
//! contrast, is **row-major** in *SIR's* convention: every existing
//! array-family frontend (`matlab-to-semantic-ir`, `scilab-to-semantic-ir`)
//! and the JS backend's own `indexGet`/`indexSet` (`const [rowArg, colArg] =
//! indices`) put the **row** selector at `indices[0]` and the **column**
//! selector at `indices[1]` — confirmed directly in
//! `semantic-ir-to-javascript/src/runtime.rs`.
//!
//! Putting these together: lowering IDL's `a[i, j]` **verbatim**, in written
//! order, into `indices: [lower(i), lower(j)]` would silently swap rows and
//! columns relative to `idl-runtime`'s own, already-fixed behavior — the
//! exact class of bug this task's brief warns about for `#`/`##`. The
//! correct lowering **reverses** the two subscripts:
//! `indices: [lower(j), lower(i)]` (`indices[0]` = the row selector = the
//! *second*-written IDL subscript; `indices[1]` = the column selector = the
//! *first*-written one) — see [`Lowerer::lower_index_args`].
//!
//! The 1-subscript form needs no such swap: both `idl-runtime`'s own
//! single-subscript arm (`arr.data()[i]`, a flat read over the whole
//! column-major buffer) and the JS backend's own 1-argument `indexGet`
//! (`a.data.length`-scoped) index the same flat storage directly — there is
//! only one axis, so there is nothing to reorder.
//!
//! ## Wildcard `*` and ranges: existing `IndexArg` shapes, no gap
//!
//! `a[*]` → [`IndexArg::Whole`] directly (MA12 §5 names this mapping
//! explicitly). `a[s0:s1]`/`a[s0:s1:n]` → [`IndexArg::Range`], wrapping an
//! [`Expr::Range`] built from the same `start`/`step`/`stop` slots —
//! confirmed exact (not merely plausible) by checking
//! `semantic-ir-to-javascript::runtime::range`'s own inclusive-of-`stop`
//! semantics against `idl-runtime::eval::range_subscript_positions`'s own
//! doc comment ("IDL's `[s0:s1]` is **inclusive of both endpoints**" — the
//! *identical* convention MATLAB's `1:5` already uses, which is exactly why
//! `Expr::Range`'s existing runtime behavior needs no adjustment here).
//!
//! ## Negative-from-end and wildcard-range-end: a genuine, disclosed gap
//!
//! `a[-1]` (last element) and `a[s0:*]` (from `s0` to the end) both need
//! the **runtime length of the indexed axis** to resolve — the identical
//! class of problem MATLAB's `end`-relative indexing is (`matlab-to-
//! semantic-ir` rejects it for the same reason: "no `size`/`shape` builtin
//! is wired up yet to resolve 'the current indexing dimension's size' at
//! lowering time"). Two candidate existing primitives were checked and
//! rejected as **unsound** rather than merely inconvenient:
//! - `BuiltinCall("tally", [target])` (J's monadic `#`, already registered
//!   in the JS backend) computes `shape[0]` — exactly the axis length for a
//!   RANK-1 vector target, but only `nrows` (not the total flat length) for
//!   a rank-2 matrix target, since IDL's own single-subscript rule indexes
//!   the FULL flat column-major buffer (confirmed directly against
//!   `idl-runtime::eval::resolve_subscripts`'s 1-subscript arm, `arr.len()`,
//!   not `arr.nrows()`). This frontend has no type inference (the same
//!   disclosed limitation `scilab-to-semantic-ir`'s own
//!   `expr_is_known_scalar` documents), so it cannot tell, for a general
//!   `VarRef` target, whether `tally` would be exact (vector) or silently
//!   wrong (matrix) — reusing it anyway would trade one clean rejection for
//!   a construct that is *sometimes* quietly incorrect, which this repo's
//!   own discipline treats as strictly worse than an honest error.
//! - A brand-new dedicated length builtin would need backend work
//!   (`semantic-ir-to-javascript`) outside this task's own scope (a pure
//!   frontend crate).
//!
//! So both forms are rejected with a clear [`IdlLowerError`] — a documented
//! gap (see this crate's `README.md`), not a silent mis-lowering. A general
//! (non-literal) subscript expression that merely *might* be negative at
//! runtime is not specially rejected: it lowers through unchanged, and if
//! it turns out negative when the compiled program actually runs, the JS
//! backend's own `indexGet`/`indexSet` throw a clean "out of bounds"
//! exception (fail loud, not a silently wrong value) — the same disclosed
//! residual-limitation shape `scilab-to-semantic-ir`'s own
//! `hoist_assigned_names` doc comment describes for an analogous
//! no-type-inference blind spot.
//!
//! # `#` vs `##`: verified against `idl-runtime`'s own, already-fixed
//! operand order — do not re-derive from scratch
//!
//! This is the other place the task explicitly flagged as review-caught
//! this session, in `idl-runtime` itself. `idl-runtime::eval::
//! eval_multiplicative`'s current (post-fix) behavior, read directly rather
//! than assumed:
//!
//! ```text
//! "HASH_HASH" => execute(Kernel::MatMul, &acc, &rhs)   // A ## B  =>  matmul(A, B)
//! "HASH"      => execute(Kernel::MatMul, &rhs, &acc)   // A # B   =>  matmul(B, A)  -- SWAPPED
//! ```
//!
//! (`acc` is the running left operand, `rhs` the next right operand, in
//! `idl-runtime`'s own left-to-right fold.) So `##` is the ordinary,
//! direct matrix product (`MatMul { lhs: A, rhs: B }`), while `#` computes
//! the product with its **operands swapped** (`MatMul { lhs: B, rhs: A }`)
//! — see [`Lowerer::build_multiplicative`]'s `"HASH"` arm, which mirrors
//! this exactly, operand for operand, rather than guessing at IDL's own
//! documented "`#` is the reversed/column-oriented product" description
//! independently.
//!
//! # Power is left-associative
//!
//! `idl.grammar`'s own header comment and `idl-parser`'s README both
//! confirm this, checked against the official NV5/L3Harris precedence
//! reference: `2^3^2` is `(2^3)^2 = 64` in real IDL, not `2^(3^2) = 512` —
//! the opposite of Scilab's/MATLAB's right-associative `^`. `power`'s own
//! grammar production is therefore left-recursive repetition
//! (`postfix { CARET postfix }`), and [`Lowerer::lower_power`] folds it with
//! a left-to-right loop (mirroring `idl-runtime::eval::eval_power` exactly),
//! not the right-fold `try_power` scilab's own single-operator-slot template
//! uses.
//!
//! # Two namespaces: `PRO`/`FUNCTION` may share a name — mangled at lowering
//! time
//!
//! MA12 §3's headline finding: the *same* name can be both a `PRO` and a
//! `FUNCTION` simultaneously, dispatched by call-site syntax
//! (`idl-runtime` keeps two separate `procs`/`funcs` tables for exactly this
//! reason). `semantic_ir::Module::functions` is a single, FLAT namespace —
//! every `Function.name` must be unique for `DirectCall { fn_name, .. }` to
//! resolve unambiguously. This frontend therefore **mangles every
//! procedure's own SIR function name** with a `$PROC` suffix a real IDL
//! `NAME` token can never spell (`NAME = /[a-zA-Z][a-zA-Z0-9_]*/` — no `$`
//! allowed at all, confirmed against `idl.tokens`), leaving `FUNCTION`
//! definitions unmangled — the exact same "a `$`-bearing name can never
//! collide with anything a real program could declare" trick
//! `scilab-to-semantic-ir::lower_select`'s own `$select_N` hoisted-temp
//! naming already established in this repo, reused here for a different
//! purpose (namespace separation, not a hoisted temp). A
//! `procedure_call_stmt` looks its target up as `NAME$PROC`; a function
//! call (an expression-position `postfix` `call_suffix`) looks its target
//! up as plain `NAME`. See [`Lowerer::proc_sir_name`].
//!
//! # Keyword arguments: SIR's existing KW1 vocabulary, not positional
//! desugaring
//!
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md) §3's own MA-12e
//! sketch anticipated desugaring keyword arguments to positional bindings
//! against the callee's declared parameter order, leaving "whether a
//! first-class keyword-argument SIR node is warranted... a decision for
//! that implementation item." Checked directly against `semantic-ir`'s
//! *current* state (not assumed from that older sketch): `Expr::KeywordArg`
//! / `ParamKind::Keyword` (KW1) already exist in the shared core, AND —
//! contrary to `semantic_ir::manifest::Feature`'s own (stale) doc comment,
//! which still says `KeywordParams` "is NOT yet accepted by any backend" —
//! `semantic-ir-to-javascript`'s own `ACCEPTED_FEATURES` list (`src/lib.rs`)
//! **does** include `Feature::KeywordParams` today (KW4 shipped: "JavaScript
//! has no native keyword-call form, so `emit` lowers `Keyword` params to a
//! trailing `__kw` options object... and each `Expr::KeywordArg` at a call
//! site into a trailing object literal"). So this frontend uses the
//! **existing**, already-backend-accepted KW1 vocabulary directly, which is
//! both simpler than positional desugaring (no need to thread the callee's
//! declared parameter order through every call site) and strictly more
//! faithful to IDL's own calling convention (a keyword genuinely omitted at
//! a call site stays a genuinely-omitted, name-matched argument, rather
//! than collapsing into an anonymous positional slot).
//!
//! One wrinkle IDL has that Ruby/Python's own keyword parameters do not:
//! `PRO`/`FUNCTION name, ..., KW=kw, ...` binds the **call-site** keyword
//! name (`KW`) to a **body-local** variable that may spell differently
//! (`kw`) — MA12 §4's own literal example. `semantic_ir::Param` has only one
//! name field, used both as the call-site-visible keyword name (what a
//! `KeywordArg.name` must match, per `semantic_ir::validator`'s own
//! name-resolution rule) and as the in-scope binding name. This frontend
//! resolves the mismatch by declaring the `Param` under the **call-site**
//! name (`KW`) and, when the local spelling genuinely differs, prepending
//! one ordinary `Stmt::LetStarBinding { name: "kw", value: VarRef("KW",
//! Param), .. }` to the routine's own body — an explicit, zero-new-vocabulary
//! rename, not a new binding-name field on `Param`. See
//! [`Lowerer::lower_routine_def`].
//!
//! `/KEYWORD` (the boolean shorthand) lowers to `Expr::KeywordArg { name:
//! "KEYWORD", value: IntLit(1) }` directly — MA12 §3 item 3's own
//! `== KEYWORD=1` equivalence, applied at lowering time instead of at
//! runtime.
//!
//! An omitted keyword is modeled as `Param { kind: Keyword, default:
//! Some(NilLit) }` (optional, so omitting it validates cleanly) rather than
//! `default: None` (which `semantic_ir::validator` treats as *required* —
//! `semantic-ir/src/nodes.rs`'s own `missing_keywords` doc comment). This is
//! a disclosed simplification, not a perfect match for `idl-runtime`'s own
//! "omitted keyword is genuinely UNDEFINED, not defaulted" rule (MA12 §3,
//! itself flagged there as unverified against a real IDL session) — a
//! `Param` must carry *some* value when its argument is omitted for
//! `semantic_ir`'s own model to accept the call at all, and `NilLit` is the
//! same placeholder-of-last-resort `scilab-to-semantic-ir::
//! hoist_assigned_names` already uses for an analogous "must be defined,
//! but real value is a later concern" situation. One concrete consequence:
//! `N_ELEMENTS(kw)`'s idiomatic "was this keyword passed?" test (MA12 §3)
//! is not reproducible through this lowering even if `N_ELEMENTS` itself
//! were in scope (it is not — see "Builtins" below) — a keyword bound to
//! `NilLit` is *defined*, not *absent*, from this frontend's own IR
//! forward.
//!
//! # RETURN: only in tail position
//!
//! `semantic-ir` has no early-exit control-flow node (the same gap
//! documented above for `BREAK`/`CONTINUE`) — a `Block`'s value is always
//! its trailing expression, with no way to escape early from inside a
//! nested `If`/`While`/`ForRange` body. Real IDL's `RETURN`/`RETURN, expr`
//! can appear anywhere, including deep inside a branch. This frontend
//! supports exactly the one shape that needs no early-exit node at all: a
//! `RETURN`/`RETURN, expr` as the **literal last statement** of a routine's
//! own top-level body (not nested inside any `IF`/`FOR`/`WHILE`/`REPEAT`) —
//! in that position it is simply "the function's trailing value," lowered
//! directly into the `Block`'s own `value` slot, no different in spirit
//! from how `scilab-to-semantic-ir::lower_func_def` synthesizes a trailing
//! `VarRef` to the designated output variable. Every other placement
//! (nested inside a branch/loop, or a non-trailing top-level statement) is a
//! clean, disclosed [`IdlLowerError`], mirroring the identical treatment
//! `scilab-to-semantic-ir` gives `break`/`continue`.  A `FUNCTION` whose
//! body does not end this way is rejected outright (mirroring
//! `idl-runtime`'s own "FUNCTION completed without RETURN" runtime error,
//! turned into a compile-time one here); a `PRO` with no trailing `RETURN`
//! at all lowers its body value as `Expr::NilLit`, mirroring
//! `scilab-to-semantic-ir`'s own zero-output convention.
//!
//! # Builtins: only what maps onto *existing*, already-working SIR
//! vocabulary
//!
//! No sibling array-family frontend (`matlab-to-semantic-ir`,
//! `scilab-to-semantic-ir`, `apl-to-semantic-ir`, `j-to-semantic-ir`,
//! `q-to-semantic-ir`) registers a generic top-level math/reduction
//! `BuiltinCall` name (`sin`, `sqrt`, `sum`, ...) that
//! `semantic-ir-to-javascript` implements today — confirmed directly by
//! grepping that crate's own builtin dispatch table. So MA12 §4's small
//! builtin surface splits cleanly into two groups:
//!
//! **Maps onto an existing, already-working SIR node — supported:**
//! - `PRINT, x` (exactly one argument, mirroring
//!   `scilab_to_semantic_ir`'s own identical `disp` restriction) →
//!   `BuiltinCall("print", [x])`.
//! - `TRANSPOSE(a)` → `Expr::Transpose { target: a, conjugate: false }` (IDL
//!   has no complex/conjugate-transpose distinction) — the *same*
//!   `array_runtime::execute(Transpose, ...)` kernel `idl-runtime`'s own
//!   `TRANSPOSE` builtin already calls, so this is exact for every rank.
//! - `INDGEN`/`FINDGEN`/`DINDGEN`/`LINDGEN(n)` (all four identical in this
//!   `f64`-only cut, MA12 §2/§4) → `Expr::Range { start: 0, step: None,
//!   stop: n - 1 }`. Exact: `idl-runtime::builtins::indgen` computes
//!   `(0..n).map(|i| i as f64)` — precisely the inclusive `0..=(n-1)` range
//!   `Expr::Range`'s own already-verified inclusive-of-`stop` semantics
//!   (see "Subscripting" above) produce.
//!
//! **No existing SIR/backend primitive is exact for every rank a general
//! (non-literal) argument might have — rejected, a disclosed gap, not a
//! guess:**
//! - `TOTAL`/`MIN`/`MAX` — `idl-runtime::builtins` computes these via
//!   `array_runtime::ops::{sum,max,min}`, a **whole-array** reduction to one
//!   scalar regardless of rank. The nearest existing SIR node,
//!   `Expr::Reduce` (the APL-addendum fold), instead folds **per row** for a
//!   rank-2 target (`array_runtime::ops::reduce`'s own doc comment: "a
//!   matrix `[r, c]` folds each row... producing a vector `[r]`") — exact
//!   only when the target is provably rank ≤ 1, which this frontend (no
//!   type inference) cannot verify for a general `VarRef` argument.
//! - `N_ELEMENTS`/`SIZE` — no existing "total element count / dimension
//!   vector, any rank" SIR primitive (see "Subscripting" above for the
//!   identical `tally`-is-rank-1-only finding, which rules it out here for
//!   the same reason).
//! - `SIN COS TAN SQRT ABS EXP ALOG ALOG10` — no elementwise math-function
//!   `BuiltinCall` name is registered in the JS backend at all today.
//! - `INTARR FLTARR DBLARR LONARR` — no existing SIR primitive materializes
//!   "N zero-filled elements" at a *dynamic* (non-literal) runtime size.
//!
//! # Bitwise operators: a genuine, disclosed gap
//!
//! `idl-runtime::builtins::{bitwise_and,bitwise_or,bitwise_xor,bitwise_not}`
//! truncate each `f64` operand to a real **64-bit** `i64` before the
//! bitwise op (`(x as i64) & (y as i64)`, `!(x as i64)`) — genuine two's-
//! complement bitwise arithmetic, confirmed directly against that module's
//! own source, not the short-circuit boolean logic `&&`/`||` are (MA12 §4
//! spells `AND`/`OR`/`NOT`/`XOR` "logical/bitwise" for exactly this reason).
//! Two roads were checked and rejected:
//! - `Expr::LogicalAnd`/`LogicalOr` (already existing, already working) are
//!   short-circuit-**boolean** nodes: reusing them would silently disagree
//!   with `idl-runtime`'s own non-short-circuit, integer-bitwise ground
//!   truth for a non-`{0,1}`-valued operand, or for an operand whose
//!   evaluation has an observable side effect only `idl-runtime` would
//!   actually perform.
//! - JavaScript's own native `&`/`|`/`^`/`~` operators truncate to **32-bit**
//!   signed integers, not 64 — a large-magnitude IDL bitwise result (or
//!   `NOT` on any value near or beyond ±2^31) would silently disagree with
//!   `idl-runtime`'s own 64-bit truncation even if a new `BuiltinCall`
//!   routed straight to them.
//!
//! A correct implementation needs a genuinely new, 64-bit-correct (likely
//! `BigInt`-based) backend runtime helper — squarely `semantic-ir-to-
//! javascript` follow-up work, out of this frontend-only task's scope
//! (mirroring how J's own `tally`/`replicate`/`exp` and Q's five `q_*`
//! primitives were each *introduced* by one frontend PR but only wired up
//! in the shared JS backend by a later, separate one). `AND`/`OR`/`XOR`/
//! `NOT` are therefore rejected outright in this cut, documented as a known
//! gap in this crate's own `README.md` rather than silently mis-lowered
//! onto `LogicalAnd`/`LogicalOr` or a native-but-narrower-width builtin.

use std::collections::HashSet;

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, IndexArg, Metadata, Module, Param,
    ParamKind, Scope, Span, Stmt,
};

/// Maximum expression-nesting depth. `idl-parser`'s own `MAX_RULE_DEPTH`
/// (148, per its README's own measured-not-assumed methodology) already
/// bounds how deep a tree reaching this crate can possibly be, but this
/// lowering pass does its own recursive descent over that tree (a
/// completely separate native call stack), so it needs its own guard for
/// the same reason every sibling `-to-semantic-ir` frontend's identically
/// named constant does: turn a pathologically deep (but parser-accepted)
/// input into a clean [`IdlLowerError`] instead of a native stack overflow.
const MAX_EXPR_DEPTH: usize = 200;

/// Maximum statement-block nesting depth (each `IF`/`FOR`/`WHILE`/`REPEAT`/
/// `BEGIN` body, or a `PRO`/`FUNCTION` body, re-enters the block lowerer one
/// level deeper).
const MAX_BLOCK_DEPTH: usize = 200;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path) — mirrors every sibling frontend's identical `FILE`
/// constant.
const FILE: &str = "<idl>";

/// The name-mangling suffix that keeps `PRO`-namespace SIR function names
/// distinct from `FUNCTION`-namespace ones (see this file's module doc
/// comment, "Two namespaces"). Contains `$`, which a real IDL `NAME` token
/// can never spell (`idl.tokens`' own `NAME` pattern has no `$` in its
/// character class), so this can never collide with a user-declared
/// routine name.
const PROC_SUFFIX: &str = "$PROC";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during IDL → SIR lowering.
///
/// Mirrors `ScilabLowerError`/`QLowerError`'s shape exactly (`message` +
/// 1-based `line`/`column`) so tooling can treat every SIR frontend
/// uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdlLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for IdlLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IdlLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for IdlLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed IDL CST (rooted at the `program` rule) into a SIR module.
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, IdlLowerError> {
    Lowerer::new(module_name).lower_file(tree)
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// One lowered top-level / body statement: either a `Stmt` or a bare
/// expression (an expression statement) — mirrors
/// `scilab_to_semantic_ir::Lowered` exactly.
enum Lowered {
    Stmt(Box<Stmt>),
    Expr(Expr),
}

/// Which of IDL's two separate namespaces (MA12 §3) a routine belongs to —
/// used only to decide the `$PROC`-mangling at definition/call sites (see
/// [`Lowerer::proc_sir_name`]); mirrors `idl_runtime::eval::RoutineKind` in
/// spirit, though this crate does not need to keep a full callable
/// representation the way the runtime does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoutineKind {
    Procedure,
    Function,
}

/// Per-routine name-resolution context. IDL has no block scoping at all —
/// mirrors `scilab_to_semantic_ir::FunctionCtx`'s own doc comment nearly
/// verbatim: a routine call gets a wholly fresh, isolated frame
/// (`idl-runtime::eval`'s own module doc, point 3: "no automatic
/// outer/global visibility inside a call"), so `locals` simply grows for
/// the routine's lifetime by default, with the one exception below.
struct RoutineCtx {
    params: HashSet<String>,
    locals: Vec<String>,
    locals_set: HashSet<String>,
}

impl RoutineCtx {
    fn new(params: HashSet<String>) -> Self {
        Self {
            params,
            locals: Vec::new(),
            locals_set: HashSet::new(),
        }
    }

    fn top_level() -> Self {
        Self::new(HashSet::new())
    }

    fn has_local(&self, name: &str) -> bool {
        self.locals_set.contains(name)
    }

    fn push_local(&mut self, name: String) {
        // Mirrors `scilab_to_semantic_ir::FunctionCtx::push_local`'s own
        // security-review-driven guard: pushing an ALREADY-known local a
        // second time would later have `scope_rewind` drain it and forget
        // its pre-existing membership too (a genuine set has no
        // duplicate-count tracking). A name already in scope needs no
        // further tracking here.
        if self.locals_set.contains(&name) {
            return;
        }
        self.locals_set.insert(name.clone());
        self.locals.push(name);
    }
}

struct Lowerer {
    module_name: String,
    observed: FeatureManifest,
    /// Every top-level `PRO` name, in its already-`$PROC`-mangled SIR form
    /// -- collected in a first pass so a `procedure_call_stmt` anywhere in
    /// the file resolves regardless of textual definition order (mirrors
    /// `scilab_to_semantic_ir::collect_function_names`'s identical
    /// rationale, applied to IDL's own separate `PRO` namespace).
    proc_names: HashSet<String>,
    /// Every top-level `FUNCTION` name (unmangled) -- the sibling
    /// collection for IDL's other namespace.
    func_names: HashSet<String>,
    /// The lowered top-level routines, in definition order. `main` is
    /// appended last by [`Self::lower_file`].
    functions: Vec<Function>,
    /// Counter for the compiler-generated `$repeat_N` flag locals
    /// [`Self::lower_repeat`] hoists -- see that function's own doc comment
    /// for why `REPEAT...UNTIL` is desugared via a hoisted flag rather than
    /// by lowering the loop body twice (once inline, once inside a `WHILE`).
    /// A single module-wide counter (not per-routine) is simplest and still
    /// guarantees uniqueness, mirroring
    /// `scilab_to_semantic_ir::Lowerer::select_counter`'s identical
    /// "one counter, whole module" choice for its own hoisted `$select_N`
    /// temporary.
    repeat_counter: usize,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
            proc_names: HashSet::new(),
            func_names: HashSet::new(),
            functions: Vec::new(),
            repeat_counter: 0,
        }
    }

    /// The SIR function name a routine of kind `kind` named `name`
    /// (already case-folded) resolves to -- see this file's module doc
    /// comment, "Two namespaces".
    fn proc_sir_name(kind: RoutineKind, name: &str) -> String {
        match kind {
            RoutineKind::Procedure => format!("{name}{PROC_SUFFIX}"),
            RoutineKind::Function => name.to_string(),
        }
    }

    // -------------------------------------------------------------------
    // scope mark/rewind -- the ONE place IDL truly does introduce a scope
    // narrower than the whole routine (a FOR loop's own counter, see
    // `lower_for`'s own doc comment on why the counter is deliberately
    // scope-rewound rather than left visible after the loop).
    // -------------------------------------------------------------------

    fn scope_mark(ctx: &RoutineCtx) -> usize {
        ctx.locals.len()
    }

    fn scope_rewind(ctx: &mut RoutineCtx, mark: usize) {
        for name in ctx.locals.drain(mark..) {
            ctx.locals_set.remove(&name);
        }
    }

    // -------------------------------------------------------------------
    // Hoisting a branch/loop body's introduced names -- ported from
    // `scilab_to_semantic_ir::{collect_assigned_names,hoist_assigned_names}`,
    // adapted to IDL's own statement-rule names. See that crate's own
    // extensive doc comment (`lower.rs`, "Hoisting a branch/loop body's
    // introduced names") for the full rationale this section applies
    // verbatim: IDL, like Scilab, has NO block scoping (`idl-runtime`'s own
    // single-frame-per-call model, this file's module doc comment's "Case
    // folding" section's neighbor), so a name first assigned inside an
    // `IF`/`FOR`/`WHILE`/`REPEAT` body must be visible to code AFTER the
    // construct too -- `semantic_ir::validate` scopes each `Block`
    // independently, so a `LetStarBinding` nested inside a branch's own
    // `Block` does NOT, on its own, make the name visible outside it.
    // -------------------------------------------------------------------

    /// Statically scan a body -- WITHOUT lowering anything -- for every
    /// bare-`NAME` assignment target it introduces, recursing transitively
    /// through nested control flow. A purely syntactic pre-pass (no `self`
    /// mutation), so it can never double-count a `Feature`, and -- just as
    /// importantly -- never re-lowers anything (avoiding the exponential
    /// `2^K` blowup `scilab_to_semantic_ir`'s own security review rejected
    /// for an earlier "lower twice, once to discover" design).
    fn collect_assigned_names(
        &self,
        stmts: &[&GrammarASTNode],
        depth: usize,
        out: &mut HashSet<String>,
    ) -> Result<(), IdlLowerError> {
        if depth > MAX_BLOCK_DEPTH {
            return Err(IdlLowerError {
                message: format!(
                    "control-flow nesting too deep (exceeds {MAX_BLOCK_DEPTH} levels)"
                ),
                line: 1,
                column: 1,
            });
        }
        for stmt in stmts {
            self.collect_assigned_names_stmt(stmt, depth, out)?;
        }
        Ok(())
    }

    fn collect_assigned_names_stmt(
        &self,
        stmt: &GrammarASTNode,
        depth: usize,
        out: &mut HashSet<String>,
    ) -> Result<(), IdlLowerError> {
        let inner = only_node(stmt, self)?;
        match inner.rule_name.as_str() {
            "if_stmt" => {
                let kids = child_nodes(inner);
                // `[cond, then_branch, else_branch?]` -- see `lower_if`'s
                // own doc comment for the exact shape.
                if let Some(then_branch) = kids.get(1) {
                    self.collect_assigned_names(
                        &body_statements(then_branch, self)?,
                        depth + 1,
                        out,
                    )?;
                }
                if let Some(else_branch) = kids.get(2) {
                    self.collect_assigned_names(
                        &body_statements(else_branch, self)?,
                        depth + 1,
                        out,
                    )?;
                }
            }
            "while_stmt" => {
                let kids = child_nodes(inner);
                if let Some(body) = kids.get(1) {
                    self.collect_assigned_names(&body_statements(body, self)?, depth + 1, out)?;
                }
            }
            "repeat_stmt" => {
                let kids = child_nodes(inner);
                if let Some(body) = kids.first() {
                    self.collect_assigned_names(&body_statements(body, self)?, depth + 1, out)?;
                }
            }
            "for_stmt" => {
                // Deliberately do NOT collect the loop counter's own name
                // here -- it has its own, separate, always-loop-scoped
                // lifetime (`lower_for`'s own `scope_mark`/`scope_rewind`),
                // mirroring `scilab_to_semantic_ir::collect_assigned_names_stmt`'s
                // identical `for_stmt` arm and its own security-regression
                // note (round 5) on exactly this point.
                let body = find_child(inner, "for_body")?;
                self.collect_assigned_names(&body_statements(body, self)?, depth + 1, out)?;
            }
            "begin_block" => {
                let bb = find_child(inner, "block_body")?;
                self.collect_assigned_names(&block_body_statements(bb), depth + 1, out)?;
            }
            _ => {
                if inner.rule_name == "assignment_stmt" {
                    // `assignment_stmt = NAME [index_suffix] EQUALS expr` --
                    // only a PLAIN assignment (no index_suffix) can
                    // introduce a genuinely NEW name; an indexed assignment
                    // requires the base variable to already exist (see
                    // `lower_assignment`).
                    if find_child_opt(inner, "index_suffix").is_none() {
                        if let Some(ASTNodeOrToken::Token(t)) = inner.children.first() {
                            out.insert(fold_case(&t.value));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Pre-declare (hoist) every name [`Self::collect_assigned_names`]
    /// finds across `bodies`, at the current enclosing scope, with a
    /// placeholder [`Expr::NilLit`] -- mirrors
    /// `scilab_to_semantic_ir::hoist_assigned_names` exactly, including its
    /// own disclosed residual limitation (no definite-assignment analysis:
    /// a body that reads a hoisted name before any reachable path in THIS
    /// construct actually assigns it reads `NilLit` instead of erroring,
    /// which fails LOUD at the JS layer -- an uncaught `TypeError` -- not
    /// silently, for the identical reason that file's own doc comment
    /// documents at length).
    fn hoist_assigned_names(
        &mut self,
        bodies: &[Vec<&GrammarASTNode>],
        exclude: Option<&str>,
        depth: usize,
        span: &Span,
        ctx: &mut RoutineCtx,
    ) -> Result<Vec<Lowered>, IdlLowerError> {
        let mut candidates: HashSet<String> = HashSet::new();
        for body in bodies {
            self.collect_assigned_names(body, depth, &mut candidates)?;
        }
        if let Some(ex) = exclude {
            candidates.remove(ex);
        }
        let mut hoisted = Vec::new();
        let mut sorted: Vec<String> = candidates.into_iter().collect();
        sorted.sort();
        for name in sorted {
            if ctx.has_local(&name) || ctx.params.contains(&name) {
                continue;
            }
            ctx.push_local(name.clone());
            hoisted.push(Lowered::Stmt(Box::new(Stmt::LetStarBinding {
                name,
                sir_type: None,
                value: Expr::NilLit { span: span.clone() },
                span: span.clone(),
            })));
        }
        Ok(hoisted)
    }

    // -------------------------------------------------------------------
    // top level: `program` -> collect routine names, then lower
    // -------------------------------------------------------------------

    fn lower_file(&mut self, program: &GrammarASTNode) -> Result<Module, IdlLowerError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        // Every value this frontend lowers has `sir_type: None` -- IDL has
        // no static type declarations in this cut (the typed numeric tower
        // is deferred, MA12 §2/§4).
        self.observed.add(Feature::DynamicTyping);

        self.collect_routine_names(program)?;

        let mut ctx = RoutineCtx::top_level();
        let mut items: Vec<Lowered> = Vec::new();
        for top_item in child_nodes(program) {
            let inner = only_node(top_item, self)?;
            match inner.rule_name.as_str() {
                "pro_def" => {
                    let f = self.lower_routine_def(inner, RoutineKind::Procedure)?;
                    self.functions.push(f);
                }
                "func_def" => {
                    let f = self.lower_routine_def(inner, RoutineKind::Function)?;
                    self.functions.push(f);
                }
                "statement_line" => {
                    for stmt in statement_line_statements(inner) {
                        items.extend(self.lower_statement(stmt, &mut ctx, 0)?);
                    }
                }
                other => {
                    return Err(self.err_at(inner, format!("unexpected top-level node `{other}`")))
                }
            }
        }

        let span = Span::point(FILE, 1, 1);
        let main_body =
            assemble_stmts_only(items, Expr::NilLit { span: span.clone() }, span.clone());
        let main = Function {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: main_body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: span.clone(),
        };

        let mut functions = std::mem::take(&mut self.functions);
        functions.push(main);

        let metadata = Metadata::new()
            .with_source_language("idl")
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

    /// Pass 1: collect every top-level `PRO`/`FUNCTION` name (case-folded,
    /// `PRO` names pre-mangled with `$PROC`) so a call anywhere in the file
    /// -- regardless of textual order -- resolves correctly. Mirrors
    /// `scilab_to_semantic_ir::collect_function_names`'s identical
    /// two-pass rationale, applied across IDL's two separate namespaces.
    fn collect_routine_names(&mut self, program: &GrammarASTNode) -> Result<(), IdlLowerError> {
        for top_item in child_nodes(program) {
            let inner = only_node(top_item, self)?;
            match inner.rule_name.as_str() {
                "pro_def" => {
                    let name = fold_case(&self.routine_def_name(inner)?);
                    self.proc_names
                        .insert(Self::proc_sir_name(RoutineKind::Procedure, &name));
                }
                "func_def" => {
                    let name = fold_case(&self.routine_def_name(inner)?);
                    self.func_names.insert(name);
                }
                _ => {}
            }
        }
        Ok(())
    }

    // -------------------------------------------------------------------
    // routine definitions
    // -------------------------------------------------------------------

    /// `pro_def`/`func_def`'s own name: the `NAME` token at children[1]
    /// (children[0] is the `"PRO"`/`"FUNCTION"` keyword token itself) --
    /// mirrors `idl_runtime::eval::register_routine`'s identical
    /// `node.children.get(1)` access exactly.
    fn routine_def_name(&self, def: &GrammarASTNode) -> Result<String, IdlLowerError> {
        match def.children.get(1) {
            Some(ASTNodeOrToken::Token(t)) => Ok(t.value.clone()),
            _ => Err(self.err_at(def, format!("malformed `{}` (missing name)", def.rule_name))),
        }
    }

    /// One declared parameter (MA12 §3/§4): `NAME` (positional) or
    /// `KEYWORD EQUALS local` (keyword) -- mirrors
    /// `idl_runtime::eval::build_params`'s identical two-shape parsing.
    fn param_spec(&self, p: &GrammarASTNode) -> Result<ParamSpec, IdlLowerError> {
        let names: Vec<&Token> = p
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => Some(t),
                _ => None,
            })
            .collect();
        match names.len() {
            2 => Ok(ParamSpec {
                keyword: Some(fold_case(&names[0].value)),
                local: fold_case(&names[1].value),
            }),
            1 => Ok(ParamSpec {
                keyword: None,
                local: fold_case(&names[0].value),
            }),
            _ => Err(self.err_at(p, "malformed parameter declaration".to_string())),
        }
    }

    /// Lower a `pro_def`/`func_def` into a top-level [`Function`]. See this
    /// file's module doc comment's "Keyword arguments" section for the
    /// keyword-name-vs-local-name aliasing this performs, and "RETURN" for
    /// the tail-position requirement.
    fn lower_routine_def(
        &mut self,
        def: &GrammarASTNode,
        kind: RoutineKind,
    ) -> Result<Function, IdlLowerError> {
        let span = self.span_of(def);
        let name = fold_case(&self.routine_def_name(def)?);
        let sir_name = Self::proc_sir_name(kind, &name);

        let mut param_specs: Vec<ParamSpec> = Vec::new();
        if let Some(params_node) = find_child_opt(def, "params") {
            for p in child_nodes(params_node) {
                param_specs.push(self.param_spec(p)?);
            }
        }

        // `param_names` becomes `ctx.params` -- the set of names the SIR
        // `Function` itself declares as parameters (`Scope::Param`
        // references resolve against this). For a positional param this is
        // the same spelling the body uses; for a KEYWORD param it is the
        // CALL-SITE keyword name (`kw`), NOT the body-local name -- see
        // this file's module doc comment's "Keyword arguments" section.
        // The body-local name (when it differs) is instead pushed onto
        // `ctx`'s ordinary LOCALS below, via the alias `LetStarBinding`,
        // exactly like any other first-occurrence local.
        let mut param_names: HashSet<String> = HashSet::new();
        let mut params: Vec<Param> = Vec::new();
        let mut alias_stmts: Vec<Stmt> = Vec::new();
        for spec in &param_specs {
            match &spec.keyword {
                None => {
                    param_names.insert(spec.local.clone());
                    params.push(Param {
                        name: spec.local.clone(),
                        sir_type: None,
                        kind: ParamKind::Required,
                        default: None,
                        span: span.clone(),
                    });
                }
                Some(kw) => {
                    self.observed.add(Feature::KeywordParams);
                    param_names.insert(kw.clone());
                    params.push(Param {
                        name: kw.clone(),
                        sir_type: None,
                        kind: ParamKind::Keyword,
                        default: Some(Box::new(Expr::NilLit { span: span.clone() })),
                        span: span.clone(),
                    });
                    if kw != &spec.local {
                        alias_stmts.push(Stmt::LetStarBinding {
                            name: spec.local.clone(),
                            sir_type: None,
                            value: Expr::VarRef {
                                name: kw.clone(),
                                scope: Scope::Param,
                                span: span.clone(),
                            },
                            span: span.clone(),
                        });
                    }
                }
            }
        }

        let body_node = find_child(def, "block_body")?;
        let stmts = block_body_statements(body_node);

        let mut ctx = RoutineCtx::new(param_names);
        // Pre-declare every keyword param's own body-local alias name (when
        // it differs from the call-site keyword) as a known LOCAL, so a
        // reference to it anywhere in the body resolves as `Scope::Local`
        // against the alias `LetStarBinding` prepended below, not as an
        // (incorrect) `Scope::Param` reference to a name the `Function`
        // never actually declares.
        for alias in &alias_stmts {
            if let Stmt::LetStarBinding { name, .. } = alias {
                ctx.push_local(name.clone());
            }
        }
        let (value, body_items) = self.lower_routine_body(&stmts, kind, &span, &mut ctx, 0)?;

        let mut block = assemble_stmts_only(body_items, value, span.clone());
        // Prepend keyword-alias bindings (see above) -- must run before any
        // body statement that might reference the local name.
        for alias in alias_stmts.into_iter().rev() {
            block.stmts.insert(0, alias);
        }

        Ok(Function {
            name: sir_name,
            params,
            return_type: None,
            captures: vec![],
            body: block,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span,
        })
    }

    /// Lower a routine's own top-level statement list, splitting off a
    /// trailing `RETURN`/`RETURN, expr` (if present) as the `Block`'s own
    /// value slot -- see this file's module doc comment, "RETURN".
    fn lower_routine_body(
        &mut self,
        stmts: &[&GrammarASTNode],
        kind: RoutineKind,
        def_span: &Span,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<(Expr, Vec<Lowered>), IdlLowerError> {
        let (body_stmts, trailing_return) = match stmts.split_last() {
            Some((last, rest)) => {
                let inner = only_node(last, self)?;
                if inner.rule_name == "return_stmt" {
                    (rest, Some(inner))
                } else {
                    (stmts, None)
                }
            }
            None => (stmts, None),
        };

        let mut items = Vec::new();
        for s in body_stmts {
            items.extend(self.lower_statement(s, ctx, depth)?);
        }

        let value = match (kind, trailing_return) {
            (RoutineKind::Function, Some(ret)) => match find_child_opt(ret, "expr") {
                Some(e) => self.lower_expr(e, ctx, 0)?,
                None => {
                    return Err(self.err_at(
                        ret,
                        "unsupported: FUNCTION used a bare RETURN with no value".to_string(),
                    ))
                }
            },
            (RoutineKind::Function, None) => {
                return Err(IdlLowerError {
                    message: "unsupported: FUNCTION completed without a trailing RETURN, value \
                              (mirrors idl-runtime's own identical runtime error, made a \
                              lowering-time error here)"
                        .to_string(),
                    line: def_span.start_line,
                    column: def_span.start_col,
                })
            }
            (RoutineKind::Procedure, Some(ret)) => {
                if find_child_opt(ret, "expr").is_some() {
                    return Err(self.err_at(
                        ret,
                        "unsupported: PRO used RETURN with a value; procedures have no return \
                         value"
                            .to_string(),
                    ));
                }
                Expr::NilLit {
                    span: self.span_of(ret),
                }
            }
            (RoutineKind::Procedure, None) => Expr::NilLit {
                span: def_span.clone(),
            },
        };

        Ok((value, items))
    }

    // -------------------------------------------------------------------
    // statement bodies
    // -------------------------------------------------------------------

    fn lower_body_items(
        &mut self,
        stmts: &[&GrammarASTNode],
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Vec<Lowered>, IdlLowerError> {
        let mut items = Vec::new();
        for s in stmts {
            items.extend(self.lower_statement(s, ctx, depth)?);
        }
        Ok(items)
    }

    fn lower_block(
        &mut self,
        stmts: &[&GrammarASTNode],
        span: &Span,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Block, IdlLowerError> {
        if depth > MAX_BLOCK_DEPTH {
            return Err(IdlLowerError {
                message: format!(
                    "control-flow nesting too deep (exceeds {MAX_BLOCK_DEPTH} levels)"
                ),
                line: span.start_line,
                column: span.start_col,
            });
        }
        let items = self.lower_body_items(stmts, ctx, depth)?;
        Ok(assemble_stmts_only(
            items,
            Expr::NilLit { span: span.clone() },
            span.clone(),
        ))
    }

    /// Dispatch one `statement` node. Returns a `Vec` (not a single
    /// `Lowered`) purely for uniformity with `lower_if`'s own
    /// hoisted-locals-plus-construct shape; every other arm returns exactly
    /// one item.
    fn lower_statement(
        &mut self,
        stmt: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Vec<Lowered>, IdlLowerError> {
        if depth > MAX_BLOCK_DEPTH {
            return Err(self.err_at(
                stmt,
                format!("control-flow nesting too deep (exceeds {MAX_BLOCK_DEPTH} levels)"),
            ));
        }
        let inner = only_node(stmt, self)?;
        match inner.rule_name.as_str() {
            "if_stmt" => self.lower_if(inner, ctx, depth),
            "for_stmt" => self.lower_for(inner, ctx, depth),
            "while_stmt" => self.lower_while(inner, ctx, depth),
            "repeat_stmt" => self.lower_repeat(inner, ctx, depth),
            "begin_block" => {
                let bb = find_child(inner, "block_body")?;
                self.lower_body_items(&block_body_statements(bb), ctx, depth + 1)
            }
            "break_stmt" => Err(self.err_at(
                inner,
                "unsupported: `BREAK` has no SIR equivalent yet (semantic-ir has no early-exit \
                 control-flow node at all -- a whole-IR gap, not specific to this frontend)"
                    .to_string(),
            )),
            "continue_stmt" => Err(self.err_at(
                inner,
                "unsupported: `CONTINUE` has no SIR equivalent yet (semantic-ir has no early-exit \
                 control-flow node at all -- a whole-IR gap, not specific to this frontend)"
                    .to_string(),
            )),
            "return_stmt" => Err(self.err_at(
                inner,
                "unsupported: `RETURN` is only supported as the literal last statement of a \
                 PRO/FUNCTION's own top-level body (semantic-ir has no early-exit control-flow \
                 node) -- see this crate's README for the full rationale"
                    .to_string(),
            )),
            "procedure_call_stmt" => Ok(vec![self.lower_procedure_call_stmt(inner, ctx, depth)?]),
            "assignment_stmt" => Ok(vec![self.lower_assignment(inner, ctx, depth)?]),
            "expr_stmt" => {
                let e = find_child(inner, "expr")?;
                let expr = self.lower_expr(e, ctx, depth)?;
                Ok(vec![Lowered::Expr(expr)])
            }
            other => Err(self.err_at(inner, format!("unknown statement kind `{other}`"))),
        }
    }

    // -------------------------------------------------------------------
    // control flow
    // -------------------------------------------------------------------

    /// `if_stmt = "IF" expr "THEN" then_branch ["ELSE" else_branch]` ->
    /// `node_children(if_stmt)` is `[cond, then_branch, else_branch?]` (the
    /// `IF`/`THEN`/`ELSE` keywords are bare tokens, not child nodes).
    fn lower_if(
        &mut self,
        if_stmt: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Vec<Lowered>, IdlLowerError> {
        let kids = child_nodes(if_stmt);
        if kids.len() < 2 {
            return Err(self.err_at(if_stmt, "malformed if_stmt".to_string()));
        }
        let cond_node = kids[0];
        let then_branch = kids[1];
        let else_branch = kids.get(2).copied();
        let if_span = self.span_of(if_stmt);

        let then_stmts = body_statements(then_branch, self)?;
        let else_stmts = match else_branch {
            Some(b) => body_statements(b, self)?,
            None => Vec::new(),
        };

        let mut stmts = self.hoist_assigned_names(
            &[then_stmts.clone(), else_stmts.clone()],
            None,
            depth + 1,
            &if_span,
            ctx,
        )?;

        let cond = to_idl_condition(self.lower_expr(cond_node, ctx, 0)?);
        let then_block = self.lower_block(&then_stmts, &if_span, ctx, depth + 1)?;
        let else_block = if else_branch.is_some() {
            self.lower_block(&else_stmts, &if_span, ctx, depth + 1)?
        } else {
            empty_block(if_span.clone())
        };

        stmts.push(Lowered::Expr(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_block),
            else_branch: Box::new(else_block),
            span: if_span,
        }));
        Ok(stmts)
    }

    /// `while_stmt = "WHILE" expr "DO" while_body` -> `[cond, body]`.
    fn lower_while(
        &mut self,
        while_stmt: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Vec<Lowered>, IdlLowerError> {
        let kids = child_nodes(while_stmt);
        let (cond_node, body_node) = match kids.as_slice() {
            [c, b] => (*c, *b),
            _ => return Err(self.err_at(while_stmt, "malformed while_stmt".to_string())),
        };
        let span = self.span_of(while_stmt);
        let body_stmts = body_statements(body_node, self)?;

        let mut stmts = self.hoist_assigned_names(
            std::slice::from_ref(&body_stmts),
            None,
            depth + 1,
            &span,
            ctx,
        )?;
        let cond = to_idl_condition(self.lower_expr(cond_node, ctx, 0)?);
        let body = self.lower_block(&body_stmts, &span, ctx, depth + 1)?;
        self.observed.add(Feature::Loops);
        stmts.push(Lowered::Stmt(Box::new(Stmt::While { cond, body, span })));
        Ok(stmts)
    }

    /// `repeat_stmt = "REPEAT" repeat_body "UNTIL" expr` -> `[body, cond]`.
    /// Real IDL's `REPEAT...UNTIL` runs the body at least once and exits
    /// when `expr` becomes true.
    ///
    /// **Security regression, caught before landing**: an earlier revision
    /// of this function desugared this by lowering the body TWICE -- once
    /// inline (the guaranteed first run) and once again as the body of a
    /// `WHILE (NOT cond) DO ...` (the remainder). That duplicates the
    /// ENTIRE lowered body at every nesting level, so K textually nested
    /// `REPEAT...UNTIL` statements (each containing the next) produce a
    /// lowered `Module` of size O(2^K) -- exponential in the SOURCE size,
    /// regardless of whether the second copy is produced by re-lowering the
    /// AST or by `.clone()`-ing the already-built `Block` (cloning is
    /// linear in the size being cloned, but that size is itself already
    /// exponential one level down) -- the exact class of DoS
    /// `scilab_to_semantic_ir::lower_select`'s own doc comment describes
    /// rejecting an analogous "lower twice" design for `select`/`case`.
    ///
    /// Fixed by lowering the body exactly ONCE, using a hoisted boolean
    /// flag to fold the "must run at least once" requirement into the
    /// `WHILE` loop's own condition instead of duplicating the body:
    ///
    /// ```text
    /// $repeat_N = 1
    /// WHILE ($repeat_N) OR (NOT cond) DO
    ///   $repeat_N = 0
    ///   <body, lowered exactly once>
    /// ENDWHILE
    /// ```
    ///
    /// `$repeat_N` starts true, so the loop's first iteration always runs
    /// regardless of `cond`; the loop body's own first statement then
    /// clears the flag, so every SUBSEQUENT iteration is gated purely on
    /// `NOT cond`, exactly matching `REPEAT...UNTIL`'s "run once, then loop
    /// while not yet satisfied" semantics. `$repeat_N` is never part of a
    /// real IDL `NAME` token (`idl.tokens`' own `NAME` pattern has no `$` in
    /// its character class), so it can never collide with a user-declared
    /// identifier -- the same collision-freedom argument
    /// `scilab_to_semantic_ir::lower_select`'s own `$select_N` and this
    /// file's own `$PROC` suffix already rely on.
    fn lower_repeat(
        &mut self,
        repeat_stmt: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Vec<Lowered>, IdlLowerError> {
        let kids = child_nodes(repeat_stmt);
        let (body_node, cond_node) = match kids.as_slice() {
            [b, c] => (*b, *c),
            _ => return Err(self.err_at(repeat_stmt, "malformed repeat_stmt".to_string())),
        };
        let span = self.span_of(repeat_stmt);
        let body_stmts = body_statements(body_node, self)?;

        let mut stmts = self.hoist_assigned_names(
            std::slice::from_ref(&body_stmts),
            None,
            depth + 1,
            &span,
            ctx,
        )?;

        let flag = format!("$repeat_{}", self.repeat_counter);
        self.repeat_counter += 1;
        ctx.push_local(flag.clone());
        stmts.push(Lowered::Stmt(Box::new(Stmt::LetStarBinding {
            name: flag.clone(),
            sir_type: None,
            value: Expr::IntLit {
                value: 1,
                span: span.clone(),
            },
            span: span.clone(),
        })));

        let cond = self.lower_expr(cond_node, ctx, 0)?;
        let not_cond = Expr::BuiltinCall {
            name: "not".to_string(),
            args: vec![to_idl_condition(cond)],
            effects: EffectSet::PURE,
            span: span.clone(),
        };
        let flag_ref = to_idl_condition(Expr::VarRef {
            name: flag.clone(),
            scope: Scope::Local,
            span: span.clone(),
        });
        self.observed.add(Feature::ShortCircuit);
        let loop_cond = Expr::LogicalOr {
            lhs: Box::new(flag_ref),
            rhs: Box::new(not_cond),
            span: span.clone(),
        };

        // Lower the body EXACTLY ONCE, with the flag-clearing assignment as
        // its own first statement.
        let mut body_items = vec![Lowered::Stmt(Box::new(Stmt::Assign {
            name: flag,
            scope: Scope::Local,
            value: Expr::IntLit {
                value: 0,
                span: span.clone(),
            },
            span: span.clone(),
        }))];
        body_items.extend(self.lower_body_items(&body_stmts, ctx, depth + 1)?);
        let body = assemble_stmts_only(
            body_items,
            Expr::NilLit { span: span.clone() },
            span.clone(),
        );

        self.observed.add(Feature::Loops);
        stmts.push(Lowered::Stmt(Box::new(Stmt::While {
            cond: loop_cond,
            body,
            span,
        })));
        Ok(stmts)
    }

    /// `for_stmt = "FOR" NAME EQUALS expr COMMA expr [COMMA expr] "DO"
    /// for_body` -> the loop variable is a bare token at children[1] (not a
    /// child node); `node_children(for_stmt)` is then `[init, limit,
    /// step?, body]` (mirrors `idl_runtime::eval::exec_for`'s own
    /// extraction exactly). `Stmt::ForRange` is half-open (`stop`
    /// exclusive); IDL's own `FOR v=a,b DO` is INCLUSIVE of `b` (confirmed
    /// directly against `idl_runtime::eval::exec_for`'s own loop condition,
    /// `i <= limit`), so the exclusive bound is `limit + step_sign` --
    /// exact only when `step` is a compile-time-known-sign literal, which
    /// is why this frontend requires the `step` slot (when present) to be a
    /// literal, non-zero constant (see the `step` handling below) rather
    /// than accepting an arbitrary runtime expression whose sign it cannot
    /// determine at lowering time.
    fn lower_for(
        &mut self,
        for_stmt: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Vec<Lowered>, IdlLowerError> {
        let span = self.span_of(for_stmt);
        let var = match for_stmt.children.get(1) {
            Some(ASTNodeOrToken::Token(t)) => fold_case(&t.value),
            _ => {
                return Err(
                    self.err_at(for_stmt, "malformed for_stmt: no loop variable".to_string())
                )
            }
        };

        // Mirrors `scilab_to_semantic_ir::lower_for`'s identical, hard-won
        // fix: the shared JS backend's `ForRange` codegen JS-block-scopes
        // the loop variable, so reusing an ALREADY-known name as the
        // counter would silently read a STALE pre-loop value if referenced
        // (without first reassigning it) after the loop -- rejected
        // outright rather than "supported but sometimes silently wrong".
        if ctx.has_local(&var) || ctx.params.contains(&var) {
            return Err(self.err_at(
                for_stmt,
                format!(
                    "unsupported: reusing an existing variable `{var}` as a FOR-loop counter is \
                     out of scope for v0.1.0 (the shared JS backend's ForRange codegen \
                     block-scopes the loop variable, so its value would not correctly persist \
                     past the loop if read without first reassigning it)"
                ),
            ));
        }

        let exprs: Vec<&GrammarASTNode> = child_nodes(for_stmt)
            .into_iter()
            .filter(|n| n.rule_name == "expr")
            .collect();
        if exprs.len() < 2 {
            return Err(self.err_at(
                for_stmt,
                "malformed for_stmt: missing init/limit".to_string(),
            ));
        }
        let for_body = find_child(for_stmt, "for_body")?;

        let init = self.lower_expr(exprs[0], ctx, 0)?;
        let limit = self.lower_expr(exprs[1], ctx, 0)?;
        // A literal integer step, defaulting to `+1` -- see this function's
        // own doc comment for why a non-literal step is out of scope (its
        // sign decides which direction the exclusive `stop` bound must be
        // adjusted, and this frontend has no way to know that sign for a
        // general runtime expression).
        let step_value: i64 = if exprs.len() >= 3 {
            literal_int_value(exprs[2]).ok_or_else(|| {
                self.err_at(
                    exprs[2],
                    "unsupported: a FOR-loop step must be a literal, non-zero integer constant \
                     in this cut (its sign decides which direction the inclusive limit is \
                     adjusted to SIR's half-open ForRange, which this frontend cannot determine \
                     for a general runtime expression)"
                        .to_string(),
                )
            })?
        } else {
            1
        };
        if step_value == 0 {
            return Err(self.err_at(for_stmt, "FOR step must not be zero".to_string()));
        }

        let limit_span = limit.span().clone();
        let stop = Expr::BuiltinCall {
            name: "+".to_string(),
            args: vec![
                limit,
                Expr::IntLit {
                    value: step_value.signum(),
                    span: limit_span.clone(),
                },
            ],
            effects: EffectSet::PURE,
            span: limit_span,
        };
        let step = Expr::IntLit {
            value: step_value,
            span: span.clone(),
        };

        self.observed.add(Feature::Loops);

        let body_stmts = body_statements(for_body, self)?;
        let mut stmts = self.hoist_assigned_names(
            std::slice::from_ref(&body_stmts),
            Some(var.as_str()),
            depth + 1,
            &span,
            ctx,
        )?;

        let mark = Self::scope_mark(ctx);
        ctx.push_local(var.clone());
        let body = self.lower_block(&body_stmts, &span, ctx, depth + 1)?;
        // See this function's own doc comment: the counter is scope-rewound
        // (removed) after the loop, mirroring `scilab_to_semantic_ir`'s
        // identical choice -- a program that reads the counter's final
        // value AFTER the loop fails to LOWER with a clean "undefined
        // variable" error, a safe, disclosed divergence from real
        // `idl-runtime`'s own same-frame counter persistence, rather than
        // compiling to something the JS backend's own block-scoped
        // `ForRange` codegen would silently get wrong.
        Self::scope_rewind(ctx, mark);

        stmts.push(Lowered::Stmt(Box::new(Stmt::ForRange {
            var,
            start: init,
            stop,
            step,
            body,
            span,
        })));
        Ok(stmts)
    }

    // -------------------------------------------------------------------
    // assignment
    // -------------------------------------------------------------------

    /// `assignment_stmt = NAME [index_suffix] EQUALS expr`.
    fn lower_assignment(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Lowered, IdlLowerError> {
        let span = self.span_of(node);
        let name = match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) => fold_case(&t.value),
            _ => return Err(self.err_at(node, "malformed assignment_stmt".to_string())),
        };
        let expr_node = find_child(node, "expr")?;
        let index_suffix = find_child_opt(node, "index_suffix");

        match index_suffix {
            None => {
                let value = self.lower_expr(expr_node, ctx, depth)?;
                if ctx.has_local(&name) || ctx.params.contains(&name) {
                    self.observed.add(Feature::MutableBindings);
                    Ok(Lowered::Stmt(Box::new(Stmt::Assign {
                        name,
                        scope: Scope::Local,
                        value,
                        span,
                    })))
                } else {
                    ctx.push_local(name.clone());
                    Ok(Lowered::Stmt(Box::new(Stmt::LetStarBinding {
                        name,
                        sir_type: None,
                        value,
                        span,
                    })))
                }
            }
            Some(idx) => {
                if !(ctx.has_local(&name) || ctx.params.contains(&name)) {
                    return Err(self.err_at(
                        node,
                        format!(
                            "cannot subscript-assign into `{name}`: not previously assigned \
                             (auto-vivification is out of scope for v0.1.0)"
                        ),
                    ));
                }
                let subscript_list = find_child(idx, "subscript_list")?;
                let indices = self.lower_index_args(subscript_list, ctx, 0)?;
                let value = self.lower_expr(expr_node, ctx, depth)?;
                self.observed.add(Feature::NDArrays);
                Ok(Lowered::Stmt(Box::new(Stmt::IndexSet {
                    target: Box::new(Expr::VarRef {
                        name,
                        scope: Scope::Local,
                        span: span.clone(),
                    }),
                    indices,
                    value: Box::new(value),
                    span,
                })))
            }
        }
    }

    // -------------------------------------------------------------------
    // procedure calls (statement position)
    // -------------------------------------------------------------------

    /// `procedure_call_stmt = NAME COMMA arg_list`.
    fn lower_procedure_call_stmt(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Lowered, IdlLowerError> {
        let span = self.span_of(node);
        let name = match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) => fold_case(&t.value),
            _ => return Err(self.err_at(node, "malformed procedure_call_stmt".to_string())),
        };
        let arg_list = find_child_opt(node, "arg_list");

        if name == "PRINT" {
            let (positional, keyword) = self.lower_call_args(arg_list, ctx, depth)?;
            if !keyword.is_empty() {
                return Err(self.err_at(
                    node,
                    "unsupported: PRINT does not accept keyword arguments in this cut".to_string(),
                ));
            }
            if positional.len() != 1 {
                return Err(self.err_at(
                    node,
                    "unsupported: PRINT takes exactly one argument in this cut (mirrors \
                     scilab-to-semantic-ir's own identical `disp` restriction)"
                        .to_string(),
                ));
            }
            return Ok(Lowered::Expr(Expr::BuiltinCall {
                name: "print".to_string(),
                args: positional,
                effects: EffectSet::PURE,
                span,
            }));
        }

        let mangled = Self::proc_sir_name(RoutineKind::Procedure, &name);
        if self.proc_names.contains(&mangled) {
            let args = self.lower_direct_call_args(arg_list, ctx, depth)?;
            return Ok(Lowered::Expr(Expr::DirectCall {
                fn_name: mangled,
                args,
                effects: EffectSet::PURE,
                span,
            }));
        }

        Err(self.err_at(
            node,
            format!(
                "unsupported: unknown procedure `{name}` (not a known PRO, and only PRINT is \
                 recognised as a builtin procedure in this cut)"
            ),
        ))
    }

    // -------------------------------------------------------------------
    // call arguments
    // -------------------------------------------------------------------

    /// Split `arg_list` into (positional expressions, keyword-name ->
    /// lowered-value pairs, in written order) -- the raw pieces a builtin
    /// dispatch or a `DirectCall` args assembly is built from. `/KEYWORD`
    /// lowers here too (to `IntLit(1)`, MA12 §3 item 3).
    fn lower_call_args(
        &mut self,
        arg_list: Option<&GrammarASTNode>,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<CallArgParts, IdlLowerError> {
        let mut positional = Vec::new();
        let mut keyword = Vec::new();
        let Some(al) = arg_list else {
            return Ok((positional, keyword));
        };
        for arg in child_nodes(al) {
            match arg.children.first() {
                Some(ASTNodeOrToken::Node(n)) if n.rule_name == "keyword_arg" => {
                    let kw_name = match n.children.first() {
                        Some(ASTNodeOrToken::Token(t)) => fold_case(&t.value),
                        _ => return Err(self.err_at(n, "malformed keyword_arg".to_string())),
                    };
                    let e = find_child(n, "expr")?;
                    let value = self.lower_expr(e, ctx, depth)?;
                    keyword.push((kw_name, value));
                }
                Some(ASTNodeOrToken::Node(n)) if n.rule_name == "bool_keyword_arg" => {
                    let kw_name = match n.children.get(1) {
                        Some(ASTNodeOrToken::Token(t)) => fold_case(&t.value),
                        _ => return Err(self.err_at(n, "malformed bool_keyword_arg".to_string())),
                    };
                    keyword.push((
                        kw_name,
                        Expr::IntLit {
                            value: 1,
                            span: self.span_of(n),
                        },
                    ));
                }
                Some(ASTNodeOrToken::Node(n)) if n.rule_name == "expr" => {
                    positional.push(self.lower_expr(n, ctx, depth)?);
                }
                _ => return Err(self.err_at(arg, "malformed arg node".to_string())),
            }
        }
        Ok((positional, keyword))
    }

    /// Assemble a `DirectCall`'s own `args` vec from `arg_list`: every
    /// positional expression first (in written order), then every keyword
    /// argument as an `Expr::KeywordArg` (in written order) -- the
    /// validator requires this exact bucketing regardless of the SOURCE's
    /// own interleaving (`semantic_ir::validator`'s own "positional
    /// argument may not follow a keyword argument" rule) -- see this file's
    /// module doc comment's "Keyword arguments" section.
    fn lower_direct_call_args(
        &mut self,
        arg_list: Option<&GrammarASTNode>,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Vec<Expr>, IdlLowerError> {
        let (positional, keyword) = self.lower_call_args(arg_list, ctx, depth)?;
        if !keyword.is_empty() {
            self.observed.add(Feature::KeywordParams);
        }
        let mut args = positional;
        for (name, value) in keyword {
            let span = value.span().clone();
            args.push(Expr::KeywordArg {
                name,
                value: Box::new(value),
                span,
            });
        }
        Ok(args)
    }

    // -------------------------------------------------------------------
    // expressions: precedence dispatch
    // -------------------------------------------------------------------

    fn lower_expr(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Expr, IdlLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        match node.rule_name.as_str() {
            "expr" | "group" => self.lower_expr(only_node(node, self)?, ctx, depth + 1),
            "logical" => self.lower_logical(node, ctx, depth),
            "comparison" => self.lower_comparison(node, ctx, depth),
            "additive" => self.lower_additive(node, ctx, depth),
            "unary" => self.lower_unary(node, ctx, depth),
            "multiplicative" => self.lower_multiplicative(node, ctx, depth),
            "power" => self.lower_power(node, ctx, depth),
            "postfix" => self.lower_postfix(node, ctx, depth),
            "primary" => self.lower_primary(node, ctx, depth),
            other => Err(self.err_at(node, format!("unexpected expression node `{other}`"))),
        }
    }

    /// `logical = comparison { ("AND"|"OR"|"XOR") comparison }`. See this
    /// file's module doc comment, "Bitwise operators": rejected whenever at
    /// least one such operator is present (the no-operator case passes
    /// through to `comparison` unchanged, same as every other tier in this
    /// cascade).
    fn lower_logical(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Expr, IdlLowerError> {
        let (first, rest) = chain_parts(node);
        if rest.is_empty() {
            return self.lower_expr(first, ctx, depth + 1);
        }
        self.check_chain_length(node)?;
        Err(self.err_at(
            node,
            "unsupported: `AND`/`OR`/`XOR` (bitwise) are out of scope for v0.1.0 -- no existing \
             SIR/backend primitive reproduces IDL's genuine 64-bit bitwise semantics faithfully \
             (see this crate's README and lower.rs's own module doc comment, \"Bitwise \
             operators\", for the full rationale)"
                .to_string(),
        ))
    }

    /// `comparison = additive { ("EQ"|"NE"|"LE"|"LT"|"GE"|"GT") additive }`.
    fn lower_comparison(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Expr, IdlLowerError> {
        let (first, rest) = chain_parts(node);
        if rest.is_empty() {
            return self.lower_expr(first, ctx, depth + 1);
        }
        self.check_chain_length(node)?;
        let mut acc = self.lower_expr(first, ctx, depth + 1)?;
        for (op, operand) in rest {
            let rhs = self.lower_expr(operand, ctx, depth + 1)?;
            let name = match op.value.as_str() {
                "EQ" => "=",
                "NE" => "!=",
                "LT" => "<",
                "LE" => "<=",
                "GT" => ">",
                "GE" => ">=",
                other => {
                    return Err(self.err_at(node, format!("unknown comparison operator `{other}`")))
                }
            };
            if matches!(name, "<" | "<=" | ">" | ">=") {
                self.reject_string_operand(&acc, node, "ordering comparisons")?;
                self.reject_string_operand(&rhs, node, "ordering comparisons")?;
            }
            let span = acc.span().clone();
            acc = Expr::BuiltinCall {
                name: name.to_string(),
                args: vec![acc, rhs],
                effects: EffectSet::PURE,
                span,
            };
        }
        Ok(acc)
    }

    /// `additive = unary { (PLUS|MINUS) unary }`.
    fn lower_additive(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Expr, IdlLowerError> {
        let (first, rest) = chain_parts(node);
        if rest.is_empty() {
            return self.lower_expr(first, ctx, depth + 1);
        }
        self.check_chain_length(node)?;
        let mut acc = self.lower_expr(first, ctx, depth + 1)?;
        let mut acc_scalar = expr_is_known_scalar(&acc);
        for (op, operand) in rest {
            let rhs = self.lower_expr(operand, ctx, depth + 1)?;
            self.reject_string_operand(&acc, node, "arithmetic (`+`/`-`)")?;
            self.reject_string_operand(&rhs, node, "arithmetic (`+`/`-`)")?;
            let rhs_scalar = expr_is_known_scalar(&rhs);
            let op_name = match op.effective_type_name() {
                "PLUS" => "+",
                "MINUS" => "-",
                other => {
                    return Err(self.err_at(node, format!("unknown additive operator `{other}`")))
                }
            };
            let span = acc.span().clone();
            if acc_scalar && rhs_scalar {
                acc = Expr::BuiltinCall {
                    name: op_name.to_string(),
                    args: vec![acc, rhs],
                    effects: EffectSet::PURE,
                    span,
                };
            } else {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                let kind = if op_name == "+" {
                    semantic_ir::ElementwiseOpKind::Add
                } else {
                    semantic_ir::ElementwiseOpKind::Sub
                };
                acc = Expr::ElementwiseOp {
                    op: kind,
                    lhs: Box::new(acc),
                    rhs: Box::new(rhs),
                    span,
                };
            }
            acc_scalar = acc_scalar && rhs_scalar;
        }
        Ok(acc)
    }

    /// `unary = (PLUS|MINUS|"NOT") unary | multiplicative`. `NOT` (bitwise
    /// complement) is rejected here -- see this file's module doc comment,
    /// "Bitwise operators".
    fn lower_unary(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Expr, IdlLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Token(op), ASTNodeOrToken::Node(inner)] => {
                let op_name = if op.effective_type_name() == "KEYWORD" {
                    op.value.as_str()
                } else {
                    op.effective_type_name()
                };
                if op_name == "NOT" {
                    return Err(self.err_at(
                        node,
                        "unsupported: `NOT` (bitwise complement) is out of scope for v0.1.0 -- \
                         see this crate's README, \"Bitwise operators\""
                            .to_string(),
                    ));
                }
                let operand = self.lower_expr(inner, ctx, depth + 1)?;
                let span = operand.span().clone();
                match op_name {
                    "PLUS" => Ok(operand),
                    "MINUS" => {
                        self.reject_string_operand(&operand, node, "unary `-`")?;
                        Ok(match operand {
                            Expr::IntLit { value, span } => Expr::IntLit {
                                value: value.wrapping_neg(),
                                span,
                            },
                            Expr::FloatLit { value, span } => Expr::FloatLit {
                                value: -value,
                                span,
                            },
                            other => Expr::BuiltinCall {
                                name: "neg".to_string(),
                                args: vec![other],
                                effects: EffectSet::PURE,
                                span,
                            },
                        })
                    }
                    other => Err(self.err_at(node, format!("unknown unary operator `{other}`"))),
                }
            }
            [ASTNodeOrToken::Node(inner)] => self.lower_expr(inner, ctx, depth + 1),
            _ => Err(self.err_at(node, "malformed unary node".to_string())),
        }
    }

    /// `multiplicative = power { (STAR|SLASH|HASH_HASH|HASH) power }`. `*`
    /// is ALWAYS elementwise in IDL (confirmed directly against
    /// `idl_runtime::eval::eval_multiplicative`'s own `"STAR" =>
    /// ops::mul(...)` arm -- never a matmul disambiguation the way MATLAB's/
    /// Scilab's bare `*` needs, since IDL's matrix product is exclusively
    /// `#`/`##`), so no scalar-vs-array *matmul* branch is needed here at
    /// all -- only the ordinary scalar-fast-path-vs-broadcast choice every
    /// other elementwise op in this file makes. `#`/`##` are handled in
    /// [`Self::build_matmul`] -- see this file's module doc comment's `#`
    /// vs `##` section for the exact, verified operand order.
    fn lower_multiplicative(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Expr, IdlLowerError> {
        let (first, rest) = chain_parts(node);
        if rest.is_empty() {
            return self.lower_expr(first, ctx, depth + 1);
        }
        self.check_chain_length(node)?;
        let mut acc = self.lower_expr(first, ctx, depth + 1)?;
        let mut acc_scalar = expr_is_known_scalar(&acc);
        for (op, operand) in rest {
            let rhs = self.lower_expr(operand, ctx, depth + 1)?;
            self.reject_string_operand(&acc, node, "arithmetic (`*`/`/`/`#`/`##`)")?;
            self.reject_string_operand(&rhs, node, "arithmetic (`*`/`/`/`#`/`##`)")?;
            let rhs_scalar = expr_is_known_scalar(&rhs);
            let op_name = op.effective_type_name();
            let span = acc.span().clone();
            acc = match op_name {
                "STAR" | "SLASH" => {
                    if acc_scalar && rhs_scalar {
                        Expr::BuiltinCall {
                            name: if op_name == "STAR" { "*" } else { "/" }.to_string(),
                            args: vec![acc, rhs],
                            effects: EffectSet::PURE,
                            span,
                        }
                    } else {
                        self.observed.add(Feature::MatrixOps);
                        self.observed.add(Feature::ArrayColumnMajor);
                        Expr::ElementwiseOp {
                            op: if op_name == "STAR" {
                                semantic_ir::ElementwiseOpKind::Mul
                            } else {
                                semantic_ir::ElementwiseOpKind::Div
                            },
                            lhs: Box::new(acc),
                            rhs: Box::new(rhs),
                            span,
                        }
                    }
                }
                "HASH_HASH" | "HASH" => {
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    // `A ## B` -> matmul(A, B); `A # B` -> matmul(B, A),
                    // SWAPPED -- see this file's module doc comment's `#`
                    // vs `##` section, verified directly against
                    // idl-runtime::eval::eval_multiplicative's own current
                    // (already-fixed) behavior.
                    let (lhs, rhs) = if op_name == "HASH_HASH" {
                        (acc, rhs)
                    } else {
                        (rhs, acc)
                    };
                    Expr::MatMul {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                        span,
                    }
                }
                other => {
                    return Err(
                        self.err_at(node, format!("unknown multiplicative operator `{other}`"))
                    )
                }
            };
            acc_scalar = acc_scalar && rhs_scalar && matches!(op_name, "STAR" | "SLASH");
        }
        Ok(acc)
    }

    /// `power = postfix { CARET postfix }` -- LEFT-associative (see this
    /// file's module doc comment, "Power is left-associative"), so this
    /// folds with a left-to-right loop, unlike a right-recursive template.
    fn lower_power(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Expr, IdlLowerError> {
        let (first, rest) = chain_parts(node);
        if rest.is_empty() {
            return self.lower_expr(first, ctx, depth + 1);
        }
        self.check_chain_length(node)?;
        let mut acc = self.lower_expr(first, ctx, depth + 1)?;
        for (_caret, operand) in rest {
            let rhs = self.lower_expr(operand, ctx, depth + 1)?;
            self.reject_string_operand(&acc, node, "power (`^`)")?;
            self.reject_string_operand(&rhs, node, "power (`^`)")?;
            let span = acc.span().clone();
            self.observed.add(Feature::MatrixOps);
            self.observed.add(Feature::ArrayColumnMajor);
            acc = Expr::ElementwiseOp {
                op: semantic_ir::ElementwiseOpKind::Pow,
                lhs: Box::new(acc),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(acc)
    }

    // -------------------------------------------------------------------
    // postfix: indexing / call
    // -------------------------------------------------------------------

    /// `postfix = primary { index_suffix | call_suffix }`. Mirrors
    /// `idl_runtime::eval::eval_postfix`'s own dispatch: a bare-`NAME`
    /// primary immediately followed by a `call_suffix` is ALWAYS a function
    /// call (IDL has no first-class function values, so `NAME(...)` can
    /// never mean "index a variable named NAME") -- decided BEFORE
    /// evaluating `primary` the ordinary way, exactly mirroring that
    /// function's own comment.
    fn lower_postfix(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Expr, IdlLowerError> {
        let primary_node = match node.children.first() {
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "primary" => n,
            _ => return Err(self.err_at(node, "malformed postfix node".to_string())),
        };
        let suffixes: Vec<&GrammarASTNode> = node.children[1..]
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .collect();
        if suffixes.len() > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!(
                    "expression chain too long ({} operands, exceeds {MAX_EXPR_DEPTH})",
                    suffixes.len()
                ),
            ));
        }

        let mut value: Expr;
        let mut start = 0;
        if let (Some(name), Some(first_suffix)) =
            (bare_name_primary(primary_node), suffixes.first())
        {
            if first_suffix.rule_name == "call_suffix" {
                value =
                    self.lower_call_expr(&fold_case(name), primary_node, first_suffix, ctx, depth)?;
                start = 1;
            } else {
                value = self.lower_primary(primary_node, ctx, depth + 1)?;
            }
        } else {
            value = self.lower_primary(primary_node, ctx, depth + 1)?;
        }

        for suffix in &suffixes[start..] {
            value = match suffix.rule_name.as_str() {
                "index_suffix" => {
                    let subscript_list = find_child(suffix, "subscript_list")?;
                    let indices = self.lower_index_args(subscript_list, ctx, depth + 1)?;
                    self.observed.add(Feature::NDArrays);
                    let span = value.span().clone();
                    Expr::IndexGet {
                        target: Box::new(value),
                        indices,
                        span,
                    }
                }
                "call_suffix" => {
                    return Err(self.err_at(
                        suffix,
                        "unsupported: cannot call a value that is not a bare routine name (IDL \
                         has no first-class function values in this cut)"
                            .to_string(),
                    ))
                }
                other => {
                    return Err(self.err_at(suffix, format!("unsupported postfix suffix `{other}`")))
                }
            };
        }
        Ok(value)
    }

    /// Dispatch a function call (expression position): builtins recognised
    /// by this frontend first (see this file's module doc comment,
    /// "Builtins"), then the `FUNCTION` namespace.
    fn lower_call_expr(
        &mut self,
        name: &str,
        primary: &GrammarASTNode,
        call_suffix: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Expr, IdlLowerError> {
        let span = self.span_of(primary);
        let arg_list = find_child_opt(call_suffix, "arg_list");

        match name {
            "TRANSPOSE" => {
                let (positional, keyword) = self.lower_call_args(arg_list, ctx, depth)?;
                if !keyword.is_empty() || positional.len() != 1 {
                    return Err(self.err_at(
                        primary,
                        "TRANSPOSE takes exactly one positional argument in this cut".to_string(),
                    ));
                }
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                return Ok(Expr::Transpose {
                    target: Box::new(positional.into_iter().next().expect("len checked above")),
                    conjugate: false,
                    span,
                });
            }
            "INDGEN" | "FINDGEN" | "DINDGEN" | "LINDGEN" => {
                let (positional, keyword) = self.lower_call_args(arg_list, ctx, depth)?;
                if !keyword.is_empty() || positional.len() != 1 {
                    return Err(self.err_at(
                        primary,
                        format!("{name} takes exactly one positional argument in this cut"),
                    ));
                }
                let n = positional.into_iter().next().expect("len checked above");
                let n_span = n.span().clone();
                let stop = Expr::BuiltinCall {
                    name: "-".to_string(),
                    args: vec![
                        n,
                        Expr::IntLit {
                            value: 1,
                            span: n_span.clone(),
                        },
                    ],
                    effects: EffectSet::PURE,
                    span: n_span,
                };
                self.observed.add(Feature::NDArrays);
                return Ok(Expr::Range {
                    start: Box::new(Expr::IntLit {
                        value: 0,
                        span: span.clone(),
                    }),
                    step: None,
                    stop: Box::new(stop),
                    span,
                });
            }
            "SIN" | "COS" | "TAN" | "SQRT" | "ABS" | "EXP" | "ALOG" | "ALOG10" | "TOTAL"
            | "MIN" | "MAX" | "N_ELEMENTS" | "SIZE" | "INTARR" | "FLTARR" | "DBLARR" | "LONARR" => {
                return Err(self.err_at(
                    primary,
                    format!(
                        "unsupported: `{name}` is a documented v0.1.0 scope gap (see this \
                         crate's README, \"Builtins\") -- no existing SIR/backend primitive is \
                         exact for every array rank a general argument might have"
                    ),
                ))
            }
            _ => {}
        }

        if self.func_names.contains(name) {
            let args = self.lower_direct_call_args(arg_list, ctx, depth)?;
            return Ok(Expr::DirectCall {
                fn_name: name.to_string(),
                args,
                effects: EffectSet::PURE,
                span,
            });
        }

        Err(self.err_at(
            primary,
            format!(
                "unsupported: unknown identifier `{name}` (not a known FUNCTION, and only \
                 TRANSPOSE/INDGEN/FINDGEN/DINDGEN/LINDGEN are recognised as builtin functions in \
                 this cut)"
            ),
        ))
    }

    fn lower_primary(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Expr, IdlLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NUMBER" => {
                Ok(self.number_literal_expr(t, &self.span_of(node)))
            }
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "STRING" => {
                self.observed.add(Feature::Strings);
                Ok(Expr::StrLit {
                    value: t.value.clone(),
                    span: self.span_of(node),
                })
            }
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => {
                let name = fold_case(&t.value);
                let span = self.span_of(node);
                if ctx.params.contains(&name) {
                    Ok(Expr::VarRef {
                        name,
                        scope: Scope::Param,
                        span,
                    })
                } else if ctx.has_local(&name) {
                    Ok(Expr::VarRef {
                        name,
                        scope: Scope::Local,
                        span,
                    })
                } else {
                    Err(self.err_at(
                        node,
                        format!("undefined variable `{name}` (not previously assigned)"),
                    ))
                }
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "array_literal" => {
                self.lower_array_literal(n, ctx, depth + 1)
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "group" => {
                let inner = find_child(n, "expr")?;
                self.lower_expr(inner, ctx, depth + 1)
            }
            _ => Err(self.err_at(node, "malformed primary node".to_string())),
        }
    }

    /// `array_literal = LBRACKET [array_elements] RBRACKET`, `array_elements
    /// = expr {COMMA expr}` -- always FLAT (no row-separator token exists in
    /// this grammar at all), so this always emits exactly ONE row -- a
    /// genuine rank-1 array, never a scalar (MA12 §2), matching
    /// `idl_runtime::eval::eval_array_literal`'s own `Array::from_vec`
    /// (always shape `[n]`) exactly.
    fn lower_array_literal(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Expr, IdlLowerError> {
        if depth > MAX_BLOCK_DEPTH {
            return Err(self.err_at(
                node,
                format!("array literal nesting too deep (exceeds {MAX_BLOCK_DEPTH} levels)"),
            ));
        }
        self.observed.add(Feature::NDArrays);
        self.observed.add(Feature::ArrayColumnMajor);
        let span = self.span_of(node);
        let mut elements = Vec::new();
        if let Some(elems) = find_child_opt(node, "array_elements") {
            for e in child_nodes(elems) {
                elements.push(self.lower_expr(e, ctx, depth + 1)?);
            }
        }
        Ok(Expr::ArrayLit {
            rows: vec![elements],
            span,
        })
    }

    // -------------------------------------------------------------------
    // subscripting
    // -------------------------------------------------------------------

    /// Lower a `subscript_list` into `Vec<IndexArg>`, applying the 2-D
    /// column/row SWAP this file's module doc comment documents at length
    /// -- verified against `idl_runtime::eval::resolve_subscripts`'s own,
    /// already-fixed behavior, not re-derived independently.
    fn lower_index_args(
        &mut self,
        subscript_list: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<Vec<IndexArg>, IdlLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                subscript_list,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        let subs = child_nodes(subscript_list);
        match subs.len() {
            1 => {
                // The single-subscript form indexes the array's own flat
                // column-major storage directly -- one axis, no swap
                // needed (see this file's module doc comment).
                Ok(vec![self.lower_one_subscript(subs[0], ctx, depth + 1)?])
            }
            2 => {
                // subs[0] (written FIRST) is IDL's own COLUMN selector;
                // subs[1] (written SECOND) is IDL's own ROW selector.
                // SIR's `IndexGet`/`IndexSet` expects `indices[0]` = ROW,
                // `indices[1]` = COLUMN -- so the two must be SWAPPED here.
                // See this file's module doc comment's own dedicated
                // section for the full, verified derivation.
                let col_arg = self.lower_one_subscript(subs[0], ctx, depth + 1)?;
                let row_arg = self.lower_one_subscript(subs[1], ctx, depth + 1)?;
                Ok(vec![row_arg, col_arg])
            }
            n => Err(self.err_at(
                subscript_list,
                format!("{n}-D subscripting is not supported in this cut (only 1-D and 2-D)"),
            )),
        }
    }

    /// `subscript = STAR | range_subscript | expr`. No index-base shift at
    /// all (IDL is already 0-based, MA12 §2) -- see this file's module doc
    /// comment's own dedicated "Subscripting" section for why this is
    /// simpler than every other array-family frontend's own `-1` shift, and
    /// for exactly why negative-from-end and wildcard-range-end are
    /// rejected below rather than silently mis-lowered.
    fn lower_one_subscript(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<IndexArg, IdlLowerError> {
        if let Some(tok) = node.token() {
            if tok.effective_type_name() == "STAR" {
                return Ok(IndexArg::Whole);
            }
        }
        match node.children.first() {
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "range_subscript" => {
                self.lower_range_subscript(n, ctx, depth)
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "expr" => {
                self.reject_negative_literal_subscript(n)?;
                let e = self.lower_expr(n, ctx, depth)?;
                Ok(IndexArg::Scalar(Box::new(e)))
            }
            _ => Err(self.err_at(node, "malformed subscript".to_string())),
        }
    }

    /// `range_subscript = expr COLON range_end [COLON expr]`,
    /// `range_end = STAR | expr`. Maps directly onto `IndexArg::Range`
    /// wrapping an `Expr::Range` -- see this file's module doc comment for
    /// why this needs no adjustment (IDL's `[s0:s1]` is already inclusive of
    /// both endpoints, matching `Expr::Range`'s own existing runtime
    /// behavior exactly) and why a wildcard `range_end` (`a[s0:*]`) is
    /// rejected (needs the axis's runtime length, the same unresolved gap
    /// as negative-from-end).
    fn lower_range_subscript(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut RoutineCtx,
        depth: usize,
    ) -> Result<IndexArg, IdlLowerError> {
        let span = self.span_of(node);
        let exprs: Vec<&GrammarASTNode> = child_nodes(node)
            .into_iter()
            .filter(|n| n.rule_name == "expr")
            .collect();
        if exprs.is_empty() {
            return Err(self.err_at(node, "malformed range_subscript".to_string()));
        }
        self.reject_negative_literal_subscript(exprs[0])?;
        let start = self.lower_expr(exprs[0], ctx, depth)?;

        let range_end = find_child(node, "range_end")?;
        let stop = match range_end.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "STAR" => {
                return Err(self.err_at(
                    range_end,
                    "unsupported: a wildcard range-end subscript (`a[s0:*]`) is out of scope \
                     for v0.1.0 -- see this crate's README, \"Subscripting\", for why (it needs \
                     the axis's runtime length, which no existing SIR/backend primitive resolves \
                     soundly for a general, non-literal target)"
                        .to_string(),
                ))
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "expr" => {
                self.reject_negative_literal_subscript(n)?;
                self.lower_expr(n, ctx, depth)?
            }
            _ => return Err(self.err_at(range_end, "malformed range_end".to_string())),
        };

        self.observed.add(Feature::NDArrays);
        let step = if exprs.len() >= 2 {
            self.reject_negative_literal_subscript(exprs[1])?;
            Some(Box::new(self.lower_expr(exprs[1], ctx, depth)?))
        } else {
            None
        };

        Ok(IndexArg::Range(Box::new(Expr::Range {
            start: Box::new(start),
            step,
            stop: Box::new(stop),
            span,
        })))
    }

    /// Reject the SYNTACTICALLY obvious negative-from-end subscript shape
    /// (a bare unary `-` immediately applied to a `NUMBER` literal, e.g.
    /// `a[-1]`) -- see this file's module doc comment's "Subscripting"
    /// section. A general expression that merely *might* evaluate negative
    /// at runtime (a variable, a computed value) is NOT rejected here: this
    /// frontend has no type/value inference to catch that case, and it
    /// fails loud (a runtime "out of bounds" exception), not silently, if
    /// it ever actually happens -- a disclosed residual limitation, not a
    /// gap this check is trying to fully close.
    fn reject_negative_literal_subscript(
        &self,
        expr_node: &GrammarASTNode,
    ) -> Result<(), IdlLowerError> {
        if let Some(unary) = peel_to_named(expr_node, "unary") {
            if let [ASTNodeOrToken::Token(op), ASTNodeOrToken::Node(_)] = unary.children.as_slice()
            {
                if op.effective_type_name() == "MINUS" {
                    return Err(self.err_at(
                        expr_node,
                        "unsupported: a negative-from-end subscript (e.g. `a[-1]`) is out of \
                         scope for v0.1.0 -- see this crate's README, \"Subscripting\", for why \
                         (it needs the axis's runtime length, which no existing SIR/backend \
                         primitive resolves soundly for a general, non-literal target)"
                            .to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    // -------------------------------------------------------------------
    // small helpers
    // -------------------------------------------------------------------

    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::point(
            FILE,
            node.start_line.unwrap_or(1),
            node.start_column.unwrap_or(1),
        )
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> IdlLowerError {
        IdlLowerError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }

    /// Err if `e` is a direct string literal -- mirrors
    /// `scilab_to_semantic_ir::reject_string_operand` exactly (see that
    /// function's own doc comment for the full rationale): a syntactic,
    /// non-evaluating check that catches the direct, obvious case (a
    /// literal string reaching an arithmetic/ordering operator) but not a
    /// variable merely known by the programmer to hold a string -- a
    /// disclosed limitation, not a full fix.
    fn reject_string_operand(
        &self,
        e: &Expr,
        node: &GrammarASTNode,
        op_desc: &str,
    ) -> Result<(), IdlLowerError> {
        if matches!(e, Expr::StrLit { .. }) {
            return Err(self.err_at(
                node,
                format!(
                    "unsupported: {op_desc} is not implemented over string operands in this cut \
                     (MA12 §2 scopes `IdlValue::Str` to assignment/display/PRINT/equality/\
                     keyword-value only)"
                ),
            ));
        }
        Ok(())
    }

    /// A `NUMBER` lexeme is a float if it has a decimal point or exponent,
    /// otherwise an int -- mirrors
    /// `scilab_to_semantic_ir::number_literal_expr` exactly.
    fn number_literal_expr(&mut self, tok: &Token, span: &Span) -> Expr {
        let text = &tok.value;
        if text.contains('.') || text.contains('e') || text.contains('E') {
            self.observed.add(Feature::Floats);
            Expr::FloatLit {
                value: text.parse::<f64>().unwrap_or(0.0),
                span: span.clone(),
            }
        } else {
            match text.parse::<i64>() {
                Ok(v) => Expr::IntLit {
                    value: v,
                    span: span.clone(),
                },
                Err(_) => {
                    self.observed.add(Feature::Floats);
                    Expr::FloatLit {
                        value: text.parse::<f64>().unwrap_or(0.0),
                        span: span.clone(),
                    }
                }
            }
        }
    }

    /// Reject a same-precedence operator chain with more than
    /// `MAX_EXPR_DEPTH` operands -- mirrors
    /// `scilab_to_semantic_ir::check_chain_length` exactly (see that
    /// function's own extensive doc comment for the full DoS rationale):
    /// `idl.grammar`'s own `{ }`-repetition tiers (`logical`/`comparison`/
    /// `additive`/`multiplicative`/`power`) collapse a flat run of
    /// `AND`/`EQ`/`+`/`*`/`^`/... into ONE CST node with many children, so
    /// parsing itself costs no native stack for a long flat chain -- but
    /// this file's own fold loops (`lower_additive` et al.) still build a
    /// genuinely N-deep NESTED `Expr` tree (each fold step boxes the
    /// previous accumulator), and DROPPING that tree later (when the
    /// `Module` goes out of scope) recurses just as deeply as building it
    /// by naive recursion would have -- an uncatchable native stack
    /// overflow for a large enough flat chain, reachable via ordinary,
    /// two-line IDL source. Checked once, up front, before any folding
    /// begins.
    fn check_chain_length(&self, node: &GrammarASTNode) -> Result<(), IdlLowerError> {
        let operand_count = node
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(_)))
            .count();
        if operand_count > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!(
                    "expression chain too long ({operand_count} operands, exceeds \
                     {MAX_EXPR_DEPTH})"
                ),
            ));
        }
        Ok(())
    }
}

/// One declared `PRO`/`FUNCTION` parameter -- mirrors
/// `idl_runtime::eval::ParamSpec` exactly (see that struct's own doc
/// comment for the full `KEYWORD=local` rationale).
struct ParamSpec {
    keyword: Option<String>,
    local: String,
}

/// [`Lowerer::lower_call_args`]'s own return shape: (positional
/// expressions, keyword-name -> lowered-value pairs), both in written
/// order. Named to keep clippy's `type_complexity` lint quiet and to give
/// the shape a documented home rather than repeating the raw tuple type at
/// every call site.
type CallArgParts = (Vec<Expr>, Vec<(String, Expr)>);

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

/// Uppercase-fold one identifier -- see this file's module doc comment,
/// "Case folding". Mirrors `idl_runtime::eval::fold_case` exactly
/// (`str::to_uppercase`).
fn fold_case(s: &str) -> String {
    s.to_uppercase()
}

/// Collect the *node* children of `node` (dropping tokens).
fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// The sole node child of `node`, erroring via the lowerer's own `err_at`
/// if there is not exactly one.
fn only_node<'a>(
    node: &'a GrammarASTNode,
    lowerer: &Lowerer,
) -> Result<&'a GrammarASTNode, IdlLowerError> {
    match child_nodes(node).as_slice() {
        [only] => Ok(*only),
        _ => Err(lowerer.err_at(node, format!("malformed `{}` node", node.rule_name))),
    }
}

fn find_child<'a>(
    node: &'a GrammarASTNode,
    rule_name: &str,
) -> Result<&'a GrammarASTNode, IdlLowerError> {
    find_child_opt(node, rule_name).ok_or_else(|| IdlLowerError {
        message: format!(
            "malformed `{}` node (missing `{rule_name}`)",
            node.rule_name
        ),
        line: node.start_line.unwrap_or(1),
        column: node.start_column.unwrap_or(1),
    })
}

fn find_child_opt<'a>(node: &'a GrammarASTNode, rule_name: &str) -> Option<&'a GrammarASTNode> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == rule_name => Some(n),
        _ => None,
    })
}

/// Peel through a chain of single-Node-child wrapper rules until reaching a
/// node named `name`, or `None` if the chain branches or runs out first --
/// mirrors `scilab_to_semantic_ir::peel_to_named`'s identical helper,
/// bounded by [`MAX_EXPR_DEPTH`] for the same reason.
fn peel_to_named<'a>(node: &'a GrammarASTNode, name: &str) -> Option<&'a GrammarASTNode> {
    fn go<'a>(node: &'a GrammarASTNode, name: &str, depth: usize) -> Option<&'a GrammarASTNode> {
        if depth > MAX_EXPR_DEPTH {
            return None;
        }
        if node.rule_name == name {
            return Some(node);
        }
        match child_nodes(node).as_slice() {
            [only] if node.children.len() == 1 => go(only, name, depth + 1),
            _ => None,
        }
    }
    go(node, name, 0)
}

/// If `node` (an `expr`, at any point in the precedence cascade) is
/// SYNTACTICALLY a literal, non-decimal integer constant -- a bare
/// `primary` `NUMBER` token, optionally wrapped in exactly one unary `-`
/// (`unary = MINUS unary | multiplicative`) -- return its value. Used only
/// by [`Lowerer::lower_for`] to decide a FOR-loop's step sign at lowering
/// time (see that function's own doc comment for why a non-literal step is
/// out of scope). Peels through the ordinary single-Node-child wrapper
/// chain (`expr -> logical -> comparison -> additive -> unary ->
/// multiplicative -> power -> postfix -> primary`) the same way
/// [`peel_to_named`] does, since a bare literal descends through every
/// tier with no sibling operator present.
fn literal_int_value(node: &GrammarASTNode) -> Option<i64> {
    fn go(node: &GrammarASTNode, depth: usize) -> Option<i64> {
        if depth > MAX_EXPR_DEPTH {
            return None;
        }
        if node.rule_name == "unary" {
            if let [ASTNodeOrToken::Token(op), ASTNodeOrToken::Node(inner)] =
                node.children.as_slice()
            {
                if op.effective_type_name() == "MINUS" {
                    return go(inner, depth + 1).map(|v| -v);
                }
                return None;
            }
        }
        if let Some(tok) = node.token() {
            if tok.effective_type_name() == "NUMBER" && !tok.value.contains(['.', 'e', 'E']) {
                return tok.value.parse::<i64>().ok();
            }
            return None;
        }
        match child_nodes(node).as_slice() {
            [only] if node.children.len() == 1 => go(only, depth + 1),
            _ => None,
        }
    }
    go(node, 0)
}

/// If `node` (a `primary`) is a bare `NAME` token, return its raw
/// (un-folded) source spelling -- mirrors
/// `idl_runtime::eval::bare_name_primary` exactly.
fn bare_name_primary(node: &GrammarASTNode) -> Option<&str> {
    match node.children.first() {
        Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => {
            Some(t.value.as_str())
        }
        _ => None,
    }
}

/// Split a left-associative chain rule's children (`logical`/`comparison`/
/// `additive`/`multiplicative`/`power`) into the first operand and a list of
/// (operator token, next operand) pairs -- mirrors
/// `idl_runtime::eval::chain_parts` exactly.
fn chain_parts(node: &GrammarASTNode) -> (&GrammarASTNode, Vec<(&Token, &GrammarASTNode)>) {
    let mut first: Option<&GrammarASTNode> = None;
    let mut rest = Vec::new();
    let mut pending_op: Option<&Token> = None;
    for c in &node.children {
        match c {
            ASTNodeOrToken::Node(n) => {
                if first.is_none() {
                    first = Some(n);
                } else {
                    let op = pending_op
                        .take()
                        .expect("an operator token precedes every operand after the first");
                    rest.push((op, n));
                }
            }
            ASTNodeOrToken::Token(t) => pending_op = Some(t),
        }
    }
    (
        first.expect("a chain rule always has at least one operand"),
        rest,
    )
}

/// A conditional/loop body has two forms (MA12 §3): a single `statement`
/// with no closer, or a `BEGIN block_body ENDxxx|END` block -- used
/// uniformly for `then_branch`/`else_branch`/`for_body`/`while_body`/
/// `repeat_body`, mirroring `idl_runtime::eval::body_statements` exactly.
fn body_statements<'a>(
    node: &'a GrammarASTNode,
    lowerer: &Lowerer,
) -> Result<Vec<&'a GrammarASTNode>, IdlLowerError> {
    let nodes = child_nodes(node);
    if nodes.len() == 1 && nodes[0].rule_name == "statement" {
        Ok(vec![nodes[0]])
    } else if let Some(bb) = nodes.iter().find(|n| n.rule_name == "block_body") {
        Ok(block_body_statements(bb))
    } else {
        Err(lowerer.err_at(node, format!("malformed `{}` node", node.rule_name)))
    }
}

/// `statement_line = statement {STMT_SEP statement} [NEWLINE] | NEWLINE` --
/// every `statement` child (a blank line yields none).
fn statement_line_statements(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    child_nodes(node)
        .into_iter()
        .filter(|n| n.rule_name == "statement")
        .collect()
}

/// `block_body = {statement_line}` -- flattens every contained
/// `statement_line` into its own `statement` children, in order.
fn block_body_statements(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    child_nodes(node)
        .into_iter()
        .flat_map(statement_line_statements)
        .collect()
}

/// Coerce an already-lowered IDL expression to genuine IDL truthiness at
/// the point it reaches a boolean context (an `IF`/`WHILE`/`REPEAT...UNTIL`
/// condition). IDL's own truthiness rule for numeric values is the same
/// "nonzero is true" rule every array-family frontend in this repo already
/// shares (confirmed directly: `idl_runtime::eval::eval_condition` is
/// `self.eval_scalar(node)? != 0.0`) -- so this reuses the exact same
/// runtime intrinsic `scilab_to_semantic_ir::to_scilab_condition` already
/// established, `BuiltinCall("matlab_truthy", [expr])`, rather than
/// inventing a same-shaped `"idl_truthy"` builtin the shared JS backend
/// would need a new, currently-nonexistent implementation for. This is the
/// same well-known "a `BuiltinCall` name is a generic operation any backend
/// implements polymorphically" cross-frontend reuse `apl-to-semantic-ir`/
/// `scilab-to-semantic-ir` both already document for this exact name.
fn to_idl_condition(expr: Expr) -> Expr {
    let span = expr.span().clone();
    Expr::BuiltinCall {
        name: "matlab_truthy".to_string(),
        args: vec![expr],
        effects: EffectSet::PURE,
        span,
    }
}

/// Is `e` provably a scalar? Mirrors
/// `scilab_to_semantic_ir::expr_is_known_scalar` exactly (see this file's
/// module doc comment's own cross-references for the full rationale): an
/// operand is "known scalar" iff it is a bare `IntLit`/`FloatLit`, or a
/// `BuiltinCall` of `+ - * / neg` whose own arguments are (transitively)
/// known-scalar. Falling through to the array-domain node in an ambiguous
/// case is always semantically safe, just occasionally more conservative
/// than a full type-inference pass would be.
fn expr_is_known_scalar(e: &Expr) -> bool {
    fn go(e: &Expr, depth: usize) -> bool {
        if depth > MAX_EXPR_DEPTH {
            return false;
        }
        match e {
            Expr::IntLit { .. } | Expr::FloatLit { .. } => true,
            Expr::BuiltinCall { name, args, .. }
                if matches!(name.as_str(), "+" | "-" | "*" | "/" | "neg") =>
            {
                args.iter().all(|a| go(a, depth + 1))
            }
            _ => false,
        }
    }
    go(e, 0)
}

fn empty_block(span: Span) -> Block {
    Block {
        stmts: vec![],
        value: Expr::NilLit { span: span.clone() },
        span,
    }
}

/// Assemble a list of lowered items into a `Block` whose every item is a
/// statement (bare expressions wrapped as `ExprStmt`) and whose value is
/// always `value`.
fn assemble_stmts_only(items: Vec<Lowered>, value: Expr, span: Span) -> Block {
    let stmts: Vec<Stmt> = items
        .into_iter()
        .map(|item| match item {
            Lowered::Stmt(s) => *s,
            Lowered::Expr(expr) => {
                let s = expr.span().clone();
                Stmt::ExprStmt { expr, span: s }
            }
        })
        .collect();
    Block { stmts, value, span }
}
