# form-urlencoded

Bounded repository-owned `application/x-www-form-urlencoded` serialization.

The crate validates an ordered, caller-borrowed sequence of UTF-8 name/value
pairs under explicit ceilings. It preserves empty and duplicate fields and
serializes into caller-owned storage without allocation: ASCII alphanumerics
and `*`, `-`, `.`, `_` pass through, spaces become `+`, and all other UTF-8
bytes use uppercase percent encoding. The complete output size is checked before
writing, so an undersized output remains unchanged. Stable errors retain no
input bytes or metadata, and Debug redacts names and values.

This package does not validate OAuth field semantics, interpret or release
credentials, wipe caller output, emit audits, create HTTP framing, perform TLS
or I/O, or make OAuth, HTTPS, or live-provider use ready. Secret-bearing callers
must use audited custody and wipe output after its separately audited use.

## Validation

```bash
bash BUILD
cargo clippy -p form-urlencoded --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p form-urlencoded --no-deps
```
