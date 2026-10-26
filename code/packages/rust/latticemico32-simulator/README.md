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

`Lm32Simulator::new`/`Default` use Rust's ordinary infallible allocation for
the initial fixed 64-KiB memory. The typed `AllocationFailed` guarantee begins
after construction and covers every later checked snapshot, load, execution,
trace, and result allocation.

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
python3 latticemico32-simulator/tools/generate_vectors.py --check
# With an offline checkout of the exact pinned commit:
python3 latticemico32-simulator/tools/generate_vectors.py --check \
  --rtl-root /path/to/lm32-84b3e3ca
```

`tools/generate_vectors.py` is an independent manual oracle plus a distinct
RTL-topology evidence path; it never imports the Rust implementation. The
manual path reproduces 308 exact architectural boundary transitions. The RTL
path separately parses the decoder-wire roster and validates concrete decode,
immediate-mux, narrow-store, lane-select, interrupt, exception, and DC.RE
constructs from an offline checkout after checking all five immutable source
hashes. Those source-derived constructs select 183 bounded topology cases;
the independent manual oracle supplies their expected architectural states
because the Verilog is evidence for the later gate cell, not an alternate
functional authority. The normal offline-free check uses the checked-in wire
manifest, while `--rtl-root` re-extracts and verifies it from source.

Both transition corpora compare all 32 GPRs, every scalar state field, the
complete 64-KiB memory, image metadata, and every trace/transaction/interrupt/
exception field. They span every implemented family, both outcomes and maximal
offsets for every branch predicate, top-of-memory accesses, pseudo-instruction
equivalences, narrow-store lanes, every CSR, both exception-remap states, every
interrupt line, ignored-field classes, and declared integer/control boundaries.
A separate 56-row generated corpus drives every typed lifecycle/fault rollback
under both DC.RE states, including externally supplied misaligned fetch and
invalid-register errors. The legacy 64-row opcode, 61-row canonical decode, and
32-row operand corpora remain as small classification checks; the operand rows
drive 672 additional complete-state ALU transitions. In total, 712 bounded
checked-in evidence rows and all 34 release integration tests pass. Measured
release line coverage is 94.33% (882/935).

The source-evidence table pins these SHA-256 values for commit
`84b3e3ca0ad9535acaef201c1482342871358b08`:

- `lm32_decoder.v`: `9fd441f3ba425ed79042e5305e229087ba5525b3bfb4cb13057c92119c7173c1`
- `lm32_load_store_unit.v`: `172706c683180f5cae98b7f4bd26efcc674b03de164cd4e0922ee4bd4f22af39`
- `lm32_interrupt.v`: `495f56e562abb418fccb099a1ad2f4ce86757e223248346844a8bb1e729fc7ca`
- `lm32_cpu.v`: `e54421ff9fbc6c996cc81ebeb5d98cb16af6737ca2f8571ebd5bd0e79ea8aa17`
- `lm32_debug.v`: `08e45a33e38c91415d3cd9fb8bcb190567839646ab0b7bb06fa8786cd867328d`

The pinned load/store boundary replicates the low byte across all four bus
lanes and the low half-word across both half-word lanes. `DataWrite.value`
records that actual 32-bit RTL bus value while `byte_enable` selects the legal
lane; every SB/SH/SW lane is checked for both successful and bus-error traces.
The decoder also explicitly accepts every ignored-bit class of `rcsr`,
`sextb`, and `sexth`, matching primary-opcode-only RTL selection.

One evidence discrepancy is intentionally not hidden: the manual-visible
functional EBA/DEBA rule permits bit 8 (256-byte alignment), while the pinned
RTL physically stores bits 31:9. The dependent gate specification must review
an adapter or revised boundary before claiming full-state equality.
