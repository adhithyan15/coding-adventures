# host-vm-lowering

Shared Rust lowering facts for host VM backends.

This crate ports the hard-won details from the Python ALGOL to IR/WASM/JVM work
into Rust data structures and small planning helpers. The intent is that a
frontend can lower to the LANG VM chain once, and the JVM, CLR, BEAM, WASM, and
future host VM backends can share the same frame layout, descriptor ABI,
procedure signature rules, helper labels, and backend capability facts.

The crate is deliberately target-neutral. It does not emit class files, CIL,
BEAM chunks, or WASM modules itself. Existing backend crates consume these
plans while keeping their own byte encoders.
