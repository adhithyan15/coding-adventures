# window-appkit

AppKit-backed desktop window backend for `window-core`.

## Layer 11

This package is part of Layer 11 of the coding-adventures computing stack.

## What It Contains

This first slice now supports a real macOS smoke path while staying small:

- validates which `window-core` requests make sense on AppKit
- creates an `NSApplication` and `NSWindow`
- exposes AppKit render-target handles on the returned `Window`
- runs the AppKit event loop for native launch testing
- rejects renderer preferences that belong to other platforms

The backend still does not translate live AppKit events into `WindowEvent`
values yet, and Metal-layer attachment is still the next step after the window
launch path.

## Dependencies

- window-core

## Development

```bash
cargo test -p window-appkit
cargo run -p window-appkit --example launch_window
```
