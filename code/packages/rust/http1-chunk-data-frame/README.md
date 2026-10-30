# http1-chunk-data-frame

Strict, bounded, allocation-free decoding of one non-terminal HTTP/1 chunk-data
frame after its hexadecimal size has already been validated.

The package enforces a caller-owned maximum chunk size, rejects zero so a
terminal chunk cannot be mistaken for ordinary data, requires the exact CRLF
after the declared data bytes, and retains borrowed data, encoded-frame, and
remainder slices. Returned Debug output and failures never expose those bytes.

This is not a chunked transfer-coding decoder. It does not parse the chunk-size
line or extensions, accumulate decoded body length, parse the terminal chunk or
trailers, perform I/O or TLS, interpret or release credentials, emit audit or
trace events, or make concrete HTTPS/live-provider use ready.

## Development

```bash
cargo test -p http1-chunk-data-frame --all-targets
cargo clippy -p http1-chunk-data-frame --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p http1-chunk-data-frame --no-deps
```
