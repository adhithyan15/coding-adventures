---
category: CI & GitHub Actions
---

# A generated artifact committed so it stays in sync turns every upstream change into a CI failure; build it in the consumer BUILD instead

**What went wrong.** `spreadsheet_engine.wasm`, `task_engine.wasm`,
`engram_engine.wasm` and the base64 VisiCalc bundle derived from the first were
committed so the demos would open without a toolchain. To stop them drifting,
`spreadsheet-wasm` got a `--check` that hashed every crate in its `cargo tree`.
That check then failed unrelated PRs (lexer, parser, grammar-tools: three
separate lessons), and `engram`'s artifact, which had no check, went two months
stale while its smoke test agreed with it. A committed binary is unreviewable
and bloats history, and `Cargo.lock` is gitignored here, so even an upstream
crates.io release can flip such a check.

**The fix (BUILD-001).** Do not commit what a toolchain derives from committed
sources. Ignore `pkg/` and the vendor bundle; have each crate's BUILD build the
wasm and smoke-test that fresh build; give humans one command,
`code/scripts/build-demo-wasm.sh`; let tests that need the bundle build it.

**What to do differently.** Before adding a "stale?" check for a committed
build output, ask whether the output needs to be committed at all. Checking
freshness is a second system you must keep alive; not storing the file removes
the question. When you do remove a committed artifact, grep for every consumer
(BUILD files, scripts, workflows, path filters, registries) and give each one a
build step, or it will read a file that no longer exists on a clean checkout.
