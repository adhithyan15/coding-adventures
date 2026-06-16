# ADJ73 — Defeasible rule precedence (context specificity / `prefer` ordering)

**Status:** DESIGN (specs-first; no implementation committed with this doc).
**Stream:** ADJ language. **Depends on:** the `rule { head: … when: … }` keystone (PR #5995),
relational binding queries (REL-3), negation-as-failure in rule bodies (logic-engine).
**Unblocks:** clean `decide_timing` refactor (MYCIN CC-5), specialist-over-general medical
rules, and the long-term context-scoped legal-corpus vision
(`project_adj_universal_rule_substrate`: federal > state, higher court > lower, newer > older).

> One-line: let a more-specific / higher-authority rule **defeat** a conflicting general rule,
> so the engine derives the *governing* conclusion instead of all conclusions at once.

---

## 1. Motivation — why plain Datalog/NAF is not enough

The `rule {}` keystone gives Horn clauses with negation-as-failure. `enumerate_all` then
collects **every** derivation of a query (see `logic-engine/src/enumerate.rs`): it has no
notion that one derived conclusion should **override** another. That is fine for monotonic
knowledge (a drug's class, an enzyme deficiency) but wrong for the most common shape of real
rulebooks — **defaults with exceptions**, where conclusions conflict and a priority decides
which holds.

Three concrete drivers, one shared mechanism:

1. **MYCIN timing (CC-5, `chart_to_cop.decide_timing`).** The wait-vs-treat decision is a
   priority ladder: `culture resulted` ⟹ targeted **overrides** everything; else
   `time-critical OR unstable` ⟹ treat-now **overrides** the wait default; else
   `stable + routine + culture pending` ⟹ await; else ⟹ treat-now (conservative default).
   Encoding this in plain NAF means every lower rule must re-state the negation of every
   higher rule's guard — O(n²) fragile guards, and the "default" rule must enumerate
   "none of the above". A priority order states it directly.

2. **Specialist over general (medicine).** "Avoid β-lactams in severe penicillin allergy"
   (general) vs "aztreonam is safe in penicillin allergy" (specific exception, grounded in
   `ci_aztreonam_safe_penicillin`). The specific rule should **carve out** the general one.

3. **Context precedence (law — the north star).** The same term means different things by
   jurisdiction, and jurisdictions are ordered: **federal > state**, **higher court >
   lower**, **newer precedent > older**, **lex specialis** (specific statute > general). A
   contraindication/meaning derived under a governing context must defeat one derived under a
   subordinate context. This is McCarthy's context *lifting / specificity* relation made
   operational — and it is the SAME machinery as (1) and (2). Building it generically is the
   investment that makes the US-Code vision tractable.

**Design bar (from `feedback_generic_engines_over_domain_specific` +
`project_adj_universal_rule_substrate`):** the feature must be domain-neutral. If a primitive
reads as "medical", it is wrong. The test: *would this also express a statute overridden by a
higher court's reading?* Everything below is phrased to pass that test.

---

## 2. The two things precedence needs

Defeasance = **(a) a conflict relation** (when do two conclusions compete?) + **(b) a
priority relation** (which competitor wins?). Keep them orthogonal and both *declared in the
language*, never inferred.

### 2.1 Conflict — when do conclusions compete?

A derived head `H₁` **conflicts with** `H₂` when they cannot both hold. Two declaration
styles, smallest-first:

- **Functional predicates (the common case).** Declare a predicate *functional on its last
  argument*: at most one value may hold for a given key. `timing(X)` is functional (one
  decision); `means(term, reading, context)` is functional on `reading` per `(term, context)`.
  Syntax sketch in the dictionary:
  ```
  define timing : decision functional        % at most one timing(_) may survive
  define contraindicated : relation from drug to clinical_context   % NOT functional — many may hold
  ```
  Two derivations of `timing(await)` and `timing(treat_now)` conflict automatically.

- **Explicit conflict sets** for the irregular cases:
  ```
  conflict { await_culture, treat_now_empiric, targeted }   % pairwise mutually exclusive
  ```

A conclusion with **no** declared conflict is never defeated — monotonic knowledge (drug
classes, enzyme edges) keeps today's `enumerate_all` semantics exactly. **This is the
backwards-compatibility guarantee: precedence is opt-in per predicate.**

### 2.2 Priority — which competitor wins?

Attach an ordering to rules. Two layers, used together:

- **Explicit rule priority** — a non-negative integer (or named tier) on a rule; higher
  defeats lower among *conflicting* heads:
  ```
  rule { head: timing(targeted)        when: culture_status(resulted)            priority: 30 }
  rule { head: timing(treat_now)       when: disease_acuity(time_critical)       priority: 20 }
  rule { head: timing(await_culture)   when: stable, routine, culture_pending     priority: 10 }
  rule { head: timing(treat_now)       when: true                                priority: 0  }  % default
  ```
- **Context specificity (the generic precedence)** — when rules fire under different
  *contexts* (the `active_context` convention from the contraindication rulebook), a partial
  order over contexts induces rule priority *without* hand-numbering. Declared once:
  ```
  context_order { federal > state }            % law
  context_order { specialist > general }       % medicine
  context_order { idsa_2024 > idsa_2004 }      % newer guideline > older
  ```
  A conclusion grounded in a context that is **greater** in the order defeats a conflicting
  conclusion grounded in a lesser context. Numeric `priority` is the degenerate
  total-order case; `context_order` is the general partial order and is what the legal vision
  needs. The two combine: explicit `priority` breaks ties the context order leaves unordered.

**Why both:** numeric priority is ergonomic for a small local ladder (timing); context order
is the scalable, *declared-once* mechanism for thousands of cross-referencing rules where
hand-numbering is infeasible (the US Code). Numeric priority lowers to a trivial total
`context_order` internally, so the engine has one mechanism.

---

## 3. Semantics — how the engine resolves defeat

Resolution is a **post-pass over the derivations `enumerate_all` already produces** — it does
not change SLD search, so monotonic queries are byte-identical to today.

1. **Enumerate** all proofs of the query (current behavior).
2. **Group** surviving head instantiations by conflict class (functional key or explicit
   conflict set). A group with one member → no contest.
3. **Resolve** each contested group by the priority/context order:
   - Compute the max element(s) under the order among the group's rules.
   - If there is a **unique** maximum → that conclusion **wins**; the others are **defeated**
     (marked, not deleted — see provenance).
   - If the maximum is **tied / incomparable** (two equal-priority rules, or two
     order-incomparable contexts) → **CONFLICT**: surface *both* with an
     `undefeated_peers` marker. The engine never silently picks one. (This mirrors the
     existing `INDETERMINATE/CONFLICT` differential stance —
     `feedback_deterministic_is_probabilistic_special_case`.)
4. **Emit** the winners as the answer; defeated/peer derivations remain in the proof DAG,
   tagged, so the audit trail shows *what was overridden and by what* — never discarded.

**Interaction with probability / the differential.** Precedence is about *which clause
governs*, the differential is about *weight of evidence*. They compose: defeat prunes the
rule set that feeds a hypothesis **before** weighted-model-counting / LR aggregation, so a
defeated default never contributes evidence. A defeated rule's `probability` is irrelevant
once it is defeated. (Deterministic precedence is the special case where probabilities are
all `Certain` — consistent with the one-engine principle.)

**Negation-as-failure interaction.** NAF in a body sees only **undefeated** facts/heads, so
"unless a higher rule applies" falls out for free: a default rule body can stay
`when: requires_x` without enumerating every exception, because a higher rule's win defeats
the default in the conflict group rather than via the default's body. This is the ergonomic
payoff over hand-written NAF guards.

---

## 4. Surface syntax (additions only — fully backward compatible)

```
# dictionary: opt a predicate into functional-conflict
define_kind        = … | "decision" [ "functional" ] | "relation" "from" IDENT "to" IDENT [ "functional" ] ;

# rule: optional priority annotation (after the body, before the close brace)
rule_decl          = "rule" LBRACE "head" COLON term "when" COLON body_literal { COMMA body_literal }
                       { annotation } [ "priority" COLON INT ] RBRACE ;

# top-level conflict + context-order declarations
conflict_decl      = "conflict" LBRACE term { COMMA term } RBRACE ;
context_order_decl = "context_order" LBRACE IDENT ">" IDENT { COMMA IDENT ">" IDENT } RBRACE ;
```

All four are additive: existing rulebooks parse and run unchanged (no `functional`, no
`priority`, no `conflict`, no `context_order` → today's enumerate-all semantics). `priority`,
`conflict`, `context_order` are IDENT-matched literals, not new lexer tokens (consistent with
`rule`/`relate`/`define`).

---

## 5. Worked examples

### 5.1 MYCIN timing (replaces `decide_timing`'s if/elif ladder)
```
define timing : decision functional
rule { head: timing(targeted)      when: culture_status(resulted)                         priority: 30 }
rule { head: timing(treat_now)     when: disease_acuity(time_critical)                    priority: 20 }
rule { head: timing(treat_now)     when: clinical_status(critical)                        priority: 20 }
rule { head: timing(await_culture) when: clinical_status(stable), disease_acuity(routine),
                                          culture_status(pending)                          priority: 10 }
rule { head: timing(treat_now)     when: any_case                                         priority: 0  }
? timing($D)
```
A stable, routine, culture-pending patient derives `await_culture` (pri 10) **and** the
default `treat_now` (pri 0); they conflict (functional `timing`); pri 10 wins →
`timing(await_culture)`. A time-critical patient derives `treat_now` (20) and `await_culture`
does not fire → `treat_now`. The Python ladder's priority is now *declared*, and the proof
DAG shows the default was defeated by the acuity rule.

### 5.2 Specialist exception (medicine)
```
context_order { specific > general }
rule { head: contraindicated($D, allergy) when: in_context(general),  has_class($D, betalactam) }
rule { head: safe($D, allergy)            when: in_context(specific), is($D, aztreonam) }
conflict { contraindicated, safe }   % per (drug, context)
```
Aztreonam derives both `contraindicated(aztreonam, allergy)` (general) and
`safe(aztreonam, allergy)` (specific); `specific > general` ⟹ `safe` wins for aztreonam,
while β-lactams keep `contraindicated` (no specific rule fires). No hand-written NAF.

### 5.3 Legal context (the north star)
```
context_order { federal > state, ninth_circuit > district_court }
rule { head: means(navigable_waters, broad)  when: in_context(ninth_circuit) }
rule { head: means(navigable_waters, narrow) when: in_context(district_court) }
? means(navigable_waters, $Reading)
```
Under a case in both contexts, `ninth_circuit > district_court` ⟹ the broad reading governs;
the narrow reading stays in the DAG, tagged "defeated by ninth_circuit". Tie/incomparable
courts ⟹ CONFLICT surfaced, not silently resolved.

---

## 6. Implementation plan (incremental PRs, specs-first)

- **PR-1 (engine core — functional conflict + integer priority).** `logic-engine`: add
  `priority: i64` to `Rule` (default 0, builder `with_priority`) and `functional_predicates`
  to `KnowledgeBase` (`declare_functional(functor, arity)` — a predicate is functional on its
  last argument, keyed by the preceding args). New `enumerate_governing(query, kb)` =
  `enumerate_all` + the §3 resolution post-pass over functional conflict groups, returning a
  `GovernedResult` (each answer tagged `Governing` / `Defeated { by }` / `ConflictPeer`).
  `enumerate_all` itself is untouched (back-compat). Unit tests: unique-winner, tie→conflict,
  non-functional-unchanged (monotonic), fact-beats-rule. **Scope note:** PR-1 ships the
  functional-predicate conflict relation + total integer priority only; **explicit
  `conflict {}` sets and the `context_order` partial order move to PR-1b** (they reuse the same
  resolution post-pass — only the conflict-grouping and priority-derivation inputs grow).
- **PR-2 (adj-lang surface).** Grammar (§4) + AST + adapter + lower; `functional`/`priority`/
  `conflict`/`context_order` lower to the engine `ConflictPolicy`. Regenerate
  `_parser_grammar.rs`. Tests: each construct lowers; back-compat (no annotations) identical.
- **PR-3 (CLI + provenance).** adj-lang-cli emits a `governed`/`defeated` section
  (winner + the rules/contexts that defeated each loser) so the audit trail shows the
  override chain. JSON-render the conflict/peer case.
- **PR-4 (MYCIN CC-5 refactor).** Replace `decide_timing` with a `timing.adj` rulebook (§5.1)
  + a `derive_timing(cli, facts)` runtime (same pattern as `contraindications.py` /
  `step_therapy.py`); delete the Python if/elif. Behaviour-preserving (the 4 decisions +
  delay-risk + grounded threshold provenance kept).
- **PR-5 (specialist/allergy + legal demo).** Use `context_order` to ground the
  aztreonam-safe carve-out (§5.2) and ship a non-medical legal worked example (§5.3) proving
  the mechanism is domain-neutral.

Each PR: spec-sync note, tests incl. a CONFLICT/abstain case, `/security-review`, babysit.

---

## 7. Open design questions (for review before PR-1)

1. **Priority scope.** Per-rule integer vs named tiers (`priority: authoritative` mapped to a
   number)? Named tiers read better in grounded rulebooks and align with the `trust` ladder —
   lean named, with integers as the escape hatch.
2. **Context order source.** Should `context_order` be *grounded* (a precedence itself has a
   source — e.g. the Supremacy Clause for federal > state) rather than authored? Likely yes,
   long-term: precedence edges are facts in the CAS with provenance, same as any other.
3. **Transitivity / cycles.** `context_order` is a strict partial order; the loader must
   reject cycles (`a > b, b > a`) at compile time. Lex-specialis vs lex-superior can conflict
   (a specific *lower* rule vs a general *higher* one) — do we need rule-level priority to
   adjudicate *between ordering principles*? Defer to PR-5 once real cases exist; PR-1 ships
   single-dimension orders.
4. **Defeat vs probability discount.** Hard defeat (loser contributes nothing) vs soft
   (loser's probability is discounted)? Start hard (matches statutes/clinical contraindication);
   soft defeat is a later `ADJ` extension if a domain needs graded override.

---

## 8. Why this is the right next ADJ-language investment

It is the smallest primitive that simultaneously (a) makes the MYCIN timing/allergy rules
clean, (b) expresses *every* "default with exceptions" rulebook (the dominant real-world
shape), and (c) is exactly the context-specificity operator the legal-corpus north star
needs. One generic mechanism, three streams unblocked — and it composes with, rather than
replaces, the probabilistic differential. Build it once, in the substrate.
