# board-vm-eject

Target-independent eject artifact helpers for Board VM.

This crate packages validated BVM modules with the metadata a board-specific
backend needs when the interactive session is ready to become standalone code.
It does not know about Uno R4, ESP32, Pico, flashing tools, or simulator
internals; those layers consume the artifact and decide whether to embed
bytecode, store it in board flash, or pass it to a later AOT backend.

`build_module_eject_artifact` accepts any already-built BVM module plus the
capability list discovered or inferred by the frontend/compiler. It re-derives
the required capabilities from the BVM module before packaging and rejects
mismatches, keeping frontend-owned metadata thin over the Rust-owned bytecode
contract. The blink helper remains as the MVP program generator, but the generic
builder is the path for REPL sessions and non-JS frontends once they emit
bytecode directly.

Each packaged artifact can produce a compact summary covering program identity,
slot, boot policy, BVM module metadata, CRC, required-capability count, and byte
length. CLI reports and board-specific firmware crates should use that
Rust-owned summary instead of reparsing generated constants or reassembling the
metadata contract themselves.
