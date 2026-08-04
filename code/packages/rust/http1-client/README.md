# http1-client

`http1-client` is Venture's first concrete network transport. It composes the
existing `url-parser`, `tcp-client`, `http1`, and `http-core` packages into a
bounded synchronous HTTP/1.0 GET client.

## What It Provides

- HTTP-only URL validation and origin-form request construction
- DNS/TCP connection and one-request-per-connection HTTP/1.0 exchange
- Content-Length, bodyless, and read-until-EOF response framing
- Relative 301/302 redirect following
- Configurable connection, response-head, response-body, and redirect limits
- Structured errors for every transport stage

```rust,no_run
use http1_client::get;

let response = get("http://info.cern.ch/")?;
assert_eq!(response.head.status, 200);
println!("{} bytes from {}", response.body.len(), response.final_url);
# Ok::<(), http1_client::HttpClientError>(())
```

The client deliberately sends HTTP/1.0 with `Connection: close`. HTTPS,
cookies, caching, authentication, request bodies, and HTTP/1.1 chunk decoding
remain outside this package.

## Browser Integration

The returned body bytes can be parsed by `coding-adventures-html-parser`.
Image responses can be adapted to `html-to-paint::FetchedImage` by the Venture
host without coupling the protocol package to HTML or paint policy.

## Development

```bash
cargo test -p http1-client -- --nocapture
```
