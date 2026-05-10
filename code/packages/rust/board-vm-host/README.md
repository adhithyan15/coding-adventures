# board-vm-host

Reference host-side builders for Board VM.

This crate intentionally stops below physical transport. It builds validated
BVM bytecode modules and byte-exact BVM01 request frames into caller-provided
buffers so serial, WebSerial, BLE, TCP, REPL, and language SDK layers can reuse
the same session logic.

`write_module` wraps caller-produced bytecode and constant-pool bytes in a BVM1
module header without allocating. Language frontends can use that generic
builder directly, while `write_blink_module` remains the smallest fixture-driven
program generator.
