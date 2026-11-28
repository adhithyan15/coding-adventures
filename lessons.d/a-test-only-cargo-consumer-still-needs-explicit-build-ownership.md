---
category: BUILD files & dependency management
---

# A test-only Cargo consumer still needs explicit build ownership

The orphan check scans every `Cargo.toml` under `code`, including test-only
consumer crates. A fixture with its own Cargo workspace remains an independent
root even when another package's `BUILD` invokes a script that compiles and runs
it. The guest consumer fixture failed discovery for this reason despite its
runtime assertion being exercised by the Mosaic and Starlark acceptance steps.

If a test-only crate is intentionally owned by another package's acceptance
command, add a reasoned `EXCLUDED` ledger entry that names the owner BUILD files,
the command path, and the assertion that provides coverage. If it needs its own
tests or lint lane, give it a real `BUILD` instead. Keep the source visible to
discovery; never narrow sparse inputs to conceal the manifest. Re-run the source
build-tool's complete orphan validation after changing ownership, and remove or
revise the ledger entry when the fixture or its owning acceptance steps move.
