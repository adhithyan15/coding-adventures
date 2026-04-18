# window-c

C ABI wrapper for the shared native windowing contract.

## What It Contains

- C enums and POD structs mirroring `window-core`
- an opaque native window handle for FFI consumers
- AppKit-backed native window creation on Apple platforms
- render-target inspection for AppKit windows
- thread-local last-error reporting for bridge callers

This crate is the bridge seam for languages that already consume repository
owned C wrappers such as Go, Swift, C#, and F#.

## Development

```bash
cargo test -p window-c
```
