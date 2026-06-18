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

> **Design decisions (2026-06-16, user).** (1) Explicit priority uses **named enum tiers**, not
> raw integers. (2) Context precedence is **itself grounded** — its own rulebook, each edge
> byte-provenanced (§2.3). (3) The principle that *resolves a precedence conflict* (lex-specialis
> vs lex-superior vs recency vs appeal-status) is **also a grounded rule** — precedence is
> recursive, and every layer carries byte-provenance. (4) Cycles are rejected at load.

Attach an ordering to rules. Three layers, weakest-to-strongest in generality:

- **Explicit rule priority — a NAMED ENUM tier** (not a magic integer) on a rule; a higher tier
  defeats a lower one among *conflicting* heads. The tiers are domain-neutral and totally
  ordered:
  ```
  rule { head: timing(targeted)        when: culture_status(resulted)         priority: mandatory  }
  rule { head: timing(treat_now)       when: disease_acuity(time_critical)    priority: authoritative }
  rule { head: timing(await_culture)   when: stable, routine, culture_pending priority: specific }
  rule { head: timing(treat_now)       when: any_case                         priority: default }  % fallback
  ```
  Proposed tier ladder (lowest→highest), open to your edit:
  `default < specific < authoritative < mandatory`. `default` is the implicit tier when none is
  written (so existing rules are unchanged). A bare ground **fact** sits above all tiers
  (asserted truth). Integers are *not* exposed in the surface; internally the enum has a total
  order so the resolver can compare.
- **Context specificity (the generic precedence)** — when rules fire under different
  *contexts* (the `active_context` convention from the contraindication rulebook), an order
  over contexts induces rule priority *without* hand-numbering. But — **decision (2)** — that
  order is **not config**. It is a grounded relation in its own rulebook (§2.3).

- **Grounded conflict-resolution principles (the recursive layer, §2.3)** — when an explicit
  tier and a context order *disagree* (a `specific` lower-court rule vs an `authoritative`
  higher-court rule), which wins is itself decided by **grounded meta-rules**, not a hardcoded
  precedence-of-precedence. Recency, appeal status, court level, and lex-specialis are each a
  byte-provenanced rule (§2.3).

### 2.3 Precedence is grounded and recursive (decisions 2 + 3)

> The headline architectural commitment. "Everything in this work is recursive and each has its
> own byte provenance." Precedence is not a hardcoded `max(priority)`; it is **derived** by a
> grounded **precedence rulebook**, the same way every clinical/legal fact is derived.

1. **Context order edges are grounded facts.** `federal > state` is not config — it is
   `outranks_context(federal, state)` **with a source**: the Supremacy Clause (US Const. Art.
   VI, cl. 2), a byte-quote, a locator, a trust tier. It enters the CAS through the *same*
   spider → provenance → adversarial-gate pipeline as any other fact
   (`feedback_nothing_human_authored`). Its own rulebook: `context-precedence.adj`.

2. **Rules carry typed attributes, not a baked-in rank.** A rule (esp. a grounded legal/clinical
   one) records *why it might out- or under-rank another*: its **authority level** (court /
   guideline-body), **decision/effective date**, **appeal status** (good-law / reversed /
   vacated / superseded), and **specificity**. These are typed (dates via datetime-core, etc.)
   and themselves provenanced.

