# Microwatt oracle evidence

The implementation and its source-locked boundary tests use Microwatt commit
`f341bed306a9a7281bf65200df8cc1f0a002617d` and Power ISA Version 3.0C PDF
SHA-256
`0c0064eda27161dcddf52f69a5d26799aaadc5ec73a2de9d90ed09a6ce015010`.

`source_locked.rs` records the reviewed RTL choices whose generic Power ISA
result is not enough to define this profile:

- `execute1.vhdl` sets sticky SO only from 64-bit OV, not OV32;
- `multiply.vhdl` returns the complete signed 32-by-32 product for `mullw`
  while its word-width overflow drives both OV and OV32;
- `execute1.vhdl` duplicates the `mulhw[u]` high word into both halves;
- `divider.vhdl` zeroes the upper half of `divw[u]`; and
- Rc observes the complete shaped 64-bit destination.

The committed corpus is an executed full-core RTL differential, not a
hand-derived substitute. `rtl/generate.py` compiles the exact Microwatt commit
in VHDL-2008 mode with FPU and branch-target cache disabled, runs 164
deterministic one-instruction rows, and exports all 32 GPRs, CR, XER, LR, CTR,
CIA, RAM writes, and the complete 64 KiB memory hash. `rtl_corpus.rs`
independently validates the manifest/schema and pinned boundary choices;
`rtl_differential.rs` compares every row with the production simulator.

The generator was executed with the official GHDL 6.0.0 macOS 15 arm64 LLVM
release from the URL pinned in `rtl/manifest.txt`, after verifying archive
SHA-256
`69beb5913a490b5971980bd045af6a63b6f52e861b444fa990bffac436587478`.
The compiler and Microwatt checkout lived in a disposable directory; no
system-wide installation or Gatekeeper bypass was used. Reproduce with:

```sh
python3 tests/rtl/generate.py \
  --microwatt /path/to/microwatt-at-f341bed306a9a7281bf65200df8cc1f0a002617d \
  --ghdl /path/to/verified-ghdl-6.0.0/bin/ghdl
cargo test --release --test rtl_corpus --test rtl_differential
```

`rtl/manifest.txt` pins the compiler identity, archive URL and checksum,
harness/generator hashes, exact command shape, seed, row count, and vector
content hash. Any generator or harness change therefore requires a real RTL
regeneration before the corpus consumer will pass.
