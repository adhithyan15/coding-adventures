# tls13-cookie

`tls13-cookie` is an allocation-free owner for the RFC 9846 section 4.3.2
`Cookie` extension-data shape and the exact echo relation used after a
`HelloRetryRequest`.

The decoder requires a non-empty, two-byte-length-prefixed cookie to consume
the complete input and applies a caller-selected payload ceiling before
exposing any borrowed bytes. The encoder performs all length and capacity
checks before changing caller-owned output. `bind_cookie_echo` then consumes
the retry and second-ClientHello parses and proves that their complete encoded
extension data is byte-for-byte equal. Debug values and stable failures expose
neither cookie contents nor their lengths.

```rust
use tls13_cookie::{bind_cookie_echo, decode_cookie, CookieLimits};

let limits = CookieLimits::new(64)?;
let retry_bytes = [0x00, 0x03, 1, 2, 3];
let client_bytes = [0x00, 0x03, 1, 2, 3];
let retry = decode_cookie(&retry_bytes, limits)?;
let client = decode_cookie(&client_bytes, limits)?;
let echoed = bind_cookie_echo(retry, client)?;
assert_eq!(echoed.client_hello().cookie(), &[1, 2, 3]);
# Ok::<(), tls13_cookie::CookieError>(())
```

This package treats cookie contents as opaque. It does not locate extension
type 44 in a handshake message, apply extension-context policy, generate a
cookie, authenticate server state, compare other ClientHello fields, require
retry progress, reject a second retry, update a transcript, sequence a
handshake, send alerts, derive keys, protect records, validate certificates,
establish trust, check revocation, perform I/O, authorize HTTP/OAuth transport,
or make concrete HTTPS or live-provider use ready.

## Development

```bash
bash BUILD
cargo clippy -p tls13-cookie --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-cookie --no-deps
```
