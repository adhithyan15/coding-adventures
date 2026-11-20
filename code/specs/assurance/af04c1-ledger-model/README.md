# AF-04c1 finite ledger model receipt

This small reproducer encodes the fixed callback vector
`pass, crash, crash, timeout, oom, pass, timeout, oom, crash`, with five
triage outcomes followed by four mutation outcomes. Each call has four
one-hot outcome variables. The sole `clean` constraint is
`clean => all nine outcomes are pass`.

This is a consistency check for that fixed vector. It does **not** encode
phase counters, seed admission, edge totals, energy scheduling, remaining
budget, or campaign progress, and it proves nothing about the TypeScript
implementation. It is not native coverage or Warrant source binding.

The model uses `cdcl-sat` 0.3.2 to show the fixed vector is SAT with
`clean=false` and UNSAT with `clean=true`; `drat-check` 0.1.0 independently
checks the resulting DRAT certificate. The package inputs are local path
dependencies. The recorded source commit is
`12e28f070342282825f255f3907b71842e041791`; the dependency tree OIDs are
`6cab5c13eda63430bc597f696a4691f0a8448133` for `cdcl-sat` and
`6a3fd83db83ef4edb1a74692b65ae64f161c8063` for `drat-check`.

For this source, the generated artifact hashes are:

| Artifact | SHA-256 |
| --- | --- |
| `clean-false.cnf` | `d8c4c4a240861e2249a51313666df51b215fce1c8d8bb475a28cf2e05b899a06` |
| `clean-true.cnf` | `b359dd453557f07a06ff237e2c1066973b17d02aa69b2e9e823dcaf9c5a64e94` |
| `clean-true.drat` | `c5b0ef305dc682829de01b5ec0a3d1b0fd62819b019e92008bb47f075c805502` |

Reproduce with Rust 1.100.0 from the repository root:

```sh
git diff --quiet 12e28f070342282825f255f3907b71842e041791 -- \
  code/packages/rust/cdcl-sat code/packages/rust/drat-check
cargo run --locked --offline \
  --manifest-path code/specs/assurance/af04c1-ledger-model/Cargo.toml \
  -- /tmp/af04c1-ledger-model-regenerated
cmp code/specs/assurance/af04c1-ledger-model/clean-false.cnf \
  /tmp/af04c1-ledger-model-regenerated/clean-false.cnf
cmp code/specs/assurance/af04c1-ledger-model/clean-true.cnf \
  /tmp/af04c1-ledger-model-regenerated/clean-true.cnf
cmp code/specs/assurance/af04c1-ledger-model/clean-true.drat \
  /tmp/af04c1-ledger-model-regenerated/clean-true.drat
```

The checked-in DIMACS files and proof are the durable inputs to the independent
checker. `Cargo.lock` pins the local package graph; no solver build artifacts
are checked in.
