# http1-empty-trailer-section

Strict, allocation-free decoding of the empty HTTP/1 trailer section that may
follow an already validated zero-size chunk line.

The package accepts only one exact CRLF and retains its borrowed encoding plus
the unconsumed remainder. Any field line is rejected until trailer syntax,
field-name restrictions, declared-trailer policy, and aggregate bounds receive
a separate review. Debug output and failures never expose caller bytes.

This is not a terminal-chunk or chunked transfer-coding decoder. It does not
parse the zero-size line or extensions, accept trailer fields, accumulate body
length, perform I/O or TLS, interpret or release credentials, emit audit or
trace events, or make concrete HTTPS/live-provider use ready.

## Development

```bash
cargo test -p http1-empty-trailer-section --all-targets
cargo clippy -p http1-empty-trailer-section --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p http1-empty-trailer-section --no-deps
```
