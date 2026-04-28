# LP18 - Logic Prolog Functionality Batches

## Overview

LP00 through LP17 established the Python logic-programming library foundation:

- symbolic terms, variables, unification, substitutions, and reification
- depth-first solving with facts, rules, conjunction, disjunction, and
  disequality
- relational list and sequence helpers
- high-level instruction, VM, bytecode, and bytecode-VM layers
- control, arithmetic, collection, term, persistent-database, and
  clause-introspection builtins

That is enough to write meaningful Prolog-shaped programs from Python, but it
is not yet at "full Prolog-level functionality" without syntax. The remaining
gaps are large enough that implementing each as a tiny PR would create too much
coordination overhead, while implementing all of them in one PR would mix
unrelated risk classes.

LP18 defines the batching plan for the next layer. The goal is a small number
of substantial implementation PRs, each with a coherent semantic boundary and
end-to-end examples.

## Design Goal

Move from many small feature PRs to a few larger functionality batches:

- batch pure, library-level metaprogramming features together
- batch stateful database features together
- batch solver-control protocol changes together
- keep finite-domain constraint solving separate because it introduces a new
  constraint domain and propagation model

The parser remains out of scope for these batches. Every feature should be
usable directly from Python first, so the future Prolog parser can compile onto
the same engine rather than inventing another runtime.

## Current State

Already specified and implemented:

```text
LP00 logic core
LP01 logic engine
LP02 disequality constraints
LP03 relational stdlib
LP04 relational combinators
LP05 structural list relations
LP06 sequence relations
LP07 logic instructions
LP08 logic VM
LP09 logic bytecode
LP10 logic bytecode VM
LP11 control and type builtins
LP12 arithmetic builtins
LP13 collection builtins
LP14 advanced control builtins
LP15 term metaprogramming builtins
LP16 persistent clause database
LP17 clause introspection builtins
LP18 Batch A metaprogramming completion
LP18 Batch B dynamic runtime database
LP18 Batch C search control and real cut
LP18 Batch D1 CLP(FD) foundation
LP18 Batch D2/D3 CLP(FD) arithmetic and all-different
LP18 Batch D4 CLP(FD) examples and labeling ergonomics
LP18 Batch D5 CLP(FD) n-ary sum ergonomics
PR00 Prolog lexer
```

Known future-extension items that still need implementation:

- soft-cut variants if a concrete API need emerges
- advanced CLP(FD) propagation such as division, reified constraints, and
  Hall-set global constraint pruning

## Batch Plan

### Batch A - Metaprogramming Completion

Batch A should finish the pure Prolog metaprogramming surface that does not
require mutable solver state or choicepoint pruning.

Packages:

```text
code/packages/python/logic-engine
code/packages/python/logic-builtins
```

Features:

- callable goal-term lowering
- `calltermo(term_goal)` for executing reified goals
- standard term comparison helpers
- predicate enumeration and predicate-property inspection
- examples proving that `clauseo` output can be executed again

Why this is one batch:

- all features operate on immutable terms, clauses, and program metadata
- no feature requires database rollback or cut semantics
- `clauseo` from LP17 becomes immediately more useful when its returned bodies
  can be passed to `calltermo`

Expected public API additions:

```python
goal_from_term(term)
calltermo(term_goal)
compare_termo(order, left, right)
termo_lto(left, right)
termo_leqo(left, right)
termo_gto(left, right)
termo_geqo(left, right)
current_predicateo(name, arity)
predicate_propertyo(name, arity, property_term)
```

Exact names can still be refined during implementation, but the batch should
ship the whole metaprogramming loop:

```python
clauseo(ancestor(X, Y), Body) & calltermo(Body)
```

### Batch B - Dynamic Runtime Database

Batch B should introduce Prolog-style runtime database mutation while preserving
the engine's backtracking guarantees.

Packages:

```text
code/packages/python/logic-core
code/packages/python/logic-engine
code/packages/python/logic-builtins
code/packages/python/logic-instructions
code/packages/python/logic-vm
```

Features:

- a `State.database` extension slot for branch-local runtime overlays
- dynamic predicate declarations
- scoped database state inside an active search
- `assertao(clause_term)`
- `assertzo(clause_term)`
- `retracto(clause_term)`
- `retractallo(head_term)`
- `abolisho(name, arity)`
- optional instruction/VM support for dynamic database operations when the
  engine surface is stable

Why this is one batch:

- all features depend on the same state overlay and rollback machinery
- shipping only `assert` without `retract`, or vice versa, would leave a
  half-usable database story
- dynamic predicate metadata belongs with runtime mutation because
  `predicate_propertyo` must be able to observe dynamic declarations

The core rule for this batch is that database mutations are visible to later
goals in the same proof branch and are undone when the solver backtracks past
the mutation point.

Implementation shape:

