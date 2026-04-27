# window-win32

Win32 desktop window backend for `window-core`.

## Layer 11

This package is part of Layer 11 of the coding-adventures computing stack.

## What It Contains

This first slice is a Win32 backend shell. It currently:

- validates which `window-core` requests make sense for Win32
- chooses the `HWND` host model expected by Direct2D or software renderers
- rejects Apple-only and browser-only surface requests

Actual `HWND` creation and message-pump integration are the next step.

## Dependencies

- window-core

## Development

```bash
cargo test -p window-win32
```
