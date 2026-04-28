# PR02 - Prolog Dialects and Logic VM Gap Analysis

## Overview

PR00 introduced grammar-driven Prolog tokenization and PR01 introduced
`code/grammars/prolog.grammar` plus lowering into `logic-engine`. The next
question is broader: can the Logic VM become the common execution substrate for
Prolog and the major Prolog dialects?

The answer is yes for the shared Prolog core, but not yet for full dialect
coverage. The current VM is a loader VM for facts, rules, relation declarations,
dynamic declarations, and queries. It can host a common Prolog subset, but major
dialects add syntax, compilation phases, module semantics, control semantics,
constraint stores, tabling, foreign interfaces, and runtime services that need
explicit support.

This document defines the dialect surface and the support gaps so future work
can land in a few coherent batches rather than many tiny compatibility PRs.

## Current Project Surface

Already present:

- `symbol-core`: interned symbolic names
- `logic-core`: atoms, numbers, strings, variables, compound terms, lists
- `logic-engine`: unification, facts, rules, conjunction, disjunction, cut,
  equality, disequality, fresh variables, deferred/native goals, solving
- `logic-builtins`: control, arithmetic, collection, term, dynamic database,
  clause introspection, and finite-domain builtins
- `logic-instructions`: high-level relation/fact/rule/query instruction stream
- `logic-vm`: stepwise loader VM for high-level instructions
- `logic-bytecode` and `logic-bytecode-vm`: compact loader bytecode and VM
- `prolog-lexer`: grammar-driven tokenization from `prolog.tokens`
- `prolog-parser`: grammar-driven parsing from `prolog.grammar` and lowering
  into `logic-engine`

This is enough for a meaningful library-first Prolog core. It is not enough for
full SWI, SICStus, GNU Prolog, XSB, YAP, Ciao, ECLiPSe, Scryer, Trealla, Tau,
Visual Prolog, or Logtalk compatibility.

## Dialect Families To Track

### ISO/Core Prolog

The shared baseline should be ISO-style terms, clauses, queries, unification,
search, arithmetic, standard term predicates, database predicates, exceptions,
I/O predicates, flags, operators, DCGs, and common control constructs.

Current support:

- Good: terms, clauses, facts, rules, unification, conjunction, disjunction,
  cut, equality, disequality, many term builtins, arithmetic builtins, some
  database and introspection builtins.
- Missing or incomplete: ISO operator declarations, directives, modules,
  exceptions, streams/I/O, flags, complete arithmetic syntax, complete standard
  predicate library, DCG expansion, consult/include, and complete error terms.

### Edinburgh, DEC-10, Quintus, SICStus Family

Many mainstream systems share heritage around operator syntax, modules,
directives, meta-predicates, term expansion, and library conventions.

Important support areas:

- Quintus-style module imports and exports
- `meta_predicate` declarations and module-sensitive `call/N`
- operator tables scoped globally or per module
- term and goal expansion hooks
- coroutining features such as `freeze/2`, `when/2`, or block declarations
- CLP(FD), CLP(Q/R), and CHR-style extension libraries

### SWI-Prolog

SWI has a large practical surface: modules, packs, dicts, strings, Unicode,
quasiquotations, attributed variables, tabling, constraints, engines, threads,
foreign interfaces, rich I/O, and dialect emulation.

VM implications:

- Needs module namespaces and imports in the instruction model.
- Needs a runtime predicate registry that distinguishes builtins, static
  predicates, dynamic predicates, multifile predicates, and imported predicates.
- Needs attributed variables for SWI-style constraints, `dif/2`, coroutining,
  and tabling with constraints.
- Needs optional tabling semantics beyond ordinary depth-first search.
- Needs syntax support for dicts, quasiquotations, strings/char flags, and
  SWI-specific directives.

### GNU Prolog

GNU Prolog is close to ISO in many places and is notable for native compilation
and finite-domain constraints. Its manual also states that it has no module
facility, so dialect support cannot assume a universal module model.

VM implications:

- A no-module dialect profile must be first-class, not treated as missing data.
- GNU-style finite-domain predicates and operators need a compatibility layer
  over our existing finite-domain builtins.
- Native compilation is out of scope for the current loader VM, but the
  instruction and bytecode layers can remain the compilation boundary.

### Scryer and Trealla

