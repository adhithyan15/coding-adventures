# window-appkit

AppKit-backed desktop window backend for `window-core`.

## Layer 11

This package is part of Layer 11 of the coding-adventures computing stack.

## What It Contains

This package supports a real macOS host path while staying small:

- validates which `window-core` requests make sense on AppKit
- creates an `NSApplication` and `NSWindow`
- exposes AppKit render-target handles on the returned `Window`
- runs the AppKit event loop for native launch testing
- translates AppKit wheel input into shared `WindowEvent::Scroll` values
- lets a main-thread host install a normalized event handler on its window
- rejects renderer preferences that belong to other platforms

Metal-layer attachment is available for renderer hosts. Pointer-button,
keyboard, resize, and close translation remain future event slices.

## Dependencies

- window-core
- objc-bridge

## Development

```bash
cargo test -p window-appkit
cargo run -p window-appkit --example launch_window
```
