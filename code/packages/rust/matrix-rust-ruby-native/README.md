# matrix-rust-ruby-native — Rust half of the `matrix_rust_ruby` gem

This crate is the Rust-side native extension that powers the
`matrix_rust_ruby` Ruby gem.  It exposes exactly one method to Ruby:

```ruby
MatrixRustRuby.run_graph_on_cpu(envelope_json_str) -> envelope_json_str
```

Pass in a matrix-ir-json envelope (a JSON string containing a graph
definition plus hex-encoded input tensors), get back a JSON envelope
with the hex-encoded outputs.  On any error — malformed JSON, missing
fields, invalid hex, executor failure — raises a Ruby `RuntimeError`.

## How it fits into the multi-language stack

```text
┌──────────────────────────────────────────────────────────────────┐
│  ml_framework_core (Ruby gem)              ← PRs #4–#8           │
│    idiomatic Tensor + autograd, Ruby ergonomics                  │
│    ↓                                                              │
│  matrix_rust_ruby (Ruby gem)               ← PR #3                │
│    require_relative the native extension below                   │
│    ↓                                                              │
│  matrix_rust_ruby_native.{so,bundle,dll}   ← THIS CRATE           │
│    Ruby module: MatrixRustRuby                                   │
│    Singleton method: run_graph_on_cpu(envelope_str)              │
│    ↓                                                              │
│  c-bridge (Rust workspace crate)           ← PR #1 (merged)       │
│    pure-Rust run_graph_on_cpu_via_json_envelope                  │
│    ↓                                                              │
│  matrix-ir-json → matrix-ir → matrix-runtime → matrix-cpu        │
└──────────────────────────────────────────────────────────────────┘
```

Three things to notice:

1. **All the heavy lifting lives in `c-bridge`.**  This crate is ~30 LOC
   of Rust that just translates between `VALUE` and `String` and
   raises a Ruby exception on `Err`.  That keeps envelope shape and
   error semantics defined in exactly one place across every future
   language binding (Ruby today; Lua, Go, Swift on deck).

2. **No `unsafe` in this crate.**  Every Ruby C API call goes through
   the workspace `ruby-bridge` crate's safe wrappers
   (`define_module`, `str_to_rb`, `str_from_rb`, `raise_error`, ...).
   `ruby-bridge` owns the `unsafe`; we get to write boring code.

3. **No panics across the FFI boundary.**  `c-bridge` already wraps
   its core in `std::panic::catch_unwind` and surfaces panics as
   `Err(String)`, so a runaway panic in the executor turns into a
   Ruby `RuntimeError`, never undefined behaviour.

## Building

```bash
cargo build -p matrix-rust-ruby-native --release
# Produces target/release/libmatrix_rust_ruby_native.{so,dylib,dll}
```

In practice you don't build it by hand — the `matrix_rust_ruby` gem's
`extconf.rb` runs this command for you when the gem is installed.

### Why a workspace crate (not just an `ext/` directory inside the gem)?

The conduit gem follows the more common pattern of nesting its Rust
native ext inside `ext/conduit_native/`.  We went the other way for
`matrix-rust-ruby-native`:

* **Workspace integration.**  As a workspace member it gets `cargo
  check`-ed and `cargo test`-ed by CI on every PR alongside every
  other Rust crate.  An `ext/`-nested crate would need separate CI
  wiring.
* **Path deps stay short.**  `path = "../c-bridge"` is one hop;
  `path = "../../../../rust/c-bridge"` (the conduit-style path)
  works but is hard to grep.
* **One Rust toolchain across the workspace.**  No risk of the
  gem-installed Rust drifting from the workspace's edition / MSRV.

The gem (PR #3) will reference the built `.so` by absolute path,
discovered via `cargo metadata`.  This is documented in the gem's
`extconf.rb`.

## Memory + thread safety

* **Thread-safe.**  Each call to `run_graph_on_cpu` constructs a
  fresh `CpuExecutor` inside `c-bridge`; no shared mutable state
  between calls.  Ruby can call this from any thread.
* **No allocator mixing.**  The crate never hands a raw pointer to
  Ruby.  Strings cross the boundary as Ruby `String` objects
  (allocated by Ruby) and Rust `String` (allocated by Rust); each side
  frees what it owns.

## Tests

The Ruby-level integration tests live in the `matrix_rust_ruby` gem
(PR #3).  This crate is a thin shim; once Ruby's loaded the `.so` and
calls our singleton method, all the interesting behaviour is in
`c-bridge`, which has its own 8 + 7 unit/integration tests
(`cargo test -p c-bridge`).

## What's next

PR #3 will create the `matrix_rust_ruby` Ruby gem: `.gemspec`,
`extconf.rb`, `lib/coding_adventures/matrix_rust_ruby.rb`, and
`test/test_matrix_rust_ruby.rb`.  Once that lands, PRs #4–#8 build
the idiomatic `ml_framework_core` Ruby layer on top.
