# frozen_string_literal: true

# coding_adventures/matrix_rust_ruby.rb — Ruby façade for the Rust matrix-cpu engine
# ====================================================================================
#
# What you get after `require "coding_adventures/matrix_rust_ruby"`:
#
#   MatrixRustRuby.run_graph_on_cpu(envelope_json_str) -> envelope_json_str
#
# Pass in a matrix-ir-json envelope (a JSON string containing a graph
# definition plus hex-encoded input tensors), receive a JSON envelope with
# hex-encoded outputs.  Any error (malformed JSON, missing fields, bad hex,
# executor failure) raises a Ruby `RuntimeError` with a descriptive message.
#
# `MatrixRustRuby` itself is defined by the native extension when it loads
# (see ext/matrix_rust_ruby_native/src/lib.rs — actually in the workspace at
# code/packages/rust/matrix-rust-ruby-native/src/lib.rs).  This file just
# does three things:
#
#   1. Loads the version constant (so users can introspect it).
#   2. Triggers the native loader, which `require`s the .{so,bundle,dll}
#      that adds the `MatrixRustRuby.run_graph_on_cpu` method to the
#      top-level module.
#   3. Provides a `CodingAdventures::MatrixRustRuby` alias so callers who
#      prefer the namespaced form get it (the native ext defines the
#      top-level module, but we want both spellings to work).

require_relative "matrix_rust_ruby/version"
require_relative "matrix_rust_ruby/native_loader"

# Make `CodingAdventures::MatrixRustRuby.run_graph_on_cpu(...)` work too.
#
# The native ext defines the top-level `MatrixRustRuby` (short and
# pronounceable for examples).  But the rest of this gem family lives under
# the `CodingAdventures::` namespace, and we already have
# `CodingAdventures::MatrixRustRuby::VERSION` defined.  So define a
# singleton-method delegator on the namespaced module that just forwards to
# the top-level module.
module CodingAdventures
  module MatrixRustRuby
    # NOTE: `::MatrixRustRuby` (the top level) is the module the native ext
    # registered.  `MatrixRustRuby` inside the `CodingAdventures` namespace
    # is the one defined in version.rb above.  Two distinct modules; we
    # bridge them here.
    def self.run_graph_on_cpu(envelope_json_str)
      ::MatrixRustRuby.run_graph_on_cpu(envelope_json_str)
    end
  end
end
