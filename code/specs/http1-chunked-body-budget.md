# HTTP/1 Chunked Body Budget

## Status

Bounded policy prerequisite for a future repository-owned HTTP/1 chunked
transfer-coding decoder.

## Purpose

Per-chunk bounds do not bound a complete decoded body. A peer can send many
individually acceptable chunks whose combined payload exceeds the caller's
response limit, or an excessive number of tiny chunks whose framing work
amplifies CPU cost. This package owns the cumulative policy for those two
dimensions without parsing wire bytes or allocating body storage.

## Contract

`Http1ChunkedBodyLimits` retains two explicit caller-owned ceilings:

- maximum decoded body bytes; and
- maximum non-terminal chunk count.

Both ceilings may be zero. This permits a policy that accepts only an empty
chunked body without inventing a special unlimited value.

`Http1ChunkedBodyBudget::new` starts at zero decoded bytes and zero
non-terminal chunks. `advance` consumes the budget and a strictly positive
already-validated chunk size. It returns a new budget only when:

1. the next non-terminal chunk count does not exceed the configured count;
2. decoded-length addition does not overflow `usize`; and
3. the resulting decoded length does not exceed the configured byte ceiling.

Failures retain no input bytes and expose stable categories for a zero-sized
non-terminal chunk, too many chunks, arithmetic overflow, and excessive
decoded length. Consuming the previous value prevents accidental direct
branching, but this is not anti-replay or stream-binding evidence: later
composition must own one budget for the complete response body and must not
reset, reconstruct, or replay it before terminal completion.

## Security Boundary

The package receives only lengths. It does not parse chunk-size lines, chunk
extensions, chunk data, CRLF framing, a terminal chunk, or trailers. It does
not allocate or assemble a response body, count encoded wire bytes, perform
I/O or TLS, open a socket, interpret credentials, emit audit or trace events,
or authorize OAuth HTTPS transport. A later decoder must compose this policy
with all framing primitives and its own encoded-input and transport bounds.
