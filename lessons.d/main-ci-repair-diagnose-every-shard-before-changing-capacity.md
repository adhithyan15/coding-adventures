# Main CI repair: diagnose every shard before changing capacity

The September 3 full build combined npm 10.9.8 Arborist edgesOut crashes, missing Perl test prerequisites, stale generated Swift grammar, a private Flutter return type, repeated corpus generation, and oversized formula parsing. Reproduce installer failures with the runner's exact npm version; silent npm output can hide the root cause. Keep full-corpus test setup shared when assertions inspect an immutable checkout. Check formula input budgets before both dependency discovery and evaluation. Measure prerequisite-closed shard costs before changing the matrix.

The first PR run exposed two repair omissions: the four independent conformance jobs also needed the npm pin, and a new human-language changelog shard lacked its mandatory heading digest. Validate actual document shards after adding one. Windows BUILD commands need an explicit Bash boundary for shell syntax. Preserve native startup errors at the FFI boundary; returning only null conceals the cause of a remote runtime failure.

The next run identified EINTR from a native socket read inside Flutter. Retry Interrupted at the underlying Read boundary so both buffered header reads and direct body reads survive runtime signals; do not retry timeouts or resets. Audit every consumer of a broken BUILD idiom in the same repair: fixing only the first CAS package exposed identical Windows failures farther down the dependency graph.

After native navigation worked, Venture exposed nested Expanded widgets around a styled HostInput. When a parent emitter owns an explicit flex wrapper, suppress the child's implicit wrapper; generated code can type-check while still violating Flutter's ParentDataWidget runtime contract.

Hindi changelog fragments must begin with their level-2 entry heading. The document title belongs only in CHANGELOG.d/_meta.md; copying it into a numbered shard breaks the independent section-order test even when the heading digest and byte round trip pass.

- **Scope workflow wiring tests to the owning job before matching step names.** CI repeats `Set up Rust` across contract and build jobs. VM-032 initially matched an unrelated contract setup step and falsely reported a missing runtime flag. Inspect the build job specifically, then assert its selection, toolchain, and execution guards together.

- **Shared Cargo test helpers may assume workspace/target even when Cargo honors CARGO_TARGET_DIR.** VM-024 reused a target directory, but lang-aot tests/common built gc-core-capi there and looked only in the new workspace target/release. Validate with the default layout until archive discovery follows Cargo artifact metadata; a missing guessed archive is not a compiler semantic failure (VM-035).
