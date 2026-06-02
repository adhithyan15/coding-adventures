# frozen_string_literal: true

# matrix_rust_ruby_test.rb — minitest suite for the matrix_rust_ruby gem.
# =========================================================================
#
# These tests dlopen the matrix_rust_ruby_native shared library (built by
# `rake compile`) and exercise the Ruby-visible method MatrixRustRuby.run_graph_on_cpu.
#
# What we're covering at this layer:
#
#   * Happy path:   identity graph round-trip (input bytes == output bytes)
#   * Wire format:  output envelope is a JSON string with `"outputs"` key
#   * Error path:   malformed JSON envelope → Ruby RuntimeError
#   * Error path:   missing `graph` field → Ruby RuntimeError
#   * Type check:   non-String argument → Ruby RuntimeError
#
# What we DON'T cover here (covered by c-bridge tests at the Rust layer):
#
#   * All the matrix-ir-json op shapes (matmul, add, relu, ...)
#   * Tensor shape validation, dtype handling
#   * CpuExecutor internals
#
# Layering principle: each language binding tests "did the FFI work?", not
# "did the Rust math come out right?"

require "minitest/autorun"
require "json"
require "coding_adventures/matrix_rust_ruby"

class MatrixRustRubyTest < Minitest::Test
  # The smallest possible envelope that survives a full round-trip through
  # parse → decode → execute → encode.  Mirrors c-bridge's make_empty_envelope.
  #
  # Graph:    one f32 tensor of shape [2], declared as both input and output.
  # Inputs:   "0000803f0000004f"
  #             = bytes 00 00 80 3f  (little-endian f32 = 1.0)
  #             + bytes 00 00 00 4f  (little-endian f32 = 2.1474836e9)
  #
  # We pick 1.0 (clean bit pattern) and 2^31 (large but exactly representable)
  # so any silent endianness or precision corruption shows up obviously in
  # the hex string comparison.
  IDENTITY_ENVELOPE = <<~JSON
    {
      "graph": {
        "matrix_ir_version": 1,
        "tensors": [{"id": 0, "dtype": "f32", "shape": [2]}],
        "inputs": [0],
        "outputs": [0],
        "ops": [],
        "constants": []
      },
      "inputs": ["0000803f0000004f"]
    }
  JSON

  def test_smoke_identity_graph_round_trips_bytes_unchanged
    out_json = MatrixRustRuby.run_graph_on_cpu(IDENTITY_ENVELOPE)

    parsed = JSON.parse(out_json)
    assert parsed.key?("outputs"), "output envelope must contain 'outputs' key, got: #{parsed.inspect}"
    assert_equal 1, parsed["outputs"].length, "identity graph has exactly 1 output"

    # The output hex must equal the input hex byte-for-byte.  Any bit-flip
    # — endianness, float→int coercion, allocation reuse — would manifest as
    # a mismatch here.
    assert_equal "0000803f0000004f", parsed["outputs"][0]
  end

  def test_output_envelope_is_valid_json
    out_json = MatrixRustRuby.run_graph_on_cpu(IDENTITY_ENVELOPE)
    # Should not raise.
    assert_kind_of Hash, JSON.parse(out_json)
  end

  def test_malformed_json_raises_runtime_error
    assert_raises(RuntimeError) do
      MatrixRustRuby.run_graph_on_cpu("not valid json at all")
    end
  end

  def test_missing_graph_field_raises_runtime_error
    bad = '{"inputs": []}'
    error = assert_raises(RuntimeError) do
      MatrixRustRuby.run_graph_on_cpu(bad)
    end
    # c-bridge's error string mentions the missing field name — we assert on
    # "graph" appearing somewhere in the message so this stays useful if the
    # exact phrasing changes upstream.
    assert_match(/graph/i, error.message)
  end

  def test_missing_inputs_field_raises_runtime_error
    bad = '{"graph": {"matrix_ir_version": 1, "tensors": [], "inputs": [], "outputs": [], "ops": [], "constants": []}}'
    error = assert_raises(RuntimeError) do
      MatrixRustRuby.run_graph_on_cpu(bad)
    end
    assert_match(/inputs/i, error.message)
  end

  def test_invalid_hex_in_inputs_raises_runtime_error
    bad = <<~JSON
      {
        "graph": {
          "matrix_ir_version": 1,
          "tensors": [{"id": 0, "dtype": "f32", "shape": [2]}],
          "inputs": [0],
          "outputs": [0],
          "ops": [],
          "constants": []
        },
        "inputs": ["zzzz"]
      }
    JSON
    error = assert_raises(RuntimeError) do
      MatrixRustRuby.run_graph_on_cpu(bad)
    end
    assert_match(/hex/i, error.message)
  end

  def test_non_string_argument_raises_type_error
    # Ruby's rb_string_value_cstr (called inside ruby-bridge's str_from_rb)
    # raises TypeError when handed a non-String — that's the idiomatic Ruby
    # signal for "wrong type" and beats us to the punch before our code
    # path would emit a RuntimeError.  Either error class signals "wrong
    # type" loudly enough for callers, so we assert on the common ancestor
    # StandardError to stay robust if the underlying behavior shifts (e.g.
    # str_from_rb later returns nil instead of letting TypeError through).
    error = assert_raises(StandardError) do
      MatrixRustRuby.run_graph_on_cpu(nil)
    end
    assert_match(/string|nil|nilclass/i, error.message)
  end

  def test_integer_argument_raises_type_error
    error = assert_raises(StandardError) do
      MatrixRustRuby.run_graph_on_cpu(42)
    end
    assert_match(/string|integer/i, error.message)
  end

  def test_namespaced_alias_delegates_to_top_level
    # CodingAdventures::MatrixRustRuby.run_graph_on_cpu should produce the
    # same result as the top-level form — they're the same underlying impl.
    direct  = MatrixRustRuby.run_graph_on_cpu(IDENTITY_ENVELOPE)
    aliased = CodingAdventures::MatrixRustRuby.run_graph_on_cpu(IDENTITY_ENVELOPE)
    assert_equal direct, aliased
  end

  def test_version_constant_is_defined
    assert_kind_of String, CodingAdventures::MatrixRustRuby::VERSION
    assert_match(/\A\d+\.\d+\.\d+\z/, CodingAdventures::MatrixRustRuby::VERSION)
  end
end
