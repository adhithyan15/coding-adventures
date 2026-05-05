# board-vm-ir

Portable bytecode decoding and validation for the Board VM runtime.

This crate intentionally stays `no_std` and allocation-free so firmware targets
can use the same instruction decoder as host-side tests and tools.
