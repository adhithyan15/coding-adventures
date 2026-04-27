defmodule CodingAdventures.WasmExecution.Decoder do
  @moduledoc """
  WASM bytecode decoder -- bridges variable-length WASM bytecodes to
  fixed-format GenericVM instructions.

  WASM bytecodes are variable-length (opcode is 1 byte, immediates can
  be 1-10+ bytes via LEB128). The GenericVM expects fixed-format
  Instruction structs. This module decodes all instructions in a
  function body and builds the control flow map.
  """

  alias CodingAdventures.WasmLeb128
  alias CodingAdventures.WasmOpcodes
  alias CodingAdventures.VirtualMachine.Types.Instruction

  @type decoded_instruction :: %{
          opcode: non_neg_integer(),
          operand: any(),
          offset: non_neg_integer(),
          byte_size: non_neg_integer()
        }

  @type control_target :: %{end_pc: non_neg_integer(), else_pc: non_neg_integer() | nil}

  @doc """
  Decode all instructions in a function body's bytecodes.

  Returns a list of decoded instruction maps.
  """
  @spec decode_function_body(binary()) :: [decoded_instruction()]
  def decode_function_body(code) when is_binary(code) do
    decode_loop(code, 0, [])
  end

  defp decode_loop(code, offset, acc) when offset >= byte_size(code) do
    Enum.reverse(acc)
  end

  defp decode_loop(code, offset, acc) do
    start_offset = offset
    <<_::binary-size(offset), opcode_byte::8, _::binary>> = code
    offset = offset + 1

    info =
      case WasmOpcodes.get_opcode(opcode_byte) do
        {:ok, opcode_map} -> opcode_map
        {:error, _} -> nil
      end

    immediates = if info, do: Map.get(info, :immediates, []), else: []

    {operand, new_offset} = decode_immediates(code, offset, immediates)

    instr = %{
      opcode: opcode_byte,
      operand: operand,
      offset: start_offset,
      byte_size: new_offset - start_offset
    }

    decode_loop(code, new_offset, [instr | acc])
  end

  # ===========================================================================
  # Immediate Decoding
  # ===========================================================================

  defp decode_immediates(_code, offset, []), do: {nil, offset}

  defp decode_immediates(code, offset, [single]) do
    decode_single_immediate(code, offset, single)
  end

  defp decode_immediates(code, offset, imm_list) when is_list(imm_list) do
    {values, final_offset} =
      Enum.reduce(imm_list, {%{}, offset}, fn imm_type, {acc_map, pos} ->
        {value, new_pos} = decode_single_immediate(code, pos, imm_type)
        {Map.put(acc_map, imm_type, value), new_pos}
      end)

    {values, final_offset}
  end

  defp decode_single_immediate(code, offset, type) do
    case type do
      "i32" ->
        {value, consumed} = unwrap_leb!(WasmLeb128.decode_signed(code, offset))
        {value, offset + consumed}

      t when t in ~w(labelidx funcidx typeidx localidx globalidx tableidx memidx) ->
        {value, consumed} = unwrap_leb!(WasmLeb128.decode_unsigned(code, offset))
        {value, offset + consumed}

      "i64" ->
        {value, consumed} = decode_signed_64(code, offset)
        {value, offset + consumed}

      "f32" ->
        <<_::binary-size(offset), float_val::little-float-size(32), _::binary>> = code
        {float_val, offset + 4}

      "f64" ->
        <<_::binary-size(offset), float_val::little-float-size(64), _::binary>> = code
        {float_val, offset + 8}

      "blocktype" ->
        <<_::binary-size(offset), type_byte::8, _::binary>> = code

        cond do
          type_byte == 0x40 ->
            {0x40, offset + 1}

          type_byte in [0x7F, 0x7E, 0x7D, 0x7C] ->
            {type_byte, offset + 1}

          true ->
            {value, consumed} = unwrap_leb!(WasmLeb128.decode_signed(code, offset))
            {value, offset + consumed}
        end

      "memarg" ->
        {align, align_size} = unwrap_leb!(WasmLeb128.decode_unsigned(code, offset))

        {mem_offset, offset_size} =
          unwrap_leb!(WasmLeb128.decode_unsigned(code, offset + align_size))

        {%{align: align, offset: mem_offset}, offset + align_size + offset_size}

      "vec_labelidx" ->
        {count, count_size} = unwrap_leb!(WasmLeb128.decode_unsigned(code, offset))
        pos = offset + count_size

        {labels, pos} =
          Enum.reduce(1..max(count, 0)//1, {[], pos}, fn _, {labels_acc, p} ->
            {label, label_size} = unwrap_leb!(WasmLeb128.decode_unsigned(code, p))
            {[label | labels_acc], p + label_size}
          end)

        {default_label, default_size} = unwrap_leb!(WasmLeb128.decode_unsigned(code, pos))

        {%{labels: Enum.reverse(labels), default_label: default_label}, pos + default_size}

      _ ->
        {nil, offset}
    end
  end

  # Unwrap {:ok, {value, consumed}} from LEB128 decode functions.
  defp unwrap_leb!({:ok, {value, consumed}}), do: {value, consumed}
  defp unwrap_leb!({:error, msg}), do: raise(RuntimeError, "LEB128 decode error: #{msg}")

  # ===========================================================================
  # 64-bit Signed LEB128
  # ===========================================================================

  defp decode_signed_64(data, offset) do
    decode_signed_64_loop(data, offset, 0, 0, 0)
  end

  defp decode_signed_64_loop(data, offset, result, shift, bytes_consumed) do
    <<_::binary-size(offset), current_byte::8, _::binary>> = data
    new_bytes = bytes_consumed + 1
    payload = Bitwise.band(current_byte, 0x7F)
    new_result = Bitwise.bor(result, Bitwise.bsl(payload, shift))
    new_shift = shift + 7

    if Bitwise.band(current_byte, 0x80) == 0 do
      # Sign extend if needed
      final =
        if new_shift < 64 and Bitwise.band(current_byte, 0x40) != 0 do
          Bitwise.bor(new_result, Bitwise.bsl(-1, new_shift))
        else
          new_result
        end

      # Clamp to signed 64-bit range
      masked = Bitwise.band(final, 0xFFFFFFFFFFFFFFFF)

      signed =
        if Bitwise.band(masked, 0x8000000000000000) != 0 do
          masked - 0x10000000000000000
        else
          masked
        end

      {signed, new_bytes}
    else
      decode_signed_64_loop(data, offset + 1, new_result, new_shift, new_bytes)
    end
  end

  # ===========================================================================
  # Control Flow Map
  # ===========================================================================

  @doc """
  Build a control flow map for decoded instructions.

  Maps each block/loop/if instruction index to its matching end
  (and else for if instructions). Built by a single O(n) scan.

  Returns `%{instruction_index => %{end_pc: end_index, else_pc: else_index | nil}}`.
  """
  @spec build_control_flow_map([decoded_instruction()]) :: %{
          non_neg_integer() => control_target()
        }
  def build_control_flow_map(instructions) do
    {result, _stack} =
      instructions
      |> Enum.with_index()
      |> Enum.reduce({%{}, []}, fn {instr, idx}, {map_acc, stack} ->
        case instr.opcode do
          op when op in [0x02, 0x03, 0x04] ->
            # block, loop, if -- push onto stack
            {map_acc, [{idx, op, nil} | stack]}

          0x05 ->
            # else -- record on the top stack entry
            case stack do
              [{opener_idx, opener_op, _else_pc} | rest] ->
                {map_acc, [{opener_idx, opener_op, idx} | rest]}

              _ ->
                {map_acc, stack}
            end

          0x0B ->
            # end -- pop stack and record mapping
            case stack do
              [{opener_idx, _opener_op, else_pc} | rest] ->
                new_map = Map.put(map_acc, opener_idx, %{end_pc: idx, else_pc: else_pc})
                {new_map, rest}

              [] ->
                # Function's trailing end
                {map_acc, stack}
            end

          _ ->
            {map_acc, stack}
        end
      end)

    result
  end

  @doc """
  Convert decoded instructions to GenericVM Instruction structs.
  """
  @spec to_vm_instructions([decoded_instruction()]) :: [Instruction.t()]
  def to_vm_instructions(decoded) do
    Enum.map(decoded, fn d ->
      %Instruction{opcode: d.opcode, operand: d.operand}
    end)
  end
end
