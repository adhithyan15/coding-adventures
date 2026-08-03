# adj-lang (Rust)

Surface-syntax frontend for the adjudication framework. Lexes,
parses, and lowers a small domain-expert-readable rulebook DSL into
a `logic-engine` `KnowledgeBase`.

## What this is

Adj-Lang is the language layer of the adjudication framework. Its
v0.1 grammar covers the five clause kinds the LP19e LR-aggregation
engine exposes:

- `prior <p> for <conclusion>` — Bayesian baseline.
- `contributes <lr> from <evidence> to <conclusion>` — atomic LR.
  `<evidence>` is either a term (`pmh(hypertension)`) or a numeric
  **predicate** over a valued slot (`gross_income >= 14600`).
- `interacts <lr> when <e1> and <e2> [and ...] for <conclusion>` —
  joint-evidence interaction term.
- `observe <term>` — assert a Certain Fact. Terms may carry numeric
  arguments (`observe gross_income(18000)`) — the *valued facts* that
  predicates read.
- `? <conclusion>` — query the engine.

### Predicate-gated contributions — deterministic = saturating probabilistic (v0.5)

A **deterministic** rule is just the saturating limit of a probabilistic
one. Write a numeric predicate as the evidence and give it a large LR:

```
prior 0.10 for required_to_file
contributes 1000000 from gross_income >= 14600 to required_to_file
  source "IRS Pub 501 (2024)" trust authoritative
observe gross_income(18000)
? required_to_file
```

The engine evaluates `gross_income >= 14600` on the CPU at decision time —
the model that authored the rulebook never ran the comparison. The right-hand
side may be a full arithmetic expression such as `answer == 3 / 10`, including
native `latex "<math>"` wherever an expression is legal. The proof step records
the comparison that fired (`slot`, `op`, `threshold`, `observed`), so the audit
trail shows the numbers, not a model's claim.
DETERMINATE / INDETERMINATE / CONFLICT still fall out of the differential
(leader / insufficient-evidence / kickback) — **one engine, not two**.
Operators: `>= <= > < ==`.

### `let` + arithmetic — computed values (v0.6)

The model writes the **formula**; the engine computes it on the CPU and a
predicate fires over the result like any observed slot:

```
observe csf_glucose(quantity(40, mg_dl))
observe serum_glucose(quantity(100, mg_dl))
observe line_item(12000)
observe line_item(6000)

let csf_ratio = csf_glucose / serum_glucose     % = 0.4
let total     = sum(line_item)                  % = 18000

contributes 1000000 from csf_ratio <= 0.4 to bacterial
contributes 1000000 from total    >= 14600 to required_to_file
```

`<expr>` is `+ - * /` (standard precedence, parentheses), references to
observed slots and earlier `let`s, numeric literals, and aggregations
`sum/count/min/max/avg(slot)`. Every computed value carries a **derivation
tree** back to the cited facts, so a reviewer can audit the arithmetic — the
model never evaluates it. **Space your operators** (`a - 5`, not `a-5`): a `-`
glued to a digit lexes as a negative literal.

LaTeX math is native input, not a caller-side normalization step. Use
`latex "<math>"` anywhere an arithmetic expression is expected:

```
let answer = latex "$5 \times 12$"
let ratio  = latex "\frac{csf_glucose}{serum_glucose}"
```

The frontend parses the string with the repo's LaTeX `MathFrontend` and lowers
the supported arithmetic subset into the same expression tree as ASCII ADJ.
Unsupported math is a compile error, not a guessed rewrite.

### Parser-backed formula source maps

Provenance tooling can call `formula_source_map(source)` to inventory every formula with the
same grammar and typed adapter used for execution. Each entry carries its `FormulaDef` plus
half-open UTF-8 byte spans for the complete declaration and final executable body. Multi-step
formulas point at the final expression, quoted math retains its exact source spelling, and any
formula-boundary, order, count, or name disagreement is an error rather than a best-effort match.
The source map is structural: import resolution, lowering, derivation replay, and execution
coverage remain separate gates.

### Constraints — `symbol` / `constrain` / `solve` / `check` (v0.7)

The model extracts the policy's **unknowns and constraints**; the engine solves
them (the solver backends land in the next slice). The surface:

