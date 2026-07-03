# REL-1 — Relational recall: fact-lookup as a binding query (the board-exam substrate)

**Directive (2026-06-14):** *"The long-term goal would be to get this to a point where it
can pass any of these exams … subspecialty by subspecialty, organ by organ. Even fact recalls
should be possible. 'Which enzyme are you deficient in' should be answerable with a query or a
constraint query, right?"*

Yes — and this document specs **why fact recall is the same engine we already have**, plus the
one representational layer we must add to make it native: a **grounded relational (Datalog/Prolog)
clause type with binding queries**, on top of the existing likelihood-ratio differential.

> Invariant, unchanged: decision **support**, never replacement. A recalled fact is returned with
> a **proof** — the byte-provenanced edge(s) that justify it and their citation — and the system
> **abstains** (UNKNOWN) rather than fabricate an answer when no grounded edge supports it. Humans
> correct edges; they do not author them ([[feedback_nothing_human_authored]]).

---

## 1. The thesis: recall is the single-hop, zero-uncertainty special case of the differential

Watch one board question arrive two ways and collapse onto the **same grounded edge**:

```
FORWARD recall  "Tay-Sachs disease is a deficiency of which enzyme?"
    ?  deficient_in(tay_sachs, $Enzyme)              ← one relational hop
    ⇒  $Enzyme = hexosaminidase_a   [proof: OMIM #272800 edge, trust authoritative]

REVERSE diagnostic  "Ashkenazi infant, cherry-red macula, hyperacusis, regression —
                     which enzyme is deficient?"
    ?  disease                                        ← differential (priors + LR contributions)
    ⇒  tay_sachs  (MAP, posterior 0.8x)              [proof: the contributing findings]
    ?  deficient_in(tay_sachs, $Enzyme)              ← SAME final hop
    ⇒  $Enzyme = hexosaminidase_a
```

Both terminate in the identical edge `deficient_in(tay_sachs, hexosaminidase_a)`. The only
difference is how the first argument got bound: **stated** (forward recall, zero uncertainty) vs
**inferred by the differential** (reverse). This is exactly the principle we already committed to —
**deterministic is a special case of probabilistic; one engine, not two**
([[feedback_deterministic_is_probabilistic_special_case]]). Fact recall is the deterministic,
single-hop end of the same continuum the LR differential lives on.

This is the concrete first step of the long-planned move to make reasoning **CPU-bound** by
distilling knowledge into a grounded, correctable clause library — LLM as *translator*, solver as
*reasoner*, the proof tree as machine-checked recall ([[project_cpu_bound_reasoning_problog]]).

### Why this keeps "pass any board" from becoming N expert systems

Every board question type is a **query tactic over one grounded store**:

| Board question type | Query tactic over the same CAS | Status |
|---|---|---|
| "Most likely diagnosis" | argmax posterior (LR differential) | ✅ native today |
| "Which enzyme / inheritance / accumulated substrate / mechanism" | **relational binding / path query** | ⛳ this spec |
| "Next best step / management" | constraint solve (chart-as-constraints, CC arc) | 🟡 partial |
| "Pre/post-test odds, sensitivity, NNT" | the LR engine, read off directly | ✅ near-free |

One store, several tactics. Organ-by-organ build-out = adding **nodes + edges to one knowledge
graph**, each byte-provenanced, each scored by the board-eval harness (§7).

---

## 2. What exists today, and the exact gap

adj-lang today (`code/grammars/adj_lang.grammar`) is a **differential engine plus a constraint
sublanguage**. Its clause kinds: `prior N for term`, `contributes N from evidence to term`
(predicate-gated evidence included), `interacts`, `uncertain`, `observe`, `let`/arithmetic,
`symbol`/`constrain`/`solve`/`check`/`minimize`/`maximize`, `dictionary`/`define`,
`rulebook`/`use`, `import`. Queries are `query_decl = QUESTION term` and return a **posterior over a
hypothesis term**.

Two things are missing for recall:

1. **Ground relational edges as a first-class clause** — there is no way to assert
   `deficient_in(tay_sachs, hexosaminidase_a)` as a typed, provenanced fact. `contributes`/`prior`
   are about *belief over hypotheses*, not *relations between entities*.
2. **Variables + binding queries** — every `term` today is **ground**
   (`term = IDENT [ LPAREN (term|NUMBER){…} RPAREN ]`; no variable production). A query returns a
   number for a named hypothesis; it cannot return *which entity* satisfies a relation.

REL-1 adds exactly these two, reusing every existing idiom (annotations, dictionaries, `import`,
the `?` query lead, the proof DAG, the trust tiers).

---

## 3. The surface design (grammar extension — specs only; lowering staged to REL-2/3)

### 3.1 Typed nodes and edges in the dictionary

Extend `define_kind` so the controlled vocabulary — the linchpin that keeps the decomposer and the
rulebook on the same terms — can also declare **entity node types** and **relation edge types**
(with a domain → range signature, so the graph is typed and the compiler can reject an edge whose
arguments are the wrong kind):

