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

This crate exports a normal Rust API for the repo's language bridge packages.
Ruby bindings should be built on `ruby-bridge`, Python bindings on
`python-bridge`, and other runtimes should follow the same pattern with their
own bridge crates. Those bridge packages provide the language-native surface;
this crate provides the shared Board VM bytes underneath them.

The small C ABI-friendly surface is intentionally secondary. It is useful for
tools, experiments, or runtimes that do not yet have a first-class bridge crate,
but it should not become a parallel protocol implementation path for Ruby,
Python, or any other supported language.

The crate deliberately does not open serial ports or USB devices yet. Language
frontends may own ergonomic transport discovery in the short term, but the bytes
they send and decode should come from this Rust core.