3. **The conflict-resolution PRINCIPLES are grounded meta-rules.** "Which rule governs" is a
   derived relation `outranks($R1, $R2)`, produced by rules like — each byte-provenanced:
   - *lex superior*: a higher authority outranks a lower (`outranks_context` grounded in the
     hierarchy's charter).
   - *stare decisis / recency*: a later **controlling** opinion supersedes an earlier one (cite
     the doctrine).
   - *good-law gate*: a rule **reversed/vacated on appeal** is defeated outright (cite the
     reversing decision — recursive: the defeater is itself a grounded rule).
   - *lex specialis*: the more specific provision governs the general — **and when lex specialis
     and lex superior point opposite ways, a further grounded meta-rule decides** (e.g. "a
     specific statute controls over a general one unless the general is constitutional"), cited
     to the governing canon. There is no built-in tiebreaker the engine invents; if no grounded
     meta-rule resolves it, the result is `CONFLICT` (abstain), surfaced honestly.

   So the resolver does not compare integers — it **queries the precedence rulebook**:
   `? outranks($winner, $loser)` over the conflicting rules' attributes. The named-enum tier
   (§2.2) is just the simplest grounded meta-rule ("a higher explicit tier outranks a lower"),
   used for local ladders where no richer principle applies.

4. **Cycles are rejected at load (decision 4).** `outranks_context` (and any derived `outranks`
   that is asserted as ground) must be acyclic; the loader runs a cycle check and refuses a
   rulebook that asserts `a > b, b > a`. Derived `outranks` from meta-rules is checked for
   contradiction (both `outranks(A,B)` and `outranks(B,A)` derivable ⇒ `CONFLICT`, not a pick).

**Why this is the right shape:** it makes the legal-corpus vision tractable *and honest* — the
reason one authority beats another is auditable down to the clause, the system never invents a
hierarchy, and "the law changed / was overruled" is a CAS edit to a grounded rule that
re-propagates. It is the McCarthy context-lifting relation, but *every lift is justified by
cited bytes*. Medicine uses the identical machinery (specialist guideline > general; a
retracted study defeats its claims; a newer IDSA edition supersedes an older).

**Why explicit tiers still exist:** a named-enum tier is ergonomic for a small local ladder
(timing) where authoring a full grounded meta-rule would be overkill; it is the degenerate
grounded principle "higher tier wins". The scalable mechanism for thousands of cross-referencing
rules (the US Code) is the grounded `outranks` rulebook.

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

## 7. Resolved design decisions (2026-06-16, user)

1. **Priority = named enum tiers**, not raw integers (§2.2). Proposed ladder
   `default < specific < authoritative < mandatory` (open to edit); `default` implicit; a ground
   fact sits above all tiers. Integers are not exposed in the surface.
2. **Context precedence is grounded, in its own rulebook, with provenance on WHY** (§2.3). Each
   `outranks_context` edge cites its charter (Supremacy Clause for federal > state, etc.) and
   enters via the standard spider → gate → CAS pipeline. New artifact: `context-precedence.adj`.
3. **Conflict-resolution principles are themselves grounded, recursive meta-rules** (§2.3) —
   lex-superior, recency/stare-decisis, appeal-status (reversed ⇒ defeated), lex-specialis, and
   the meta-meta-rule for when specialis and superior disagree. The resolver *queries* the
   precedence rulebook (`? outranks($w,$l)`); it never invents a hierarchy. Unresolved ⇒
   `CONFLICT` (abstain). Rules carry typed, provenanced attributes (authority level, decision
   date, appeal status, specificity) that the meta-rules read.
4. **Cycles rejected at load**; contradictory derived `outranks` ⇒ `CONFLICT`, not a silent pick.
5. **Defeat is hard** for now (a defeated rule contributes nothing). Soft probability-discount is
   deferred unless a domain needs graded override. *(This was the one question with no strong
   user signal; hard defeat is the conservative default and matches statutes/contraindications.)*

### Revised PR staging (supersedes §6's PR-1b/PR-2/PR-5 framing)

- **PR-A — named-enum priority (engine).** Replace `Rule.priority: i64` with `Priority` enum
  (`Default < Specific < Authoritative < Mandatory`, `Ord`; fact = above all). Update
  `with_priority`, `enumerate_governing` comparison, the 2 adjudication-connector sites, tests.
- **PR-B — grounded `context-precedence` rulebook + `outranks` resolver.** A grounded rulebook
  of `outranks_context` edges (each byte-provenanced) + the meta-rules (lex-superior / recency /
  appeal-status / lex-specialis). Engine resolution queries `outranks` instead of comparing enum
  tiers when rule attributes are present; cycle/contradiction → CONFLICT. Rule attributes
  (authority/date/appeal/specificity) added as typed, provenanced metadata.
  - **PR-B engine core ✅ DONE (logic-engine 0.19).** `Rule::context` + `Knowledge­Base::{add_context_outranks,
    context_outranks (cycle-safe), context_order_has_cycle}` + a `defeats(a,b)` resolution that
    makes context precedence PRIMARY (lex superior) and the tier secondary — generalizing the
    pure-tier rule (no context order ⇒ unchanged). A cyclic order crowns nothing (safe).
  - **PR-B surface ✅ DONE (adj-lang 0.16).** `context:` on a rule + `context_order { a > b }`.
  - **PR-B-2 grounded edges ✅ DONE (logic-engine 0.20).** A ground `outranks_context(higher,
    lower)` **fact** now participates in the context order exactly like an explicit edge, so the
    precedence edge carries `source`/`locator`/`trust` provenance (the *reason* — Supremacy
    Clause, circuit precedence, guideline year — rides on the edge, not host code). `context_edges()`
    unions explicit + grounded edges; cycle detection spans both. This is the decision-§2.3
    keystone: "context precedence is itself grounded, in its own rulebook, with provenance on WHY."
  - **PR-B-3 grounded rulebook + worked example ✅ DONE (adj-lang-cli 0.8).**
    `code/specs/data/context-precedence/`: a grounded `context-precedence.adj` rulebook —
    `outranks_context(federal, state)` byte-quoting the Supremacy Clause and
    `outranks_context(ninth_circuit, district_court)` byte-quoting vertical stare decisis (verbatim
    primary-source quotes + locator + `authoritative` tier; `SOURCES.md` ledger) — plus a worked
    legal example that `import`s it and proves *lex superior* end-to-end through the CLI: the
    circuit's broad reading **governs** a district court's narrow reading **despite its higher
    `mandatory` tier**. Governing answers now carry their `context` in the JSON. Golden test
    `adj-lang-cli/tests/context_precedence_e2e.rs`.
  - **PR-B-4 grounded meta-rules ✅ DONE (logic-engine 0.21).** `context_adjacency` now reads
    **rule-derived** `outranks_context` edges (enumerated via `enumerate_all` when any
    `outranks_context/2` rule exists; the cheap ground-fact scan is kept otherwise). So the
    conflict-resolution canons are themselves grounded meta-rules in `context-precedence-meta.adj`:
    `outranks_context($H,$L) :- reverses($H,$L)` (appeal status, citing the overruling doctrine)
    and `… :- supersedes($New,$Old)` (lex posterior / recency, citing implied-repeal). Two worked
    examples prove it end-to-end through the CLI — a Supreme Court reversal flips a `mandatory`-tier
    reversed Ninth Circuit reading (`worked-appeal-example.adj`), and `idsa_2024 > idsa_2004` is
    *derived* from a grounded `supersedes` fact (`worked-supersession-example.adj`). The recursion
    bottoms out at cited primitive facts — an edge that can be derived is derived, not duplicated.
    Golden test `adj-lang-cli/tests/context_metarules_e2e.rs`.
  - **PR-B-5 lex specialis ✅ DONE (no engine change — pure grounded meta-rule on the PR-B-4
    machinery).** Added canon 3 `outranks_context($S,$G) :- more_specific($S,$G)` (citing *lex
    specialis derogat legi generali*) to `context-precedence-meta.adj` + `worked-lex-specialis-example.adj`
    (a specific wilderness-trail statute governs a general traffic statute despite the general's
    `mandatory` tier). Completes the three classical canons (lex superior / lex posterior / lex
    specialis).
  - **PR-B-6 §4.3 honest CONFLICT ✅ DONE (logic-engine 0.22).** When two canons point opposite
    ways (lex superior `federal > state` vs lex specialis `state > federal`) the defeat is MUTUAL;
    the resolver now uses **strict domination** (`j` defeats `i` only if `i` does not defeat back),
    so neither is silently `Defeated` — the group abstains as `ConflictPeer` / `has_conflict`. This
    closes the "else CONFLICT (abstain)" guarantee (previously the engine produced a misleading
    "both defeated, no conflict flag"). `worked-canon-conflict-example.adj` +
    `colliding_canons_abstain_with_an_honest_conflict`. Still to come (the RESOLVING tiebreaker):
    a grounded meta-rule that turns a *chosen* collision into a cited decision — needs each derived
    edge tagged with its canon + a grounded canon-ordering, so the engine picks with provenance.
- **PR-C — adj-lang surface** (`priority: <tier>`, `functional`/`decision`, the attribute
  annotations) + regen grammar.
- **PR-D — MYCIN `decide_timing` → `timing.adj`** on the named-enum ladder (was PR-4).
- **PR-E — legal `context-precedence` worked example** — the lex-superior half (federal>state /
  ninth_circuit>district_court grounded to the Supremacy Clause + vertical stare decisis) is
  ✅ DONE in **PR-B-3**; the reversed-on-appeal defeat folds into the appeal-status meta-rule (PR-B-4).

Each PR: spec-sync note, tests incl. a CONFLICT/abstain case, `/security-review`, babysit.

---

## 8. Why this is the right next ADJ-language investment

It is the smallest primitive that simultaneously (a) makes the MYCIN timing/allergy rules
clean, (b) expresses *every* "default with exceptions" rulebook (the dominant real-world
shape), and (c) is exactly the context-specificity operator the legal-corpus north star
needs. One generic mechanism, three streams unblocked — and it composes with, rather than
replaces, the probabilistic differential. Build it once, in the substrate.
