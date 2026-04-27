# tcp-client (WebAssembly Stub)

WebAssembly stub for the tcp-client crate — TCP sockets are unavailable in browser Wasm.

Raw TCP sockets are prohibited by the browser security model. This package exports the `ConnectOptions` type for downstream type-checking, but `connect()` always returns an error.

For actual TCP networking:
- **Node.js**: Use the TypeScript `tcp-client` package
- **Browser**: Use `fetch()` or `WebSocket` APIs
- **Native**: Use the Rust `tcp-client` crate

## Development

```bash
cargo test -- --nocapture
```
