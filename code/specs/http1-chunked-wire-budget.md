# HTTP/1 Chunked Wire Budget

## Status

Bounded policy prerequisite for a future repository-owned HTTP/1 chunked
transfer-coding decoder.

## Purpose

A decoded-body limit does not bound the bytes consumed from the wire. A peer
can spend disproportionate transfer and parsing work on chunk-size lines,
per-chunk delimiters, the terminal chunk, and trailer framing while delivering
little decoded payload. This package owns one cumulative encoded-input ceiling
without parsing wire bytes, allocating input buffers, or performing I/O.

## Contract

`Http1ChunkedWireLimit` retains one exact caller-owned maximum for the complete
chunked transfer-coding, measured from the first octet of the first chunk-size
line through the final octet of the trailer section's terminating CRLF. The
ceiling may be zero; no integer value means unlimited.

`Http1ChunkedWireBudget::new` starts at zero accounted wire bytes. `advance`
consumes the budget and a strictly positive count of newly consumed wire
bytes. It returns a new budget only when:

1. cumulative addition does not overflow `usize`; and
2. the resulting wire length does not exceed the configured ceiling.

Failures retain no input bytes and expose stable categories for a zero-length
transition, arithmetic overflow, and ceiling excess. Consuming the previous
value prevents accidental direct branching, but this is not anti-replay or
stream-binding evidence: later composition must own exactly one budget for the
complete transfer-coding and account every consumed octet exactly once without
reset, reconstruction, omission, or replay.

The future decoder must also cap each read or slice operation by the remaining
budget before it obtains or allocates storage for peer input. Accounting only
after an unbounded read does not enforce this boundary.

## Security Boundary

The package receives lengths only. It does not read or parse chunk-size lines,
chunk extensions, chunk data, CRLF framing, the terminal chunk, or trailers. It
does not bound decoded body bytes or chunk count, allocate input or output
storage, prevent a caller from omitting or double-counting bytes, perform I/O
or TLS, open a socket, interpret credentials, emit audit or trace events, or
authorize OAuth HTTPS transport. A later decoder must compose this policy with
all framing primitives, the decoded-body budget, pre-read buffer bounds, and
the repository-owned transport stack.
