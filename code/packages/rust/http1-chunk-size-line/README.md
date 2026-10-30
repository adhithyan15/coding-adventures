# http1-chunk-size-line

Strict, bounded, allocation-free decoding of one HTTP/1 chunk-size line.

The package requires exact CRLF termination, bounds the line before exposing
results, parses checked hexadecimal sizes, enforces a caller-owned chunk limit,
and retains the exact borrowed line plus unconsumed tail. Chunk extensions are
rejected until their grammar and policy receive a separate review.

This is not a chunked transfer-coding decoder. It does not consume chunk data,
validate the data CRLF, accumulate decoded length, parse the terminal chunk or
trailers, perform I/O or TLS, interpret or release credentials, emit audit or
trace events, or make concrete HTTPS/live-provider use ready.

## Development

```bash
cargo test -p http1-chunk-size-line --all-targets
cargo clippy -p http1-chunk-size-line --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p http1-chunk-size-line --no-deps
```
