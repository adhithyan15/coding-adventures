# Starlark VM bytecode microbenchmark

`starlark-bytecode-integer-add-chain-256` executes 256 integer additions in
the Starlark VM. The baseline compiles 256 simple assignments through the
current stub source compiler; the candidate uses inspected, hand-assembled
operand-stack bytecode. The source fixture is packaged under `testdata/` and
checked byte-for-byte against the generator. Source compilation, code-object
assembly, and initial output validation occur before timing; each measured
operation runs the actual `interpret_bytecode` API with a fresh interpreter.
There is no source loop because this compiler stub does not support loops.

Stdout contains the portable VER07 core report; `--manifest FILE` writes the
versioned guest envelope. `--replay REPORT` and `--replay-envelope MANIFEST`
strictly validate either artifact, including duplicate raw JSON keys. Defaults
are 40 paired observations, 100,000 ns target and 65,536 max batch.

This is one guest execution lane alongside 18 native library implementations.
It does not substitute for a native port and says nothing about physical CPU
cycles or cache measurements. Empty host no-op and empty interpreter calls are
separate raw envelope diagnostics; neither is subtracted from results.

```text
cargo test --manifest-path Cargo.toml
cargo run --release --manifest-path Cargo.toml --bin starlark-microbenchmark -- --pairs 40 --revision <commit> --manifest starlark.json
```
