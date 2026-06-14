# silicon-rust-ruby-native

Rust workspace crate that compiles the silicon simulation stack
(`device-physics`, `mosfet-models`, `fab-process-simulation`) into a Ruby
native extension loaded by the `silicon_rust_ruby` gem.

## What it does

Provides 26 Ruby-callable functions on the `SiliconRustRuby` module via
zero-dependency `ruby-bridge` (raw `extern "C"` Ruby C API — no Magnus,
no rb-sys, no bindgen, no Ruby headers at build time).

## How it fits in the stack

```
silicon_rust_ruby (gem, code/packages/ruby/silicon_rust_ruby/)
  ↓ require "silicon_rust_ruby_native"
silicon-rust-ruby-native (this crate)     ← cdylib
  ↓ Rust function calls
device-physics   mosfet-models   fab-process-simulation
```

## Building

The `silicon_rust_ruby` gem's Rakefile and extconf.rb invoke:

```bash
cargo build --release -p silicon-rust-ruby-native
```

The resulting `libsilicon_rust_ruby_native.{so,dylib}` / `silicon_rust_ruby_native.dll`
is copied by the gem into its `lib/` directory.

## Platform notes

| Platform | Linker behaviour |
|----------|-----------------|
| Linux    | ELF auto-resolves Ruby symbols at `dlopen()` time |
| macOS    | `build.rs` emits `-undefined dynamic_lookup` |
| Windows  | `build.rs` locates `libruby` via `rbconfig` and links it |

## Testing

Tests are in the Ruby gem's minitest suite (`test/silicon_rust_ruby_test.rb`).
Run via `bundle exec rake test` from `code/packages/ruby/silicon_rust_ruby/`.