- immutable source `Program` values remain the persistent database layer
- dynamic predicates opt in through program declarations or a branch-local
  `dynamico(name, arity)` goal
- active runtime mutations live in an immutable `DynamicDatabase` attached to
  `State.database`, so ordinary generator backtracking restores previous
  database snapshots naturally
- static source predicates cannot be modified by runtime assert/retract/abolish
  unless they were declared dynamic at the program/instruction level
- `DYNAMIC_REL` instructions declare dynamic predicates for future parser and
  VM frontends without requiring parser-specific runtime behavior

### Batch C - Search Control and Real Cut

Batch C should add real Prolog search-control semantics.

Packages:

```text
code/packages/python/logic-engine
code/packages/python/logic-builtins
```

Features:

- scoped choicepoint frames in the solver protocol
- `cut()` in `logic-engine` and `cuto()` in `logic-builtins` as the library
  form of `!/0`
- cut encoding/lowering through `goal_as_term(...)` and `goal_from_term(...)`
  as the atom `!`
- examples that distinguish real cut from `onceo`

Why this is one batch:

- cut is not just another builtin; it changes how choicepoints are represented
  and pruned
- related control constructs should share the same solver protocol instead of
  layering fake behavior on top of normal goals
- keeping this separate from Batch B avoids mixing state rollback and
  choicepoint pruning in the same first implementation

Batch C should not fake cut by using `onceo`. LP14 explicitly deferred `cuto`
because a correct implementation needs scoped pruning of surrounding clause and
disjunction alternatives.

Implementation shape:

- the public solver still yields ordinary `State` objects, but the private
  solver protocol threads a cut flag alongside each state
- conjunctions propagate a cut signal to prune alternatives created by earlier
  goals in the same conjunction
- disjunctions stop exploring later branches after a branch raises cut
- relation calls act as cut frames: a cut inside a rule body prunes later body
  alternatives and later clauses for that predicate invocation, then the
  relation consumes the cut before returning to the caller
- cut succeeds once, so choices introduced after the cut remain backtrackable

Deliberately deferred:

- soft-cut remains out of scope until there is a concrete library API need
- `calltermo(...)` and other native-goal boundaries currently execute nested
  goals through the public solver API, so cut is scoped to the nested call
  rather than propagated through meta-call boundaries

### Batch D - CLP(FD) Foundation

Batch D should start finite-domain constraint logic programming, but it should
be treated as a larger track rather than one oversized PR.

Packages:

```text
code/packages/python/logic-core
code/packages/python/logic-engine
code/packages/python/logic-builtins
```

Track features:

- finite-domain variables and domain stores
- domain constraints such as membership, equality, disequality, ordering, and
  arithmetic relations
- propagation hooks integrated with the solver state
- `labelingo` for enumerating finite assignments
- practical constraints such as `all_differento`

Why this is not bundled with A, B, or C:

- it introduces a second constraint domain beyond ordinary unification and
  disequality
- propagation needs careful termination and consistency rules
- useful examples, like Sudoku or scheduling, need a broader surface than one
  or two predicates

CLP(FD) is being delivered in a few sub-batches:

- D1: finite domains, domain narrowing, ordering constraints, and labeling
- D2: arithmetic expression constraints
- D3: global constraints such as `all_differento`
- D4: real-world examples, solver ergonomics, and performance tuning
- D5: modeling ergonomics such as n-ary sums for allocation and scheduling

## Detailed Batch A Semantics

Batch A is the recommended next implementation PR.

### Callable Goal-Term Lowering

Add an engine helper that converts first-order terms back into executable goals.

Supported terms:

- atom `true` lowers to `succeed()`
- atom `fail` lowers to `fail()`
- compound `=(Left, Right)` lowers to equality
- compound `\=(Left, Right)` lowers to disequality
- compound `,(Left, Right)` lowers to conjunction
- compound `;(Left, Right)` lowers to disjunction
- any other compound lowers to a relation call with the same functor and arity

Unsupported terms should raise `TypeError` from host helpers and logically fail
from builtins when failure is the existing package convention.

The lowering must preserve logic variables embedded in the term. It must not
standardize them apart unless the caller explicitly asks for that behavior.

### `calltermo(term_goal)`

`calltermo` executes a reified goal term inside the current program and search
state.

Examples:

```python
calltermo(parent(homer, bart))
calltermo(Compound(",", (parent(X, Y), parent(Y, Z))))
calltermo(Atom("true"))
```

The builtin should allow this LP17 round trip:

```python
clauseo(ancestor(X, Y), Body) & calltermo(Body)
```

That example should enumerate `ancestor` clauses, expose their bodies as data,
and then execute those bodies as goals.

### Standard Term Ordering

Add deterministic term comparison compatible with a documented Prolog-inspired
standard order:

