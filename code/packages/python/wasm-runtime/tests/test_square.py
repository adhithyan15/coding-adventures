"""End-to-end test: square function.

Hand-assembles a WASM module that exports a ``square(x: i32) -> i32``
function, then runs it through the full pipeline: parse -> validate ->
instantiate -> call.

The WASM bytecode implements:
    (module
      (type (func (param i32) (result i32)))
      (func (type 0) (param i32) (result i32)
        local.get 0
        local.get 0
        i32.mul)
      (export "square" (func 0)))
"""

from wasm_runtime import WasmRuntime


def _leb128(n: int) -> bytes:
    """Encode an unsigned integer as LEB128."""
    result = bytearray()
    while True:
        byte = n & 0x7F
        n >>= 7
        if n > 0:
            byte |= 0x80
        result.append(byte)
        if n == 0:
            break
    return bytes(result)


def _build_section(section_id: int, payload: bytes) -> bytes:
    """Build a WASM section: id + LEB128(size) + payload."""
    return bytes([section_id]) + _leb128(len(payload)) + payload


def _build_square_wasm() -> bytes:
    """Hand-assemble the square.wasm binary.

    Layout:
      magic + version
      type section:   1 type: (i32) -> (i32)
      function section: 1 function using type 0
      export section: "square" -> func 0
      code section:   1 body: local.get 0; local.get 0; i32.mul; end
    """
    header = bytes([0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00])

    # Type section: 1 type entry (i32) -> (i32)
    type_payload = _leb128(1) + bytes([0x60, 0x01, 0x7F, 0x01, 0x7F])
    type_section = _build_section(1, type_payload)

    # Function section: 1 function using type index 0
    func_payload = _leb128(1) + _leb128(0)
    func_section = _build_section(3, func_payload)

    # Export section: "square" -> func 0
    export_name = b"square"
    export_payload = (_leb128(1) + _leb128(len(export_name))
                      + export_name + bytes([0x00]) + _leb128(0))
    export_section = _build_section(7, export_payload)

    # Code section: 1 body
    body_code = bytes([
        0x20, 0x00,  # local.get 0
        0x20, 0x00,  # local.get 0
        0x6C,        # i32.mul
        0x0B,        # end
    ])
    body = _leb128(0) + body_code  # 0 local groups + code
    code_payload = _leb128(1) + _leb128(len(body)) + body
    code_section = _build_section(10, code_payload)

    return header + type_section + func_section + export_section + code_section


class TestSquareEndToEnd:
    """End-to-end tests for the square function."""

    def test_square_5(self) -> None:
        """square(5) = 25"""
        wasm_bytes = _build_square_wasm()
        runtime = WasmRuntime()
        result = runtime.load_and_run(wasm_bytes, "square", [5])
        assert result == [25]

    def test_square_0(self) -> None:
        """square(0) = 0"""
        wasm_bytes = _build_square_wasm()
        runtime = WasmRuntime()
        result = runtime.load_and_run(wasm_bytes, "square", [0])
        assert result == [0]

    def test_square_negative(self) -> None:
        """square(-3) = 9"""
        wasm_bytes = _build_square_wasm()
        runtime = WasmRuntime()
        result = runtime.load_and_run(wasm_bytes, "square", [-3])
        assert result == [9]

    def test_square_max_int(self) -> None:
        """square(2147483647) wraps to 1 in i32 arithmetic.

        2147483647^2 = 4611686014132420609
        In i32: (2147483647 * 2147483647) mod 2^32 = 1
        """
        wasm_bytes = _build_square_wasm()
        runtime = WasmRuntime()
        result = runtime.load_and_run(wasm_bytes, "square", [2147483647])
        assert result == [1]
