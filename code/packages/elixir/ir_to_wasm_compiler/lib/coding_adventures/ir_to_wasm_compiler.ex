defmodule CodingAdventures.IrToWasmCompiler.FunctionSignature do
  @moduledoc """
  WASM-facing function signature metadata for a lowered IR function.
  """

  defstruct label: nil, param_count: 0, export_name: nil

  @type t :: %__MODULE__{
          label: String.t(),
          param_count: non_neg_integer(),
          export_name: String.t() | nil
        }

  @spec new(keyword() | map()) :: t()
  def new(attrs), do: struct!(__MODULE__, Enum.into(attrs, %{}))

  @spec exported(String.t(), non_neg_integer()) :: t()
  def exported(label, param_count) do
    %__MODULE__{label: label, param_count: param_count, export_name: label}
  end
end

defmodule CodingAdventures.IrToWasmCompiler.WasmLoweringError do
  defexception [:message]
end

defmodule CodingAdventures.IrToWasmCompiler do
  @moduledoc """
  Lower the generic compiler IR into a WebAssembly 1.0 module.
  """

  alias CodingAdventures.CompilerIr.{IrImmediate, IrInstruction, IrLabel, IrProgram, IrRegister}
  alias CodingAdventures.IrToWasmCompiler.{FunctionSignature, WasmLoweringError}
  alias CodingAdventures.WasmLeb128
  alias CodingAdventures.WasmOpcodes

  alias CodingAdventures.WasmTypes.{
    DataSegment,
    Export,
    FuncType,
    FunctionBody,
    Import,
    Limits,
    MemoryType,
    WasmModule
  }

  @loop_start_re ~r/^loop_\d+_start$/
  @if_else_re ~r/^if_\d+_else$/
  @function_comment_re ~r/^function:\s*([A-Za-z_][A-Za-z0-9_]*)\((.*)\)$/

  @syscall_write 1
  @syscall_read 2
  @syscall_exit 10
  @syscall_arg0 4

  @wasi_module "wasi_snapshot_preview1"
  @wasi_iovec_offset 0
  @wasi_count_offset 8
  @wasi_byte_offset 12
  @wasi_scratch_size 16

  @memory_ops MapSet.new([:load_addr, :load_byte, :store_byte, :load_word, :store_word])
  @block_type_empty 0x40

  @opcode_names [
    "nop",
    "block",
    "loop",
    "if",
    "else",
    "end",
    "br",
    "br_if",
    "return",
    "call",
    "local.get",
    "local.set",
    "i32.load",
    "i32.load8_u",
    "i32.store",
    "i32.store8",
    "i32.const",
    "i32.eqz",
    "i32.eq",
    "i32.ne",
    "i32.lt_s",
    "i32.gt_s",
    "i32.add",
    "i32.sub",
    "i32.and"
  ]

  @opcodes Enum.into(@opcode_names, %{}, fn name ->
             {:ok, info} = WasmOpcodes.get_opcode_by_name(name)
             {name, info.opcode}
           end)

  @reg_scratch 1
  @reg_var_base 2

  @spec compile(IrProgram.t(), [FunctionSignature.t()] | nil) :: WasmModule.t()
  def compile(%IrProgram{} = program, function_signatures \\ nil) do
    signatures =
      infer_function_signatures_from_comments(program)
      |> Map.merge(Map.new(function_signatures || [], fn sig -> {sig.label, sig} end))

    functions = split_functions(program, signatures)
    imports = collect_wasi_imports(program)
    {type_indices, types} = build_type_table(functions, imports)
    data_offsets = layout_data(program.data)

    scratch_base =
      if needs_wasi_scratch?(program) do
        align_up(Enum.reduce(program.data, 0, fn decl, total -> total + decl.size end), 4)
      else
        nil
      end

    imports_list =
      Enum.map(imports, fn import ->
        %Import{
          module_name: @wasi_module,
          name: import.name,
          kind: :function,
          type_info: {:function, Map.fetch!(type_indices, {:wasi, import.syscall_number})}
        }
      end)

    function_index_base = length(imports_list)

    function_indices =
      functions
      |> Enum.with_index(function_index_base)
      |> Enum.into(%{}, fn {function, index} -> {function.label, index} end)

    module = %WasmModule{
      types: types,
      imports: imports_list,
      functions: Enum.map(functions, &Map.fetch!(type_indices, &1.label))
    }

    total_bytes =
      Enum.reduce(program.data, 0, fn decl, total -> total + decl.size end)
      |> maybe_expand_for_scratch(scratch_base)

    module =
      if needs_memory?(program) or not is_nil(scratch_base) do
        page_count = max(1, div(total_bytes + 65_535, 65_536))

        memories = [%MemoryType{limits: %Limits{min: page_count, max: nil}}]
        exports = [%Export{name: "memory", kind: :memory, index: 0}]

        data_segments =
          Enum.map(program.data, fn decl ->
            offset = Map.fetch!(data_offsets, decl.label)

            %DataSegment{
              memory_index: 0,
              offset_expr: const_expr(offset),
              data: :binary.copy(<<Bitwise.band(decl.init, 0xFF)>>, decl.size)
            }
          end)

        %{module | memories: memories, exports: exports, data: data_segments}
      else
        module
      end

    wasi_context = %{
      function_indices:
        imports
        |> Enum.with_index()
        |> Enum.into(%{}, fn {import, index} -> {import.syscall_number, index} end),
      scratch_base: scratch_base
    }

    {code, export_entries} =
      Enum.reduce(functions, {[], module.exports}, fn function, {code_acc, export_acc} ->
        body = lower_function(function, signatures, function_indices, data_offsets, wasi_context)

        new_exports =
          if function.signature.export_name do
            export_acc ++
              [
                %Export{
                  name: function.signature.export_name,
                  kind: :function,
                  index: Map.fetch!(function_indices, function.label)
                }
              ]
          else
            export_acc
          end

        {code_acc ++ [body], new_exports}
      end)

    %{module | code: code, exports: export_entries}
  end

  @spec infer_function_signatures_from_comments(IrProgram.t()) ::
          %{optional(String.t()) => FunctionSignature.t()}
  def infer_function_signatures_from_comments(%IrProgram{instructions: instructions}) do
    {signatures, _pending_comment} =
      Enum.reduce(instructions, {%{}, nil}, fn instruction, {signatures, pending_comment} ->
        cond do
          instruction.opcode == :comment ->
            {signatures, operand_text(List.first(instruction.operands))}

          true ->
            case function_label_name(instruction) do
              "_start" ->
                signature = %FunctionSignature{
                  label: "_start",
                  param_count: 0,
                  export_name: "_start"
                }

                {Map.put(signatures, "_start", signature), nil}

              label_name when is_binary(label_name) ->
                export_name = String.trim_leading(label_name, "_fn_")

                signatures =
                  case pending_comment && Regex.run(@function_comment_re, pending_comment) do
                    [_, ^export_name, params_blob] ->
                      param_count =
                        params_blob
                        |> String.trim()
                        |> case do
                          "" ->
                            0

                          blob ->
                            blob |> String.split(",") |> Enum.count(&(String.trim(&1) != ""))
                        end

                      Map.put(
                        signatures,
                        label_name,
                        %FunctionSignature{
                          label: label_name,
                          param_count: param_count,
                          export_name: export_name
                        }
                      )

                    _ ->
                      signatures
                  end

                {signatures, nil}

              nil ->
                {signatures, nil}
            end
        end
      end)

    signatures
  end

  defp build_type_table(functions, imports) do
    {type_index_by_signature, function_types, function_to_type_index} =
      Enum.reduce(imports, {%{}, [], %{}}, fn import, {signature_map, types, index_map} ->
        {signature_map, types, type_index} =
          ensure_type_index(signature_map, types, import.func_type)

        {signature_map, types, Map.put(index_map, {:wasi, import.syscall_number}, type_index)}
      end)

    {_, function_types, function_to_type_index} =
      Enum.reduce(
        functions,
        {type_index_by_signature, function_types, function_to_type_index},
        fn function, {signature_map, types, index_map} ->
          func_type = %FuncType{
            params: List.duplicate(:i32, function.signature.param_count),
            results: [:i32]
          }

          {signature_map, types, type_index} = ensure_type_index(signature_map, types, func_type)
          {signature_map, types, Map.put(index_map, function.label, type_index)}
        end
      )

    {function_to_type_index, function_types}
  end

  defp ensure_type_index(signature_map, function_types, func_type) do
    case Map.fetch(signature_map, func_type) do
      {:ok, index} ->
        {signature_map, function_types, index}

      :error ->
        index = length(function_types)
        {Map.put(signature_map, func_type, index), function_types ++ [func_type], index}
    end
  end

  defp layout_data(decls) do
    {offsets, _cursor} =
      Enum.reduce(decls, {%{}, 0}, fn decl, {offsets, cursor} ->
        {Map.put(offsets, decl.label, cursor), cursor + decl.size}
      end)

    offsets
  end

  defp needs_memory?(%IrProgram{data: data, instructions: instructions}) do
    data != [] or
      Enum.any?(instructions, fn instruction ->
        MapSet.member?(@memory_ops, instruction.opcode)
      end)
  end

  defp needs_wasi_scratch?(%IrProgram{instructions: instructions}) do
    Enum.any?(instructions, fn instruction ->
      instruction.opcode == :syscall and
        match?(%IrImmediate{}, List.first(instruction.operands)) and
        expect_immediate(List.first(instruction.operands), "syscall number").value in [
          @syscall_write,
          @syscall_read
        ]
    end)
  end

  defp collect_wasi_imports(%IrProgram{instructions: instructions}) do
    required_syscalls =
      Enum.reduce(instructions, MapSet.new(), fn instruction, acc ->
        if instruction.opcode == :syscall and instruction.operands != [] do
          syscall = expect_immediate(List.first(instruction.operands), "syscall number").value
          MapSet.put(acc, syscall)
        else
          acc
        end
      end)

    ordered_imports = [
      %{
        syscall_number: @syscall_write,
        name: "fd_write",
        func_type: %FuncType{params: [:i32, :i32, :i32, :i32], results: [:i32]}
      },
      %{
        syscall_number: @syscall_read,
        name: "fd_read",
        func_type: %FuncType{params: [:i32, :i32, :i32, :i32], results: [:i32]}
      },
      %{
        syscall_number: @syscall_exit,
        name: "proc_exit",
        func_type: %FuncType{params: [:i32], results: []}
      }
    ]

    supported_syscalls = MapSet.new(Enum.map(ordered_imports, & &1.syscall_number))
    unsupported = MapSet.difference(required_syscalls, supported_syscalls) |> Enum.sort()

    if unsupported != [] do
      raise WasmLoweringError,
        message:
          "unsupported SYSCALL number(s): #{Enum.join(Enum.map(unsupported, &Integer.to_string/1), ", ")}"
    end

    Enum.filter(ordered_imports, fn import ->
      MapSet.member?(required_syscalls, import.syscall_number)
    end)
  end

  defp split_functions(%IrProgram{instructions: instructions}, signatures) do
    {functions, start_index, start_label} =
      Enum.with_index(instructions)
      |> Enum.reduce({[], nil, nil}, fn {instruction, index},
                                        {functions, start_index, start_label} ->
        case function_label_name(instruction) do
          nil ->
            {functions, start_index, start_label}

          label_name ->
            functions =
              if is_integer(start_index) and is_binary(start_label) do
                functions ++
                  [
                    make_function_ir(
                      start_label,
                      Enum.slice(instructions, start_index, index - start_index),
                      signatures
                    )
                  ]
              else
                functions
              end

            {functions, index, label_name}
        end
      end)

    if is_integer(start_index) and is_binary(start_label) do
      functions ++
        [
          make_function_ir(
            start_label,
            Enum.slice(instructions, start_index, length(instructions) - start_index),
            signatures
          )
        ]
    else
      functions
    end
  end

  defp make_function_ir(label, instructions, signatures) do
    signature =
      case {label, Map.get(signatures, label)} do
        {"_start", nil} ->
          %FunctionSignature{label: "_start", param_count: 0, export_name: "_start"}

        {_, %FunctionSignature{} = signature} ->
          signature

        _ ->
          raise WasmLoweringError, message: "missing function signature for #{label}"
      end

    operand_regs =
      instructions
      |> Enum.flat_map(fn instruction ->
        Enum.flat_map(instruction.operands, fn
          %IrRegister{index: index} -> [index]
          _ -> []
        end)
      end)

    syscall_regs =
      instructions
      |> Enum.flat_map(fn
        %IrInstruction{opcode: :syscall} -> [@syscall_arg0]
        _ -> []
      end)

    max_reg =
      ([1, @reg_var_base + max(signature.param_count - 1, 0)] ++ operand_regs ++ syscall_regs)
      |> Enum.max()

    %{
      label: label,
      instructions: instructions,
      signature: signature,
      max_reg: max_reg
    }
  end

  defp maybe_expand_for_scratch(total_bytes, nil), do: total_bytes

  defp maybe_expand_for_scratch(total_bytes, scratch_base),
    do: max(total_bytes, scratch_base + @wasi_scratch_size)

  defp lower_function(function, signatures, function_indices, data_offsets, wasi_context) do
    label_to_index =
      function.instructions
      |> Enum.with_index()
      |> Enum.reduce(%{}, fn {instruction, index}, acc ->
        case label_name(instruction) do
          nil -> acc
          name -> Map.put(acc, name, index)
        end
      end)

    lowerer = %{
      function: function,
      signatures: signatures,
      function_indices: function_indices,
      data_offsets: data_offsets,
      wasi_context: wasi_context,
      param_count: function.signature.param_count,
      instructions: function.instructions,
      label_to_index: label_to_index,
      code: []
    }

    lowerer =
      lowerer
      |> copy_params_into_ir_registers()
      |> emit_region(1, length(function.instructions))
      |> emit_opcode("end")

    %FunctionBody{
      locals: List.duplicate(:i32, function.max_reg + 1),
      code: lowerer.code |> Enum.reverse() |> IO.iodata_to_binary()
    }
  end

  defp copy_params_into_ir_registers(lowerer) do
    reduce_indices(lowerer.param_count, lowerer, fn param_index, acc ->
      acc
      |> emit_opcode("local.get")
      |> emit_u32(param_index)
      |> emit_opcode("local.set")
      |> emit_u32(local_index(acc, @reg_var_base + param_index))
    end)
  end

  defp emit_region(lowerer, start_index, end_index) do
    {lowerer, _index} = do_emit_region(lowerer, start_index, end_index)
    lowerer
  end

  defp do_emit_region(lowerer, index, end_index) when index >= end_index, do: {lowerer, index}

  defp do_emit_region(lowerer, index, end_index) do
    instruction = instruction_at(lowerer, index)

    cond do
      instruction.opcode == :comment ->
        do_emit_region(lowerer, index + 1, end_index)

      is_binary(label_name(instruction)) and Regex.match?(@loop_start_re, label_name(instruction)) ->
        {lowerer, next_index} = emit_loop(lowerer, index)
        do_emit_region(lowerer, next_index, end_index)

      branch_if_else?(instruction) ->
        {lowerer, next_index} = emit_if(lowerer, index)
        do_emit_region(lowerer, next_index, end_index)

      instruction.opcode == :label ->
        do_emit_region(lowerer, index + 1, end_index)

      instruction.opcode in [:jump, :branch_z, :branch_nz] ->
        raise WasmLoweringError,
          message: "unexpected unstructured control flow in #{lowerer.function.label}"

      true ->
        lowerer = emit_simple(lowerer, instruction)
        do_emit_region(lowerer, index + 1, end_index)
    end
  end

  defp emit_if(lowerer, branch_index) do
    branch = instruction_at(lowerer, branch_index)
    cond_reg = expect_register(Enum.at(branch.operands, 0), "if condition")
    else_label = expect_label(Enum.at(branch.operands, 1), "if else label").name
    end_label = String.replace_trailing(else_label, "_else", "_end")

    else_index = require_label_index(lowerer, else_label)
    end_index = require_label_index(lowerer, end_label)
    jump_index = find_last_jump_to_label(lowerer, branch_index + 1, else_index, end_label)

    lowerer =
      lowerer
      |> emit_local_get(cond_reg.index)
      |> maybe_emit_eqz(branch.opcode == :branch_nz)
      |> emit_opcode("if")
      |> emit_byte(@block_type_empty)
      |> emit_region(branch_index + 1, jump_index)

    lowerer =
      if else_index + 1 < end_index do
        lowerer
        |> emit_opcode("else")
        |> emit_region(else_index + 1, end_index)
      else
        lowerer
      end

    {emit_opcode(lowerer, "end"), end_index + 1}
  end

  defp emit_loop(lowerer, label_index) do
    start_label = label_name(instruction_at(lowerer, label_index))

    if is_nil(start_label) do
      raise WasmLoweringError, message: "loop lowering expected a start label"
    end

    end_label = String.replace_trailing(start_label, "_start", "_end")
    end_index = require_label_index(lowerer, end_label)
    branch_index = find_first_branch_to_label(lowerer, label_index + 1, end_index, end_label)
    backedge_index = find_last_jump_to_label(lowerer, branch_index + 1, end_index, start_label)

    branch = instruction_at(lowerer, branch_index)
    cond_reg = expect_register(Enum.at(branch.operands, 0), "loop condition")

    lowerer =
      lowerer
      |> emit_opcode("block")
      |> emit_byte(@block_type_empty)
      |> emit_opcode("loop")
      |> emit_byte(@block_type_empty)
      |> emit_region(label_index + 1, branch_index)
      |> emit_local_get(cond_reg.index)
      |> maybe_emit_eqz(branch.opcode == :branch_z)
      |> emit_opcode("br_if")
      |> emit_u32(1)
      |> emit_region(branch_index + 1, backedge_index)
      |> emit_opcode("br")
      |> emit_u32(0)
      |> emit_opcode("end")
      |> emit_opcode("end")

    {lowerer, end_index + 1}
  end

  defp emit_simple(lowerer, instruction) do
    case instruction.opcode do
      :load_imm ->
        dst = expect_register(Enum.at(instruction.operands, 0), "LOAD_IMM dst")
        imm = expect_immediate(Enum.at(instruction.operands, 1), "LOAD_IMM imm")

        lowerer
        |> emit_i32_const(imm.value)
        |> emit_local_set(dst.index)

      :load_addr ->
        dst = expect_register(Enum.at(instruction.operands, 0), "LOAD_ADDR dst")
        label = expect_label(Enum.at(instruction.operands, 1), "LOAD_ADDR label")

        offset =
          case Map.fetch(lowerer.data_offsets, label.name) do
            {:ok, offset} -> offset
            :error -> raise WasmLoweringError, message: "unknown data label: #{label.name}"
          end

        lowerer
        |> emit_i32_const(offset)
        |> emit_local_set(dst.index)

      :load_byte ->
        dst = expect_register(Enum.at(instruction.operands, 0), "LOAD_BYTE dst")
        base = expect_register(Enum.at(instruction.operands, 1), "LOAD_BYTE base")
        offset = expect_register(Enum.at(instruction.operands, 2), "LOAD_BYTE offset")

        lowerer
        |> emit_address(base.index, offset.index)
        |> emit_opcode("i32.load8_u")
        |> emit_memarg(0, 0)
        |> emit_local_set(dst.index)

      :store_byte ->
        src = expect_register(Enum.at(instruction.operands, 0), "STORE_BYTE src")
        base = expect_register(Enum.at(instruction.operands, 1), "STORE_BYTE base")
        offset = expect_register(Enum.at(instruction.operands, 2), "STORE_BYTE offset")

        lowerer
        |> emit_address(base.index, offset.index)
        |> emit_local_get(src.index)
        |> emit_opcode("i32.store8")
        |> emit_memarg(0, 0)

      :load_word ->
        dst = expect_register(Enum.at(instruction.operands, 0), "LOAD_WORD dst")
        base = expect_register(Enum.at(instruction.operands, 1), "LOAD_WORD base")
        offset = expect_register(Enum.at(instruction.operands, 2), "LOAD_WORD offset")

        lowerer
        |> emit_address(base.index, offset.index)
        |> emit_opcode("i32.load")
        |> emit_memarg(2, 0)
        |> emit_local_set(dst.index)

      :store_word ->
        src = expect_register(Enum.at(instruction.operands, 0), "STORE_WORD src")
        base = expect_register(Enum.at(instruction.operands, 1), "STORE_WORD base")
        offset = expect_register(Enum.at(instruction.operands, 2), "STORE_WORD offset")

        lowerer
        |> emit_address(base.index, offset.index)
        |> emit_local_get(src.index)
        |> emit_opcode("i32.store")
        |> emit_memarg(2, 0)

      :add ->
        emit_binary_numeric(lowerer, "i32.add", instruction)

      :add_imm ->
        dst = expect_register(Enum.at(instruction.operands, 0), "ADD_IMM dst")
        src = expect_register(Enum.at(instruction.operands, 1), "ADD_IMM src")
        imm = expect_immediate(Enum.at(instruction.operands, 2), "ADD_IMM imm")

        lowerer
        |> emit_local_get(src.index)
        |> emit_i32_const(imm.value)
        |> emit_opcode("i32.add")
        |> emit_local_set(dst.index)

      :sub ->
        emit_binary_numeric(lowerer, "i32.sub", instruction)

      :and ->
        emit_binary_numeric(lowerer, "i32.and", instruction)

      :and_imm ->
        dst = expect_register(Enum.at(instruction.operands, 0), "AND_IMM dst")
        src = expect_register(Enum.at(instruction.operands, 1), "AND_IMM src")
        imm = expect_immediate(Enum.at(instruction.operands, 2), "AND_IMM imm")

        lowerer
        |> emit_local_get(src.index)
        |> emit_i32_const(imm.value)
        |> emit_opcode("i32.and")
        |> emit_local_set(dst.index)

      :cmp_eq ->
        emit_binary_numeric(lowerer, "i32.eq", instruction)

      :cmp_ne ->
        emit_binary_numeric(lowerer, "i32.ne", instruction)

      :cmp_lt ->
        emit_binary_numeric(lowerer, "i32.lt_s", instruction)

      :cmp_gt ->
        emit_binary_numeric(lowerer, "i32.gt_s", instruction)

      :call ->
        emit_call(lowerer, instruction)

      :ret ->
        lowerer
        |> emit_local_get(@reg_scratch)
        |> emit_opcode("return")

      :halt ->
        lowerer
        |> emit_local_get(@reg_scratch)
        |> emit_opcode("return")

      :nop ->
        emit_opcode(lowerer, "nop")

      :syscall ->
        emit_syscall(lowerer, instruction)

      opcode ->
        raise WasmLoweringError, message: "unsupported opcode: #{opcode}"
    end
  end

  defp emit_call(lowerer, instruction) do
    label = expect_label(Enum.at(instruction.operands, 0), "CALL target")

    signature =
      Map.get(lowerer.signatures, label.name) ||
        raise(WasmLoweringError, message: "missing function signature for #{label.name}")

    target_index =
      Map.get(lowerer.function_indices, label.name) ||
        raise(WasmLoweringError, message: "unknown function label: #{label.name}")

    lowerer =
      reduce_indices(signature.param_count, lowerer, fn param_index, acc ->
        emit_local_get(acc, @reg_var_base + param_index)
      end)

    lowerer
    |> emit_opcode("call")
    |> emit_u32(target_index)
    |> emit_local_set(@reg_scratch)
  end

  defp emit_syscall(lowerer, instruction) do
    syscall = expect_immediate(Enum.at(instruction.operands, 0), "SYSCALL number").value

    case syscall do
      @syscall_write -> emit_wasi_write(lowerer)
      @syscall_read -> emit_wasi_read(lowerer)
      @syscall_exit -> emit_wasi_exit(lowerer)
      other -> raise WasmLoweringError, message: "unsupported SYSCALL number: #{other}"
    end
  end

  defp emit_wasi_write(lowerer) do
    scratch_base = require_wasi_scratch(lowerer)
    iovec_ptr = scratch_base + @wasi_iovec_offset
    nwritten_ptr = scratch_base + @wasi_count_offset
    byte_ptr = scratch_base + @wasi_byte_offset

    lowerer
    |> emit_i32_const(byte_ptr)
    |> emit_local_get(@syscall_arg0)
    |> emit_opcode("i32.store8")
    |> emit_memarg(0, 0)
    |> emit_store_const_i32(iovec_ptr, byte_ptr)
    |> emit_store_const_i32(iovec_ptr + 4, 1)
    |> emit_i32_const(1)
    |> emit_i32_const(iovec_ptr)
    |> emit_i32_const(1)
    |> emit_i32_const(nwritten_ptr)
    |> emit_wasi_call(@syscall_write)
    |> emit_local_set(@reg_scratch)
  end

  defp emit_wasi_read(lowerer) do
    scratch_base = require_wasi_scratch(lowerer)
    iovec_ptr = scratch_base + @wasi_iovec_offset
    nread_ptr = scratch_base + @wasi_count_offset
    byte_ptr = scratch_base + @wasi_byte_offset

    lowerer
    |> emit_i32_const(byte_ptr)
    |> emit_i32_const(0)
    |> emit_opcode("i32.store8")
    |> emit_memarg(0, 0)
    |> emit_store_const_i32(iovec_ptr, byte_ptr)
    |> emit_store_const_i32(iovec_ptr + 4, 1)
    |> emit_i32_const(0)
    |> emit_i32_const(iovec_ptr)
    |> emit_i32_const(1)
    |> emit_i32_const(nread_ptr)
    |> emit_wasi_call(@syscall_read)
    |> emit_local_set(@reg_scratch)
    |> emit_i32_const(byte_ptr)
    |> emit_opcode("i32.load8_u")
    |> emit_memarg(0, 0)
    |> emit_local_set(@syscall_arg0)
  end

  defp emit_wasi_exit(lowerer) do
    lowerer
    |> emit_local_get(@syscall_arg0)
    |> emit_wasi_call(@syscall_exit)
    |> emit_i32_const(0)
    |> emit_opcode("return")
  end

  defp emit_store_const_i32(lowerer, address, value) do
    lowerer
    |> emit_i32_const(address)
    |> emit_i32_const(value)
    |> emit_opcode("i32.store")
    |> emit_memarg(2, 0)
  end

  defp emit_wasi_call(lowerer, syscall_number) do
    function_index =
      Map.get(lowerer.wasi_context.function_indices, syscall_number) ||
        raise(WasmLoweringError, message: "missing WASI import for SYSCALL #{syscall_number}")

    lowerer
    |> emit_opcode("call")
    |> emit_u32(function_index)
  end

  defp require_wasi_scratch(lowerer) do
    case lowerer.wasi_context.scratch_base do
      nil -> raise WasmLoweringError, message: "SYSCALL lowering requires WASM scratch memory"
      scratch_base -> scratch_base
    end
  end

  defp emit_binary_numeric(lowerer, wasm_op, instruction) do
    dst = expect_register(Enum.at(instruction.operands, 0), "#{instruction.opcode} dst")
    left = expect_register(Enum.at(instruction.operands, 1), "#{instruction.opcode} lhs")
    right = expect_register(Enum.at(instruction.operands, 2), "#{instruction.opcode} rhs")

    lowerer
    |> emit_local_get(left.index)
    |> emit_local_get(right.index)
    |> emit_opcode(wasm_op)
    |> emit_local_set(dst.index)
  end

  defp emit_address(lowerer, base_index, offset_index) do
    lowerer
    |> emit_local_get(base_index)
    |> emit_local_get(offset_index)
    |> emit_opcode("i32.add")
  end

  defp emit_local_get(lowerer, reg_index) do
    lowerer
    |> emit_opcode("local.get")
    |> emit_u32(local_index(lowerer, reg_index))
  end

  defp emit_local_set(lowerer, reg_index) do
    lowerer
    |> emit_opcode("local.set")
    |> emit_u32(local_index(lowerer, reg_index))
  end

  defp emit_i32_const(lowerer, value) do
    lowerer
    |> emit_opcode("i32.const")
    |> append(WasmLeb128.encode_signed(value))
  end

  defp emit_memarg(lowerer, align, offset) do
    lowerer
    |> emit_u32(align)
    |> emit_u32(offset)
  end

  defp emit_opcode(lowerer, name), do: emit_byte(lowerer, Map.fetch!(@opcodes, name))
  defp emit_u32(lowerer, value), do: append(lowerer, WasmLeb128.encode_unsigned(value))
  defp emit_byte(lowerer, value), do: append(lowerer, <<value>>)
  defp append(lowerer, bytes), do: %{lowerer | code: [bytes | lowerer.code]}

  defp maybe_emit_eqz(lowerer, true), do: emit_opcode(lowerer, "i32.eqz")
  defp maybe_emit_eqz(lowerer, false), do: lowerer

  defp local_index(lowerer, reg_index), do: lowerer.param_count + reg_index
  defp instruction_at(lowerer, index), do: Enum.at(lowerer.instructions, index)

  defp require_label_index(lowerer, label) do
    case Map.fetch(lowerer.label_to_index, label) do
      {:ok, index} ->
        index

      :error ->
        raise WasmLoweringError, message: "missing label #{label} in #{lowerer.function.label}"
    end
  end

  defp find_first_branch_to_label(lowerer, start_index, end_index, label),
    do: do_find_first_branch_to_label(lowerer, start_index, end_index, label)

  defp do_find_first_branch_to_label(_lowerer, index, end_index, label) when index >= end_index do
    raise WasmLoweringError, message: "expected branch to #{label}"
  end

  defp do_find_first_branch_to_label(lowerer, index, end_index, label) do
    instruction = instruction_at(lowerer, index)

    if instruction.opcode in [:branch_z, :branch_nz] and
         operand_target_label(Enum.at(instruction.operands, 1)) == label do
      index
    else
      do_find_first_branch_to_label(lowerer, index + 1, end_index, label)
    end
  end

  defp find_last_jump_to_label(lowerer, start_index, end_index, label),
    do: do_find_last_jump_to_label(lowerer, end_index - 1, start_index, label)

  defp do_find_last_jump_to_label(_lowerer, index, start_index, label) when index < start_index do
    raise WasmLoweringError, message: "expected jump to #{label}"
  end

  defp do_find_last_jump_to_label(lowerer, index, start_index, label) do
    instruction = instruction_at(lowerer, index)

    if instruction.opcode == :jump and
         operand_target_label(Enum.at(instruction.operands, 0)) == label do
      index
    else
      do_find_last_jump_to_label(lowerer, index - 1, start_index, label)
    end
  end

  defp branch_if_else?(%IrInstruction{opcode: opcode, operands: [_, %IrLabel{name: label_name}]})
       when opcode in [:branch_z, :branch_nz] do
    Regex.match?(@if_else_re, label_name)
  end

  defp branch_if_else?(_instruction), do: false

  defp function_label_name(instruction) do
    case label_name(instruction) do
      "_start" -> "_start"
      "_fn_" <> _ = label -> label
      _ -> nil
    end
  end

  defp label_name(%IrInstruction{opcode: :label, operands: [%IrLabel{name: name} | _]}), do: name
  defp label_name(_instruction), do: nil

  defp operand_target_label(%IrLabel{name: name}), do: name
  defp operand_target_label(_operand), do: nil

  defp operand_text(%IrLabel{name: name}), do: name
  defp operand_text(_operand), do: nil

  defp expect_register(%IrRegister{} = operand, _context), do: operand

  defp expect_register(operand, context) do
    raise WasmLoweringError, message: "#{context}: expected register, got #{inspect(operand)}"
  end

  defp expect_immediate(%IrImmediate{} = operand, _context), do: operand

  defp expect_immediate(operand, context) do
    raise WasmLoweringError, message: "#{context}: expected immediate, got #{inspect(operand)}"
  end

  defp expect_label(%IrLabel{} = operand, _context), do: operand

  defp expect_label(operand, context) do
    raise WasmLoweringError, message: "#{context}: expected label, got #{inspect(operand)}"
  end

  defp reduce_indices(count, acc, _fun) when count <= 0, do: acc
  defp reduce_indices(count, acc, fun), do: Enum.reduce(0..(count - 1), acc, fun)

  defp const_expr(value) do
    IO.iodata_to_binary([
      <<Map.fetch!(@opcodes, "i32.const")>>,
      WasmLeb128.encode_signed(value),
      <<Map.fetch!(@opcodes, "end")>>
    ])
  end

  defp align_up(value, alignment) do
    div(value + alignment - 1, alignment) * alignment
  end
end
