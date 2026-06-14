# Changelog

## Unreleased

### Added — v0.1.0: Ruby gem wrapping the Rust matrix-cpu engine

First release.  Ships the user-facing Ruby gem on top of the
matrix_rust_ruby_native workspace cdylib (PR #2) and c-bridge (PR #1).

#### Public API

```ruby
require "coding_adventures/matrix_rust_ruby"

MatrixRustRuby.run_graph_on_cpu(envelope_json_str) -> envelope_json_str
# or, namespaced:
CodingAdventures::MatrixRustRuby.run_graph_on_cpu(envelope_json_str) -> envelope_json_str
```

Both forms are equivalent; the namespaced form delegates to the top-level
one (which is defined by the native ext at load time).

#### File layout

```
matrix_rust_ruby/
├── coding_adventures_matrix_rust_ruby.gemspec
├── Gemfile
├── Rakefile                                     # compile + test
├── ext/matrix_rust_ruby_native/
│   ├── extconf.rb                               # gem install hook
│   └── build_config.rb                          # shared cargo config
├── lib/coding_adventures/
│   ├── matrix_rust_ruby.rb                      # entry point
│   └── matrix_rust_ruby/
│       ├── version.rb
│       └── native_loader.rb                     # finds + requires the .so
└── test/matrix_rust_ruby_test.rb                # 10 minitests
```

#### Why the Rust crate lives in the workspace, not in `ext/`

The conduit gem nests its Rust crate at `ext/conduit_native/`.  We
chose a different layout: the Rust crate is `code/packages/rust/matrix-rust-ruby-native/`
(merged in PR #2) and the gem builds it via `cargo build -p matrix-rust-ruby-native`.

Trade-off:

- **Pro**: workspace `cargo build`/`cargo check`/`cargo test` covers the
  Rust side; the workspace's shared `target/` dir avoids rebuilding the
  matrix-* dependency chain from scratch on every `rake compile`.
- **Pro**: matches the architecture pattern in the multi-language plan —
  every future language binding (Lua, Go, Swift) gets its own
  `<lang>-bridge` workspace crate, with the language-specific package
  living under `code/packages/<lang>/`.
- **Con**: extconf.rb and the Rakefile need to walk up from the gem dir
  to the workspace root.  We do that via `File.expand_path(...)` in
  `build_config.rb`; clearly commented.

#### Test coverage (10 minitests)

- `test_smoke_identity_graph_round_trips_bytes_unchanged` — happy path
- `test_output_envelope_is_valid_json` — wire format sanity
- `test_malformed_json_raises_runtime_error`
- `test_missing_graph_field_raises_runtime_error`
- `test_missing_inputs_field_raises_runtime_error`
- `test_invalid_hex_in_inputs_raises_runtime_error`
- `test_non_string_argument_raises_runtime_error` (nil)
- `test_integer_argument_raises_runtime_error`
- `test_namespaced_alias_delegates_to_top_level`
- `test_version_constant_is_defined`

This layer tests "did the FFI work?"  Per-op math correctness lives in
c-bridge's Rust-side tests, not duplicated here.

#### Safety

- All error conditions surface as Ruby `RuntimeError` with descriptive
  messages.
- The underlying Rust executor is wrapped in `std::panic::catch_unwind`
  (by c-bridge); panics become `Err(String)` → Ruby `RuntimeError`,
  never undefined behaviour.
- The native loader emits a helpful remediation message on `LoadError`
  (most common failure: user forgot to run `rake compile`).

#### Build dependencies

- Ruby >= 2.6
- `minitest ~> 5.0` (development)
- `rake ~> 13.0` (development)
- Rust toolchain (cargo on PATH) at gem-install time, until we publish
  pre-compiled platform gems.

#### What's next

- PR #4: `ml_framework_core` Ruby — pure-Ruby `Tensor` class
- PR #5: autograd engine
- PR #6: forward op dispatch
- PR #7: backward dispatch + end-to-end MLP test
- PR #8: benchmark + RubyGems polish
