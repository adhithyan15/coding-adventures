# matrix_rust_ruby — drive the Rust matrix-cpu engine from Ruby

The Ruby gem in the multi-language ML framework stack.  Gives you one
method:

```ruby
require "coding_adventures/matrix_rust_ruby"

envelope_json = build_envelope(...)   # see "The envelope format" below
output_json   = MatrixRustRuby.run_graph_on_cpu(envelope_json)
```

The envelope describes a matrix-ir graph plus its hex-encoded input tensors;
the output is the same shape with hex-encoded outputs.  All real work
happens in Rust on the `matrix-cpu` execution engine.

## Where this fits

```
┌──────────────────────────────────────────────────────────────────┐
│  ml_framework_core (Ruby gem)             ← PRs #4–#8 (planned) │
│    idiomatic Tensor + autograd, Ruby ergonomics                  │
│    ↓                                                              │
│  matrix_rust_ruby (THIS GEM, PR #3)                              │
│    lib/coding_adventures/matrix_rust_ruby.rb                     │
│    require "matrix_rust_ruby_native"                             │
│    ↓                                                              │
│  matrix_rust_ruby_native.{so,bundle,dll}   ← PR #2 (merged)      │
│    Rust cdylib; defines MatrixRustRuby.run_graph_on_cpu          │
│    ↓                                                              │
│  c-bridge::run_graph_on_cpu_via_json_envelope  ← PR #1 (merged)  │
│    ↓                                                              │
│  matrix-ir-json → matrix-ir → matrix-runtime → matrix-cpu        │
└──────────────────────────────────────────────────────────────────┘
```

This gem is the **low-level binding** tier — thin, mostly just plumbing.
The next layer up (`ml_framework_core` for Ruby) will give you `Tensor`,
autograd, and the usual `tensor.relu.matmul(weight).sum.backward` ergonomics.

## Installing

### From source (workspace)

```bash
cd code/packages/ruby/matrix_rust_ruby
bundle install
bundle exec rake compile      # runs cargo build -p matrix-rust-ruby-native
bundle exec rake test         # runs the minitest suite
```

`rake compile` invokes `cargo build -p matrix-rust-ruby-native --release`
against the workspace's `Cargo.toml`, then copies the resulting
`libmatrix_rust_ruby_native.{dylib,so,dll}` into
`lib/coding_adventures/matrix_rust_ruby/` under the basename Ruby will
`require`.

### From RubyGems (planned — not yet published)

```bash
gem install coding_adventures_matrix_rust_ruby
```

`gem install` runs `ext/matrix_rust_ruby_native/extconf.rb`, which writes
a tiny `Makefile` that shells out to `cargo`.  You'll need a working Rust
toolchain on the install machine until we ship pre-compiled platform gems.

## The envelope format

Identical to what `c-bridge` and `matrix-rust-python` accept.  See
`code/packages/rust/matrix-ir-json/` for the schema and
`code/packages/python/ml-framework-core/src/ml_framework_core/_rust_backend.py`
for ~30 worked examples of envelope construction (one per supported op).

Minimal example (the identity graph the test suite uses):

```ruby
envelope = {
  "graph" => {
    "matrix_ir_version" => 1,
    "tensors"           => [{"id" => 0, "dtype" => "f32", "shape" => [2]}],
    "inputs"            => [0],
    "outputs"           => [0],
    "ops"               => [],
    "constants"         => []
  },
  "inputs" => ["0000803f0000004f"]   # 1.0, 2^31 as little-endian f32 hex
}.to_json

output = MatrixRustRuby.run_graph_on_cpu(envelope)
JSON.parse(output)
# => {"outputs" => ["0000803f0000004f"]}    # bytes preserved through identity
```

## Errors

Every error condition — malformed JSON, missing fields, invalid hex,
executor failure, non-String argument — raises a Ruby `RuntimeError` with a
descriptive message.  No panics ever cross the FFI boundary (the underlying
Rust core wraps execution in `std::panic::catch_unwind`).

## Why JSON?

Every language has JSON.  Binary formats (FlatBuffers, Cap'n Proto) would
be marginally faster but force every language binding to drag in a schema
compiler.  For typical matrix-cpu workloads (matmul, reduction, activation
on ≥100k cells), JSON encode/decode is a fraction of one percent of total
call cost.  See `scripts/benchmark_mx10.py` in `ml-framework-core` for
measurements.

## What's next

PRs #4–#8 build the `ml_framework_core` Ruby layer on top:

* **PR #4**: pure-Ruby `Tensor` class — factories, shape ops, operator overloads
* **PR #5**: autograd engine — `Function` base class, topological sort, `backward`
* **PR #6**: forward op dispatch — 15+ `Function` subclasses; envelopes
* **PR #7**: backward dispatch + end-to-end MLP training test
* **PR #8**: benchmark script + RubyGems publishing polish
