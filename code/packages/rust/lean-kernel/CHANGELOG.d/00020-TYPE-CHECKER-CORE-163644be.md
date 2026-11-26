## [0.2.0] - 2026-11-24

### Added

- KRN-002 slice 2: the core of the type checker (VER11a section 4.2 rows "Type inference and checking" and the core of "Reduction and definitional equality").
- `Environment` (`env.rs`): append-only map of `Declaration`s (axiom, definition with `ReducibilityHints`, theorem, opaque) owning the `ExprStore`. `add_declaration` checks, in order: fresh name, distinct universe parameters, closed type and value, declared universe parameters, type is a sort, value type def-eq declared type, theorem type is a `Prop`, only already-declared constants referenced. On error the environment is unchanged.
- `LocalContext` (`lctx.rs`): free variable to (name, type, optional let-value); ids are never reused, which keeps the checker's caches sound.
- `TypeChecker` (`checker.rs`, `infer.rs`): `infer`, `check`, `ensure_sort`, `ensure_pi`, one case per expression kind; no infer-only fast path. `Proj` is `KernelError::Unsupported`. A `let`'s inferred type keeps the `let` binder only when the variable occurs in the type (as Lean 4's kernel does). Literals (`Nat`, `String`) are `Unsupported`: typing them by the *name* `Nat` is unsound (`def Nat : Prop := False` then `theorem bad : False := 5` was accepted in an earlier draft), so they wait for primitive-definition validation (VER11a section 3.3) in slice 3.
- `whnf` / `whnf_core` (`whnf.rs`): beta, zeta (let and let-bound fvars) and delta for definitions only (never theorems, opaques or axioms).
- `is_def_eq` (`defeq.rs`): identical handle, verdict cache, `Sort` by level equivalence, constants, binders under one shared fresh variable, application congruence, then whnf and retry once.
- `KernelError`: one error enum for every rejection, with no reachable panic.
- Resource `Limits` (call depth 200, term depth 1000, 2M steps, 4M new nodes) reported as `KernelError::ResourceLimit`; telescopes and spines are loops, and term depth is measured with an explicit stack.
- Tests: positive environment and inference cases, one negative test per error variant, 100k-deep terms on a 512 KiB stack, an exponential delta chain, "False from nothing" and level-confusion regressions, and line-budget gates for the three VER11a section 4.2 rows in `tests/ceiling.rs`.

### Fixed (hardening found in review of this slice)

- Universe levels are bounded: every level in a `Sort` or constant must be at most `MAX_LEVEL_DEPTH` (128) deep and `MAX_LEVEL_SIZE` (512) nodes as a tree, enforced in the checker's `bounded` gate and `check_level` (`ResourceLimit`). Levels cache `depth` and `size`.
- `Level::normalize` no longer panics or overflows on huge offsets (`succ^70000 (max u v)`): it computes with checked arithmetic, and a level outside the limits is returned unchanged, so `is_equiv` / `is_geq` only claim the trivial structural case there.
- Level equality, ordering, `instantiate`, `is_never_zero` and `Display` are iterative (with visited sets for shared nodes), so a million-deep level or a DAG with 2^100 paths cannot overflow the stack or hang, even inside `ExprStore` hash-consing.
- `ExprStore::set_node_cap` / `budget_exceeded`: a single `instantiate` / `abstract` traversal stops once the checker's node budget is exceeded, and the checker turns that into `ResourceLimit` at its next step. The cap is lifted when the `TypeChecker` is dropped.
- `ExprStore::owns` and `KernelError::ForeignExpr`: a handle from another store is refused by `add_declaration`, `infer`, `whnf`, `is_def_eq`, `ensure_sort` and `ensure_pi` instead of panicking.

### Not yet (slice 3)

- Lazy delta with hints, eta, proof irrelevance, iota, projections, quotients, literal acceleration, inductive types. All make the checker reject more, never accept more.
