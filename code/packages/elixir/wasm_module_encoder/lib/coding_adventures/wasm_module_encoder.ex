defmodule CodingAdventures.WasmModuleEncoder.WasmEncodeError do
  defexception [:message]
end

defmodule CodingAdventures.WasmModuleEncoder do
  @moduledoc """
  Encode a `CodingAdventures.WasmTypes.WasmModule` into raw WebAssembly bytes.
  """

  alias CodingAdventures.WasmLeb128
  alias CodingAdventures.WasmModuleEncoder.WasmEncodeError
  alias CodingAdventures.WasmTypes

  alias CodingAdventures.WasmTypes.{
    CustomSection,
    DataSegment,
    Element,
    Export,
    FuncType,
    FunctionBody,
    Global,
    GlobalType,
    Import,
    Limits,
    MemoryType,
    TableType,
    WasmModule
  }

  @wasm_magic <<0x00, 0x61, 0x73, 0x6D>>
  @wasm_version <<0x01, 0x00, 0x00, 0x00>>

  @spec encode_module(WasmModule.t()) :: binary()
  def encode_module(%WasmModule{} = module) do
    sections =
      []
      |> maybe_custom_sections(module.customs)
      |> maybe_section(1, module.types, &encode_func_type/1)
      |> maybe_section(2, module.imports, &encode_import/1)
      |> maybe_section(3, module.functions, &u32/1)
      |> maybe_section(4, module.tables, &encode_table_type/1)
      |> maybe_section(5, module.memories, &encode_memory_type/1)
      |> maybe_section(6, module.globals, &encode_global/1)
      |> maybe_section(7, module.exports, &encode_export/1)
      |> maybe_start_section(module.start)
      |> maybe_section(9, module.elements, &encode_element/1)
      |> maybe_section(10, module.code, &encode_function_body/1)
      |> maybe_section(11, module.data, &encode_data_segment/1)

    IO.iodata_to_binary([@wasm_magic, @wasm_version | sections])
  end

  defp maybe_custom_sections(acc, customs) do
    Enum.reduce(customs, acc, fn custom, acc_sections ->
      acc_sections ++ [section(0, encode_custom(custom))]
    end)
  end

  defp maybe_section(acc, _id, [], _encoder), do: acc

  defp maybe_section(acc, id, values, encoder) do
    acc ++ [section(id, vector(values, encoder))]
  end

  defp maybe_start_section(acc, nil), do: acc
  defp maybe_start_section(acc, start_index), do: acc ++ [section(8, u32(start_index))]

  defp section(section_id, payload) do
    [<<section_id>>, u32(IO.iodata_length(payload)), payload]
  end

  defp u32(value), do: WasmLeb128.encode_unsigned(value)

  defp name(text) do
    data = :unicode.characters_to_binary(text, :utf8)
    [u32(byte_size(data)), data]
  end

  defp vector(values, encoder) do
    encoded = Enum.map(values, encoder)
    [u32(length(values)) | encoded]
  end

  defp encode_value_types(types) do
    [u32(length(types)), Enum.map(types, &encode_value_type/1)]
  end

  defp encode_func_type(%FuncType{params: params, results: results}) do
    [<<0x60>>, encode_value_types(params), encode_value_types(results)]
  end

  defp encode_limits(%Limits{min: min, max: nil}) do
    [<<0x00>>, u32(min)]
  end

  defp encode_limits(%Limits{min: min, max: max}) do
    [<<0x01>>, u32(min), u32(max)]
  end

  defp encode_memory_type(%MemoryType{limits: limits}), do: encode_limits(limits)

  defp encode_table_type(%TableType{element_type: element_type, limits: limits}) do
    [<<element_type>>, encode_limits(limits)]
  end

  defp encode_global_type(%GlobalType{value_type: value_type, mutable: mutable}) do
    [<<encode_value_type(value_type), if(mutable, do: 0x01, else: 0x00)>>]
  end

  defp encode_import(%Import{} = import) do
    payload =
      case {import.kind, import.type_info} do
        {:function, {:function, type_idx}} ->
          u32(type_idx)

        {:function, type_idx} when is_integer(type_idx) ->
          u32(type_idx)

        {:table, {:table, %TableType{} = table_type}} ->
          encode_table_type(table_type)

        {:table, %TableType{} = table_type} ->
          encode_table_type(table_type)

        {:memory, {:memory, %MemoryType{} = memory_type}} ->
          encode_memory_type(memory_type)

        {:memory, %MemoryType{} = memory_type} ->
          encode_memory_type(memory_type)

        {:global, {:global, %GlobalType{} = global_type}} ->
          encode_global_type(global_type)

        {:global, %GlobalType{} = global_type} ->
          encode_global_type(global_type)

        {:function, other} ->
          raise WasmEncodeError,
            message: "function imports require a type index, got: #{inspect(other)}"

        {:table, other} ->
          raise WasmEncodeError,
            message: "table imports require TableType metadata, got: #{inspect(other)}"

        {:memory, other} ->
          raise WasmEncodeError,
            message: "memory imports require MemoryType metadata, got: #{inspect(other)}"

        {:global, other} ->
          raise WasmEncodeError,
            message: "global imports require GlobalType metadata, got: #{inspect(other)}"

        {kind, _other} ->
          raise WasmEncodeError, message: "unsupported import kind: #{inspect(kind)}"
      end

    [
      name(import.module_name),
      name(import.name),
      <<encode_external_kind(import.kind)>>,
      payload
    ]
  end

  defp encode_export(%Export{name: name, kind: kind, index: index}) do
    [name(name), <<encode_external_kind(kind)>>, u32(index)]
  end

  defp encode_global(%Global{global_type: global_type, init_expr: init_expr}) do
    [encode_global_type(global_type), init_expr]
  end

  defp encode_element(%Element{
         table_index: table_index,
         offset_expr: offset_expr,
         function_indices: indices
       }) do
    [u32(table_index), offset_expr, u32(length(indices)), Enum.map(indices, &u32/1)]
  end

  defp encode_data_segment(%DataSegment{
         memory_index: memory_index,
         offset_expr: offset_expr,
         data: data
       }) do
    [u32(memory_index), offset_expr, u32(byte_size(data)), data]
  end

  defp encode_function_body(%FunctionBody{locals: locals, code: code}) do
    local_groups = group_locals(locals)

    payload = [
      u32(length(local_groups)),
      Enum.map(local_groups, fn {count, value_type} ->
        [u32(count), <<encode_value_type(value_type)>>]
      end),
      code
    ]

    [u32(IO.iodata_length(payload)), payload]
  end

  defp group_locals([]), do: []

  defp group_locals([first | rest]) do
    {groups, count, current_type} =
      Enum.reduce(rest, {[], 1, first}, fn value_type, {groups, count, current_type} ->
        if value_type == current_type do
          {groups, count + 1, current_type}
        else
          {[{count, current_type} | groups], 1, value_type}
        end
      end)

    Enum.reverse([{count, current_type} | groups])
  end

  defp encode_custom(%CustomSection{name: name, data: data}) do
    [name(name), data]
  end

  defp encode_value_type(type), do: WasmTypes.value_type(type)

  defp encode_external_kind(kind), do: WasmTypes.external_kind(kind)
end
