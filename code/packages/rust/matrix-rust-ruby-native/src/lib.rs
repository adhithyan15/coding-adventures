//! matrix-rust-ruby-native — Ruby ↔ matrix-cpu native extension
//! ============================================================
//!
//! ## The one-line story
//!
//! This crate exposes exactly one Ruby method:
//!
//! ```ruby
//! MatrixRustRuby.run_graph_on_cpu(envelope_json_str) -> envelope_json_str
//! ```
//!
//! It takes a matrix-ir-json envelope (a JSON string describing a graph plus
//! its hex-encoded input tensors), runs it on the Rust `matrix-cpu` engine,
//! and returns a JSON envelope with the hex-encoded outputs.  On error
//! (malformed envelope, missing fields, hex decoding failure, …) it raises
//! a Ruby `RuntimeError`.
//!
//! ## Why is the implementation so small?
//!
//! All the actual work — JSON parsing, graph construction, planning,
//! executing on `matrix-cpu`, re-encoding outputs — lives in the workspace
//! crate [`c-bridge`].  We just call its `run_graph_on_cpu_via_json_envelope`
//! function and translate its `Result<String, String>` into either a Ruby
//! string or a Ruby exception.
//!
//! Putting the heavy lifting in `c-bridge` means three things stay in lockstep
//! across every language binding:
//!
//!   1. **Envelope shape** — Ruby and Python (and tomorrow's Lua / Go /
//!      Swift bindings) all send and receive the *exact* same JSON.
//!   2. **Error semantics** — every binding turns "malformed envelope" into
//!      a runtime-level error in the host language.
//!   3. **Performance characteristics** — the CPU executor is constructed
//!      identically per call, so benchmarks across languages compare apples
//!      to apples.
//!
//! ## What about safety?
//!
//! The Ruby C API is, by Rust standards, deeply unsafe: it traffics in raw
//! `VALUE` integers, longjmps on exceptions, and has fiddly rules about
//! holding the GVL.  We minimize exposure here by:
//!
//!   * **Doing all real work in pure Rust** before touching Ruby.  The
//!     envelope-execution call returns a plain `Result<String, String>`;
//!     only the conversion of that result into a Ruby value happens via FFI.
//!   * **Letting `ruby-bridge` own the unsafe.**  Every Ruby C API call here
//!     goes through a safe-ish wrapper in `ruby-bridge` (`str_to_rb`,
//!     `str_from_rb`, `raise_error`, `define_module`, ...).  This crate
//!     contains zero `unsafe` blocks.
//!   * **Catching panics at the FFI boundary.**  `c-bridge` already wraps
//!     its core in `std::panic::catch_unwind` and surfaces panics as
//!     `Err(String)`, so we never let a panic cross into Ruby (where it
//!     would be UB).
//!
//! ## Architecture: how this fits the multi-language stack
//!
//! ```text
//!   ┌─────────────────────────────────────────────────────────────┐
//!   │  matrix_rust_ruby (Ruby gem)            ← PR #3            │
//!   │    ↓ require_relative                                       │
//!   │  matrix_rust_ruby_native.{so,bundle}    ← THIS CRATE        │
//!   │    ↓ Rust function call                                     │
//!   │  c-bridge::run_graph_on_cpu_via_json_envelope               │
//!   │    ↓ pure Rust                                              │
//!   │  matrix-ir-json → matrix-ir → matrix-runtime → matrix-cpu   │
//!   └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! On top of this gem will sit `ml_framework_core` for Ruby — an idiomatic
//! Tensor + autograd library (PRs #4–#8).
//!
//! ## Entry point
//!
//! Ruby calls `Init_matrix_rust_ruby_native` when it dlopen()s the .so;
//! that's where we register the module and method.

use std::os::raw::c_void;

use ruby_bridge::VALUE;

// =============================================================================
// The singleton method:  MatrixRustRuby.run_graph_on_cpu(envelope_str)
// =============================================================================
//
// Ruby's calling convention for an `argc=1` C function defined via
// `rb_define_singleton_method` is:
//
//     extern "C" fn(self_value, arg1) -> VALUE
//
// `self_value` here is the module object itself (MatrixRustRuby).  We don't
// need it — singleton methods on modules act like module functions.
extern "C" fn run_graph_on_cpu(_self_val: VALUE, envelope_val: VALUE) -> VALUE {
    // Step 1.  Decode the Ruby string argument into a Rust &str.
    //
    // `str_from_rb` returns None if the argument is not a String (e.g. the
    // caller passed nil, an Integer, ...).  Treat that as a TypeError-like
    // condition by raising RuntimeError with an explanatory message — the
    // Ruby-side wrapper in the matrix_rust_ruby gem can choose to upgrade
    // this to a TypeError if/when desired, but RuntimeError keeps things
    // simple at this layer.
    let envelope = match ruby_bridge::str_from_rb(envelope_val) {
        Some(s) => s,
        None => ruby_bridge::raise_runtime_error(
            "MatrixRustRuby.run_graph_on_cpu: envelope must be a String",
        ),
    };

    // Step 2.  Run the graph.  All real work happens here — JSON parse,
    // graph construction, planning, CPU execution, output re-encoding.
    //
    // c-bridge wraps its core in catch_unwind, so a panic in the executor
    // becomes Err(String) rather than crossing the FFI boundary.
    // Crate name in Cargo.toml is `c-bridge` (package) → `matrix_c_bridge`
    // (lib).  The `[lib] name = "matrix_c_bridge"` override is what we
    // import here; it matches the .so/.dylib basename produced by
    // `cargo build -p c-bridge`.
    let result = matrix_c_bridge::run_graph_on_cpu_via_json_envelope(&envelope);

    // Step 3.  Translate Result<String, String> into Ruby.
    match result {
        Ok(output_envelope) => ruby_bridge::str_to_rb(&output_envelope),
        Err(msg) => ruby_bridge::raise_runtime_error(&format!(
            "MatrixRustRuby.run_graph_on_cpu failed: {msg}"
        )),
    }
}

// =============================================================================
// Init_matrix_rust_ruby_native — Ruby's dlopen() entry point
// =============================================================================
//
// Ruby looks for a function named `Init_<basename>` in every .so it loads as
// a native extension.  Our gem will require_relative the `.so` whose basename
// is `matrix_rust_ruby_native`, so this function must be named exactly
// `Init_matrix_rust_ruby_native`.
//
// What we do here:
//
//   1. Define a top-level Ruby module `MatrixRustRuby`.
//   2. Attach `run_graph_on_cpu(envelope_str)` as a singleton method on it.
//
// We deliberately put the module at the top level (not under
// `CodingAdventures::MatrixRustRuby`) so that the gem-side Ruby code can
// write `MatrixRustRuby.run_graph_on_cpu(...)` directly — short,
// pronounceable, and matches the README examples in c-bridge.
#[no_mangle]
pub extern "C" fn Init_matrix_rust_ruby_native() {
    let module = ruby_bridge::define_module("MatrixRustRuby");

    // argc=1 → one Ruby-side argument.  Cast through *const c_void because
    // that's what define_singleton_method_raw takes.
    ruby_bridge::define_singleton_method_raw(
        module,
        "run_graph_on_cpu",
        run_graph_on_cpu as *const c_void,
        1,
    );
}
