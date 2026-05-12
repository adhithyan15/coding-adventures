# MACSYMA Browser REPL

Browser-hosted MACSYMA REPL backed by `@coding-adventures/macsyma-runtime`.

This proves the pure TypeScript symbolic runtime can be bundled into a browser
application without a Python, Rust, server, or WASM dependency.

The editor can also load local `.mac` source files through the browser File API.
Imported files are evaluated by the same in-memory `MacsymaSession`, so history,
bindings, displayed statements, and suppressed statements behave like typed
input.