Scryer and Trealla are modern compact ISO-oriented systems with strong interest
in declarative constraints. Scryer exposes CLP(ℤ) as `clpz`; Trealla documents
UTF-8 atoms, chars-list strings, unbounded integers/rationals, CLP(Z), DCGs,
FFI, engines, and concurrency-oriented facilities.

VM implications:

- Needs dialect flags for string representation, Unicode atom classification,
  and rational/integer arithmetic.
- Needs CLP(ℤ) compatibility layered over or alongside current CLP(FD).
- Needs engines/coroutines only after the core module/directive work stabilizes.

### XSB and YAP

XSB is centered on tabled Prolog and well-founded semantics. YAP also has a
tabling engine and high-performance implementation concerns.

VM implications:

- Current depth-first backtracking cannot model tabled evaluation.
- Need a tabled solver backend or solver mode with subgoal tables, answer
  tries, suspension/resumption, and completion.
- For XSB-level semantics, negation needs well-founded semantics, not just
  negation-as-failure.

### Ciao

Ciao is modular and supports per-module language extensions, packages,
constraints, objects, assertions, and program transformation.

VM implications:

- Dialect profiles should not be only global. A source file or module may
  activate packages that change syntax and semantics locally.
- Need an expansion phase before VM loading.
- Need assertion metadata if we want to support Ciao-style analysis later.

### ECLiPSe

ECLiPSe is best treated as a Prolog-compatible constraint logic programming
system with a broad solver-library ecosystem.

VM implications:

- Current finite-domain constraints are not enough.
- Need a generic constraint-store interface for finite domains, finite sets,
  intervals, reals/rationals, CHR, and external solvers.

### Tau Prolog and tuProlog

These embeddable Prologs emphasize host integration: JavaScript for Tau,
JVM/OO environments for tuProlog.

VM implications:

- Need a foreign predicate boundary that is safe, typed, and traceable.
- Need async/host callback semantics only after pure Prolog execution is stable.

### Visual Prolog, Mercury, Logtalk, Picat

These are important relatives, but they should not be folded into the first
Prolog dialect plan:

- Visual Prolog has typed and object-oriented language semantics that diverge
  heavily from classic Prolog.
- Mercury is a separate strongly typed logic/functional language.
- Logtalk is an object-oriented extension that runs on multiple Prolog systems.
- Picat is Prolog-adjacent but changes the programming model substantially.

They are future frontend targets, not compatibility modes for the first Prolog
runtime.

## Can The Current Logic VM Support All Dialects?

Not yet.

The current Logic VM can support:

- pure facts and rules
- ordinary depth-first backtracking
- relation declarations
- dynamic relation declarations
- queries with inferred outputs
- source frontends that lower into existing `logic-engine` goals
- loader-level tracing and validation

The current Logic VM cannot yet support full dialect behavior for:

- module systems and module-sensitive predicate lookup
- directive execution and compile-time expansion
- operator declarations and per-dialect operator tables
- DCG expansion
- complete ISO standard predicate semantics and ISO error terms
- stream and file I/O
- exceptions
- Prolog flags
- attributed variables
- generalized coroutining
- tabled execution and well-founded semantics
- CHR and non-FD constraint domains
- foreign predicate calls with host effects
- concurrency, engines, and async host integration
- dialect-specific syntax such as SWI dicts and quasiquotations

## Recommended Architecture

### 1. Dialect Profiles

Introduce a dialect profile object that controls syntax and runtime policy:

```python
DialectProfile(
    name="iso",
    token_grammar="prolog.tokens",
    parser_grammar="prolog.grammar",
    operators=ISO_OPERATORS,
    double_quotes="codes",
    modules="none",
    enabled_extensions=frozenset(),
)
```

Profiles should be data, not subclasses. A source frontend should be able to
select `iso`, `swi`, `gnu`, `scryer`, `trealla`, `xsb`, or `ciao` and receive a
specific token grammar, parser grammar, operator table, and runtime policy.

### 2. Operator Tables Before More Syntax

Full Prolog syntax is operator-heavy. We should add a table-driven expression
parser or grammar extension before implementing arithmetic, `is/2`, CLP syntax,
if-then-else, and user-defined `op/3`.

The parser should lower operator syntax into ordinary terms and goals:

