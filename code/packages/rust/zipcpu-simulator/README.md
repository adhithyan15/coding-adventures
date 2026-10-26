# ZipCPU simulator (Rust)

This package is a checked instruction-level model of the bounded
`ZipCpuRev3NoCisFunctionalV1` profile in Spec 07be. It implements the fixed
32-bit, big-endian ZipCPU revision-3 ISA boundary with two complete register
banks, per-bank CC reserved storage, retained PCs, sampled interrupt input, a
64 KiB teaching bus responder, exact transactions and complete-state traces.

The admitted surface includes the documented integer ALU, arbitrary shifts,
bit reverse, low-immediate replacement, all three multiply selections,
signed and unsigned division, byte/halfword/word loads and stores, conditional
execution, cross-bank supervisor MOV, LDI, BREAK and NOOP. CC writes model the
fixed CPU-information projection, privilege filtering, RTU/trap, WAIT/HALT,
STEP and sticky status behavior. Taken PC writes commit before the following
fetch is classified, matching the pinned RTL boundary.

All checked lifecycle calls are transactional. `step`, `run`, and `execute`
build a candidate transition, preflight every retained allocation, and commit
once. Named deterministic seams reach each real allocation boundary,
including final-state and wrapper allocations after successful candidate
instructions, so tests prove exact rollback rather than exercising dead
failure counters.

```rust
use zipcpu_simulator::{encoding, StopReason, ZipCpuSimulator, CC_SLEEP};

let program = encoding::assemble(&[
    encoding::ldi(1, 42),
    encoding::mov_to_cc(CC_SLEEP),
])?;
let mut cpu = ZipCpuSimulator::new()?;
let result = cpu.execute(&program, 0x100, 2)?;
assert_eq!(result.final_state.supervisor_gpr[1], 42);
assert_eq!(result.stop_reason, StopReason::Halted);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Independent upstream evidence

Production code neither embeds nor calls upstream ZipCPU code. Tests consume
two checked-in corpora generated independently from public upstream commit
`42606d2d6ef55df313772232977621b2d72f0159`:

- 1,954 common, well-defined rows from `sim/cpp/zsim.cpp`, SHA-256
  `7997267037dce83304afbc29c481f57a9703b89cd893853f7197397f401045a2`;
- 2,016 rows from the actual Verilated `cpuops.v` + `mpyop.v` + `slowmpy.v`
  datapath under `OPT_MPY=36`, shifts enabled and low-power disabled, SHA-256
  `00d76c788d82db25f97b49f6797183415bda73d2192c519444fc5d6c81ace951`.

The consumer compares the entire projected Rust state and exact ordered
instruction-fetch transaction for every common row. Memory, privilege,
interrupt, architectural-fault and host-lifecycle matrices are covered by
independent manual-derived tests because neither bounded upstream harness is a
complete lifecycle oracle. The generator excludes the older C++ simulator's
known overflow/N disagreements with current RTL and documents that filter in
the harness instead of silently accepting either result.

Reproduction is offline after preparing a checkout at the pinned commit:

```bash
python3 tools/generate_upstream_corpora.py \
  --upstream /path/to/zipcpu-at-42606d2d6ef55df313772232977621b2d72f0159
```

The recorded development run used Apple clang 21.0.0 in `g++` compatibility
mode and Verilator 5.052. The generator verifies the commit and all seven
source hashes from Spec 07be before compiling either harness. Ordinary builds
and tests do not require C++, Verilator, a network connection, or the upstream
checkout.

## Validation

```bash
cargo test -p zipcpu-simulator --release --no-fail-fast
cargo clippy -p zipcpu-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p zipcpu-simulator --no-deps
cargo llvm-cov -p zipcpu-simulator --release --all-targets --summary-only
```

The release suite contains 31 completion-contract tests, two corpus consumers
covering 3,970 checked-in rows, and one doctest. Measured release line coverage
is 92.67% (1,265/1,365).

Compressed instructions, FPU, LOCK/SIM options, caches, MMU, local
peripherals, interrupt-controller policy, debug/external-halt wrappers,
pipeline timing and physical FPGA/transistor claims are deliberately excluded.
