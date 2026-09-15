## Unreleased — FLOW-MATIC WASM EOF (VM-039b)

Add the `input_more` builtin as `env.__input_more() -> i64` and connect
feature detection, import indices and destination writes. The host returns
1 when another input field remains and 0 at EOF, without consuming input.
Four existing FLOW-MATIC matrix rows now execute on WASM; a shared-buffer
regression verifies repeated peeks before and after integer consumption.

