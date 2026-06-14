# Changelog

## Unreleased

### Added — v0.1.0: Ruby ↔ matrix-cpu native extension

First release.  Ships the Rust half of the forthcoming `matrix_rust_ruby`
Ruby gem.

#### What it exports

Exactly one Ruby method, registered at extension-load time:

```ruby
MatrixRustRuby.run_graph_on_cpu(envelope_json_str) -> envelope_json_str
```

`envelope_json_str` is a matrix-ir-json envelope: a JSON string
containing the graph definition + hex-encoded input tensors.  The
return value is a JSON envelope with hex-encoded outputs.  On any
error (malformed JSON, missing fields, invalid hex, executor failure)
the method raises a Ruby `RuntimeError` with a descriptive message.

#### Architecture: thin shim over `c-bridge`

The implementation is ~30 LOC.  All graph parsing, planning, and CPU
execution happens in the workspace `c-bridge` crate's pure-Rust
`run_graph_on_cpu_via_json_envelope` function (~50 LOC of envelope
plumbing on top of `matrix-ir-json` + `matrix-cpu`).  We just:

1. `str_from_rb` the Ruby argument into a Rust `String`
2. Call `c_bridge::run_graph_on_cpu_via_json_envelope`
3. Translate `Result<String, String>` into either `str_to_rb` or
   `raise_runtime_error`

Putting the work in `c-bridge` keeps envelope shape + error semantics
defined in exactly one place across every language binding (Ruby
today; Lua, Go, Swift on deck).

#### Why depend on `c-bridge` rather than the lower-level matrix-* crates?

The first instinct is to depend directly on `matrix-ir-json`,
`matrix-runtime`, and `matrix-cpu` and copy the envelope helper.  We
chose `c-bridge` because:

- **Single source of truth.**  Envelope shape changes (new ops, new
  tensor dtypes) flow through one crate, not N.
- **Panic safety inherited.**  `c-bridge`'s core is wrapped in
  `catch_unwind`; we get that for free.
- **Easier future refactor.**  When we DRY into a `matrix-rust-core`
  crate (likely once Lua + Go bindings land), changing the path is
  one edit.

#### Safety

- **Zero `unsafe` in this crate.**  Every Ruby C API call goes through
  the workspace `ruby-bridge` safe wrappers
  (`define_module`, `define_singleton_method_raw`, `str_to_rb`,
  `str_from_rb`, `raise_runtime_error`).
- **Never panics across FFI.**  `c-bridge` wraps its executor in
  `std::panic::catch_unwind`; panics become `Err(String)` and then a
  Ruby `RuntimeError`.
- **Type-checks input.**  Non-String envelope argument → clean
  `RuntimeError`, not undefined behaviour.

#### Build script (`build.rs`)

Copied verbatim from `conduit_native`'s build.rs — same Windows /
macOS linking dance.  On macOS we pass `-undefined dynamic_lookup` so
Ruby symbols stay unresolved in the .dylib (the host ruby process
resolves them at dlopen time).  On Windows we discover Ruby's import
library via `rbconfig` and link against it.  Linux needs nothing.

#### Crate type

`cdylib` only (no `rlib`).  Nothing in the Rust workspace depends on
this crate — it exists purely to be dlopen()'d by Ruby.  Hence the
single crate type.

#### What's next

- PR #3: `matrix_rust_ruby` Ruby gem — `.gemspec`, `extconf.rb`,
  `lib/coding_adventures/matrix_rust_ruby.rb`, minitest suite.
- PRs #4–#8: `ml_framework_core` for Ruby — idiomatic Tensor +
  autograd that calls into this gem for the hot loop.
