# http1-chunked-wire-budget

Dependency-free cumulative encoded-input policy for a future repository-owned
HTTP/1 chunked transfer-coding decoder.

A decoded-body limit does not bound chunk-size lines, delimiters, the terminal
chunk, or trailer framing. `Http1ChunkedWireBudget` consumes and replaces its
prior state for each strictly positive number of newly consumed wire bytes,
rejecting length overflow and aggregate wire-length excess.

Consumption prevents accidental direct branching; it is not anti-replay or
stream-binding evidence. A later decoder must own one budget for the complete
transfer-coding and account every consumed octet exactly once. It must also cap
reads and allocations by the remaining budget before obtaining peer input;
accounting after an unbounded read is insufficient.

```rust
use http1_chunked_wire_budget::{Http1ChunkedWireBudget, Http1ChunkedWireLimit};

let limit = Http1ChunkedWireLimit::new(12);
let budget = Http1ChunkedWireBudget::new(limit)
    .advance(5)?
    .advance(7)?;
assert_eq!(budget.accounted_wire_bytes(), 12);
assert_eq!(budget.remaining_wire_bytes(), 0);
# Ok::<(), http1_chunked_wire_budget::Http1ChunkedWireBudgetError>(())
```

The package receives lengths only. It does not parse any chunked framing,
bound decoded output or chunk count, allocate buffers, perform I/O or TLS,
handle credentials, emit audit or trace events, or make concrete
HTTPS/live-provider use ready.

## Development

```bash
cargo test -p http1-chunked-wire-budget --all-targets
cargo clippy -p http1-chunked-wire-budget --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p http1-chunked-wire-budget --no-deps
```
