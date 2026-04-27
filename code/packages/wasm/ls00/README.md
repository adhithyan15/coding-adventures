# ls00-wasm

WebAssembly bindings for the [ls00](../../rust/ls00/) LSP framework, exposing a message-passing LSP server for use in browsers, web workers, and Deno.

## What This Does

This package wraps the Rust `coding-adventures-ls00` LSP framework with `wasm-bindgen` so it can run in environments without stdio (browsers, web workers, VS Code web extensions). Instead of reading/writing Content-Length-framed messages on stdin/stdout, the WASM server uses a callback-based message-passing API.

## How It Fits in the Stack

```
+------------------------------+
|  Browser / Web Worker / Deno |  <-- JavaScript consumer
+------------------------------+
|  ls00-wasm (this)            |  <-- wasm-bindgen wrapper (cdylib)
+------------------------------+
|  ls00 (Rust)                 |  <-- core LSP framework
+------------------------------+
|  json-rpc (Rust)             |  <-- JSON-RPC types (used for capabilities)
+------------------------------+
```

The core `ls00` crate contains all the LSP logic (document management, parse caching, capability advertisement, semantic token encoding). This crate adds only the WASM transport layer.

## Building

```bash
# Native unit tests (no WASM tooling needed):
cargo test

# Build WASM for browsers:
wasm-pack build --target web

# Build WASM for Node.js/Deno:
wasm-pack build --target nodejs
```

## Usage (JavaScript)

```javascript
import init, { WasmLspServer, WasmLanguageBridge } from './ls00_wasm.js';

await init();  // initialize WASM module

// 1. Create a language bridge with tokenize and parse callbacks
const bridge = new WasmLanguageBridge(
  (source) => {
    // Tokenize: return JSON array of tokens
    const tokens = myLexer.tokenize(source);
    return JSON.stringify(tokens.map(t => ({
      token_type: t.type,
      value: t.value,
      line: t.line,
      column: t.column
    })));
  },
  (source) => {
    // Parse: return JSON with ast + diagnostics
    const result = myParser.parse(source);
    return JSON.stringify({
      ast: JSON.stringify(result.ast),
      diagnostics: result.errors.map(e => ({
        range: { start: e.start, end: e.end },
        severity: 1,
        message: e.message
      }))
    });
  }
);

// 2. Create the server with a callback for outgoing notifications
const server = new WasmLspServer(bridge, (jsonStr) => {
  // Handle server-initiated messages (e.g., diagnostics)
  const msg = JSON.parse(jsonStr);
  if (msg.method === 'textDocument/publishDiagnostics') {
    showDiagnostics(msg.params);
  }
});

// 3. Send incoming messages and handle responses
const initResponse = server.handleMessage(JSON.stringify({
  jsonrpc: '2.0',
  id: 1,
  method: 'initialize',
  params: {}
}));
console.log(JSON.parse(initResponse));  // { capabilities: {...} }
```

## API Reference

### WasmLanguageBridge

| Method | Description |
|--------|-------------|
| `new WasmLanguageBridge(tokenizeFn, parseFn)` | Create a bridge with JS callbacks |

### WasmLspServer

| Method | Description |
|--------|-------------|
| `new WasmLspServer(bridge, sendCallback)` | Create a server with a language bridge and outgoing message callback |
| `handleMessage(json)` | Process an incoming JSON-RPC message; returns response string (or empty for notifications) |
| `isInitialized()` | Whether the server has received `initialize` |
| `isShutdown()` | Whether the server has received `shutdown` |

### Supported LSP Methods

**Requests** (return a response via `handleMessage`):
- `initialize` / `shutdown`
- `textDocument/hover`
- `textDocument/definition`
- `textDocument/references`
- `textDocument/completion`
- `textDocument/rename`
- `textDocument/documentSymbol`
- `textDocument/semanticTokens/full`
- `textDocument/foldingRange`
- `textDocument/signatureHelp`
- `textDocument/formatting`

**Notifications** (no response, side effects via callback):
- `initialized`
- `textDocument/didOpen`
- `textDocument/didChange`
- `textDocument/didClose`
- `textDocument/didSave`
