# board-vm-ir

Portable bytecode decoding and validation for the Board VM runtime.

This crate intentionally stays `no_std` and allocation-free so firmware targets
can use the same instruction decoder as host-side tests and tools.

`collect_required_capabilities` scans decoded calls into a caller-provided slice
without allocation. Host frontends and eject tooling can use it to attach a
module's capability requirements to target-independent artifacts before a
board-specific backend validates or compiles them.
