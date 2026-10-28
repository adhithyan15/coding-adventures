# IBM System/370 gate-level simulator

This crate is the educational digital-logic partner to
`ibm-system370-simulator`. It preserves the complete bounded System/360
problem-state instruction surface and adds the System/370 `MVCL`, `CLCL`,
`CLM`, `STCM`, and `ICM` orders.

Exactly one embedded `ibm-system360-gatelevel` engine owns all 8,389,403
persistent architectural bits: 1 MiB of storage, sixteen 32-bit general
registers, four 64-bit floating registers, the 24-bit instruction address,
the two-bit condition code, and the halt latch. The System/370 extension
decoder and datapaths settle next values through repository gate networks and
clock that same stable-Q bank. There is no wrapper register file, memory
mirror, or oracle-authored transition.

The extension networks provide one-hot decoding, fixed-width 24-bit address
and length arithmetic, destructive-overlap qualification, first-unequal byte
comparison, mask prefix counting, byte selection and register merging, and
condition-code selection. The checked functional simulator is used only as a
fail-closed trace, fault, and complete-state oracle after the independent gate
candidate has settled.

This is an ISA-visible educational gate model, not a reconstruction of the
SLT cards, timing, channels, storage technology, or packaging of a particular
System/370 model. The exact boundary and source pins are defined by
[`../../../specs/07as2-ibm-system370-gatelevel.md`](../../../specs/07as2-ibm-system370-gatelevel.md).

## Example

```rust
use ibm_system370_gatelevel::System370GateLevel;

let mut cpu = System370GateLevel::try_new()?;
cpu.load(&[0x0a, 0x00], 0)?; // SVC 0 is the bounded teaching halt sentinel.
let trace = cpu.step()?;
assert_eq!(trace.mnemonic, "SVC");
assert!(cpu.get_state().halted);
# Ok::<(), ibm_system370_gatelevel::System370Error>(())
```

`step` is atomic. `run` and `execute` additionally restore the complete entry
state on any architectural, oracle, allocation, or step-limit failure.
Deterministic allocation seams are exposed for validation and never supply
architectural data.

## Evidence

- 21 release integration tests cover the literal 256-first-byte by installed
  length transport matrix, every inherited family, all extension masks and
  condition-code relations, corrected destructive-overlap `MVCL`, lifecycle
  rollback, allocation failures, independent truth tables, and structural
  authority.
- Compiler-checked complete temporary crates prove each hostile mutation
  compiles before parsed-token structural validation rejects it.
- Package-source line coverage is 94.75% (812/857), above the 80% requirement.
- Package-scoped Rustfmt, strict Clippy, strict rustdoc, and the exact BUILD
  recipe pass.
- The current-source repository planner identifies seven affected packages;
  those seven BUILD recipes pass. The repository's strict pre-build orphan
  validator is presently blocked by stale `BUILD-EXEMPTIONS` entries already
  present on `origin/main`, so this branch does not claim that unrelated
  repository-wide metadata gate as green.

## Development

```bash
bash BUILD
cargo llvm-cov -p ibm-system370-gatelevel --all-targets \
  --summary-only --ignore-filename-regex 'tests/'
```
