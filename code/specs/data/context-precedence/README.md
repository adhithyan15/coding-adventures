# context-precedence — grounded lex-superior precedence (ADJ73 PR-B-3)

A worked, byte-provenanced demonstration that **context precedence is itself grounded** — its
own rulebook, with a citation on *why* one context outranks another (ADJ73 decision §2.3).

## Files

| File | What |
|------|------|
| [`context-precedence.adj`](context-precedence.adj) | The grounded precedence order: `outranks_context(higher, lower)` edges, each carrying a verbatim charter (`source`/`locator`/`trust`). The logic-engine (≥ 0.20) reads these facts as directed edges and applies *lex superior* before the priority tier. |
| [`worked-legal-example.adj`](worked-legal-example.adj) | A case that `import`s the rulebook: two courts read "navigable waters" differently; the Ninth Circuit's reading **governs** the district court's *despite a lower tier*, because `ninth_circuit` outranks `district_court`. |
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
- **this** (PR-B-3) — the grounded rulebook + worked legal example end-to-end through the CLI.
- **next** (PR-B-4) — the recursive conflict-resolution **meta-rules** (lex posterior / recency,
  lex specialis, appeal status) that *derive* precedence from grounded rule attributes rather
  than asserting it edge-by-edge. See [`SOURCES.md`](SOURCES.md) and the spec
  `code/specs/ADJ73-defeasible-rule-precedence.md` §7.
