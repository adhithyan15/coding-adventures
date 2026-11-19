# VER09b — Native Go CDCL SAT Solver and Tseitin Bit-Blaster

**Status:** Accepted contract; implementation is being delivered in
`code/packages/go/sat-solver`.
**Builds on:** [VER09](VER09-cdcl-sat.md), [VER09a](VER09a-typescript-sat-solver.md).
**Related identities:** TypeScript `@coding-adventures/sat-solver` and Rust
`cdcl-sat` are distinct packages. This Go package ports the dedicated
TypeScript package contract; it is not the optional parser SAT miter and does
not implement DRAT proof production or checking.

## 1. Purpose and boundary

Provide a dependency-free Go implementation of the dedicated TypeScript
`sat-solver` package's public behavior: CDCL Boolean solving, incremental
clauses and assumptions, failed-assumption cores, Tseitin gates, bounded
bit-vector helpers, and DIMACS transport. Package presence alone is not
behavioral parity. The acceptance suite therefore consumes the shared
language-neutral corpus and checks public behavior against independent
oracles.

The solver answers SAT, UNSAT, or UNKNOWN. SAT models are copied assignments
that must satisfy the original clauses and current assumptions. UNSAT is an answer
from the solver's Boolean engine; this package does not emit a certificate, so
it does not claim independently checkable proof evidence. A bounded search
that exhausts its conflict or propagation budget returns UNKNOWN.

## 2. Public contract

The package exposes typed `Var` and packed `Lit` values; `LBool` values
`Undef`, `True`, and `False`; checked literal construction, negation,
variable/sign access, and DIMACS rendering; `NewSolver`; `SolverOptions` with
per-call `MaxConflicts` and `MaxSteps`; and `SolveResult` values SAT, UNSAT,
and UNKNOWN. `Solver` supports `NewVar`, `AddClause`, `Solve`,
`SolveWithAssumptions`, `Propagate`, `Model`, `FailedCore`, `VarValue`,
`Value`, `NVars`, `NClauses`, cumulative `Conflicts`, `Decisions`,
`Propagations`, and `Restarts`, `Okay`, and reset-to-root while preserving the
formula. `CnfBuilder` can wrap a supplied solver and exposes it through
`GetSolver`.

The Go signatures are `NewSolver(config ...SolverConfig) (*Solver, error)`,
`NewVar() (Var, error)`, `AddClause([]Lit) (bool, error)`,
`Solve(options ...SolverOptions) (SolveResult, error)`, and
`SolveWithAssumptions([]Lit, options ...SolverOptions) (SolveResult, error)`.
`Model() ([]bool, bool)` returns a copied valid model and a validity flag;
`FailedCore() []Lit` returns a copy. `Value(Lit) (LBool, error)` rejects a
malformed packed encoding; `VarValue(Var) LBool` returns `Undef` for an
unallocated but representable variable. `Var` and `Lit` use `uint32`; `LBool`
uses `int8`. `SolverConfig` contains `MaxVars`, `MaxClauses`,
`MaxClauseLiterals`, and `MaxStoredLiterals` ceilings. `SolverOptions` stores
optional `*uint64` `MaxConflicts` and `MaxSteps` limits. `CnfFormula` contains
`NumVars` and clauses; `ParseDimacs(text, options...)` and
`WriteDimacs(formula)` return results with errors. Builder construction,
clause insertion, and every gate operation return errors. Gate methods are
`NewVar`, `NewLit`, `AddClause`, `TrueLit`, `FalseLit`, `And`, `Or`, `Xor`,
`Not`, `Ite`, `Equiv`, `FullAdder`, `BvEq`, and `BvAdd`.

