# http1-chunked-body-budget

Dependency-free cumulative byte and non-terminal chunk-count policy for a
future repository-owned HTTP/1 chunked transfer-coding decoder.

Per-chunk limits alone are insufficient: a peer can split an oversized body
across many acceptable chunks, or send many tiny chunks to amplify framing
work. `Http1ChunkedBodyBudget` consumes and replaces its prior state for each
strictly positive already-validated chunk size, rejecting count exhaustion,
length overflow, and aggregate decoded-length excess.

Consumption prevents accidental direct branching; it is not anti-replay or
stream-binding evidence. A later decoder must own one budget for the complete
response body and must not reset, reconstruct, or replay it before terminal
completion.

```rust
use http1_chunked_body_budget::{Http1ChunkedBodyBudget, Http1ChunkedBodyLimits};

let limits = Http1ChunkedBodyLimits::new(10, 2);
let budget = Http1ChunkedBodyBudget::new(limits)
    .advance(4)?
    .advance(6)?;
assert_eq!(budget.decoded_body_bytes(), 10);
assert_eq!(budget.non_terminal_chunk_count(), 2);
# Ok::<(), http1_chunked_body_budget::Http1ChunkedBodyBudgetError>(())
```

The package receives lengths only. It does not parse chunk-size lines,
extensions, data, CRLF framing, the terminal chunk, or trailers; assemble a
body; count encoded wire bytes; perform I/O or TLS; handle credentials; emit
audit or trace events; or make concrete HTTPS/live-provider use ready.

## Development

```bash
cargo test -p http1-chunked-body-budget --all-targets
cargo clippy -p http1-chunked-body-budget --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p http1-chunked-body-budget --no-deps
```
