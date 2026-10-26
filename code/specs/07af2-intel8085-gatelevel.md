# Spec 07af2 — Intel 8085A Gate-Level Simulator

## Purpose

`intel8085-gatelevel` is the architectural gate-level companion to the checked
`intel8085-simulator`. The 8085 retains the documented Intel 8080 datapath and
adds RIM/SIM, restart masks and pending inputs, vectored interrupt arbitration,
SID/SOD, and delayed EI. The package therefore composes the completed 8080 gate
engine with an independent DFF-backed 8085 control plane.

The functional model is used only by differential tests as a trace, fault, and
complete-state oracle. Production transitions never call it.

## Persistent topology

Every persistent bit changes through `logic_gates::sequential::register`.

| Component | D flip-flops |
|---|---:|
| Complete 8080 core: memory, registers, flags, PC/SP, ports, IE, halt | 528,479 |
| Installed origin | 16 |
| Installed length, including 65,536 | 17 |
| RST 7.5/6.5/5.5 masks | 3 |
| TRAP and RST 7.5/6.5/5.5 pending latches | 4 |
| INTR presence and acknowledge byte | 9 |
| SID and SOD | 2 |
| Delayed-EI latch | 1 |
| **Total** | **528,531** |

At six primitive gates per master/slave DFF, persistent storage alone represents
3,171,186 gate equivalents. Transient decoder, arbitration, selection, adder,
flag, and routing networks are additional simulated combinational logic.

## Gate transition engine

- The shared 244 documented 8080 encodings execute in the independent 8080
  gate decoder, register file, ALU, ripple adders, memory, stack, and I/O paths.
- RIM constructs its accumulator byte from eight gated signals.
- SIM gates mask writes, RST 7.5 reset, and serial-output writes with their
  corresponding enable bits.
- Interrupt arbitration is a priority network: TRAP, RST 7.5, RST 6.5,
  RST 5.5, then INTR/RST.
- Accepted interrupts perform checked stack writes, disable IE, clear halt and
  delayed EI, clear the accepted pending latch, and select the vector.
- EI is held in a DFF for one complete following instruction; DI clears both IE
  and the delayed enable immediately.
- Exact installed-image bounds are checked before the 8080 core is clocked.

This is an educational architectural model, not a transistor-accurate physical
8085 die, clock-phase model, or cycle-timed external bus.

## Checked contract

The machine fixes the architectural memory topology at 64 KiB and exposes
atomic load, restore, step, interrupt request, reset, bounded run, full snapshot,
port, and serial-input operations. Unknown and truncated instructions,
out-of-image fetch, invalid INTR acknowledge, post-halt step, malformed restore,
oversized image, and excessive retained-trace limits return typed errors without
partial mutation. A transactional `run` commits only after the whole bounded
execution succeeds.

## Completion criteria

1. Pin the topology to exactly 528,531 DFFs.
2. Compare all 256 first-byte classifications, exact trace fields, and complete
   before/after state against the functional simulator.
3. Cover RIM, SIM, EI delay, DI, priority, every vector, stack byte order, halt
   release, all eight valid INTR/RST bytes, and invalid INTR atomically.
4. Cover reset, arbitrary-origin loading, owned snapshots, ports, serial pins,
   bounds, rollback, and trace limits.
5. Pass release tests, strict formatting, Clippy, rustdoc, package BUILD, lesson
   validation, security review, and at least 95% line coverage.
