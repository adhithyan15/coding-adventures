# Motorola MC6800 gate-level simulator

This package is the gate-level partner to `motorola6800-simulator`. It stores
the complete 64 KiB memory, A/B accumulators, X, stack pointer, program counter,
condition-code register, and wait latch in simulated D flip-flops. Its
independent combinational engine executes all 198 documented base-MC6800 opcode
encodings through fixed-width decode, Boolean, ripple-adder, flag, shift,
selection, branch, address, and stack networks.

The checked functional simulator supplies only the successful trace and fault
oracle. A candidate gate transition must agree with the oracle's complete state
before any DFF is clocked. Unknown/truncated instructions, invalid state,
out-of-image fetch, post-WAI execution, and bounded-run failures are atomic.

This is an architectural educational model, not a transistor-accurate die or
cycle-timed peripheral model. IRQ/NMI pins, clock phases, buses, and devices
belong to the later system/SoC layer. See
[`../../../specs/07ae2-motorola6800-gatelevel.md`](../../../specs/07ae2-motorola6800-gatelevel.md).