Invalid public inputs return errors rather than panicking. `MakeLit` returns
`(Lit, error)`; `NegateLit` and `LitToString` return `(result, error)` for
malformed raw encodings; clause and assumption entry points return errors.
Clauses, assumptions, and builder operands are fully validated before any
formula or solver state changes. Builder gate operations preflight their
variable/clause/literal quota and are failure-atomic. An out-of-range but
well-formed variable query returns `Undef`, as in TypeScript. `NewVar` and
`AddClause` check an independent configurable resource ceiling before growing
state: defaults are at most `1<<20` variables, `1<<20` original clauses,
`1<<24` total original clause literals, and `1<<24` total stored literals
including learned clauses. Configured ceilings may lower these values but
cannot raise representability limits. `Solve()` with omitted options is
unlimited except for state ceilings. `SolverOptions` uses optional `*uint64`
budget fields: nil is unlimited and a pointer to zero means zero work; setting
one limit leaves the other unlimited. At most one options value is accepted.

`NVars` counts allocated variables. `NClauses` follows the TypeScript arena:
it includes stored non-unit original and learned clauses, excludes root
unit assignments, and reflects learned clauses removed by reduction.
`Propagate` returns `-1` for no conflict, `-2` for step-budget exhaustion, or
a nonnegative conflicting clause arena index. `Value` and `VarValue` return
`Undef` for valid but not-yet-allocated queries.

Literal encoding matches the dedicated TypeScript package:
`lit = 2*var + sign`, where sign 1 is negated; negation toggles the low bit.
The Go implementation checks values before indexing or growing solver state
and rejects encodings outside the representable TypeScript-compatible range
(variable indices `0..2^30-1`, packed literals `0..2^31-1`). It must not rely on
integer wraparound. A separate conservative allocation ceiling bounds actual
variable and clause storage; representability is not permission to allocate
billions of entries. TypeScript does not enforce the encoding upper bound at
every public entry point; this Go-side validation is an explicit hardening
difference.

`CnfBuilder` exposes the same operations as the dedicated TypeScript package:
new variables/literals, clause addition, true/false literals, AND, OR, XOR,
NOT, ITE, equivalence, full adder, bit-vector equality, and ripple-carry
addition. Gate operations return `(Lit, error)`; full adder returns sum,
carry, and error. Bit vectors are little-endian. Addition returns the low `n`
sum bits and discards final carry; unequal widths are refused. Empty vectors
compare equal and add to an empty vector. NOT returns the negated input
literal. Tseitin outputs are constrained in both directions by the generated
CNF.

## 3. Solver behavior

Use two-watched-literal propagation, first-UIP conflict analysis, non-
chronological backjumping, VSIDS-style variable activity, phase saving, Luby
restarts, and learned-clause reduction. Keep assignments and propagation
state bounded by allocated variables and clauses. Clauses may be added between
solve calls; adding a clause must preserve it even after a previous SAT,
UNSAT-under-assumptions, or UNKNOWN result.

Each solve call starts from a clean level-zero search state, applies only that
call's assumptions, and clears the failed core and saved model before any
early return. After SAT, the failed core is empty; after SAT or UNKNOWN, no
failed core remains from an older call. Adding a clause or resetting also
invalidates saved model and core results. A SAT model is a copy valid until a
clause is added, reset is called, or another solve starts. `Model` reports no valid model after UNKNOWN
or UNSAT. Core/model slices are caller-owned snapshots. Budget counters are
cumulative `uint64` statistics, but limits apply to work in that call and
relative-budget arithmetic cannot overflow. UNKNOWN leaves no assumption
assignments that can affect a later solve; a later unbounded solve must agree
with a fresh solver on the same formula. A known root-level contradiction
returns UNSAT and an empty failed core. Do not run a hidden unbounded solve to
classify a base formula.

For UNSAT under assumptions, `FailedCore` is a duplicate-free subset of the
assumptions whose conjunction with the permanent formula is UNSAT. It is
sound but is not guaranteed minimal, ordered, or unique. Core values are
returned as a copy. A known root contradiction has an empty core; if the base
formula is globally UNSAT but not yet established at root, a sound nonempty
subset may be returned. This matches the observed TypeScript traversal:
VER09a's phrase “minimal subset” is stronger than the implementation and is
corrected here. TypeScript's already-known-root-UNSAT fast return also leaves
the previous failed core untouched; record this as a distinct upstream bug,
not as intended behavior.

