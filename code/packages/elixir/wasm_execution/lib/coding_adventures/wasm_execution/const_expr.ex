defmodule CodingAdventures.WasmExecution.ConstExpr do
  @moduledoc """
  Evaluate WASM constant expressions.

  Constant expressions are tiny programs used to initialize globals,
  data segment offsets, and element segment offsets. Only a handful of
  opcodes are allowed: i32.const, i64.const, f32.const, f64.const,
  global.get, and end.
  """

  alias CodingAdventures.WasmLeb128
  alias CodingAdventures.WasmExecution.TrapError
  alias CodingAdventures.WasmExecution.Values

  @doc """
  Evaluate a constant expression and return its result as a WasmValue.

  The expression is a binary of WASM bytecodes ending with 0x0B (end).
  `globals` is the list of already-initialized global values (for global.get).
  """
  @spec evaluate(binary(), [Values.wasm_value()]) :: Values.wasm_value()
  def evaluate(expr, globals \\ []) when is_binary(expr) do
    do_evaluate(expr, 0, nil, globals)
  end

  defp do_evaluate(expr, pos, result, globals) when pos < byte_size(expr) do
    <<_::binary-size(pos), opcode::8, _::binary>> = expr
    pos = pos + 1

    case opcode do
      # i32.const
      0x41 ->
        {:ok, {value, consumed}} = WasmLeb128.decode_signed(expr, pos)
        do_evaluate(expr, pos + consumed, Values.i32(value), globals)

      # i64.const
      0x42 ->
        {value, consumed} = decode_signed_64(expr, pos)
        do_evaluate(expr, pos + consumed, Values.i64(value), globals)

      # f32.const
      0x43 ->
        <<_::binary-size(pos), float_val::little-float-size(32), _::binary>> = expr
        do_evaluate(expr, pos + 4, Values.f32(float_val), globals)

      # f64.const
      0x44 ->
        <<_::binary-size(pos), float_val::little-float-size(64), _::binary>> = expr
        do_evaluate(expr, pos + 8, Values.f64(float_val), globals)

      # global.get
      0x23 ->
        {:ok, {global_index, consumed}} = WasmLeb128.decode_unsigned(expr, pos)

        if global_index >= length(globals) do
          raise TrapError,
                "global.get: index #{global_index} out of bounds (#{length(globals)} globals)"
        end

        global_value = Enum.at(globals, global_index)
        do_evaluate(expr, pos + consumed, global_value, globals)

      # end
      0x0B ->
        if result == nil do
          raise TrapError, "Constant expression produced no value"
        end

        result

      other ->
        raise TrapError,
              "Illegal opcode 0x#{Integer.to_string(other, 16) |> String.pad_leading(2, "0")} in constant expression"
    end
  end

  defp do_evaluate(_expr, _pos, _result, _globals) do
    raise TrapError, "Constant expression missing end opcode (0x0B)"
  end

  # Signed LEB128 64-bit decoder
  defp decode_signed_64(data, offset) do
    decode_s64_loop(data, offset, 0, 0, 0)
  end

  defp decode_s64_loop(data, offset, result, shift, bytes) do
    <<_::binary-size(offset), current_byte::8, _::binary>> = data
    new_bytes = bytes + 1
    payload = Bitwise.band(current_byte, 0x7F)
    new_result = Bitwise.bor(result, Bitwise.bsl(payload, shift))
    new_shift = shift + 7

    if Bitwise.band(current_byte, 0x80) == 0 do
      final =
        if new_shift < 64 and Bitwise.band(current_byte, 0x40) != 0 do
          Bitwise.bor(new_result, Bitwise.bsl(-1, new_shift))
        else
          new_result
        end

      masked = Bitwise.band(final, 0xFFFFFFFFFFFFFFFF)

      signed =
        if Bitwise.band(masked, 0x8000000000000000) != 0 do
          masked - 0x10000000000000000
        else
          masked
        end

      {signed, new_bytes}
    else
      decode_s64_loop(data, offset + 1, new_result, new_shift, new_bytes)
    end
  end
end