```
symbol premium : money(usd)
observe base_rate(1200)
observe cap(2000)

constrain premium >= base_rate
constrain premium <= cap

solve for { premium }          % find a value satisfying the constraints
% or:  check                   % is the constraint set satisfiable?
```

- `symbol <name> : <sort>` declares an unknown (`sort` = `scalar`, `money(usd)`, …).
- `constrain <expr> <relop> <expr>` with `relop ∈ { >= <= > < == = != }`;
  operands are arithmetic exprs over symbols, observed slots, earlier `let`s,
  and numbers. Compare against a typed value by `observe`-ing it and using its
  name (constraint operands are arithmetic exprs, not term literals).
- `constrain latex "<equation>"` parses a LaTeX relation directly, e.g.
  `constrain latex "$x^2 = 4$"`, and feeds the native solver.
- `solve for { … }` / `check` drive the solver. The lowerer builds a
  `ConstraintSystem` (on `LoweredProgram.constraints`) with each constraint's
  sides kept as unevaluated expression trees.

### Dictionary — `dictionary` / `define` (v0.9, MYCIN-2026)

A **dictionary** is the controlled vocabulary the decomposer and the rulebook
agree on, written as a first-class, named construct:

```adj
dictionary meningitis_vocab {
  define bacterial_meningitis : hypothesis
    surface "bacterial meningitis", "pyogenic meningitis"
  define csf_glucose : finding values [low, normal]
    surface "CSF glucose", "spinal fluid glucose"
}
```

- `define <name> : hypothesis` registers a hypothesis term.
- `define <name> : finding values [v…]` registers a finding functor whose value
  argument is drawn from a **closed** domain (so "observed normal" is
  distinguishable from "not yet observed").
- `surface "…", "…"` lists the prose forms a decomposer may map onto the term —
  documentation for the warm pipeline, *not* engine-semantic.
- A `define` is legal bare or inside a `dictionary { … }` block.

When a program declares a dictionary (at least one `define`), the lowerer
**enforces the vocabulary at compile time**: every hypothesis used in a
`prior`/`contributes`/`interacts`/`uncertain`/`?` and every finding used in an
`observe`/`contributes`/`interacts`/`uncertain` must be defined, and a finding
value must lie in its declared domain — otherwise `LowerError::UndefinedTerm` or
`ValueNotInDomain`. The IR a decomposer emits and the rulebook it compiles
against therefore share one closed vocabulary by construction. A program with no
dictionary is unchecked (backward-compatible).

### Rulebook — `rulebook` / `use` (v0.10, MYCIN-2026)

A **rulebook** is a named, reusable block of the clauses that make up a body of
adjudicatable knowledge — written once, checked in as code, and (M3) importable.
A `use` binds the dictionary the rulebook is checked against:

```adj
dictionary meningitis_vocab {
  define bacterial : hypothesis
  define viral     : hypothesis
  define csf_glucose : finding values [low, normal]
}

rulebook meningitis {
  use meningitis_vocab
  prior 0.30 for bacterial   source "Tunkel IDSA 2004" trust authoritative
  prior 0.30 for viral       source "Tunkel IDSA 2004" trust authoritative
  contributes 5 from csf_glucose(low) to bacterial
    source "low CSF glucose favors bacterial" trust empirical
}

observe csf_glucose(low)
? bacterial
? viral
```

- `rulebook <name> { … }` groups clauses under one name. The rulebook is a
  **container, not a namespace**: its clauses lower into the `KnowledgeBase`
  exactly as if written at top level. The name is for reuse / addressing.
- `use <dictionary>` (inside a rulebook or at top level) binds a declared
  dictionary as the vocabulary that scope's clauses are checked against.
- **Enforcement is scoped by `use`.** When any `use` appears, a top-level
  `use D` checks the top-level clauses against `D`, and each rulebook is checked
  against its own `use` (falling back to a top-level one). A scope with no `use`
  is unchecked — a rulebook opts in to checking by `use`-ing a dictionary. A
  `use` of an undeclared dictionary is `LowerError::UndefinedDictionary`. With no
  `use` anywhere, the M1 whole-program rule above is unchanged.

### Rule — `rule { head: … when: … }` (v0.14, derivation rules)

Where `relate` asserts a **ground** edge, a **rule** lets the engine **derive** a
head whenever its body holds — a Horn clause / Datalog rule. This is what lets a
`rulebook` carry *conditional* domain knowledge (a contraindication, a step-therapy
policy) instead of only facts + likelihood ratios:

