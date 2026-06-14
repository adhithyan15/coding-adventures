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
