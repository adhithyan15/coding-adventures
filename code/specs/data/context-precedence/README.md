# context-precedence — grounded lex-superior precedence (ADJ73 PR-B-3)

A worked, byte-provenanced demonstration that **context precedence is itself grounded** — its
own rulebook, with a citation on *why* one context outranks another (ADJ73 decision §2.3).

## Files

| File | What |
|------|------|
| [`context-precedence.adj`](context-precedence.adj) | The grounded precedence order: `outranks_context(higher, lower)` edges, each carrying a verbatim charter (`source`/`locator`/`trust`). The logic-engine (≥ 0.20) reads these facts as directed edges and applies *lex superior* before the priority tier. |
| [`worked-legal-example.adj`](worked-legal-example.adj) | A case that `import`s the rulebook: two courts read "navigable waters" differently; the Ninth Circuit's reading **governs** the district court's *despite a lower tier*, because `ninth_circuit` outranks `district_court`. |
| [`context-precedence-meta.adj`](context-precedence-meta.adj) | **PR-B-4** — the recursive conflict-resolution **canons** as grounded meta-rules: `outranks_context($H,$L) :- reverses($H,$L)` (appeal status) and `… :- supersedes($New,$Old)` (lex posterior / recency), each citing its doctrine. The engine (≥ 0.21) reads rule-*derived* edges, so a primitive grounded fact (`reverses`/`supersedes`) becomes precedence. |
| [`worked-appeal-example.adj`](worked-appeal-example.adj) | A Supreme Court reversal flips a (now-reversed) Ninth Circuit reading at the **highest** tier — the precedence edge is **derived** by the appeal-status meta-rule from a grounded `reverses` fact, not asserted. |
| [`worked-supersession-example.adj`](worked-supersession-example.adj) | Lex posterior (bridges to MYCIN): the 2024 guideline edition supersedes the 2004 one, so the current recommendation governs the legacy one — `idsa_2024 > idsa_2004` derived from a grounded `supersedes` fact. |
| [`SOURCES.md`](SOURCES.md) | The provenance ledger — where each edge's verbatim quote came from. |

## Run it

```sh
adj-lang-cli code/specs/data/context-precedence/worked-legal-example.adj
```

The `governing` section resolves the conflict (0 answer-time model calls):

```json
{ "term": "means(navigable_waters, broad)",  "status": "governing", "context": "ninth_circuit",   "standing": "default" }
{ "term": "means(navigable_waters, narrow)", "status": "defeated",  "context": "district_court",
  "standing": "mandatory", "defeated_by": "means(navigable_waters, broad)" }
```

The broad circuit reading wins on **context** (lex superior), not tier — the narrow reading sits
at the higher `mandatory` tier and is still defeated. The precedence is auditable: a binding
query `? outranks_context(ninth_circuit, $lower)` recalls the governing edge **with** its
charter (the verbatim stare-decisis quote).

## How it fits

This is the data/worked-example layer of the ADJ73 precedence arc:

- **engine** (logic-engine 0.19) — `context_outranks`, lex-superior `defeats`.
- **grounded edges** (logic-engine 0.20, PR-B-2) — `outranks_context` facts participate as edges.
- **surface** (adj-lang 0.16) — `context:` on a rule, `context_order { … }`.
- **edges** (logic-engine 0.20, PR-B-3) — the grounded lex-superior rulebook + worked legal example.
- **meta-rules** (logic-engine 0.21, PR-B-4) — the recursive conflict-resolution **canons** (appeal
  status, lex posterior / recency) as grounded meta-rules that *derive* precedence from primitive
  grounded facts (`reverses`, `supersedes`). The engine reads rule-derived `outranks_context` edges,
  so the recursion bottoms out at cited primitives — an edge that can be derived is derived.
- **next** — lex specialis (the more specific rule controls), once rules carry a comparable
  specificity attribute. See [`SOURCES.md`](SOURCES.md) and the spec
  `code/specs/ADJ73-defeasible-rule-precedence.md` §7.
