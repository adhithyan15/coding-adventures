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
- target upload plans for Arduino CLI, ESP ROM serial, and Pico UF2 adapters

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

Firmware flashing follows the same rule: frontends can present native options,
but they should query Rust-owned upload helpers to learn the artifact kind,
transport requirements, reset behavior, port hint, adapter steps, and Arduino
CLI platform/FQBN metadata for each board family. Generic clients can use
`upload_plan_for_target`; Arduino CLI-specific clients can use
`arduino_cli_upload_options_for_target` so native USB, USB-serial bridge, and
external-adapter port selection stay in Rust instead of language-local tables.
`arduino_cli_port_discovery_for_target` adds the matching discovery/reset
metadata, including the 1200-baud native USB bootloader touch convention and
runtime port rediscovery expectation for boards whose Arduino package owns that
reset path.

Frontends that already have an Arduino CLI platform or FQBN should pass it back
to Rust rather than carrying their own selector table. `detect_target` resolves
unique FQBNs to a concrete board target, and `targets_for_upload_selector`
returns every target for broader or intentionally shared package identities.
