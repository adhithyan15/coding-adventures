# board-vm-language-core

`board-vm-language-core` is the Rust-owned boundary for high-level Board VM
frontends such as Ruby, Python, Lua, Java, and future REPLs.

Language packages should treat themselves as syntax sugar over this crate. The
binary protocol details stay in Rust:

- BVM module construction for common programs such as onboard LED blink
- request ids and request frame construction
- COBS stream framing and CRC handling
- program upload/run frame construction
- raw response frame decoding and payload offset reporting

This crate exports both a normal Rust API and a small C ABI-friendly surface.
Ruby can load the dynamic library with `Fiddle`, Python can use `ctypes` or
`cffi`, and other runtimes can wrap the same symbols without reimplementing the
wire format.

The crate deliberately does not open serial ports or USB devices yet. Language
frontends may own ergonomic transport discovery in the short term, but the bytes
they send and decode should come from this Rust core.
