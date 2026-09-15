---
category: BUILD files & dependency management
---

# A missing BUILD file is now a CI failure, not a silent gap — `code/BUILD-EXEMPTIONS` is the ledger

`build-tool -validate-build-files` fails on any directory that has a `Cargo.toml`, no `BUILD`, and no entry in `code/BUILD-EXEMPTIONS`. Entries are `EXCLUDED <path>  # <reason>` (genuinely never built — a compile-only JNI bridge, a wasm-only cdylib) or `PENDING <path>  # <reason>` (a tracked backlog item), and the reason is mandatory. **Stale entries fail too**: land a BUILD for a `PENDING` crate and the same PR must delete its exemption line, so the ledger can never outlive the problem. This closed an 84-crate gap; it did NOT come from a scaffold-generator bug (that tool has templated BUILD since 2026-03-21) but from crates hand-rolled afterwards. If you add a crate and CI complains, the fix is a BUILD file — reach for an exemption only when the crate genuinely has nothing to run, and say where it IS covered.
