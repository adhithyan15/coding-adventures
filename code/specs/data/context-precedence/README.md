# context-precedence — grounded lex-superior precedence (ADJ73 PR-B-3)

A worked, byte-provenanced demonstration that **context precedence is itself grounded** — its
own rulebook, with a citation on *why* one context outranks another (ADJ73 decision §2.3).

## Files

| File | What |
|------|------|
| [`context-precedence.adj`](context-precedence.adj) | The grounded precedence order: `outranks_context(higher, lower)` edges, each carrying a verbatim charter (`source`/`locator`/`trust`). The logic-engine (≥ 0.20) reads these facts as directed edges and applies *lex superior* before the priority tier. |
| [`worked-legal-example.adj`](worked-legal-example.adj) | A case that `import`s the rulebook: two courts read "navigable waters" differently; the Ninth Circuit's reading **governs** the district court's *despite a lower tier*, because `ninth_circuit` outranks `district_court`. |
| [`context-precedence-meta.adj`](context-precedence-meta.adj) | **PR-B-4/5** — the recursive conflict-resolution **canons** as grounded meta-rules: `outranks_context($H,$L) :- reverses($H,$L)` (appeal status), `… :- supersedes($New,$Old)` (lex posterior / recency), and `… :- more_specific($S,$G)` (lex specialis), each citing its doctrine. The engine (≥ 0.21) reads rule-*derived* edges, so a primitive grounded fact (`reverses`/`supersedes`/`more_specific`) becomes precedence. |
| [`worked-appeal-example.adj`](worked-appeal-example.adj) | A Supreme Court reversal flips a (now-reversed) Ninth Circuit reading at the **highest** tier — the precedence edge is **derived** by the appeal-status meta-rule from a grounded `reverses` fact, not asserted. |
| [`worked-supersession-example.adj`](worked-supersession-example.adj) | Lex posterior (bridges to MYCIN): the 2024 guideline edition supersedes the 2004 one, so the current recommendation governs the legacy one — `idsa_2024 > idsa_2004` derived from a grounded `supersedes` fact. |
| [`worked-lex-specialis-example.adj`](worked-lex-specialis-example.adj) | Lex specialis: a specific wilderness-trail statute governs a general traffic statute on the same matter — `trail_statute > traffic_statute` derived from a grounded `more_specific` fact, despite the general statute's higher tier. |
| [`worked-canon-conflict-example.adj`](worked-canon-conflict-example.adj) | **§4.3 honest CONFLICT** — lex superior (`federal > state`) and lex specialis (`state > federal`) point opposite ways with no tiebreaker → the engine **abstains** (both `conflict_peer`, `has_conflict: true`), never silently crowning a canon. |
| [`worked-canon-tiebreaker-example.adj`](worked-canon-tiebreaker-example.adj) | **§4.3 RESOLVING tiebreaker** — the same collision, but a grounded `canon_outranks(lex_specialis, lex_superior)` fact + a negation-as-failure resolution rule (over canon-tagged `outranks_context_by/3` edges) **resolves** it to a cited decision: the specific state reading governs. No engine change — the tiebreaker is in the language; remove the canon-ordering fact and it falls back to abstention. |
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
query `? outranks_context_by(ninth_circuit, $lower, $canon)` recalls the grounded canon-tagged
edge **with** its charter (the verbatim stare-decisis quote) and the canon that asserts it.

## How it fits

This is the data/worked-example layer of the ADJ73 precedence arc:

- **engine** (logic-engine 0.19) — `context_outranks`, lex-superior `defeats`.
- **grounded edges** (logic-engine 0.20, PR-B-2) — `outranks_context` facts participate as edges.
- **surface** (adj-lang 0.16) — `context:` on a rule, `context_order { … }`.
- **edges** (logic-engine 0.20, PR-B-3) — the grounded lex-superior rulebook + worked legal example.
- **meta-rules** (logic-engine 0.21, PR-B-4/5) — the recursive conflict-resolution **canons**
  (appeal status, lex posterior / recency, **lex specialis**) as grounded meta-rules that *derive*
  precedence from primitive grounded facts (`reverses`, `supersedes`, `more_specific`). The engine
  reads rule-derived `outranks_context` edges, so the recursion bottoms out at cited primitives —
  an edge that can be derived is derived.
- **conflict handling** (logic-engine 0.22, §4.3) — when canons point opposite ways and no
  tiebreaker exists, the engine **abstains** (`ConflictPeer` / `has_conflict`), never silently
  crowning one or double-defeating both (`worked-canon-conflict-example.adj`). The "else CONFLICT".
- **resolving tiebreaker** (§4.3, in-language — no engine change) — a grounded `canon_outranks`
  ordering + a negation-as-failure resolution rule over canon-tagged `outranks_context_by/3` edges
  RESOLVES a chosen collision into a **cited decision** (`worked-canon-tiebreaker-example.adj`):
  the canon-ordering is itself a grounded fact, so the recursion stays grounded all the way up.
  Remove it → fall back to abstention.
- **uniform substrate** (the shared rulebook migrated) — `context-precedence.adj` (lex-superior
  edges) and `context-precedence-meta.adj` (the three canons) now all emit canon-tagged
  `outranks_context_by/3`, and the shared `context-precedence-resolve.adj` module (imported by
  both, idempotently) holds the NAF resolution. So a jurisdiction's grounded `canon_outranks`
  ordering applies **uniformly** across every canon; absent one, a collision abstains (§4.3). The
  audit query is now `? outranks_context_by(H, $lower, $canon)` (the tagged fact carries the
  charter). This completes the precedence arc as a uniform grounded substrate. See the spec
  `code/specs/ADJ73-defeasible-rule-precedence.md` §4.3, §7.
