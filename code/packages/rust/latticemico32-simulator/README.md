# LatticeMico32 simulator (Rust)

A checked instruction-level model of the configuration-pinned
`Lm32BaseFunctionalV1` processor. This package is the functional oracle for
the dependent educational gate cell; it is deliberately not a LatticeMico
System, Wishbone timing model, cache model, or FPGA netlist.

The architectural authority is the official LatticeMico32 Processor Reference
Manual archive release 3.8. The separate decode evidence corpus is tied to the
vendor-origin RTL commit
`84b3e3ca0ad9535acaef201c1482342871358b08`; production code does not contain,
invoke, or translate that RTL.

## Boundary

- 32 mutable 32-bit GPRs, including mutable r0 and ra/ea/ba.
- Big-endian fixed-width instruction and data transport over owned 64 KiB
  memory, with exact installed-image metadata.
- The complete pinned arithmetic, logic, comparison, shift/extension,
  byte/half/word memory, branch/call, CSR, exception, and interrupt surface.
- Exact CFG `0x0802010F`, CFG2 zero, IE/EIE/BIE, IM/IP, EBA/DEBA, and DC.RE.
- Latched-pending non-level-sensitive interrupt behavior with all 32 logical
  asserted lines represented explicitly.
- Complete post-state traces plus ordered instruction/data bus transactions.
- Typed atomic lifecycle errors and deterministic allocation-failure seams,
  including failures after prior candidate instructions in `run` and
  `execute`.

Signed `div`/`mod`, user operations, caches, MMU, cycle timing, hardware debug,
peripherals, DMA, arbitration, and SoC address decoding are explicit exclusions.

## Example

```rust
use latticemico32_simulator::{Lm32Simulator, StopReason};

// addi r1,r0,42 ; scall
let program = [0x34, 0x01, 0x00, 0x2a, 0xac, 0x00, 0x00, 0x07];
let mut cpu = Lm32Simulator::new();
let result = cpu.execute(&program, 0x100, 2)?;
assert_eq!(result.final_state.gpr[1], 42);
assert_eq!(result.reason, StopReason::Exception(7));
# Ok::<(), latticemico32_simulator::ExecutionError>(())
```

LM32 has no architectural halt. `run` succeeds at the first architectural
exception and otherwise rolls the entire call back when its nonzero step
budget is exhausted.

## Reproducing verification

From `code/packages/rust`:

```text
cargo test -p latticemico32-simulator --release --no-fail-fast
cargo clippy -p latticemico32-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p latticemico32-simulator --no-deps
cargo llvm-cov -p latticemico32-simulator --release --tests --summary-only
```

The checked-in `tests/vectors/manual-opcodes.csv` is the independent complete
64-primary-opcode classification matrix. `pinned-rtl-decode.csv` contains 61
bounded canonical decode observations spanning every implemented family plus
both raise operations. The 32-row independent manual operand seed corpus
drives 672 exact complete-state ALU transitions. The integration suite also exercises exact
state and transaction effects, register overlap, integer boundaries, every
memory width and endian extension, all branch predicates, all visible CSRs,
both exception-remap states, every interrupt line, lifecycle rollback, and
allocation seams. All 26 release integration tests pass, and measured release
line coverage is 93.83% (867/924).

One evidence discrepancy is intentionally not hidden: the manual-visible
functional EBA/DEBA rule permits bit 8 (256-byte alignment), while the pinned
RTL physically stores bits 31:9. The dependent gate specification must review
an adapter or revised boundary before claiming full-state equality.
