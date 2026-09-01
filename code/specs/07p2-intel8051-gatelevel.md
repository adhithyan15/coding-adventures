# 07p2 — Intel 8051 Gate-Level Simulator

## Status

Layer 07p2 is the complete Rust gate-level partner of Layer 07p. The normative
package is `code/packages/rust/intel8051-gatelevel`, and its behavioral oracle
is `code/packages/rust/intel8051-simulator`.

## Architectural boundary

The machine exposes the complete base-8051 architectural state used by Layer
07p: a 16-bit PC, 256 bytes of IRAM/SFR state, separate 64 KiB code and XDATA
spaces, halt state, and the installed-program boundary used to reject truncated
instructions. Timers, interrupt arbitration, UART timing, and physical port-pin
electrical behavior remain outside the Layer 07p/07p2 scope. Port SFR latches
are architectural state.

Opcode `0xA5`, undefined on real 8051 silicon, remains the repository HALT
sentinel. The other 255 opcode bytes implement the Layer 07p instruction
surface.

## Persistent topology

Every persistent architectural bit is modelled as a D flip-flop:

| Bank | Bytes/bits | DFFs |
|---|---:|---:|
| Code ROM state | 65,536 bytes | 524,288 |
| External XDATA | 65,536 bytes | 524,288 |
| IRAM and SFRs | 256 bytes | 2,048 |
| Program counter | 16 bits | 16 |
| Halt latch | 1 bit | 1 |
| **Total** | | **1,050,641** |

Large byte banks use a packed stable-Q representation. At an observable
instruction boundary, Q uniquely determines the stable master/slave latch
state. Every simulator-owned write reconstructs that state and clocks both
phases through `logic_gates::sequential::register`. PC and halt use explicit
state registers.

Installed-program origin and length are lifecycle metadata, not silicon state,
and are therefore excluded from the hardware DFF total.

## Gate data paths

- Eight- and sixteen-bit addition use fixed full-adder chains.
- ADD, ADDC, and SUBB derive CY, AC, and OV from carry wires.
- ANL, ORL, XRL, complements, bit operations, parity, and zero detection use
  logic-gate networks.
- Branch, PC, DPTR, INC, DEC, stack, and relative-address arithmetic use the
  gate adders.
- MUL AB uses eight unconditional partial-product rows and fixed ripple adds.
- DIV AB uses an eight-stage restoring divider with gate-controlled selection.
- RL and RR rotate wires without changing CY; RLC and RRC alone rotate through
  the carry latch.

Host integers may index memory, sequence fixed networks, and assemble trace
metadata. They must not replace an architectural arithmetic or logical result.

## Shared checked lifecycle

The gate crate re-exports the functional crate's `Intel8051State`,
`Intel8051Error`, `StepTrace`, and `ExecutionResult` contracts.

- `get_state` returns every owned architectural byte.
- `restore` validates both Harvard-space sizes and the installed range before
  changing the machine.
- `load_checked` and `load_at_checked` reset deterministically, clear stale code
  and XDATA, and reject overflow atomically.
- `step_checked` rejects halted, truncated, invalid-indirect, and execution
  failures without partial mutation and returns complete before/after state.
- `run_loaded_checked` and `run_checked` are transactional across the complete
  bounded run.

Legacy `load`, `execute`, and `step` remain for source compatibility. New code
should use the checked surface.

## Conformance

The required conformance boundary is:

1. Restore deterministic complete Harvard state for every opcode byte.
2. Execute exactly one checked gate transition and one functional transition.
3. Compare opcode bytes, mnemonic, PC, IRAM/SFR, XDATA, code, halt, loaded
   metadata, and complete before/after snapshots.
4. Test invalid state sizes, load overflow, truncation, halted stepping,
   deterministic clearing, exact topology, and bounded result traces.
5. Pass formatting, Clippy with warnings denied, rustdoc with warnings denied,
   and at least 80% package line coverage.

The audited implementation passes 70 unit tests, six lifecycle tests, one
aggregate all-256-opcode full-state differential, and 23 doctests. Coverage is
97.51% overall and 97.01% for the CPU engine.
