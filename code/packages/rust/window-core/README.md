# window-core

Backend-neutral windowing primitives and renderer-facing host handles.

## Layer 11

This package is part of Layer 11 of the coding-adventures computing stack.

## What It Contains

`window-core` defines the shared native-window contract that other languages can
mirror:

- `WindowId`, logical sizes, and physical sizes
- `WindowAttributes` and `WindowBuilder`
- normalized `WindowEvent` values for resize, redraw, focus, pointer, and key
  input
- explicit renderer targets such as AppKit and Win32 handles
- `Window` and `WindowBackend` traits for Rust native implementations

This crate is the Rust-native reference for the shared abstraction. The pure
browser canvas backend is intentionally not implemented here; it belongs in
TypeScript while preserving the same contract.

## Development

```bash
cargo test -p window-core
```
