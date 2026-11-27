# Unix History Simulator (Rust)

The [HOS00](../../../specs/HOS00-historical-os-simulator.md) library models file
lookup, a descriptor table and the kernel calls that connect them. The
components are available separately from the JSON command adapter.

Use `cargo run -p unix-history-simulator -- cat /hello` and
`cargo test -p unix-history-simulator` in the Rust workspace.
