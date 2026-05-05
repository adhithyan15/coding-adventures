# jit-loader-macos

Bottom-of-stack primitive for in-process JIT execution on Apple Silicon.

Hands you a `CodePage` whose contents are machine code generated at
runtime, ready to call from Rust as an `extern "C"` function.

## Quick start

```rust
use jit_loader_macos::CodePage;

// Hand-encoded ARM64 for `fn() -> u64 { 42 }`:
let bytes: [u8; 8] = [
    0x40, 0x05, 0x80, 0xD2,  // movz x0, #42
    0xC0, 0x03, 0x5F, 0xD6,  // ret
];
let page = CodePage::new(&bytes).expect("install");
let f: extern "C" fn() -> u64 = unsafe { page.as_function() };
assert_eq!(f(), 42);
```

## Where it sits

```
IIR / CIR  →  aarch64-backend  →  CodePage::new(bytes)  →  extern "C" fn pointer
                                          ↓
                                   in-process execution
```

## Status

V1: macOS / Apple Silicon only.  Linux + x86-64 ports are
out-of-scope for now.  See CHANGELOG for the full coverage.
