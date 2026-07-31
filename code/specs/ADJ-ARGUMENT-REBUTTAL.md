# ADJ-ARGUMENT-REBUTTAL — attack edges: rebutting and undercutting an argument

Status: **Spec-first** (2026-07-30). No code in this PR. The dialectic layer for
[`ADJ-ARGUMENT-IR.md`](ADJ-ARGUMENT-IR.md): a real paper does not only *support* its thesis — it
**rebuts alternatives, states limitations, and weighs competing hypotheses**. This spec adds
**attack edges** so a decomposed paper can represent, and the engine can *resolve*, its
disagreements — a defeated conclusion is **withdrawn by the engine**, not filtered in Python.

---

## 1. The two attack kinds

Following the standard argumentation distinction (and [`ADJ-ARGUMENT-IR.md`](ADJ-ARGUMENT-IR.md)
§3, which deferred attack edges to this doc):

- **REBUT** — attacks a **conclusion**. A rival conclusion, better supported (later evidence,
  higher authority, more specific), *defeats* an earlier one: "the reanalysis shows the fracture
  was a single-shear overload, **not** fatigue."
- **UNDERCUT** — attacks an **inference / warrant**. It does not assert a rival conclusion; it
  removes the *licence* for the step: "the beach-mark reading is invalid because the sample was
  contaminated" — so the fatigue inference no longer fires, without claiming any other mechanism.

## 2. Finding — both are ALREADY supported, with ZERO new engine code

Both attack kinds were **empirically verified against the built engine** (probe `.adj` runs, not
speculation). Each reuses machinery that already ships.

### 2.1 REBUT = ADJ73 defeasible precedence (proven)

Declare the conclusion predicate **`functional`** (one value per subject), tag each paragraph's
inference with a **`context:`**, and assert the paper's precedence with **`context_order`**. The
engine's `governing` query then marks the defeated conclusion `defeated` and the winner
`governing` — the rebutted conclusion is **withdrawn**:

```
functional failed_by(subject, mechanism)
rule { head: failed_by(axle, fatigue)  when: shows(surface, beach_marks) context: initial_report }
rule { head: failed_by(axle, overload) when: single_shear(surface)       context: reanalysis }
context_order { reanalysis > initial_report }
? failed_by(axle, $Mechanism)
```

→ `governing`: `failed_by(axle, fatigue)` **`status: defeated`** (context `initial_report`);
`failed_by(axle, overload)` **`status: governing`** (context `reanalysis`). The engine did the
defeat: no Python filter. This is exactly ADJ73 ([`ADJ73-defeasible-rule-precedence.md`](ADJ73-defeasible-rule-precedence.md)),
reused verbatim.

### 2.2 UNDERCUT = negation-as-failure guard (proven)

Guard the inference with a `not <warrant-defeater>` body literal; a rebuttal rule derives the
defeater from its own grounded condition. When the defeater holds, the inference **does not fire**
— no rival conclusion asserted:

```
rule { head: failed_by(axle, fatigue) when: shows(surface, beach_marks), not warrant_undercut }
rule { head: warrant_undercut when: contaminated_sample(surface) }
```

→ with `contaminated_sample(surface)` present, `? failed_by(axle, $M)` **abstains** (the warrant is
undercut); remove it and `fatigue` derives. This is plain negation-as-failure in a `rule` body,
already in the engine.

## 3. The grounding discipline (unchanged)

An attack is **not** a privileged escape hatch — it is itself grounded like everything else:

- A **rebuttal** is a paragraph's own inference (`rule`), so its premises are byte-anchored to that
  paragraph's snapshot exactly as [`ADJ-ARGUMENT-IR.md`](ADJ-ARGUMENT-IR.md) §4 requires; the
  `context_order` edge is the paper's own byte-cited precedence statement (a `relate`
  `outranks_context` fact, ADJ73 PR-B — grounded, not authored).
- An **undercut**'s defeater (`warrant_undercut`) is derived from a byte-anchored condition
  (`contaminated_sample`), so the *absence* that licenses the original inference is itself a
  checkable fact (the ADR §E.5 "negation quotes the empty proof" discipline).
- `adj-verify` re-anchors every attacking premise; `--explain` (§4 below) shows the withdrawal.

So the dialectic inherits the whole audit stack: a rebuttal that drifts from its source fails
`adj-verify` like any other citation.

## 4. What `adj-verify` and `--explain` show