```text
variables < numbers < atoms < compounds
```

The Python library also has `String` terms. Batch A should treat strings as
atomic values ordered after atoms and before compounds, while keeping atoms as
the symbolic constants used by predicate names and Prolog-style functors.

For compounds:

1. compare arity
2. compare functor name
3. compare arguments left to right

Variables should compare by stable variable identity inside a single term
comparison call. The implementation does not need to promise that variable
ordering is stable across processes or unrelated queries.

Predicates:

```python
compare_termo(order, left, right)
termo_lto(left, right)
termo_leqo(left, right)
termo_gto(left, right)
termo_geqo(left, right)
```

`compare_termo` should unify `order` with one of:

```text
<
=
>
```

### Predicate Metadata

Expose source-program predicate metadata from inside logic queries.

Predicates:

```python
current_predicateo(name, arity)
predicate_propertyo(name, arity, property_term)
```

Initial properties:

- `defined`
- `static`
- `built_in`
- `number_of_clauses(N)`

Batch B can add:

- `dynamic`
- `multifile` if that concept becomes useful for parser-backed modules
- `exported` if modules become part of the Prolog layer

## Detailed Batch B Semantics

Batch B should add a database overlay to the active search context rather than
mutating the base `Program` in place.

Required behavior:

- `assertao` inserts before existing dynamic clauses for the target predicate
- `assertzo` appends after existing dynamic clauses for the target predicate
- `retracto` removes the first matching visible dynamic clause
- `retractallo` removes all matching visible dynamic clauses
- `abolisho` removes every visible dynamic clause for a predicate
- mutations are visible to later goals in the same branch
- mutations are rolled back when backtracking crosses the mutation point
- static predicates cannot be modified unless a future spec explicitly allows
  it

Batch B should include examples for:

- asserting a fact and querying it in the same proof
- asserting multiple facts and observing source order
- retracting one fact at a time
- proving that a failed branch does not leak database changes into sibling
  branches

## Detailed Batch C Semantics

Batch C should represent choicepoints explicitly enough for cut to prune:

- alternatives to the left of the cut in the current predicate invocation
- later clauses for the same predicate invocation
- disjunction alternatives in the cut scope

It should not prune:

- choicepoints outside the current cut scope
- choices created after the cut
- unrelated outer queries

Required examples:

- green cut that removes duplicate answers
- red cut that changes answer sets, documented as intentional Prolog behavior
- interaction with disjunction
- interaction with recursive predicates
- contrast with `onceo`

## Detailed Batch D Semantics

CLP(FD) should extend the solver state with a finite-domain store. Domain
narrowing should occur before enumeration whenever possible, and labeling
should be the explicit bridge from constraints to concrete answers.

Batch D is intentionally split into implementation slices. The foundation PR
should establish a branch-local finite-domain store and the integer relations
that are useful without arithmetic expression propagation.

Foundation predicates:

```python
fd_ino(var, domain)
fd_eqo(left, right)
fd_neqo(left, right)
fd_lto(left, right)
fd_leqo(left, right)
fd_gto(left, right)
fd_geqo(left, right)
labelingo(vars)
```

D2/D3 predicates:

```python
fd_addo(left, right, result)
fd_subo(left, right, result)
fd_mulo(left, right, result)
all_differento(vars)
```

D2/D3 should continue to prefer correctness and clear semantics over advanced
propagation performance.

Foundation implementation shape:

- `State` grows a branch-local `fd_store` extension slot alongside the dynamic
  database slot, and all solver state-copying helpers must preserve it.
- The FD store maps open logic variables to finite integer domain sets and
  keeps residual binary constraints for variables that are not labeled yet.
- FD predicates narrow domains immediately when possible. If both sides are
  concrete integers, they behave as deterministic checks. If one or both sides
  are open variables, they retain enough residual constraint state for later
  narrowing and labeling.
- `labelingo(vars)` is the bridge from constraints to concrete answers. It
  enumerates the current finite domains in deterministic ascending order,
  commits each assignment through normal unification, and re-checks the FD
  store after every binding.
- Domains in the Python API may be supplied as a `range`, an iterable of
  integers, a single integer, a numeric term, a proper logic list of integer
  terms, or a `..(Low, High)` compound term.

Explicit non-goals for the foundation PR:

- no arithmetic FD propagation yet (`fd_addo`, `fd_subo`, `fd_mulo`)
- no global constraints yet (`all_differento`)
- no infinite domains; every domain must be finite before labeling

Arithmetic and global-constraint implementation shape:

- finite-domain residual constraints use a generalized `operator + terms`
  representation instead of the D1 binary-only shape
- `fd_addo(left, right, result)` enforces `left + right = result`
- `fd_subo(left, right, result)` enforces `left - right = result`
- `fd_mulo(left, right, result)` enforces `left * right = result`
- arithmetic constraints prune every known finite-domain variable to values
  that participate in at least one satisfying tuple
