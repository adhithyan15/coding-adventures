# adj-lang-cli (Rust)

Command-line driver for the [`adj-lang`](../adj-lang) adjudication DSL. It is the
**CPU-bound reasoner** that pipelines (notably the MYCIN-2026 prototype) shell out
to: read a `.adj` program, compile it, run the `logic-engine` differential, and emit
the decision **plus a byte-cited proof DAG** as JSON — with **zero model calls**.

## What it does

```
.adj program (rulebook clauses + observe/query)
   │ adj_lang::compile        (lex → parse → adapt → lower)
   ▼ LoweredProgram { kb, queries }
   │ adj_lang::decide         (logic_engine::differential over the queries)
   ▼ Differential { ranked, decision }
   │ serialize
   ▼ JSON on stdout
```

## Usage

```sh
adj-lang-cli PROGRAM.adj
```

`PROGRAM.adj` is typically a case file that `import`s its rulebook (which in turn
`import`s its dictionary) — or, equivalently, the rulebook clauses concatenated
with the case's `observe`/`?` lines. Any `import "…"` is resolved before
compiling; imports are relative to the importing file and sandboxed to the
program file's directory (a `../` escape, an absolute path, or an import cycle is
refused as a `{"error": …}`). Output (pretty-printed here):

```json
{
  "queries": ["acs"],
  "ranked": [
    { "hypothesis": "acs", "posterior": 0.369, "posterior_logit": -0.536,
      "normalized_share": 1.0,
      "proof": [
        { "kind": "prior", "logit": -2.197,
          "source": "Pope JH et al., NEJM 1995", "locator": null, "trust": "authoritative" },
        { "kind": "contribution", "evidence": "symptom_quality(pressure_like)", "logit": 0.916,
          "source": "Panju AA et al., JAMA 1998", "locator": null, "trust": "authoritative" }
      ] }
  ],
  "decision": { "type": "determinate", "leader": "acs", "posterior": 0.369,
                "margin_posterior": ..., "margin_logit": ... }
}
```

Every proof step carries the cited `source`/`locator`/`trust` of the clause it fired,
so the audit trail is reconstructable without re-running the model. The `decision` is
`determinate` (a robust leader), `kickback` (an open uncertainty could flip the
ranking — `reason` + `runner_up` given), or `empty` (no queries). Non-finite numbers
(e.g. a single-hypothesis infinite margin) serialize as JSON `null`.

## `adj-verify` — the second binary

`adj-lang-cli` answers a question and prints a trail. That trail is **testimony**:
the engine describing its own work, in a format a confidently wrong system produces
just as fluently. `adj-verify` reads the same program and *does the work again*.

```sh
adj-verify PROGRAM.adj [--snapshots DIR]
```

It re-unifies every fact and rule, re-runs every negated subgoal to confirm the
absence is still an absence, re-multiplies every log-odds contribution, and checks
every quoted span **anchored at its recorded byte offset** in the pinned snapshot.
It exits **1 when anything failed**, so it composes as a CI gate.

`--snapshots DIR` supplies the pinned documents, stored content-addressed: each
filename is the lowercase SHA-256 hex of its contents, and the bytes are re-hashed
after reading rather than trusted by name. Without it, quote checks report
`unverified / snapshot_unavailable` — honestly unchecked, never a pass.

Two verdicts, deliberately not the same claim:

| Field | Means |
|---|---|
| `verified` | every step re-executed |
| `fully_verified` | …**and** every quote was confirmed against a snapshot, over a non-empty trace |

Today's stdlib records `source` labels rather than pinned spans, so it reports
`verified: true, fully_verified: false`. That gap is the point: it is a standing,
machine-readable measure of how much of the corpus is still uncheckable.

`adj-verify` is **not** `adj-replay` (ADJ08). It never invokes a model and never
touches the network — it is the deep re-execution *inside* one engine artifact,
which ADJ08's linter should call rather than reimplement.

## Where it fits

```
adj-lang-cli  ──depends on──►  adj-lang (compile/decide)
                               logic-engine (differential, proof DAG, provenance)
                               logic-core (terms)
                               cli-builder (declarative arg parsing)
```

Argument parsing is declarative via `cli-builder` (a JSON spec embedded in
`src/main.rs`). Exit codes: `0` success, `1` compile error (emitted as
`{"error": ...}`), `2` bad arguments / unreadable file.

## Test

`cargo test -p adj-lang-cli` — golden tests run the built binary on small `.adj`
programs and assert the posteriors, the cited proof steps, and the decision.