```
define_kind = "hypothesis"
            | "finding" [ "values" LBRACK IDENT { COMMA IDENT } RBRACK ]
            | "entity"                                              # NEW: a node kind (disease, enzyme…)
            | "relation" "from" IDENT "to" IDENT                    # NEW: a typed edge kind
            ;
```

```adj
dictionary biochem_iem {
    define disease      : entity   surface "inborn error of metabolism", "metabolic disease"
    define enzyme       : entity   surface "enzyme", "enzyme deficiency"
    define substrate    : entity   surface "accumulated substrate", "stored material"
    define inheritance  : entity   surface "inheritance pattern"

    define deficient_in : relation from disease to enzyme
    define accumulates  : relation from disease to substrate
    define inherited_as : relation from disease to inheritance
}
```

### 3.2 Ground edges — the `relate` clause

A new statement asserts a **ground relational fact**, carrying the same `source`/`locator`/`trust`
annotations every grounded clause already carries (so an edge is byte-provenanced and one CAS edit
from correctable, exactly like a `contributes` LR):

```
relate_decl = "relate" IDENT LPAREN term { COMMA term } RPAREN { annotation } ;
```

```adj
rulebook iem_facts {
    use biochem_iem

    relate deficient_in(tay_sachs, hexosaminidase_a)
        source "Tay-Sachs disease results from deficient hexosaminidase A (HEXA)."
        trust authoritative
    relate accumulates(tay_sachs, gm2_ganglioside)
        source "GM2 ganglioside accumulates in Tay-Sachs disease."
        trust authoritative
    relate inherited_as(tay_sachs, autosomal_recessive)
        source "Tay-Sachs is inherited in an autosomal recessive pattern."
        trust authoritative
}
```

`relate` is an IDENT-matched literal (no lexer keyword), consistent with `solve`/`symbol`/`rulebook`.
The compiler checks each edge against the `use`d dictionary: the relation must be `define`d and its
arguments must be `entity` terms of the declared domain/range type — the same enforcement that today
rejects an undefined finding.

### 3.3 Variables and the binding query

Introduce a **variable sigil** `$` (a new `VAR` token = `DOLLAR IDENT`), unambiguous against the `?`
query lead and against ground lowercase idents. Queries generalize to **relational goals that may
contain variables**:

```
query_decl = QUESTION goal ;
goal       = IDENT [ LPAREN garg { COMMA garg } RPAREN ] ;
garg       = goal | NUMBER | VAR ;          # VAR = $Name  (a logic variable)
```

- **No variables, hypothesis term** → the existing posterior query (100% backward compatible).
- **Contains a variable, names a relation** → a **binding query**: solve the goal against the
  ground edges and return the binding(s) **with a proof**.

```adj
? deficient_in(tay_sachs, $Enzyme)      ⇒  { $Enzyme = hexosaminidase_a }   + proof edge + citation
? inherited_as(tay_sachs, $Pattern)     ⇒  { $Pattern = autosomal_recessive }
? deficient_in($Disease, hexosaminidase_a)   ⇒  { $Disease = tay_sachs }    (reverse lookup, free)
```

### 3.4 Engine semantics (REL-3)

The relational layer is **SLD-resolution / Datalog** over the ground edge set:

- **Slice-1 (this arc):** single-hop unification — match the goal against ground `relate` edges,
  unify variables, return all bindings, each with the matched edge's `Provenance` as a one-node
  proof DAG. Zero model calls at answer time (warm-path thesis preserved).
- **Later (REL-6+):** derived relations via rules
  (`deficient_enzyme(D,E) :- catalyzes(E,Step), blocked_step(D,Step)`), multi-hop proof trees,
  and *valued* relations (so a relation can also carry an LR back into the differential — the two
  tactics fully unified). Out of scope here; noted so the surface doesn't paint us in.

**Composition with the differential** is the reverse-diagnostic path in §1: run `? disease` to get
the MAP hypothesis, bind it into the recall goal, run the binding query. One CLI, one store, two
tactics, one combined proof.

---

## 4. The IEM pilot (inborn errors of metabolism — the densest fact-recall region)