- arithmetic constraints behave as deterministic checks when all terms are
  concrete integers
- `all_differento(vars)` accepts either a Python sequence of terms or a proper
  logic list and keeps the terms pairwise distinct
- `all_differento` should fail immediately on duplicate concrete assignments
  and should remove singleton assignments from neighboring finite domains

D2/D3 non-goals:

- no division or modulus CLP(FD) constraints yet
- no reified CLP(FD) constraints yet
- no Hall-set or matching-based `all_differento` pruning yet

D4 implementation shape:

- `labelingo(vars)` chooses the open finite-domain variable with the smallest
  currently-known domain first, while using the caller's variable order as a
  stable tie-breaker
- examples should solve recognizable problems directly through the Python
  library API, before any parser syntax exists
- examples should keep domains small and use `solve_n(...)` where a single
  representative solution is enough
- map coloring exercises binary disequality constraints
- Latin-square-style puzzles exercise `all_differento` across rows and columns
- task scheduling exercises arithmetic duration constraints plus ordering
  constraints

D4 non-goals:

- no parser integration
- no search strategy API beyond the default smallest-domain labeling heuristic
- no global-constraint algorithms beyond the current singleton pruning

D5 implementation shape:

- `fd_sumo(terms, total)` constrains a Python sequence or proper logic list of
  finite-domain terms so their sum equals `total`
- the sum relation should support concrete totals, finite-domain result
  variables, and the empty-sum identity where `sum([]) = 0`
- the propagator may use safe interval pruning for broad domains, while
  concrete assignments are checked exactly
- examples should show resource-allocation or scheduling-style models that are
  much clearer with an n-ary sum than with nested binary `fd_addo` helpers

D5 non-goals:

- no symbolic linear-expression DSL yet
- no weighted scalar-product API yet
- no complete subset-sum or pseudo-Boolean propagator

## PR Sizing Recommendation

Use this implementation sequence:

```text
PR 1: Batch A - metaprogramming completion
PR 2: Batch B - dynamic runtime database
PR 3: Batch C - real cut and search control
PR 4: Batch D1 - CLP(FD) foundation
PR 5: Batch D2/D3 - arithmetic propagation and all-different
PR 6: Batch D4 - larger examples and labeling ergonomics
PR 7: Batch D5 - n-ary sum ergonomics
PR 8+: later CLP(FD) extensions, parser integration, or advanced propagation
```

This is the fewest practical set of large PRs without forcing unrelated solver
changes into the same review. Batch A can be a single implementation PR. Batch
B and Batch C should also each be one PR if tests remain manageable. Batch D is
large enough that it should be planned as a track with a small number of
internally coherent sub-batches.

## Test Strategy

Each batch must include:

- package-local unit tests for new helpers and builtins
- end-to-end Python examples that solve a real logic problem
- README or changelog updates showing the new library-level Prolog capability
- regression tests for variables, backtracking, and answer ordering

Batch-specific tests:

- Batch A: round-trip `clauseo` body terms through `calltermo`
- Batch A: standard term ordering across variables, numbers, atoms, and
  compounds
- Batch A: predicate metadata observes static clauses and builtins
- Batch B: asserted clauses are branch-local under backtracking
- Batch B: static predicate mutation fails safely
- Batch C: cut prunes exactly the intended choicepoints
- Batch C: cut does not behave like a global stop
- Batch D: finite-domain constraints narrow domains before labeling
- Batch D: labeling enumerates only assignments that satisfy all constraints
- Batch D2/D3: arithmetic constraints prune domains before labeling
- Batch D2/D3: `all_differento` solves a small Latin-square-style problem
- Batch D5: `fd_sumo` handles Python sequences, logic lists, result variables,
  empty sums, and a resource-allocation example

## Security and Robustness Notes

Future implementation PRs should keep these guardrails:

- callable goal-term lowering must reject malformed host objects rather than
  constructing arbitrary native goals
- dynamic database operations must not mutate shared `Program` objects in place
- rollback state should be bounded and tied to search branches
- cut should be scoped; an over-broad cut is a correctness bug
- CLP(FD) domains need size limits before enumeration to avoid accidental
  explosive search
- examples and tests should use limits when intentionally demonstrating
  infinite or very large search spaces

## Summary

LP18 says yes: the remaining Prolog-level functionality can be knocked out in a
few big batches rather than many tiny PRs.

Batch A completed the pure metaprogramming loop, Batch B added runtime dynamic
predicates, Batch C added real cut, and Batch D now covers finite domains,
arithmetic constraints, all-different, practical examples, and n-ary sums.
Later CLP(FD) work can deepen propagation, add reification, or connect parser
syntax onto the same Python-first runtime.