```prolog
X is Y + 1        -> is(X, +(Y, 1))
X #= Y + 1        -> '#='(X, +(Y, 1))
A -> B ; C        -> ';'('->'(A, B), C)
```

### 3. Frontend Expansion Pipeline

Add an explicit pre-VM compilation pipeline:

```text
tokens
  -> grammar AST
  -> operator-normalized AST
  -> expanded clauses
  -> instruction program
  -> logic-vm or logic-bytecode-vm
```

Expansion steps should include:

- directive handling
- `op/3`
- module/import declarations
- DCG expansion
- term expansion hooks
- dialect-specific rewrites

### 4. VM-Level Module and Predicate Registry

Extend the instruction and VM state with:

- module declarations
- imports and exports
- predicate properties
- multifile and discontiguous metadata
- dynamic/static/builtin predicate kinds
- module-qualified relation keys

Current relation keys are `(symbol, arity)`. Dialect support needs something
closer to `(module, symbol, arity)`.

### 5. Constraint Store Interface

The finite-domain store should become one implementation of a generic
constraint-store protocol:

```python
ConstraintStoreProtocol:
    post(...)
    propagate(...)
    reify(...)
    project(...)
    copy_with(...)
```

This keeps CLP(FD), CLP(ℤ), CLP(Q/R), finite sets, intervals, and attributed
variables from becoming separate ad hoc engine hacks.

### 6. Tabled Solver Backend

Do not try to squeeze XSB/YAP/SWI tabling into ordinary depth-first search.
Add a solver mode or backend for tabled execution with:

- tabled predicate declarations
- subgoal variant/subsumption keys
- answer tables
- delayed goals
- completion detection
- well-founded negation

This is a major runtime feature and should be its own batch.

## Proposed Implementation Batches

### Batch 1 - Grammar and Dialect Profile Foundation

- Keep `prolog.grammar` as the current parser source of truth.
- Add `DialectProfile` data structures.
- Add ISO/Core and current-subset profiles.
- Add a compatibility matrix in tests that proves the current subset parses
  under the core profile.

### Batch 2 - Operator and Directive Frontend

- Add table-driven operator parsing.
- Lower arithmetic, comparison, if-then, and CLP-looking syntax into terms.
- Parse and record directives.
- Support `op/3` at least at file scope.

### Batch 3 - Parser-To-Instruction Lowering

- Lower parsed Prolog into `logic-instructions`, not directly into
  `logic-engine`.
- Run parsed programs through `logic-vm` and `logic-bytecode-vm`.
- Keep direct `logic-engine` lowering only as a convenience compatibility path.

### Batch 4 - Modules and Predicate Registry

- Add module-qualified relation keys.
- Add imports, exports, builtins, dynamic/static metadata, and predicate
  properties.
- Support dialect profiles with no modules, Quintus-style modules, and
  SWI-style extensions.

### Batch 5 - DCGs and Expansion Hooks

- Implement DCG expansion to ordinary clauses.
- Add term expansion and goal expansion hooks behind dialect feature flags.

### Batch 6 - Advanced Runtime Semantics

- Add attributed variables and generic constraint stores.
- Add coroutining primitives.
- Add tabling as a separate solver backend.
- Add exceptions and stream I/O.

## Sources Consulted

- SWI-Prolog manual: modules, constraints, tabling with constraints
  <https://www.swi-prolog.org/pldoc/refman/>
- GNU Prolog manual: no module facility, finite-domain solver
  <https://www.gprolog.org/manual/gprolog.html>
- Scryer Prolog docs: ISO Prolog and CLP(ℤ)
  <https://www.scryer.pl/>
- Trealla Prolog docs: ISO-oriented interpreter, UTF-8 atoms, strings, CLP(Z),
  FFI, engines, concurrency
  <https://trealla-prolog.github.io/trealla/>
- XSB Prolog overview and tabling documentation
  <https://xsb.com/xsb-prolog/>
- YAP tabling documentation
  <https://www3.dcc.fc.up.pt/~vsc/YAP/group__Tabling.html>
- Ciao documentation: modules, packages, constraints, objects, transformations
  <https://ciao-lang.org/>
- ECLiPSe CLP documentation
  <https://eclipseclp.org/doc>
- Tau Prolog documentation
  <https://tau-prolog.org/documentation>