## 4. DIMACS transport and shared corpus

`ParseDimacs` reads a single `p cnf <variables> <clauses>` header, comments
whose first non-space character is `c`, CRLF or LF, clauses spanning lines,
multiple clauses on one line, empty clauses, and the SATLIB standalone `%`
terminator (which ignores later text). Every clause ends at literal zero; the
parsed clause count must equal the header. Literal indices must be in
`1..variables`; zero ends a clause, including the conventional `-0` token.
The default declared-variable ceiling is `1<<20`, overrideable by an explicit
option. Parsing must avoid allocating directly from an attacker-controlled
clause count and must not inherit a small undocumented line-length limit.
Header token count and spelling are strict; trailing header fields are
rejected. `WriteDimacs` emits canonical `p cnf` text with one clause per line,
including empty clauses (an empty clause is serialized as ` 0` for parity
with the dedicated TypeScript writer; output has no trailing newline).

The shared valid corpus is
[`fixtures/sat-cnf-v1/cases.json`](fixtures/sat-cnf-v1/cases.json), SHA-256
`71ac009c5c65e712dfacce519d009809431663b603fbdcd52d6e51f2ff3e032d`. Every
case has a unique fixed ID, valid DIMACS bytes, and ordered public operations
(`solve`, `add_clause`, or `reset`). Every solve step declares its expected
result; SAT always triggers independent model replay, and assumption-UNSAT
always triggers core subset and brute-force soundness checks. No fixture flag
can disable these checks. The corpus includes CRLF/comments, clauses spanning
lines and multiple clauses per line, empty clauses, standalone `%`, `-0`,
duplicate/contradictory/irrelevant assumptions, clause addition, reset,
zero-budget UNKNOWN, and recovery. Both Go and TypeScript BUILD tests consume
the same bytes and pin the digest and exact ID set. Register the fixture as a
source input for both package consumers so changes select both BUILD targets.
Malformed transport differences are tested separately and recorded as
per-port refusals. Follow-up reproductions track the TypeScript stale-core
fast-return and permissive header parser; this Go delivery does not expand to
repair those unrelated TypeScript paths.

## 5. Evidence and limits

Acceptance includes:

1. Unit tests for literals, all exported solver and builder operations,
   formula growth, reset, budgets, and hostile input validation.
2. Native Go fuzz targets for bounded DIMACS and clause/literal inputs, with
   seed corpora and hard allocation/variable caps.
3. Deterministic seeded property tests and at least 100 small random
   phase-transition 3-SAT formulas compared with an independent exhaustive
   oracle over the original clauses and assumptions.
4. SAT model replay against original input; sound failed-core replay; all
   gate/ITE truth tables; full-adder and multi-bit arithmetic checks; tiny
   pigeonhole UNSAT and satisfiable controls; and N-queens board validation.
   Add a same-harness gate or incremental-state mutant that admits a concrete
   counterexample; replay that assignment through the healthy public API and
   independently evaluate the original truth table/CNF.
5. The shared corpus run by both TypeScript and Go tests; Go package `BUILD`,
   `BUILD_windows`, metadata, README, and changelog.

The original VER09a criterion asks for at least 90% line and 85% branch
coverage. Go's built-in coverage reports statements, not those metrics. This
port reports measured statement coverage and retains the VER09a line/branch
criterion as an explicit unmet evidence gap unless a real line/branch tool is
used. Seeded Go fuzz target compilation/execution reports its duration and
case count; it is not an unbounded campaign. SAT miters and bounded exhaustive
checks remain required evidence; neither a Warrant corpus nor an UNSAT result
is a source proof. The Go proof-framework, Warrant source binding,
certificate-checking, and line/branch instrumentation prerequisites remain
separate backlog work with named acceptance gates. No whole-source formal
proof, DRAT certificate, minimal-core guarantee, or native/runtime authority
is claimed.
