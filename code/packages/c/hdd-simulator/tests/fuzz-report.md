# DB-E12 C HDD fuzz receipt

This report covers the C HDD package only. Fixture bytes rejected for syntax,
schema, model consistency, or replay-domain limits are expected inputs. A
sanitizer crash or disagreement after the independent model accepts a trace is
a finding.

## Campaign

- Source revision: `d8099d92c92b041718c05ed62daadccd532f3276`
- Toolchain: Debian Clang, llvm-profdata, and llvm-cov 19.1.7
- Command: `CLANG=/workspace/scratch/db-llvm19.fnG2Z9/root/usr/lib/llvm-19/bin/clang LLVM_PROFDATA=/workspace/scratch/db-llvm19.fnG2Z9/root/usr/lib/llvm-19/bin/llvm-profdata LLVM_COV=/workspace/scratch/db-llvm19.fnG2Z9/root/usr/lib/llvm-19/bin/llvm-cov FUZZ_WORK_PARENT='/workspace/scratch/db media check' sh tools/fuzz.sh`
- Both targets used `-fsanitize=fuzzer,address,undefined` and LLVM source/branch instrumentation.
- Fixture mode: seed `12021`, maximum input 65,536 bytes, 100,000 runs in 7 seconds; final `cov:911`, `ft:3533`, corpus 262 files / 414 KiB.
- Binary history mode: seed `12022`, maximum input 523 bytes, 100,000 runs in under one second; final `cov:205`, `ft:513`, corpus 47 files / 1,083 bytes.
- Findings: no ASan/UBSan reports, runtime disagreements, or saved artifacts. Each campaign used a fresh scratch corpus; the canonical fixture was copied into fixture mode.
- Scratch logs, profiles, binaries, and artifact directories: `/workspace/scratch/db media check/db-e12-c-hdd.xVcmHm`.

The hashes below identify source bytes at campaign time; the HEAD revision was
the DB-E12 spec commit and the implementation was a dirty worktree.

| Input | SHA-256 |
|---|---|
| `src/hdd_simulator.c` | `77e8fc066fa5db28cc2f24e2646ba64fc566ef9d38ed7e74c32b2ffce3a8db8f` |
| `tests/hdd_simulator_test.c` | `7760cef0396875ff03aabe3ee654e0626f84db1001c01ed14f5e7ff77c8a2df9` |
| `tests/hdd_simulator_fuzz.c` | `c9b49e6f2ca33fa3860de93fffff6ff1a7675612a81377be0a3f640340021624` |
| `tests/hdd_simulator_fuzz_support.h` | `83e16d74e8b48a1049a7af3a6bc423bf3d80f98537e8d6d812f312e9925e8ca3` |
| `tools/fuzz.sh` | `995865d75edebd39ad93076cd7c060c5bad354ec1b549f6a8f81e8bcf0729b0a` |
| `../bounded-json/include/bounded_json.h` | `c061c218549ea0f4663cb1386ffc2eeff2e96d583ece727221e44d1885ed7422` |
| `../bounded-json/src/bounded_json.c` | `a5d9a0cff2cd8442d57d4c9c556fc5b60a40cbe90f0353462af02571ee2d54a5` |
| Canonical HDD fixture | `9e0ad749882db5c4379afdfb1becaba953854cad3daa7df4ae1888ac65f3f575` |

## Measured coverage

LLVM runtime coverage from each individual fuzzer binary:

| Target | Runtime lines | Runtime branches |
|---|---:|---:|
| Fixture vectors | 83/88 (94.32%) | 27/46 (58.70%) |
| Binary histories | 87/88 (98.86%) | 35/46 (76.09%) |

Fixture mode also measured the bounded JSON parser at 89.63% lines and 70.05%
branches. The history lane does not enter the parser. Remaining runtime coverage
includes one allocation path unavailable through the public API and clock or
status conditions outside the bounded command traces. These finite runs are
evidence, not a proof over arbitrary byte strings or arbitrary C pointers.

## Fixed corpus hashes

Fixture seeds:

| Seed | SHA-256 |
|---|---|
| `boolean-lba.json` | `246c02fe535b54c739e6bb86ab0e7a76e5ddbae05e74c4155119620b53efa8e6` |
| `duplicate-key.json` | `b365a331d92dbd228c0bfbf0ec39b36f5db0804cd61495ab8fb1a27328e3c3f0` |
| `invalid-hex.json` | `b8dae6e12681a2d928bb090eedceb492f3a43b42ab2b4a876cb0f462bf349c93` |
| `malformed-timing-probe.json` | `1c5e2535b81be61973c2e0b1f24ab2e1177fc0784cbc5209d45975493eafe60f` |
| `out-of-limit-tick.json` | `d694f64ecfc372606014251e2369bc9a4959d6eedd4d47db0b76a82ca19b7fb8` |
| `trailing-value.json` | `7149e3e5b9c1e93335d854406cc6fc0c90a9a22530e062689349e5fd7ae142c8` |
| `truncated.json` | `7e3eb5fc8da3096692ed52e9ff2a99e4f5bcb43640259131f8ab783a7723862a` |
| `unknown-error.json` | `84b430c377e13cbfc9ee81c2aa3a3094115f0506964257f3aebbd1c1c42f116a` |
| `unknown-operation.json` | `56c94ea21062d2ad2f89feeea282287de7b3c1c6fcc53c3df1a1e09342d58b0a` |
| `wrong-timing-shape.json` | `cc9307a3eeebe26a8b46d1f045b6cdfa34d6b99dfee464c6f689d30b09b65f7d` |

History seeds:

| Seed | SHA-256 |
|---|---|
| `clock-overflow-after-seek` | `6689b1047a115402a065895aee6f281d396d8caf32559d3f94658e1c42f2d29e` |
| `exact-max-tick` | `e4605773b7ec484bb740d2bd83b258eb83a0ca27b669faaa69543c06c193dcd3` |
| `last-lba` | `b354b436707687331b8c6cdbd0fb589f6f7ba2bf2305c0e3c296985883db656e` |
| `lba-refusal` | `680354fc3823fd72a146fda1b89403731c504654aa44e67e8fe1e9e230c362da` |
| `null-output` | `4fbc5f995a2a85dc3d9f319b200b49a7acb80be21ba6c47f9f26ba93279780b2` |
| `read-zero` | `93431640c478429b7f981390fc3694f96ca65b2a773272d31cdf003d69ce3cb8` |
| `short-write` | `4a323802be52461f9badad4da6be7bfc02c1372c5df4709fb0fa42cc1dac3a75` |
| `write-readback` | `e82c48dc441ba67b3864dba9a8ad73857ba8dafce64c5c2e99ccfc61f9f02f52` |

The ordinary native suite passed 33,252 checks under GCC and Clang. The
PowerShell launcher parses successfully; Windows native execution was not
available in this Linux environment.

## Read-copy negative control

In a disposable package copy, the runtime's `memcpy(out, simulator->sectors[lba],
HDD_SECTOR_BYTES)` was replaced with `(void)out`. The corrected adapter returned
`HDD_FUZZ_RUNTIME_MISMATCH` (enum value 3) for both the valid `read-zero` binary
history and the canonical fixture. This shows the read oracle depends on the
runtime's copy into a sentinel-filled output buffer.
