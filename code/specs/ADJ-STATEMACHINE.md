# ADJ-STATEMACHINE — the `statemachine` construct (RS-3)

**Status:** normative contract for RS-3 (task #279). This document is the
implementation-ready expansion of [`ADJ-RULE-SUBSTRATE.md`](ADJ-RULE-SUBSTRATE.md)
§3–§5. Where that document argues *why* a rulebook is a state machine, this one
pins *exactly* the surface grammar, the driver semantics, the termination
guarantees, and the lowering — enough that the grammar/AST/lowering (RS-3b) and
the driver (RS-3c) can be built without further design decisions.

## 1. What it is, and why

A `statemachine` is ADJ's construct for **long-horizon procedural reasoning** — a
multi-step process with explicit control flow and a guaranteed halt. It turns an
implicit resolution order (triage → work-up → decision; titrate-until-target;
iterate-until-converged) into a first-class, auditable object.

It is a *sibling* of `dictionary` / `rulebook` / `formulabook` / `table`: a
named, importable, provenance-carrying top-level declaration that **lowers onto
the existing engine** (the SLD resolver + the compute evaluator + the proof DAG).
It introduces **no new evaluator** — a guard is an ordinary predicate/compute
evaluation, an action is an assertion into the `KnowledgeBase`, and a step is one
forward-chaining transition. What is new is the *driver* that sequences those
steps and the *termination* guarantees around it.

Crucially, a state-machine run is **explainable and re-checkable by construction**:
every transition, guard test, action, and exit is a provenanced step in the
`ReasoningTrace`, so [`ADJ-REASON-MATH.md`](ADJ-REASON-MATH.md) §E.8 `--explain`
renders the whole run, and `adj-verify` re-executes it offline.

## 2. Surface grammar (normative)

Like `formulabook`/`table`, the construct keywords are **IDENT-matched literals**,
not lexer tokens — `.tokens` is untouched. Row/guard/action items reuse the
existing `term`/expression grammar.

```
statemachine_decl =
    "statemachine" IDENT "{"
        { use_decl }                 % import dictionaries, as elsewhere
        initial_decl                 % exactly one; required
        { state_decl }               % one or more
        { exit_decl }                % one or more; required
        budget_decl                  % exactly one; required
        { annotation }               % source / locator / trust envelope
    "}" ;

initial_decl    = "initial" IDENT ;                         % names a declared state
state_decl      = "state" IDENT "{" { transition_decl } "}" ;
transition_decl = "transition" "on" guard "to" IDENT [ "do" action { "," action } ] ;
exit_decl       = "exit" "when" guard "yield" expr ;
budget_decl     = "budget" NUMBER "steps" ;

guard  = expr relop expr | IDENT ;   % a numeric predicate over a slot, or a bare
                                     %   finding atom (holds iff that fact is present)
action = "assert" term               % emit a fact into the KB
       | "let" IDENT "=" expr ;       % bind a computed value (RS-1/RS-2 compute)
```

Static well-formedness (a `LowerError`, never a panic):

- **`SmMissingInitial` / `SmMissingExit` / `SmMissingBudget`** — each required
  clause is present exactly once.
- **`SmUnknownState`** — `initial <s>` and every `transition … to <s'>` name a
  declared `state`.
- **`SmBudgetNotPositive`** — `budget N steps` has `N ≥ 1`.
- **`SmMissingProvenance`** — the declaration carries a non-empty `source`
  (identical to the `formulabook`/`table` provenance gate; the linter rejects an
  unsourced machine).

> **RS-3b minimal subset.** The first implementation slice may restrict `guard`
> to `slot relop number` and a bare finding atom, and `action` to `assert term`,
> deferring `let`-binding actions and formula-valued guards to a follow-up — the
> grammar above is the full target, and any deferral MUST be an explicit
> `LowerError`, never a silent drop.

## 3. Driver semantics (normative)

The driver is deterministic and total. Let `kb` be the compiled knowledge base
(with any `observe`d facts), `budget` the declared step budget.

```
state  := initial
steps  := 0
seen   := {}                         % visited (state, relevant-binding) keys
loop:
    if some exit_decl's guard holds in kb:
        return Halted { state, result = eval(that exit's yield expr) }
    if steps >= budget:
        return StepBudgetExceeded { steps, budget, state }
    key := (state, project(kb))      % the machine-relevant facts (see §3.1)
    if key in seen:
        return NonTerminating { state, key }
    seen.insert(key)
    t := the FIRST transition of `state`, in source order, whose guard holds
    if no such t:
        return Stuck { state }        % dead end: no transition, no exit
    for a in t.actions: apply a to kb   % assert fact / bind value — each a trace step
    state := t.target
    steps := steps + 1
```

- **First-guard-wins** is the deterministic selection rule. If two transitions in
  a state could both fire, source order decides; a `priority` tier (reusing the
  existing rule `priority`) MAY override, and an *equal-priority* overlap is
  surfaced as a `SmTransitionConflict` diagnostic at lowering time, never resolved
  silently.
- **Every** loop action is one provenanced entry in the `ReasoningTrace`: the
  transition firing (cited to the transition's provenance), each guard test, each
  `assert`/`let` action, and the terminal exit. `--explain` (§E.8) renders this as
  the ordered narrative of the run; `adj-verify` re-runs it.

### 3.1 The cycle key

`project(kb)` is the set of facts over the machine's own vocabulary (its `use`d
dictionary terms) — not the entire KB — so cycle detection keys on the
*machine-relevant* configuration. Revisiting an identical `(state, project(kb))`
is a livelock and short-circuits to `NonTerminating`. Because actions may
monotonically add facts, many runs never repeat a key and instead terminate via
an exit or the budget; the cycle guard catches the genuine fixed-point loop (e.g.
a titrate step that re-asserts an unchanged value).

## 4. Termination — always halt or error, never hang (normative)

The run is **total by construction**: it returns exactly one typed outcome.

| Outcome | Meaning |
|---|---|
| `Halted { state, result }` | an exit criterion held; `result` is the yielded value, with its derivation trace. |
| `StepBudgetExceeded { steps, budget, state }` | the budget ran out before any exit — a grounded abstention ("stopped after N steps"), with the partial trace. |
| `NonTerminating { state, key }` | a `(state, relevant-binding)` configuration repeated — a livelock, caught, with the partial trace. |
| `Stuck { state }` | in `state`, no transition guard holds and no exit criterion holds — a dead end; a grounded abstention ("no transition applies in state …"). |

Every outcome carries the partial `ReasoningTrace`, so even a non-terminating or
stuck run explains *how far it got and why it stopped*. This is the same
"abstain with a reason" discipline the typed `AbstentionReason` gives recall/table
lookups (§E.4). There is no code path that hangs or silently stops.

The step budget composes with the existing sub-guards: arithmetic keeps its
`MAX_EVAL_DEPTH`, rule recursion its depth guard; a transition firing and each
rule/formula application it triggers consume from the one budget. Exceeding any
inner guard surfaces as the corresponding typed error attributed to the step that
triggered it.

## 5. Lowering (normative)

`statemachine` lowers onto the existing engine, reusing the shared provenance
surface exactly as `table`/`formulabook` do:

- The declaration's `source`/`locator`/`trust` lower through
  `annotations_to_provenance` (`lower.rs`) — one envelope for the machine; each
  transition inherits it (a per-transition `source` MAY override in a later slice).
- Each `state`/`transition` is recorded as a provenanced structure the driver
  reads; guards and actions lower to the *same* predicate/compute/assertion forms
  the rest of the language already lowers (RS-1/RS-2). No parallel evaluator.
- The driver runs at query time when the program `? run <machine>` (surface for
  invoking a machine — settled in RS-3b alongside the grammar), producing a
  `statemachine` result section in the CLI JSON and an `--explain` narrative.

## 6. Worked examples

### 6.1 A terminating drive-through (titrate-to-target)

```adj
statemachine warfarin_titration {
    use anticoagulation_vocab
    initial check

    state check {
        transition on inr < 2 to increase_dose
        transition on inr > 3 to decrease_dose
    }
    state increase_dose { transition on true to check do assert dose_changed }
    state decrease_dose { transition on true to check do assert dose_changed }

    exit when inr >= 2 yield at_target      % 2 ≤ INR ≤ 3 exits
    budget 20 steps
    source "warfarin dosing protocol (worked example)"
    trust authoritative
}
```

With `observe inr(2.5)`, the `check` state's guards both fail, the exit guard
`inr >= 2` holds, and the machine `Halted { state: check, result: at_target }` in
0 transitions — the `--explain` narrative shows the exit test citing the protocol.

### 6.2 A budget-exceeded (non-terminating) run

```adj
statemachine spin { use loop_vocab  initial a
    state a { transition on true to b }
    state b { transition on true to a }
    exit when done yield ok
    budget 8 steps
    source "loop (worked example)"  trust inferred
}
```

With no `observe done`, no exit ever holds; the driver ping-pongs `a↔b`. Cycle
detection fires first (the `(a, ∅)` key repeats), returning
`NonTerminating { state: a }` with the partial trace — never a hang. (Absent the
cycle key, the `budget 8 steps` guard would return `StepBudgetExceeded`.) Both are
typed, grounded abstentions.

## 7. Cross-references

- Explanation & re-check: the run is an ordinary `ReasoningTrace`, rendered by
  `--explain` and re-executed by `adj-verify` — [`ADJ-REASON-MATH.md`](ADJ-REASON-MATH.md)
  §E.8, §E.5.
- Substrate rationale & staging: [`ADJ-RULE-SUBSTRATE.md`](ADJ-RULE-SUBSTRATE.md)
  §3 (surface), §4 (termination), §5 (trace), §7 (RS-3 staging).
- Provenance envelope & lowering pattern: modeled on `table`
  ([`ADJ-TABLES.md`](ADJ-TABLES.md)) and `formulabook`
  ([`ADJ-FORMULA-LIBRARIES.md`](ADJ-FORMULA-LIBRARIES.md)).

## 8. Staging (each: spec-sync → tests → impl → provenance-gate → security-review → babysit)

**RS-3 is COMPLETE** — the contract (RS-3a), the grammar/AST/lowering (RS-3b), and the
driver (RS-3c) are all shipped.

- **RS-3a (this document):** the normative contract. Spec-only. **DONE.**
- **RS-3b:** grammar (`statemachine_decl` + `state`/`transition`/`exit`/`budget`),
  AST (`Statement::StateMachine`), adapter, lowering to provenanced structures +
  the `SmMissing*`/`SmUnknownState`/`SmBudgetNotPositive`/`SmMissingProvenance`
  errors, and a parse+lower e2e (a well-formed machine compiles; each malformed
  one yields its typed error). No driver yet. **DONE.**
- **RS-3c:** the driver (§3), the typed outcomes (§4), cycle detection, the CLI
  `state_machines` result section, `--explain` rendering of the run, and a
  worked-example e2e including a `NonTerminating`/budget-exceeded test. **DONE.**

  Implementation notes / deviations:
  - The driver lives in `adj-lang::statemachine::run_state_machine` (where both the
    lowered types and the engine are reachable) and runs **every** declared machine
    unconditionally, rather than gating on a `? run <machine>` surface — the §5 `? run`
    surface is not required for RS-3c and was not added (a machine's declaration is its
    invocation for now). The CLI collects the runs after `decide` and reasons over
    `lowered.kb`.
  - **Guard evaluation reuses the engine, no parallel evaluator** (§3): a *comparison*
    guard reads the subject's valued slot with `KnowledgeBase::observed_numeric`,
    evaluates the rhs with `compute`, and compares exact-first with `CmpOp::eval_values`
    — the identical three calls a predicate-gated contribution makes. A *presence* guard
    holds iff `enumerate_all(subject, kb)` yields any proof; the bare atom `true` is the
    always-holds special case.
  - **Yield evaluation**: a yield expression is run through `compute`; a numeric result
    carries its derivation tree, and a bare symbolic atom (`at_target`) — for which
    `compute` returns `UnknownSlot` — is reported as that symbol.
  - **The cycle key** (§3.1) is `(state, the sorted set of terms asserted so far)`. Because
    `assert` actions are monotone (they only add to a working KB clone) and the base KB is
    fixed for the whole run, this set is a sound, deterministic fingerprint of the
    machine-relevant configuration; a repeat is the genuine livelock. The `steps >= budget`
    guard bounds the loop independently, so a run always terminates in `≤ budget + 1`
    iterations even if a key never repeats.