- **DERIVE / govern**: the `governing` section already reports each answer's `status`
  (`governing` / `defeated` / `conflict_peer`) and its resolving `context` — that IS the rebuttal
  outcome, today.
- **`--explain`**: the ADR-6 renderer shows an argument's SLD chain. Rendering the *defeat* — "X
  was concluded (paragraph A) but is **withdrawn**, defeated by Y (paragraph B) under
  `reanalysis > initial_report`" — is a small addition to the argument surface of `--explain`
  (it reads the `governing` resolution the CLI already computes). This is an AR-3 rung, not new
  engine work.
- **`adj-verify`**: unchanged — it byte-anchors the attacking premises and re-derives; the
  defeat is a resolution over already-verified rules.

## 5. Decision: reuse the mechanism; the only gap is a thin argument-surface tag

**The defeat MACHINERY needs zero new engine/logic code** — ADJ73 does rebut, NAF does undercut,
both proven above. Per the generic-substrate principle ([[project_adj_universal_rule_substrate]]),
AR adopts them wholesale.

The **one gap** is at the `argument` surface: `infer … from <refs>` desugars to a plain `rule`
that carries **no `context:` tag** (needed for rebut) and whose `from` list is **positive-only**
(no `not` guard, needed for undercut). So a paper-level attack between two paragraph inferences
cannot be written *inside* the pure `argument` block yet. Two honest paths, both offered:

- **(a) Available today — raw rules alongside the argument (AR-2 uses this).** Express the
  supporting chain as `argument { … }` and the attack as a raw `rule { … context: … }` +
  `functional` + `context_order` (rebut) or a `not`-guarded `rule` (undercut). This mixes the two
  surfaces but needs **no new code** and is fully grounded/verifiable — it proves the dialectic
  end-to-end now.
- **(b) Ergonomic sugar (deferred, AR-3).** Extend the `argument` surface: allow `infer` to carry
  a `context:` (mirroring `rule`) and an optional `unless <defeater>` guard (desugaring to a
  `not` body literal), plus a `functional` note on the argument's thesis. This lets a whole paper
  — support **and** attack — live in one `argument` block. It is pure surface over the *same*
  proven desugaring; worth doing only once (a) shows the shape at paper scale.

AR-1 commits to **(a)** as the working model and specs **(b)** as the optional follow-up — the
same reuse-first stance AC-1 took for composition.

## 6. Worked sketch + staging

**Sketch (rebut).** A paper concludes fatigue (paragraph A); a later reanalysis paragraph rebuts
it with an overload finding under higher precedence; the engine withdraws fatigue and governs
overload — §2.1. **Sketch (undercut).** A methods-limitation paragraph reports a contaminated
sample; the fatigue warrant is undercut and the thesis abstains rather than asserting a rival —
§2.2.

- **AR-1 (this PR)** — the spec: the two attack kinds, their proven desugarings, the grounding
  discipline, the reuse decision. No code.
- **AR-2** — a worked **rebuttal** example end-to-end (committed data + e2e): a support chain
  (`argument`) + a grounded rebuttal (raw `rule` + `functional` + `context_order`, each premise
  byte-anchored to its paragraph's snapshot). Proves the engine **withdraws** the defeated
  conclusion (`governing` → `defeated`), `adj-verify` byte-anchors the rebuttal, and the winner
  governs. A companion **undercut** example (NAF-guarded warrant → thesis abstains when undercut).
- **AR-3** — the argument-surface sugar of §5(b) (`context:` + `unless` on `infer`) so support and
  attack compose in one block, plus the `--explain` "withdrawn / defeated-by" rendering.
- **Later** — the trained decomposer emits attack edges from a paper's rebuttal/limitation
  paragraphs (retarget the AD-1..5 scaffold to the attack surface).

## 7. Reuse map

- **Rebut mechanism**: [`ADJ73-defeasible-rule-precedence.md`](ADJ73-defeasible-rule-precedence.md)
  — `functional`, `context:`, `context_order`, the `governing`/`defeated` resolution.
- **Undercut mechanism**: negation-as-failure in `rule` bodies (logic-engine), the same NAF the
  ADR §E.5 audit already re-checks.
- **Argument surface + grounding + verify/explain**: [`ADJ-ARGUMENT-IR.md`](ADJ-ARGUMENT-IR.md)
  and [`ADJ-ARGUMENT-COMPOSITION.md`](ADJ-ARGUMENT-COMPOSITION.md) — attacks are grounded and
  audited exactly like premises and inferences.