IEM is chosen because **enzyme-deficiency recall is the canonical, cleanest fact-recall family** in
all of medicine (it is the user's own example), it is a Pediatrics/biochem board staple, and each
disease has a tight star of high-value edges (deficient enzyme, accumulated substrate, inheritance,
classic finding). Pilot scope — a handful of diseases, each with its edge star:

| disease | `deficient_in` | `accumulates` | `inherited_as` |
|---|---|---|---|
| Tay-Sachs | hexosaminidase A | GM2 ganglioside | autosomal recessive |
| Gaucher | glucocerebrosidase | glucocerebroside | autosomal recessive |
| Phenylketonuria | phenylalanine hydroxylase | phenylalanine | autosomal recessive |
| Pompe | acid α-glucosidase | glycogen (lysosomal) | autosomal recessive |
| Lesch-Nyhan | HGPRT | uric acid | X-linked recessive |
| von Gierke (GSD I) | glucose-6-phosphatase | glycogen / G6P | autosomal recessive |

Every edge enters the CAS **only through the grounding pipeline** (spider → byte-provenance →
adversarial gate → committed edge), per [[feedback_nothing_human_authored]]. The grounding workflow
for edges mirrors the existing per-claim spiders (one agent grounds the edge against a primary source
— OMIM, a biochemistry reference — with a verbatim byte-quote; an independent agent re-extracts and
tries to refute; the gate emits ACCEPT(trust)/FLAG/DROP). Until an edge is spider-grounded it is
carried as **authored-debt** in the ledger (the same honesty boundary the formulary started behind),
so debt is visible and drives to zero.

### The two worked vignettes (the end-to-end proof)

1. **Forward recall.** Question stem: *"Tay-Sachs disease results from a deficiency of which
   enzyme?"* → decompose to `? deficient_in(tay_sachs, $E)` → binding `hexosaminidase_a` + the OMIM
   edge citation. **0 answer-time model calls.**
2. **Reverse diagnostic→recall.** Vignette: *"Ashkenazi-Jewish infant, normal at birth, now 8 months
   with developmental regression, hyperacusis, and a cherry-red macula."* → differential `? disease`
   ranks `tay_sachs` (cherry-red macula + regression + ancestry as LR contributions) → bind →
   `? deficient_in(tay_sachs, $E)` → `hexosaminidase_a`, with a **combined proof**: the findings that
   selected the disease *and* the edge that named the enzyme.

Both are answerable, auditable, and correctable — and the system **abstains** if asked about a
disease whose edges are not grounded, rather than guessing.

---

## 5. Staging (specs-first; one PR per slice, babysit to green)

- **REL-1 (this PR):** this spec + an **executable relational-recall prototype** (`recall/recall.py`:
  a ground-edge store + single-hop binding query + proof DAG + abstention) running over a small
  **illustrative IEM edge set clearly marked authored-debt**, with a test proving the forward + reverse
  vignettes resolve with citations and 0 answer-time calls, and a **ledger artifact** so the debt is
  visible. Proves the end-to-end *semantics* deterministically before touching the Rust grammar.
- **REL-2:** grammar — add `entity`/`relation` `define_kind`s, the `relate` clause, the `VAR` token,
  and the binding `query`/`goal` productions; regenerate the parser; AST + lower into a
  `logic-engine` relation store. `cargo build --workspace`.
- **REL-3:** engine — SLD single-hop resolver + binding-query API + proof DAG, wired into the adj-lang
  CLI so `? deficient_in(tay_sachs, $E)` runs natively and serializes the binding + proof to JSON.
- **REL-4:** spider-ground the IEM edge star (retire REL-1's authored-debt → grounded), via the edge
  grounding workflow + write gate; rebuild the ledger.
- **REL-5:** board-eval harness (§7) — score the IEM recall items + the existing ID/meningitis/BSI
  differentials; report hit-rate **and abstention discipline**.
- **REL-6+:** derived (multi-hop) relations, valued relations feeding the differential, and the next
  organ system.

---

## 6. Critical files (reuse, not rebuild)

- **Grammar/engine:** `code/grammars/adj_lang.grammar`, `code/packages/rust/adj-lang/src/{lower.rs}`,
  `code/packages/rust/logic-engine/src/{proof_dag.rs,provenance.rs}`, the adj-lang CLI.
- **Grounding:** `grounding/harness.py` (gate + SourceObject + verify_citation + build_ledger),
  `grounding/ground_sources.py`, the per-claim spider workflows (edge spider models on these),
  `diagnosis/organisms/organism_id_ground.py` (`gate`/`cite`/`safe_status` reuse).
- **New (under `code/specs/data/mycin-2026/recall/`):** this spec, `recall.py`, `iem-edges.adj`
  (illustrative seed, REL-1) → grounded edges (REL-4), `test_recall.py`, the IEM dictionary.

---

## 7. The board-eval harness (REL-5, sketched here to anchor the metric)

A retired/public NBME-style item subset, each item tagged with the query tactic it needs
(`differential` | `recall` | `management` | `biostat`). For each item the harness runs the pipeline
end-to-end and records **three** outcomes, not one:

- **correct** — answered, matches the key, with a proof;
- **abstained** — returned UNKNOWN because no grounded edge/clause supports an answer (the *honest*
  failure — this is a feature, the discriminator vs a hallucinating recall LLM);
- **wrong** — answered incorrectly (the only real failure).

The headline metric is the **defensibility curve**: on the covered subset, near-100% correct-with-
proof; on the uncovered subset, abstain rather than fabricate. Coverage (facts/edges per domain)
becomes a **live number every grounding PR moves**, replacing the rough facts-per-domain estimates
with measured ones — and turning "pass any board" into a graph we can watch climb, organ by organ.

---

## 8. Non-goals (so slice-1 stays small)

Native adj-lang lowering (REL-2/3 — REL-1 prototypes in Python first); multi-hop/derived relations
and valued relations feeding the differential (REL-6+); the full IEM edge set or other organ systems
(breadth follows the pilot); automated PDF/OMIM retrieval beyond the existing spider; probabilistic
relations (a relation that is only *usually* true) — modeled later as a relation carrying an LR, the
same unification that makes recall and differential one engine.
