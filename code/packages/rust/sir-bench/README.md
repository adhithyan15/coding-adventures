# sir-bench

**Cross-backend performance benchmarks for the Semantic IR.** How fast is the
code each backend *generates* from the same Ruby program, lowered through the
narrow-waist [`semantic_ir::Module`](../semantic-ir)?

Where [`sir-conformance`](../sir-conformance) asks *"does every backend produce
the **same answer**?"*, `sir-bench` asks the sibling question: *"how **fast** is
the answer each backend produces?"* — the difference between a Ruby method
compiled to native C and the same method left running on the Ruby VM.

## What it measures

A single Ruby program is lowered **once** to SIR, then emitted to every backend
and run through its real toolchain, with a stopwatch on each phase:

```
  Ruby ──frontend──▶ SIR ──emit──▶ source ──compile──▶ binary ──run──▶ stdout
                         └─ emit ms ┘        └─ compile ms ┘   └─ run ms ┘
```

| Phase | What it is | Who to blame |
|---|---|---|
| **emit ms** | SIR → target source (our Rust backend) | the emitter |
| **compile ms** | the target's own compiler (`cc`, `rustc`, `go build`) — blank for interpreted targets | the target toolchain |
| **run ms** | the **generated program's** execution time | the target language / runtime |

`run ms` is the headline number: it's what "Ruby → C through SIR is fast/slow"
actually means.

## Running it

```
cargo run --release -p sir-bench
```

Prints a GitHub-flavoured Markdown report — one table per program, one row per
backend, sorted fastest-run first, with a `vs fastest` ratio column. A backend
whose toolchain is absent (or that a *v0* backend does not yet accept) is a
**skip**, never a fake `0 ms`.

Example shape:

```
### `fib` — recursive fibonacci fib(30) — call + arithmetic overhead

| backend | emit ms | compile ms | run ms | vs fastest |
|---|--:|--:|--:|--:|
| c       |  0.4 | 120.0 |   6.1 | 1.0× |
| rust    |  0.5 | 210.0 |   6.4 | 1.0× |
| go      |  0.6 | 180.0 |   9.0 | 1.5× |
| javascript | 0.5 | — |  35.0 | 5.7× |
| ruby    |  0.3 |     — | 210.0 | 34×  |
| python  |  0.4 |     — | 260.0 | 43×  |
```

(Illustrative — real numbers depend on the host; run it to see yours.)

## Methodology

- **Lower once.** The frontend runs a single time per program; every backend
  shares that `Module`, so no backend is charged for the parser.
- **Warm up, then median.** A few discarded warmup passes page the binary in
  (and absorb any one-time first-exec cost some hosts impose on a fresh binary),
  then the **median** of the timed passes is reported — a one-off outlier can't
  masquerade as the program's speed.
- **Optimise compiled targets.** `cc -O2`, `rustc -O`, `go build` — a debug
  binary would slander the backend. The flags are printed next to each table.
- **Skip, don't lie.** Missing toolchains and declared v0 gaps are skips.

## Corpus

Compute-heavy programs that lower and run on **every** backend (so a whole row is
comparable): recursive `fib(30)` (call + arithmetic overhead) and a 5,000,000-
iteration counting loop (loop + mutation overhead). Add more as the shared
feature surface grows.

## Where it fits

```
ruby-to-semantic-ir ─▶ semantic_ir::Module ─▶ semantic-ir-to-{python,javascript,go,rust,c,ruby}
                                              ├─ sir-conformance : same answer?
                                              └─ sir-bench       : how fast?  ← this crate
```
