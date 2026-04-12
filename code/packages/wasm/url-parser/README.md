# url-parser (WebAssembly)

WebAssembly bindings for the Rust `url-parser` crate via `wasm-bindgen`.

Exposes the full URL parsing API to JavaScript running in a browser or Node.js through WebAssembly. This is a thin adapter -- all parsing logic lives in the Rust `url-parser` crate.

## Architecture

```text
  JavaScript  --wasm-bindgen-->  WasmUrl (this crate)  -->  url_parser::Url
  (browser)                      (thin adapter)              (all the work)
```

## Usage (JavaScript)

```javascript
import { WasmUrl, percentEncode, percentDecode } from 'url-parser-wasm';

const url = new WasmUrl("http://example.com:8080/path?q=1#frag");
console.log(url.scheme);        // "http"
console.log(url.host);          // "example.com"
console.log(url.port);          // 8080
console.log(url.effectivePort); // 8080
console.log(url.toUrlString()); // "http://example.com:8080/path?q=1#frag"

const resolved = url.resolve("../other.html");
console.log(resolved.path);    // "/other.html"

console.log(percentEncode("hello world")); // "hello%20world"
console.log(percentDecode("%20"));         // " "
```

## Development

```bash
# Run tests (native, not wasm32)
cargo test -- --nocapture
```
