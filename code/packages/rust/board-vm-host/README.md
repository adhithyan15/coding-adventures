# board-vm-host

Reference host-side builders for Board VM.

This crate intentionally stops below physical transport. It builds validated
BVM bytecode modules and byte-exact BVM01 request frames into caller-provided
buffers so serial, WebSerial, BLE, TCP, REPL, and language SDK layers can reuse
the same session logic.