```adj
relate pregnant(present)
relate pregnancy_excludes(moxifloxacin)
relate pregnancy_excludes(tmp_smx)

rule { head: contraindicated($D)  when: pregnant(present), pregnancy_excludes($D)
       source "Pregnancy contraindicates fluoroquinolones (FDA label)." trust authoritative }

? contraindicated($X)        % derives moxifloxacin AND tmp_smx
```

- A `$Var` binds across head and body (clause scope, like a binding query); a body
  literal prefixed `not` is **negation-as-failure** (`not contraindicated($D)`).
- Lowers to `logic_engine::Rule { head, body }` and resolves through the same
  SLD/unification machinery `relate` facts do — `? head($X)` enumerates every
  derivable answer, each with its proof.
- Rules carry `source`/`locator`/`trust` like every grounded clause, so a rule
  extracted once from source text and gated into the CAS stays byte-traceable.
- **Why it matters:** every domain (insurance, formulary, contraindications, …) is a
  `rulebook` of `rule`s — authored once, grounded into the CAS, and the engine
  derives the consequences from per-case facts. No domain-specific host code.

### Import — `import "path"` (v0.11, MYCIN-2026)

`import "<relative path>"` composes a program across files, so a dictionary, the
rulebook that `use`s it, and a case can each be their own checked-in `.adj`:

```adj
% dictionary.adj
dictionary meningitis_vocab { define bacterial : hypothesis  define csf_glucose : finding values [low, normal] }

% rulebook.adj
import "dictionary.adj"
rulebook meningitis { use meningitis_vocab
  prior 0.30 for bacterial   source "Tunkel IDSA 2004" trust authoritative
  contributes 5 from csf_glucose(low) to bacterial  source "…" trust empirical }

% case.adj
import "rulebook.adj"
observe csf_glucose(low)
? bacterial
```

The import graph is resolved into one program *before* lowering, with four
guarantees: **relative** to the importing file, **idempotent** (a file imported
twice — e.g. a diamond — is merged once, by canonical id), **acyclic** (a cycle
is `ImportError::Cycle`, never a hang), and **bounded** depth + fan-out
(`ImportLimits`, default 32 / 256 → `DepthExceeded` / `TooManyFiles`).

The library does **no filesystem I/O**: `resolve_imports` drives an injected
[`ImportProvider`], so the graph policy is unit-testable without a disk and the
filesystem trust boundary (canonicalization, relative-only, sandbox-root
containment) lives in the caller — `adj-lang-cli`'s `FsProvider`. Use
`compile_with_imports(root_id, provider, limits)` for the resolve-then-lower
path; plain `compile` rejects a stray `import` as `LowerError::UnresolvedImport`.

### Differential over the `?` queries (v0.4)

A program's `? h` lines are read as the set of **competing hypotheses**.
`compile_and_decide(src)` (or `decide(&lowered)`) runs
`logic_engine::differential` over them: ranks by posterior, picks the argmax,
reports the between-hypothesis margin, and kicks back when an open uncertainty
could flip the ranking. A multi-`?` program is therefore a differential
(bacterial vs viral vs fungal); a single `?` yields a determinate result. No
grammar change — the competing set is already the `?` lines.

Every clause can carry annotations:

- `source "<text>"` — citation string.
- `locator "<text>"` — page / section / paragraph within the source.
- `trust <tier>` — one of `consensus | authoritative | empirical |
  inferred | unattributed`.

## The ACS rulebook in Adj-Lang

```adj
% chest-pain ACS risk rulebook (ADJ36)

prior 0.10 for acs
  source "Pope JH et al., NEJM 1995;342(16):1163-70"

contributes 1.5 from pmh(hypertension) to acs
  source "HEART Score; Six AJ et al., Neth Heart J 2008"
  trust empirical

contributes 1.8 from pmh(smoker) to acs
  source "HEART Score; Six AJ et al., Neth Heart J 2008"
  trust empirical

contributes 2.5 from symptom_quality(pressure_like) to acs
  source "Panju AA et al., JAMA 1998;280(14):1256-63"

contributes 2.0 from associated_symptom(diaphoresis) to acs
  source "Panju AA et al., JAMA 1998"

contributes 0.5 from vital_signs(within_normal_limits) to acs
  source "Panju 1998"

contributes 0.4 from denied(ecg_acute_st_changes) to acs
  source "Pope 1995"

interacts 1.3 when symptom_quality(pressure_like)
               and associated_symptom(diaphoresis)
               for acs
  source "[empirical] synergy"
  trust empirical

% The case — Jane Doe vignette from ADJ36
observe pmh(hypertension)
observe pmh(smoker)
observe symptom_quality(pressure_like)
observe associated_symptom(diaphoresis)
observe vital_signs(within_normal_limits)
observe denied(ecg_acute_st_changes)

? acs
```

