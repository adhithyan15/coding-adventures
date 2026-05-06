# board-vm-eject

Target-independent eject artifact helpers for Board VM.

This crate packages validated BVM modules with the metadata a board-specific
backend needs when the interactive session is ready to become standalone code.
It does not know about Uno R4, ESP32, Pico, flashing tools, or simulator
internals; those layers consume the artifact and decide whether to embed
bytecode, store it in board flash, or pass it to a later AOT backend.
