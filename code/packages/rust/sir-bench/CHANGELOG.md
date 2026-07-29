# Changelog

## 0.1.0 — initial cross-backend performance benchmark harness

First cut of `sir-bench`: measures how fast the code each backend *generates*
runs, for the same Ruby program lowered through the Semantic IR.

- **`Target`** — the six backends (Python, JavaScript, Go, Rust, C, Ruby) with
  `available()` toolchain probing and an `is_compiled()` split (C/Rust/Go pay a
  compile cost + run a binary; Python/JS/Ruby run their source directly).
- **`Bench`** — a compute-heavy Ruby program plus its `iters`/`warmup` counts;
  the default `corpus()` ships `fib` (recursive `fib(30)`) and `loop_sum` (a
  5,000,000-iteration counting loop), both of which lower and run on every
  backend so a whole row is comparable.
- **`measure(bench, module, target)`** — emits (timed), compiles if native
  (timed), runs `warmup` discarded passes then `iters` timed passes, and reports
  the **median** run as a `Sample::Ran { emit, compile, run, stdout }`. A missing
  toolchain or a declared v0-backend gap is a `Sample::Skipped`, never a fake
  `0 ms`; a real emit/compile/exit error is a `Sample::Failed`.
- **`markdown_report(...)`** — a GitHub-flavoured table per program, rows sorted
  fastest-run first with a `vs fastest` ratio column.
- **`sir-bench` binary** — runs the whole corpus over every available backend
  and prints the report (`cargo run --release -p sir-bench`).

**Methodology** (documented inline): lower once (never charge a backend for the
parser); warm up then take the median (so a one-time first-exec cost — e.g. an
endpoint-security scan of a fresh binary — cannot be mistaken for the program's
speed); optimise compiled targets (`cc -O2` / `rustc -O` / `go build`); skip,
don't lie. Reuses the exact emit entry points and toolchain invocations
`sir-conformance` uses, so the two harnesses agree on what "a backend" is.