Compared to the hand-written Rust encoding in ADJ46
(`code/specs/data/adj46/src/main.rs`, ~390 LOC), the Adj-Lang
source above is ~30 lines of readable English. The ACS rulebook is
no longer addressed to the engine; it's addressed to the ED
physician who wrote it.

## How it fits

```
   adj-lang source
        │ [lex] → [parse] → [lower]
        ▼
   logic-engine::KnowledgeBase     ← (this crate's output)
        │ [search, SearchMode::LRAggregate]
        ▼
   posterior + proof DAG + warnings
```

## API at a glance

```rust
use adj_lang::compile;
use logic_engine::{search, SearchMode, SearchResult};

let lowered = compile(source_text)?;
for query in &lowered.queries {
    match search(query, &lowered.kb, SearchMode::LRAggregate) {
        SearchResult::LRAggregateResult { posterior, dag, .. } => {
            println!("P({query:?}) = {posterior:.3}");
            // dag.proofs[0].steps enumerates the prior + every
            // active contribution, with provenance reachable via
            // step.origin's clause_id.
        }
        _ => unreachable!(),
    }
}
```

## What v0.1 covers — and what's deferred

Adj-Lang dissolves ADJ46 awkwardness items **A4** (joint
contributions syntactically distinct from atomic, via the `interacts`
keyword) and **A10** (rulebook surface is hand-written Rust).

**A9** — multi-source corroboration — is covered as of the ADJ-A9 change. A
clause may carry one or more **corroborating** citations, each a co-equal source
for the *same* fact, via a repeatable annotation:

```
contributes 2.5 from neutrophil_predominance to bacterial_meningitis
    source  "Tunkel et al., IDSA 2004"   locator "https://…/cid/39/9/1267"   trust authoritative
    cites   "van de Beek et al., NEJM 2006"   locator "https://www.nejm.org/…/NEJMra052116"
    cites   "Brouwer et al., Clin Microbiol Rev 2010"   locator "https://…/CMR.00070-09"
```

`cites "<source>" locator "<locator>"` is repeatable (the primary
`source`/`locator`/`trust` remain at-most-once); the `locator` keyword is reused
as the separator so no short keyword (`at`) is reserved. Corroborations are
**documentary only** — they record extra spans an auditor can re-fetch and do
**not** enter the LR arithmetic (double-counting the same fact would inflate
posteriors). They lower onto `logic_engine::Provenance::corroborations` and
render in the CLI's clause provenance as a `"corroborations":[…]` array. See
[ADJ-A9 spec](../../../specs/ADJ-A9-multi-source-corroboration.md).

Not yet covered (all language-layer follow-ups):

- **A5** — uncertainty markers (`uncertain X over {a,b,c}`).
- **A7** — kickback as a query-result variant.
- **A8** — counterfactual queries (`? acs given pmh(htn)=true`).

These are small additive extensions of the grammar; each adds one or
two arms to `parser::parse_statement` and one new variant to the
lowering pass.

## Tests

24 unit tests across `lexer`, `parser`, and `lower`. Headline test:
`lowers_full_acs_rulebook_and_reproduces_adj36_posterior` — the ACS
program above compiles, runs through `SearchMode::LRAggregate`, and
reproduces ADJ36's 28.1% posterior end-to-end through the production
engine.

## See also

- [ADJ46 — awkwardness catalogue](../../../specs/ADJ46-acs-rulebook-on-logic-engine-toolchain-shakedown.md)
- [LP19e — LR aggregation spec](../../../specs/LP19e-likelihood-ratio-aggregation.md)
- [logic-engine](../logic-engine/README.md) — the inference layer
  Adj-Lang lowers to.
