# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_wasm_runtime"

# ==========================================================================
# End-to-End Test: square(n) = n * n
# ==========================================================================
#
# This test proves the entire Ruby WASM stack works end-to-end:
#
#   Raw .wasm bytes -> Parse -> Validate -> Instantiate -> Execute -> Result
#
# The test hand-assembles a minimal WASM module that exports a `square`
# function: (i32) -> (i32) which computes n * n.
#
# In WAT (WebAssembly Text Format):
#
#   (module
#     (type (func (param i32) (result i32)))
#     (func (export "square") (type 0)
#       local.get 0
#       local.get 0
#       i32.mul)
#   )
# ==========================================================================

class TestWasmRuntime < Minitest::Test
  # Hand-assemble the square.wasm binary byte by byte.
  def build_square_wasm
    parts = []

    # ── Header ──────────────────────────────────────────────────────
    parts.push(0x00, 0x61, 0x73, 0x6D)  # Magic: "\0asm"
    parts.push(0x01, 0x00, 0x00, 0x00)  # Version: 1

    # ── Type Section (ID 1) ─────────────────────────────────────────
    type_payload = [
      0x01,       # 1 type entry
      0x60,       # function type marker
      0x01, 0x7F, # 1 param: i32
      0x01, 0x7F  # 1 result: i32
    ]
    parts.push(0x01)
    parts.concat(encode_unsigned(type_payload.length))
    parts.concat(type_payload)

    # ── Function Section (ID 3) ─────────────────────────────────────
    func_payload = [0x01, 0x00]
    parts.push(0x03)
    parts.concat(encode_unsigned(func_payload.length))
    parts.concat(func_payload)

    # ── Export Section (ID 7) ───────────────────────────────────────
    name_bytes = "square".bytes
    export_payload = [
      0x01,
      *encode_unsigned(name_bytes.length),
      *name_bytes,
      0x00,  # export kind: function
      0x00   # function index 0
    ]
    parts.push(0x07)
    parts.concat(encode_unsigned(export_payload.length))
    parts.concat(export_payload)

    # ── Code Section (ID 10) ────────────────────────────────────────
    body_code = [
      0x20, 0x00,   # local.get 0
      0x20, 0x00,   # local.get 0
      0x6C,         # i32.mul
      0x0B          # end
    ]
    body_payload = [0x00, *body_code]
    func_body = [*encode_unsigned(body_payload.length), *body_payload]
    code_payload = [0x01, *func_body]
    parts.push(0x0A)
    parts.concat(encode_unsigned(code_payload.length))
    parts.concat(code_payload)

    parts.pack("C*")
  end

  def encode_unsigned(value)
    CodingAdventures::WasmLeb128.encode_unsigned(value).bytes
  end

  # ── End-to-end Tests ──────────────────────────────────────────────

  def test_square_5_equals_25
    runtime = CodingAdventures::WasmRuntime::Runtime.new
    result = runtime.load_and_run(build_square_wasm, "square", [5])
    assert_equal [25], result
  end

  def test_square_0_equals_0
    runtime = CodingAdventures::WasmRuntime::Runtime.new
    result = runtime.load_and_run(build_square_wasm, "square", [0])
    assert_equal [0], result
  end

  def test_square_negative_3_equals_9
    runtime = CodingAdventures::WasmRuntime::Runtime.new
    result = runtime.load_and_run(build_square_wasm, "square", [-3])
    assert_equal [9], result
  end

  def test_step_by_step_load_validate_instantiate_call
    runtime = CodingAdventures::WasmRuntime::Runtime.new
    wasm_bytes = build_square_wasm

    wasm_module = runtime.load(wasm_bytes)
    assert_equal 1, wasm_module.types.length
    assert_equal 1, wasm_module.functions.length
    assert_equal 1, wasm_module.exports.length

    validated = runtime.validate(wasm_module)
    refute_nil validated

    instance = runtime.instantiate(wasm_module)
    assert instance.exports.key?("square")

    result = runtime.call(instance, "square", [7])
    assert_equal [49], result
  end

  def test_version_exists
    refute_nil CodingAdventures::WasmRuntime::VERSION
  end
end
