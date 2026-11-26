## [0.3.0] - 2026-11-24

### Added

- KRN-002 slice 3a: inductive types, recursors, iota, K-like reduction, proof irrelevance and eta (VER11a section 4.2 rows "Inductive types" and the matching part of "Reduction and definitional equality"). The kernel can now check the first real proofs: Boolean truth tables, `Nat` arithmetic by computation, `Eq.symm` / `Eq.trans`, ex falso, and the input contracts the logic gates need.
- `Environment::add_inductive(InductiveDecl)` (`inductive.rs`): one non-mutual, non-nested inductive type with universe parameters, a parameter count, and constructors (name and type). Checks: new names (type, constructors, derived `T.rec`); the type is a syntactic Pi telescope ending in a `Sort` and does not mention itself; constructors share the type's parameter binders (def-eq) and return `T.{ls} params indices` exactly, with indices that do not mention `T`; every field's sort is at most the type's sort unless the type is in `Prop`; strict positivity (types in whnf, so a definition hiding a negative occurrence is caught). The type, constructors and recursor are added atomically; any error rolls everything back.
- The kernel derives `T.rec` (motive, minor premises with induction hypotheses, indices, major premise) and one computation rule per constructor. Large elimination follows Lean: never-`Prop` types eliminate anywhere; `Prop` types with zero constructors, or one constructor whose fields are all propositions or direct indices of the result, eliminate anywhere; the rest only into `Prop` (no motive universe parameter, so `Or.rec.{1}` is an arity error). The motive universe is named `u`, or `u_1`, ... on a clash. A `Prop` inductive with one constructor and no fields is flagged K-like.
- The derived recursor and rules are re-checked by the ordinary checker before they are installed (defence in depth against a generator bug).
- New declaration kinds `DeclKind::{Inductive, Constructor, Recursor}` with `InductiveInfo`, `ConstructorInfo`, `RecursorInfo`, `RecRule`. `add_declaration` refuses them (`Unsupported`): a forged recursor would be an axiom with computation rules.
- New errors `InvalidInductive`, `NonPositiveOccurrence`, `UniverseTooLarge`; `MAX_CONSTRUCTORS` (1024).
- Iota in `whnf`: `T.rec ... (c As bs) ~> rule`, requiring a saturated recursor with the right number of universe levels and a constructor application with exactly `params + fields` arguments. K-like reduction for `Eq`-style recursors: a non-constructor major whose type is def-eq to the nullary constructor's type is replaced by that constructor (types in whnf, head must be the inductive, indices unified by def-eq), as in Lean.
- Proof irrelevance in `is_def_eq`: two proofs (type's type is `Prop`) of def-eq propositions are equal. It only ever adds "yes" answers and is skipped for `Sort`/`Pi`/literals. Eta for functions: a lambda against a non-lambda of Pi type is compared with its eta-expansion. Terms the kernel cannot type (undeclared constants, literals, projections) make these rules not apply instead of erroring.
- Tests: Bool/Nat/Eq/And/Or/False/True/Option/Exists/List built by hand; `Bool.not/and/or` by `Bool.rec`; `and_comm` over all inputs; NAND-built XOR equals XOR; `Nat.add 2 2 = 4` by iota; `true_ne_false` and `zero_ne_succ` by `Eq.rec` into `Prop`; K-like reduction; one negative per inductive check; eleven unsound-acceptance regressions for iota, K, proof irrelevance, eta and large elimination; hostile blocks (thousands of constructors or fields, 100,000-deep telescopes, shared DAGs) on a 256 MiB stack that must return an error. `tests/ceiling.rs` counts `inductive.rs` against the 1,800-line row.

### Changed

- The slice-2 test `theorems_and_opaques_are_never_unfolded_to_make_a_declaration_check` now expects a theorem and its value to be def-eq (by proof irrelevance, not by unfolding); `whnf` still leaves the theorem constant alone.

### Not yet (slice 3b+), each failing closed

- Mutual and nested inductive types (`Unsupported`), structure projections and structure eta (`Proj` stays `Unsupported`), unit-like types, quotients, literals (until primitive validation), lazy delta with hints.
- A Pi telescope with more than about 128 binders overflows the universe-level depth limit (the nested `imax` chain of `infer_pi`), so an inductive with more than about 120 constructors is a `ResourceLimit` today.
