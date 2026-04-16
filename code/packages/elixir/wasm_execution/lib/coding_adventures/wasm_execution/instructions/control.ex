defmodule CodingAdventures.WasmExecution.Instructions.Control do
  @moduledoc """
  Control flow instruction handlers for WASM.

  WASM control flow uses structured blocks (not arbitrary goto). Each block
  has a well-defined entry/exit and a type (how many values it produces).

  ## Control Flow Model

  The label stack (stored in ctx.label_stack) tracks nested blocks:

      ┌─────────────────────────────────────────────────┐
      │  Label Stack (grows downward, head = innermost)  │
      │                                                   │
      │  label_0: {kind: :block, arity: 1, pc: end+1}    │
      │  label_1: {kind: :loop,  arity: 0, pc: loop_pc}  │
      │  label_2: {kind: :block, arity: 1, pc: end+1}    │
      └─────────────────────────────────────────────────┘

  `br N` branches to the Nth label from the top:
  - For `block`: jump to end_pc + 1 (continuation after the block)
  - For `loop`: jump back to the loop_pc (re-enter the loop body)

  ## Opcode Map

      +--------+------------------+
      | Opcode | Instruction      |
      +--------+------------------+
      | 0x00   | unreachable      |
      | 0x01   | nop              |
      | 0x02   | block            |
      | 0x03   | loop             |
      | 0x04   | if               |
      | 0x05   | else             |
      | 0x0B   | end              |
      | 0x0C   | br               |
      | 0x0D   | br_if            |
      | 0x0F   | return           |
      | 0x10   | call             |
      +--------+------------------+
  """

  alias CodingAdventures.WasmExecution.TrapError
  alias CodingAdventures.VirtualMachine.GenericVM

  @doc "Register all control flow instruction handlers."
  def register(vm) do
    vm
    |> register_basic()
    |> register_blocks()
    |> register_branches()
    |> register_calls()
  end

  # -- Basic control (unreachable, nop) --
  defp register_basic(vm) do
    vm
    # 0x00: unreachable -- always traps
    |> GenericVM.register_context_opcode(0x00, fn _vm, _instr, _code, _ctx ->
      raise TrapError, "unreachable instruction executed"
    end)
    # 0x01: nop -- do nothing
    |> GenericVM.register_context_opcode(0x01, fn vm, _instr, _code, _ctx ->
      {nil, GenericVM.advance_pc(vm)}
    end)
  end

  # -- Structured blocks (block, loop, if, else, end) --
  defp register_blocks(vm) do
    vm
    # 0x02: block -- enter a block
    |> GenericVM.register_context_opcode(0x02, fn vm, instr, _code, ctx ->
      block_type = instr.operand
      arity = block_result_arity(block_type)
      cf_entry = Map.get(ctx.control_flow_map, vm.pc)
      end_pc = if cf_entry, do: cf_entry.end_pc, else: vm.pc + 1

      label = %{
        kind: :block,
        arity: arity,
        target_pc: end_pc + 1,
        stack_height: length(vm.typed_stack)
      }

      ctx = %{ctx | label_stack: [label | ctx.label_stack]}
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    # 0x03: loop -- enter a loop
    |> GenericVM.register_context_opcode(0x03, fn vm, _instr, _code, ctx ->
      # Loop branches go BACK to the loop header (vm.pc + 1 = first body instruction).
      # Loop arity for branching is 0 (loops consume no values when re-entered).
      loop_pc = vm.pc + 1
      label = %{kind: :loop, arity: 0, target_pc: loop_pc, stack_height: length(vm.typed_stack)}
      ctx = %{ctx | label_stack: [label | ctx.label_stack]}
      {nil, GenericVM.advance_pc(vm), ctx}
    end)
    # 0x04: if -- conditional block
    |> GenericVM.register_context_opcode(0x04, fn vm, instr, _code, ctx ->
      block_type = instr.operand
      arity = block_result_arity(block_type)
      {condition, vm} = GenericVM.pop_typed(vm)
      cf_entry = Map.get(ctx.control_flow_map, vm.pc)
      end_pc = if cf_entry, do: cf_entry.end_pc, else: vm.pc + 1
      else_pc = if cf_entry, do: cf_entry.else_pc, else: nil

      label = %{
        kind: :block,
        arity: arity,
        target_pc: end_pc + 1,
        stack_height: length(vm.typed_stack)
      }

      ctx = %{ctx | label_stack: [label | ctx.label_stack]}

      if condition.value != 0 do
        # Take the if-true branch (fall through to next instruction)
        {nil, GenericVM.advance_pc(vm), ctx}
      else
        # Jump to else or end
        target = if else_pc, do: else_pc + 1, else: end_pc
        {nil, GenericVM.jump_to(vm, target), ctx}
      end
    end)
    # 0x05: else -- switch from if-true to if-false branch
    |> GenericVM.register_context_opcode(0x05, fn vm, _instr, _code, ctx ->
      # When we reach else during normal execution (from if-true path),
      # we need to skip to end. The label on the stack has the target.
      case ctx.label_stack do
        [label | _rest] ->
          {nil, GenericVM.jump_to(vm, label.target_pc - 1), ctx}

        _ ->
          raise TrapError, "else without matching if"
      end
    end)
    # 0x0B: end -- close a block/loop/if or terminate the function
    |> GenericVM.register_context_opcode(0x0B, fn vm, _instr, _code, ctx ->
      case ctx.label_stack do
        [_label | rest_labels] ->
          ctx = %{ctx | label_stack: rest_labels}
          {nil, GenericVM.advance_pc(vm), ctx}

        [] ->
          # Function end -- halt execution
          vm = %{vm | halted: true}
          {nil, vm, ctx}
      end
    end)
  end

  # -- Branch instructions --
  defp register_branches(vm) do
    vm
    # 0x0C: br -- unconditional branch
    |> GenericVM.register_context_opcode(0x0C, fn vm, instr, _code, ctx ->
      depth = instr.operand
      {vm, ctx} = do_branch(vm, ctx, depth)
      {nil, vm, ctx}
    end)
    # 0x0D: br_if -- conditional branch
    |> GenericVM.register_context_opcode(0x0D, fn vm, instr, _code, ctx ->
      {condition, vm} = GenericVM.pop_typed(vm)

      if condition.value != 0 do
        depth = instr.operand
        {vm, ctx} = do_branch(vm, ctx, depth)
        {nil, vm, ctx}
      else
        {nil, GenericVM.advance_pc(vm), ctx}
      end
    end)
    # 0x0F: return -- return from the current function
    |> GenericVM.register_context_opcode(0x0F, fn vm, _instr, _code, ctx ->
      # Return unwinds all labels and halts this function's execution
      vm = %{vm | halted: true}
      {nil, vm, ctx}
    end)
  end

  # -- Call instructions --
  defp register_calls(vm) do
    vm
    # 0x10: call -- direct function call
    # The engine handles actual call dispatch; this handler just signals
    # the call by storing the target function index in the context.
    |> GenericVM.register_context_opcode(0x10, fn vm, instr, _code, ctx ->
      func_idx = instr.operand
      # Signal a call to the engine by setting a pending_call field
      ctx = Map.put(ctx, :pending_call, func_idx)
      vm = GenericVM.advance_pc(vm)
      {nil, vm, ctx}
    end)
  end

  # -- Branch helper --

  defp do_branch(vm, ctx, depth) do
    label = Enum.at(ctx.label_stack, depth)

    if label == nil do
      raise TrapError, "branch depth #{depth} exceeds label stack size #{length(ctx.label_stack)}"
    end

    # Preserve the top `arity` values from the stack
    {result_values, vm} = pop_n_typed(vm, label.arity)

    # Restore stack to the height when this label was entered
    vm = trim_typed_stack(vm, label.stack_height)

    # Push back the result values
    vm =
      Enum.reduce(Enum.reverse(result_values), vm, fn val, acc_vm ->
        GenericVM.push_typed(acc_vm, val)
      end)

    # Loop branches target the loop header and keep the loop label active for
    # subsequent iterations. Block/if branches exit the target construct, so
    # those labels are popped as part of the branch.
    labels_remaining =
      if label.kind == :loop do
        Enum.drop(ctx.label_stack, depth)
      else
        Enum.drop(ctx.label_stack, depth + 1)
      end

    ctx = %{ctx | label_stack: labels_remaining}

    # Jump to the label's target
    vm = GenericVM.jump_to(vm, label.target_pc)

    # For block/if: target_pc is end+1 (continuation)
    # For loop: target_pc is loop header (re-entry)
    # In both cases we jump directly to target_pc (no advance_pc needed)
    {vm, ctx}
  end

  defp pop_n_typed(vm, 0), do: {[], vm}

  defp pop_n_typed(vm, n) do
    Enum.reduce(1..n, {[], vm}, fn _, {acc, acc_vm} ->
      {val, acc_vm} = GenericVM.pop_typed(acc_vm)
      {[val | acc], acc_vm}
    end)
  end

  defp trim_typed_stack(vm, target_height) do
    current = length(vm.typed_stack)

    if current > target_height do
      %{vm | typed_stack: Enum.take(vm.typed_stack, -target_height)}
    else
      vm
    end
  end

  # -- Block type arity --
  # 0x40 = empty block (no result)
  # 0x7F/0x7E/0x7D/0x7C = single result of that type
  defp block_result_arity(0x40), do: 0
  defp block_result_arity(type_byte) when type_byte in [0x7F, 0x7E, 0x7D, 0x7C], do: 1
  defp block_result_arity(_), do: 0
end
