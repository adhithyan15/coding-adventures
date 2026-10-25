# Intel 8085A gate-level simulator

This package is the gate-level partner to `intel8085-simulator`. It composes
the completed Intel 8080 gate datapath with an independent, DFF-backed 8085
control plane for exact program bounds, RIM/SIM, serial input/output, interrupt
masks and pending inputs, interrupt priority, and delayed EI.

The complete 64 KiB memory, registers, flags, ports, interrupt state, serial
latches, and lifecycle metadata occupy exactly 528,531 simulated D flip-flops.
All 246 documented first-byte encodings execute through gate-level logic; ten
undocumented bytes and every checked lifecycle failure are typed and atomic.

The functional simulator is used only as the differential trace, fault, and
full-state oracle in tests. This is an architectural educational model rather
than a transistor-accurate die or cycle-timed bus model. See
[`../../../specs/07af2-intel8085-gatelevel.md`](../../../specs/07af2-intel8085-gatelevel.md).
