defmodule CodingAdventures.RegisterVM.Interpreter do
  @moduledoc """
  Register-based VM interpreter — the execution engine.

  # Execution model

  Unlike a stack-based VM where instructions push/pop an operand stack,
  this VM uses an **accumulator register** and a **fixed-size register file**
  per call frame.

  Most instructions read from and write to the accumulator (acc). Binary
  operations take one operand from a named register:

      Add r0    # acc = acc + registers[0]

  This is the same model used by V8's Ignition interpreter and Lua's VM.
  It has a key advantage over pure stack VMs: register operand encoding
  eliminates many push/pop instructions, making the instruction stream denser
  and easier for a JIT to analyse.

  # Eval loop

  The eval loop is tail-recursive in Elixir. Each instruction returns one of:

      {:continue, new_frame, globals, output}
        — keep running in the same call frame

      {:return, value, caller_frame, output}
        — return this value to the caller

      {:call, callee_code, frame, globals, output}
        — push a new call frame for callee_code

      {:error, %VMError{}}
        — halt execution with an error

  The `run/4` function matches on these tags. Elixir's tail-call optimisation
  ensures the `:continue` case does not grow the process stack.

  # Feedback vectors

  Every dynamic-dispatch instruction (arithmetic, property access, calls)
  has a feedback_slot index. On every execution, the interpreter records
  what types it saw at that instruction site. A future JIT compiler reads
  this feedback to make type-specialised decisions.

  # Call frames

  Each function call pushes a new CallFrame. The frame holds:
  - The CodeObject being executed
  - The instruction pointer (ip)
  - The accumulator value
  - The register file (tuple of size code.register_count)
  - The feedback vector (list of size code.feedback_slot_count)
  - A reference to the calling frame
  """

  import Bitwise

  alias CodingAdventures.RegisterVM.Opcodes
  alias CodingAdventures.RegisterVM.Feedback
  alias CodingAdventures.RegisterVM.Scope

  alias CodingAdventures.RegisterVM.Types.CodeObject
  alias CodingAdventures.RegisterVM.Types.RegisterInstruction
  alias CodingAdventures.RegisterVM.Types.CallFrame
  alias CodingAdventures.RegisterVM.Types.VMResult
  alias CodingAdventures.RegisterVM.Types.VMError
  alias CodingAdventures.RegisterVM.Types.TraceStep

  # Maximum call stack depth before raising a StackOverflow error.
  # Real JS engines default to ~10,000; we use 500 to keep tests fast.
  @max_call_depth 500

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Executes a CodeObject and returns a VMResult.

  ## Examples

      iex> code = %CodeObject{
      ...>   instructions: [
      ...>     %RegisterInstruction{opcode: Opcodes.lda_smi(), operands: [42]},
      ...>     %RegisterInstruction{opcode: Opcodes.halt(), operands: []}
      ...>   ],
      ...>   constants: [], names: [], register_count: 0,
      ...>   feedback_slot_count: 0
      ...> }
      iex> {:ok, result} = Interpreter.execute(code)
      iex> result.return_value
      42
  """
  def execute(%CodeObject{} = code) do
    globals = Scope.new_globals()
    frame = new_frame(code, nil)

    case run(frame, globals, 0, []) do
      {:ok, value, output, fv} ->
        {:ok, %VMResult{return_value: value, output: output, error: nil, final_feedback_vector: fv}}

      {:error, err} ->
        {:ok, %VMResult{return_value: nil, output: [], error: err, final_feedback_vector: []}}
    end
  end

  @doc """
  Executes a CodeObject and returns both a VMResult and a list of TraceStep records.

  The trace contains one entry per instruction executed. Useful for debugging
  and verifying feedback vector transitions in tests.
  """
  def execute_with_trace(%CodeObject{} = code) do
    globals = Scope.new_globals()
    frame = new_frame(code, nil)

    case run_traced(frame, globals, 0, [], []) do
      {:ok, value, output, fv, trace} ->
        result = %VMResult{return_value: value, output: output, error: nil, final_feedback_vector: fv}
        {:ok, result, trace}

      {:error, err} ->
        {:ok, %VMResult{return_value: nil, output: [], error: err, final_feedback_vector: []}, []}
    end
  end

  # ---------------------------------------------------------------------------
  # Frame construction
  # ---------------------------------------------------------------------------

  # Creates a fresh CallFrame for a CodeObject.
  # Registers are initialised to nil; feedback slots to :uninitialized.
  defp new_frame(%CodeObject{} = code, caller_frame) do
    %CallFrame{
      code: code,
      ip: 0,
      accumulator: :undefined,
      registers: Tuple.duplicate(nil, max(code.register_count, 1)),
      feedback_vector: Feedback.new_vector(code.feedback_slot_count),
      context: nil,
      caller_frame: caller_frame
    }
  end

  # ---------------------------------------------------------------------------
  # Main eval loop (no tracing)
  # ---------------------------------------------------------------------------

  # The eval loop. Fetches the instruction at frame.ip and dispatches it.
  # Tail-recursive: Elixir will optimise the :continue case so the Elixir
  # process stack does not grow with each iteration.
  defp run(frame, globals, depth, output) do
    instr = Enum.at(frame.code.instructions, frame.ip)

    if instr == nil do
      # ip has walked past the end of the instruction list.
      # Treat this as an implicit Halt — return the accumulator.
      {:ok, frame.accumulator, output, frame.feedback_vector}
    else
      case execute_instruction(instr, frame, globals, depth, output) do
        {:continue, new_frame, new_globals, new_output} ->
          # Tail call: continue executing in the same call depth
          run(new_frame, new_globals, depth, new_output)

        {:return, value, _caller_frame, new_output, fv} ->
          # Function returned; bubble up the value
          {:ok, value, new_output, fv}

        {:call, callee_code, resumed_frame, new_globals, new_output} ->
          # Push a new call frame, run the callee, then resume the caller
          if depth >= @max_call_depth do
            err = %VMError{
              message: "Stack overflow: call depth exceeded #{@max_call_depth}",
              instruction_index: frame.ip,
              opcode: instr.opcode
            }
            {:error, err}
          else
            callee_frame = new_frame(callee_code, resumed_frame)

            case run(callee_frame, new_globals, depth + 1, new_output) do
              {:ok, return_val, out2, _callee_fv} ->
                # Callee finished: load return value into accumulator of resumed frame
                # and advance ip by 1 (resume_frame already has ip pointing to the
                # instruction AFTER the call)
                resumed_with_result = %{resumed_frame | accumulator: return_val}
                run(resumed_with_result, new_globals, depth, out2)

              {:error, _} = err ->
                err
            end
          end

        {:error, _} = err ->
          err
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Traced eval loop
  # ---------------------------------------------------------------------------

  defp run_traced(frame, globals, depth, output, trace) do
    instr = Enum.at(frame.code.instructions, frame.ip)

    if instr == nil do
      {:ok, frame.accumulator, output, frame.feedback_vector, Enum.reverse(trace)}
    else
      # Capture state before execution for the trace step
      acc_before = frame.accumulator
      regs_before = frame.registers
      fv_before = frame.feedback_vector

      case execute_instruction(instr, frame, globals, depth, output) do
        {:continue, new_frame, new_globals, new_output} ->
          step = build_trace_step(depth, frame.ip, instr, acc_before, new_frame.accumulator,
                                  regs_before, new_frame.registers, fv_before, new_frame.feedback_vector)
          run_traced(new_frame, new_globals, depth, new_output, [step | trace])

        {:return, value, _caller_frame, new_output, fv} ->
          step = build_trace_step(depth, frame.ip, instr, acc_before, value,
                                  regs_before, regs_before, fv_before, fv)
          {:ok, value, new_output, fv, Enum.reverse([step | trace])}

        {:call, callee_code, resumed_frame, new_globals, new_output} ->
          if depth >= @max_call_depth do
            err = %VMError{
              message: "Stack overflow: call depth exceeded #{@max_call_depth}",
              instruction_index: frame.ip,
              opcode: instr.opcode
            }
            {:error, err}
          else
            callee_frame = new_frame(callee_code, resumed_frame)

            case run_traced(callee_frame, new_globals, depth + 1, new_output, []) do
              {:ok, return_val, out2, _callee_fv, callee_trace} ->
                resumed_with_result = %{resumed_frame | accumulator: return_val}
                combined_trace = trace ++ callee_trace
                run_traced(resumed_with_result, new_globals, depth, out2, combined_trace)

              {:error, _} = err ->
                err
            end
          end

        {:error, _} = err ->
          err
      end
    end
  end

  defp build_trace_step(depth, ip, instr, acc_before, acc_after, regs_before, regs_after, fv_before, fv_after) do
    # Compute which feedback slots changed
    feedback_delta =
      fv_before
      |> Enum.with_index()
      |> Enum.filter(fn {old, idx} -> old != Enum.at(fv_after, idx) end)
      |> Enum.map(fn {old, idx} -> {idx, old, Enum.at(fv_after, idx)} end)

    %TraceStep{
      frame_depth: depth,
      ip: ip,
      instruction: instr,
      acc_before: acc_before,
      acc_after: acc_after,
      registers_before: regs_before,
      registers_after: regs_after,
      feedback_delta: feedback_delta
    }
  end

  # ---------------------------------------------------------------------------
  # Instruction dispatch
  # ---------------------------------------------------------------------------
  # Each clause handles one opcode. The pattern match on opcode is the
  # "instruction decode" phase of a classic fetch-decode-execute cycle.
  #
  # Convention:
  #   - Advance ip FIRST: `advanced = %{frame | ip: frame.ip + 1}`
  #   - Then perform the operation and return {:continue, ...} or {:return, ...}
  #   - For jumps: ip is set after the advance, so jumps are relative to the
  #     instruction AFTER the current one.

  defp execute_instruction(%RegisterInstruction{opcode: opcode} = instr, frame, globals, depth, output) do
    advanced = %{frame | ip: frame.ip + 1}

    cond do
      # ------------------------------------------------------------------
      # 0x0_ Accumulator loads
      # ------------------------------------------------------------------

      opcode == Opcodes.lda_constant() ->
        # Load a value from the constants pool at the given index.
        # Use case: `let x = 3.14` → LdaConstant 0 (where constants[0] = 3.14)
        [idx] = instr.operands
        value = Enum.at(frame.code.constants, idx)
        {:continue, %{advanced | accumulator: value}, globals, output}

      opcode == Opcodes.lda_zero() ->
        # Load the integer 0. Common enough to deserve a dedicated opcode
        # rather than wasting a constants-pool slot.
        {:continue, %{advanced | accumulator: 0}, globals, output}

      opcode == Opcodes.lda_smi() ->
        # Load a "small integer" embedded directly in the instruction stream.
        # In real V8, SMIs (Small Integers) are tagged pointer values that
        # fit in one machine word. Here we just treat them as regular integers.
        [value] = instr.operands
        {:continue, %{advanced | accumulator: value}, globals, output}

      opcode == Opcodes.lda_undefined() ->
        # Load :undefined — JavaScript's "variable declared but not assigned."
        # We use the atom :undefined to distinguish it from Elixir's nil.
        {:continue, %{advanced | accumulator: :undefined}, globals, output}

      opcode == Opcodes.lda_null() ->
        # Load nil — JavaScript's explicit "empty value" (typeof null == "object").
        {:continue, %{advanced | accumulator: nil}, globals, output}

      opcode == Opcodes.lda_true() ->
        {:continue, %{advanced | accumulator: true}, globals, output}

      opcode == Opcodes.lda_false() ->
        {:continue, %{advanced | accumulator: false}, globals, output}

      # ------------------------------------------------------------------
      # 0x1_ Register moves
      # ------------------------------------------------------------------

      opcode == Opcodes.ldar() ->
        # Load Accumulator from Register: acc = registers[r]
        # Use case: when you need to restore a previously saved value.
        [reg] = instr.operands
        value = elem(frame.registers, reg)
        {:continue, %{advanced | accumulator: value}, globals, output}

      opcode == Opcodes.star() ->
        # Store Accumulator to Register: registers[r] = acc
        # Use case: save the current result before computing something new.
        [reg] = instr.operands
        new_regs = put_elem(advanced.registers, reg, frame.accumulator)
        {:continue, %{advanced | registers: new_regs}, globals, output}

      opcode == Opcodes.mov() ->
        # Move between registers (does NOT touch the accumulator).
        # registers[dst] = registers[src]
        [src, dst] = instr.operands
        value = elem(frame.registers, src)
        new_regs = put_elem(advanced.registers, dst, value)
        {:continue, %{advanced | registers: new_regs}, globals, output}

      # ------------------------------------------------------------------
      # 0x2_ Variable access
      # ------------------------------------------------------------------

      opcode == Opcodes.lda_global() ->
        # Load a global variable: acc = globals[names[name_idx]]
        # The feedback slot records what type the variable held at this point.
        [name_idx | rest] = instr.operands
        slot = List.first(rest)
        name = Enum.at(frame.code.names, name_idx)

        case Scope.get_global(globals, name) do
          {:ok, value} ->
            new_fv = if slot, do: Feedback.record_binary_op(advanced.feedback_vector, slot, value, value), else: advanced.feedback_vector
            {:continue, %{advanced | accumulator: value, feedback_vector: new_fv}, globals, output}

          :error ->
            err = %VMError{
              message: "ReferenceError: #{name} is not defined",
              instruction_index: frame.ip,
              opcode: opcode
            }
            {:error, err}
        end

      opcode == Opcodes.sta_global() ->
        # Store acc to a global variable: globals[names[name_idx]] = acc
        [name_idx | _rest] = instr.operands
        name = Enum.at(frame.code.names, name_idx)
        new_globals = Scope.set_global(globals, name, frame.accumulator)
        {:continue, advanced, new_globals, output}

      opcode == Opcodes.lda_local() ->
        # Convenience alias for lda_global with local semantics.
        # In this simplified VM, locals are stored in registers, so this
        # acts like LdaGlobal for variables not yet in registers.
        [name_idx | rest] = instr.operands
        _slot = List.first(rest)
        name = Enum.at(frame.code.names, name_idx)

        case Scope.get_global(globals, name) do
          {:ok, value} ->
            {:continue, %{advanced | accumulator: value}, globals, output}

          :error ->
            err = %VMError{
              message: "ReferenceError: #{name} is not defined",
              instruction_index: frame.ip,
              opcode: opcode
            }
            {:error, err}
        end

      opcode == Opcodes.sta_local() ->
        # Store acc to a local (treated as global in this simplified VM).
        [name_idx | _rest] = instr.operands
        name = Enum.at(frame.code.names, name_idx)
        new_globals = Scope.set_global(globals, name, frame.accumulator)
        {:continue, advanced, new_globals, output}

      opcode == Opcodes.lda_context_slot() ->
        # Load from a captured variable: walk `depth` links up the context chain,
        # then read slot `idx`.
        [depth, idx | _rest] = instr.operands

        case Scope.get_slot(frame.context, depth, idx) do
          {:ok, value} ->
            {:continue, %{advanced | accumulator: value}, globals, output}

          :error ->
            err = %VMError{
              message: "ContextError: invalid context slot [depth=#{depth}, idx=#{idx}]",
              instruction_index: frame.ip,
              opcode: opcode
            }
            {:error, err}
        end

      opcode == Opcodes.sta_context_slot() ->
        # Store acc to a context slot at the given depth and index.
        [depth, idx | _rest] = instr.operands

        case Scope.set_slot(frame.context, depth, idx, frame.accumulator) do
          {:ok, new_ctx} ->
            {:continue, %{advanced | context: new_ctx}, globals, output}

          :error ->
            err = %VMError{
              message: "ContextError: invalid context slot [depth=#{depth}, idx=#{idx}]",
              instruction_index: frame.ip,
              opcode: opcode
            }
            {:error, err}
        end

      opcode == Opcodes.lda_current_context_slot() ->
        # Load from the CURRENT (innermost) context at slot idx.
        [idx | _rest] = instr.operands

        case Scope.get_slot(frame.context, 0, idx) do
          {:ok, value} ->
            {:continue, %{advanced | accumulator: value}, globals, output}

          :error ->
            err = %VMError{
              message: "ContextError: invalid current context slot #{idx}",
              instruction_index: frame.ip,
              opcode: opcode
            }
            {:error, err}
        end

      opcode == Opcodes.sta_current_context_slot() ->
        # Store acc to the CURRENT context at slot idx.
        [idx | _rest] = instr.operands

        case Scope.set_slot(frame.context, 0, idx, frame.accumulator) do
          {:ok, new_ctx} ->
            {:continue, %{advanced | context: new_ctx}, globals, output}

          :error ->
            err = %VMError{
              message: "ContextError: invalid current context slot #{idx}",
              instruction_index: frame.ip,
              opcode: opcode
            }
            {:error, err}
        end

      # ------------------------------------------------------------------
      # 0x3_ Arithmetic
      # ------------------------------------------------------------------
      # Pattern for binary ops: [rhs_reg, feedback_slot]
      # acc = acc OP registers[rhs_reg]
      # Record type feedback at the given slot.

      opcode == Opcodes.add() ->
        # Addition — also handles string concatenation (like JavaScript's + operator).
        # If either operand is a string, the other is coerced to string and they concatenate.
        [reg, slot | _] = instr.operands
        right = elem(frame.registers, reg)
        left = frame.accumulator
        result = add_values(left, right)
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, right)
        {:continue, %{advanced | accumulator: result, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.sub() ->
        [reg, slot | _] = instr.operands
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, right)
        {:continue, %{advanced | accumulator: left - right, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.mul() ->
        [reg, slot | _] = instr.operands
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, right)
        {:continue, %{advanced | accumulator: left * right, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.div() ->
        # Division always returns a float in Elixir (via `/`).
        # Integer division via `div/2` would truncate — we don't want that.
        [reg, slot | _] = instr.operands
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, right)

        if right == 0 do
          {:continue, %{advanced | accumulator: :infinity, feedback_vector: new_fv}, globals, output}
        else
          {:continue, %{advanced | accumulator: left / right, feedback_vector: new_fv}, globals, output}
        end

      opcode == Opcodes.mod() ->
        [reg, slot | _] = instr.operands
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, right)
        {:continue, %{advanced | accumulator: rem(left, right), feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.pow() ->
        # Exponentiation: acc = acc ** registers[r]
        [reg, slot | _] = instr.operands
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, right)
        result = :math.pow(left * 1.0, right * 1.0)
        # Convert back to integer if the result is a whole number
        result = if trunc(result) == result and is_integer(left) and is_integer(right) and right >= 0,
          do: trunc(result), else: result
        {:continue, %{advanced | accumulator: result, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.add_smi() ->
        # Add a small integer literal: acc = acc + operands[0]
        # Saves a constants-pool slot for the extremely common "i += 1" case.
        [value, slot | _] = instr.operands
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, value)
        {:continue, %{advanced | accumulator: left + value, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.sub_smi() ->
        [value, slot | _] = instr.operands
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, value)
        {:continue, %{advanced | accumulator: left - value, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.bitwise_and() ->
        # Bitwise AND: acc = acc &&& registers[r]
        [reg, slot | _] = instr.operands
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, right)
        {:continue, %{advanced | accumulator: left &&& right, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.bitwise_or() ->
        [reg, slot | _] = instr.operands
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, right)
        {:continue, %{advanced | accumulator: left ||| right, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.bitwise_xor() ->
        [reg, slot | _] = instr.operands
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, right)
        {:continue, %{advanced | accumulator: bxor(left, right), feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.bitwise_not() ->
        # Unary bitwise NOT: acc = ~~~acc
        [slot | _] = instr.operands
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, left)
        {:continue, %{advanced | accumulator: bnot(left), feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.shift_left() ->
        [reg, slot | _] = instr.operands
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, right)
        {:continue, %{advanced | accumulator: left <<< right, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.shift_right() ->
        # Arithmetic right shift (sign-preserving)
        [reg, slot | _] = instr.operands
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, right)
        {:continue, %{advanced | accumulator: left >>> right, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.shift_right_logical() ->
        # Logical right shift: fill with zeros from the left.
        # Elixir integers are arbitrary precision (no 32-bit wrap), so we
        # mask to 32 bits first, do an unsigned right shift, then mask again.
        [reg, slot | _] = instr.operands
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = Feedback.record_binary_op(advanced.feedback_vector, slot, left, right)
        result = (left &&& 0xFFFFFFFF) >>> (right &&& 0x1F) &&& 0xFFFFFFFF
        {:continue, %{advanced | accumulator: result, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.negate() ->
        # Unary negation: acc = -acc
        {:continue, %{advanced | accumulator: -frame.accumulator}, globals, output}

      # ------------------------------------------------------------------
      # 0x4_ Comparisons
      # ------------------------------------------------------------------

      opcode == Opcodes.test_equal() ->
        # Abstract equality (like JS ==): coerces types before comparing.
        [reg | rest] = instr.operands
        slot = List.first(rest)
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = if slot, do: Feedback.record_binary_op(advanced.feedback_vector, slot, left, right), else: advanced.feedback_vector
        result = abstract_equal?(left, right)
        {:continue, %{advanced | accumulator: result, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.test_not_equal() ->
        [reg | rest] = instr.operands
        slot = List.first(rest)
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = if slot, do: Feedback.record_binary_op(advanced.feedback_vector, slot, left, right), else: advanced.feedback_vector
        result = !abstract_equal?(left, right)
        {:continue, %{advanced | accumulator: result, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.test_strict_equal() ->
        # Strict equality (===): no type coercion.
        [reg | rest] = instr.operands
        slot = List.first(rest)
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = if slot, do: Feedback.record_binary_op(advanced.feedback_vector, slot, left, right), else: advanced.feedback_vector
        {:continue, %{advanced | accumulator: left === right, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.test_strict_not_equal() ->
        [reg | rest] = instr.operands
        slot = List.first(rest)
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = if slot, do: Feedback.record_binary_op(advanced.feedback_vector, slot, left, right), else: advanced.feedback_vector
        {:continue, %{advanced | accumulator: left !== right, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.test_less_than() ->
        [reg | rest] = instr.operands
        slot = List.first(rest)
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = if slot, do: Feedback.record_binary_op(advanced.feedback_vector, slot, left, right), else: advanced.feedback_vector
        {:continue, %{advanced | accumulator: left < right, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.test_greater_than() ->
        [reg | rest] = instr.operands
        slot = List.first(rest)
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = if slot, do: Feedback.record_binary_op(advanced.feedback_vector, slot, left, right), else: advanced.feedback_vector
        {:continue, %{advanced | accumulator: left > right, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.test_less_than_or_equal() ->
        [reg | rest] = instr.operands
        slot = List.first(rest)
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = if slot, do: Feedback.record_binary_op(advanced.feedback_vector, slot, left, right), else: advanced.feedback_vector
        {:continue, %{advanced | accumulator: left <= right, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.test_greater_than_or_equal() ->
        [reg | rest] = instr.operands
        slot = List.first(rest)
        right = elem(frame.registers, reg)
        left = frame.accumulator
        new_fv = if slot, do: Feedback.record_binary_op(advanced.feedback_vector, slot, left, right), else: advanced.feedback_vector
        {:continue, %{advanced | accumulator: left >= right, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.test_in() ->
        # Membership: acc = (acc is in registers[r])
        # For lists: check if element is in the list.
        # For maps: check if key exists.
        [reg | _] = instr.operands
        container = elem(frame.registers, reg)
        key = frame.accumulator

        result = cond do
          is_list(container) -> key in container
          is_map(container) -> Map.has_key?(container, key)
          true -> false
        end

        {:continue, %{advanced | accumulator: result}, globals, output}

      opcode == Opcodes.test_instanceof() ->
        # Simplified isinstance: checks if acc is a tuple {:instance_of, type, ...}
        # or if acc is a map (representing an object) vs a function, etc.
        [reg | _] = instr.operands
        _type_val = elem(frame.registers, reg)
        # Simplified: just check if acc is a map (acting as "object instance")
        result = is_map(frame.accumulator)
        {:continue, %{advanced | accumulator: result}, globals, output}

      opcode == Opcodes.test_undetectable() ->
        # Undetectable values in JavaScript: null and undefined.
        # `document.all` is also undetectable in browsers, but we skip that.
        result = frame.accumulator == nil or frame.accumulator == :undefined
        {:continue, %{advanced | accumulator: result}, globals, output}

      opcode == Opcodes.logical_not() ->
        # !acc — logical NOT using JavaScript's truthiness rules.
        {:continue, %{advanced | accumulator: !truthy?(frame.accumulator)}, globals, output}

      opcode == Opcodes.typeof() ->
        # Returns the type of acc as a string (JavaScript typeof semantics).
        # Note: `typeof null` is "object" — a famous JavaScript quirk dating
        # to Brendan Eich's original 10-day implementation.
        {:continue, %{advanced | accumulator: type_string(frame.accumulator)}, globals, output}

      # ------------------------------------------------------------------
      # 0x5_ Control flow
      # ------------------------------------------------------------------
      # Jump operand semantics: the offset is relative to the instruction
      # AFTER the jump. Since we already advanced ip by 1 before executing,
      # `advanced.ip` is already pointing at the next instruction.
      # So: new_ip = advanced.ip + offset

      opcode == Opcodes.jump() ->
        [offset] = instr.operands
        {:continue, %{advanced | ip: advanced.ip + offset}, globals, output}

      opcode == Opcodes.jump_if_true() ->
        [offset] = instr.operands

        if truthy?(frame.accumulator) do
          {:continue, %{advanced | ip: advanced.ip + offset}, globals, output}
        else
          {:continue, advanced, globals, output}
        end

      opcode == Opcodes.jump_if_false() ->
        [offset] = instr.operands

        if !truthy?(frame.accumulator) do
          {:continue, %{advanced | ip: advanced.ip + offset}, globals, output}
        else
          {:continue, advanced, globals, output}
        end

      opcode == Opcodes.jump_if_null() ->
        [offset] = instr.operands

        if frame.accumulator == nil do
          {:continue, %{advanced | ip: advanced.ip + offset}, globals, output}
        else
          {:continue, advanced, globals, output}
        end

      opcode == Opcodes.jump_if_undefined() ->
        [offset] = instr.operands

        if frame.accumulator == :undefined do
          {:continue, %{advanced | ip: advanced.ip + offset}, globals, output}
        else
          {:continue, advanced, globals, output}
        end

      opcode == Opcodes.jump_if_null_or_undefined() ->
        [offset] = instr.operands

        if frame.accumulator == nil or frame.accumulator == :undefined do
          {:continue, %{advanced | ip: advanced.ip + offset}, globals, output}
        else
          {:continue, advanced, globals, output}
        end

      opcode == Opcodes.jump_if_to_boolean_true() ->
        [offset] = instr.operands

        if truthy?(frame.accumulator) do
          {:continue, %{advanced | ip: advanced.ip + offset}, globals, output}
        else
          {:continue, advanced, globals, output}
        end

      opcode == Opcodes.jump_if_to_boolean_false() ->
        [offset] = instr.operands

        if !truthy?(frame.accumulator) do
          {:continue, %{advanced | ip: advanced.ip + offset}, globals, output}
        else
          {:continue, advanced, globals, output}
        end

      opcode == Opcodes.jump_loop() ->
        # Loop back-edge: semantically identical to Jump but signals to a future
        # JIT that this is a loop header (triggers On-Stack Replacement).
        [offset] = instr.operands
        {:continue, %{advanced | ip: advanced.ip + offset}, globals, output}

      # ------------------------------------------------------------------
      # 0x6_ Calls and returns
      # ------------------------------------------------------------------

      opcode == Opcodes.call_any_receiver() ->
        # Call a function: operands = [func_reg, first_arg_reg, argc, feedback_slot]
        # The function value must be a {:function, code, closure_context} tuple.
        [func_reg, first_arg_reg, argc | rest] = instr.operands
        slot = List.first(rest)

        func_val = elem(frame.registers, func_reg)

        # Record call site feedback
        new_fv = if slot != nil,
          do: Feedback.record_call_site(advanced.feedback_vector, slot, func_val),
          else: advanced.feedback_vector

        resumed_frame = %{advanced | feedback_vector: new_fv}

        case func_val do
          {:function, callee_code, _closure_ctx} ->
            # Signal to the run loop to push a new call frame for the callee.
            # The run/4 loop handles the actual frame creation and argument setup.
            # `resumed_frame` has ip already advanced past the call instruction —
            # when the callee returns, execution continues from `resumed_frame`.
            _ = {first_arg_reg, argc}
            {:call, callee_code, resumed_frame, globals, output}

          {:builtin, :print} ->
            # Built-in print function: adds acc (or first arg) to output list.
            # This lets programs produce observable output without real stdout.
            arg = if argc > 0, do: elem(frame.registers, first_arg_reg), else: frame.accumulator
            str = to_string_val(arg)
            {:continue, %{resumed_frame | accumulator: nil}, globals, [str | output]}

          _ ->
            err = %VMError{
              message: "TypeError: #{inspect(func_val)} is not a function",
              instruction_index: frame.ip,
              opcode: opcode
            }
            {:error, err}
        end

      opcode == Opcodes.call_undefined_receiver() ->
        # Same as call_any_receiver but with undefined as the implicit "this"
        # (relevant for strict-mode functions in JavaScript)
        [func_reg, first_arg_reg, argc | rest] = instr.operands
        slot = List.first(rest)
        func_val = elem(frame.registers, func_reg)

        new_fv = if slot != nil,
          do: Feedback.record_call_site(advanced.feedback_vector, slot, func_val),
          else: advanced.feedback_vector

        resumed_frame = %{advanced | feedback_vector: new_fv}

        case func_val do
          {:function, callee_code, _closure_ctx} ->
            _ = {first_arg_reg, argc}
            {:call, callee_code, resumed_frame, globals, output}

          _ ->
            err = %VMError{
              message: "TypeError: #{inspect(func_val)} is not a function",
              instruction_index: frame.ip,
              opcode: opcode
            }
            {:error, err}
        end

      opcode == Opcodes.call_property() ->
        # Method call: operands = [func_reg, recv_reg, argc, feedback_slot]
        [func_reg, _recv_reg, _argc | rest] = instr.operands
        slot = List.first(rest)
        func_val = elem(frame.registers, func_reg)

        new_fv = if slot != nil,
          do: Feedback.record_call_site(advanced.feedback_vector, slot, func_val),
          else: advanced.feedback_vector

        resumed_frame = %{advanced | feedback_vector: new_fv}

        case func_val do
          {:function, callee_code, _closure_ctx} ->
            {:call, callee_code, resumed_frame, globals, output}

          _ ->
            err = %VMError{
              message: "TypeError: #{inspect(func_val)} is not a function",
              instruction_index: frame.ip,
              opcode: opcode
            }
            {:error, err}
        end

      opcode == Opcodes.return() ->
        # Return acc to the calling frame.
        # The run loop will place this value in the caller's accumulator.
        {:return, frame.accumulator, frame.caller_frame, output, frame.feedback_vector}

      opcode == Opcodes.construct() ->
        # Constructor call: creates a new object and calls the function.
        # For simplicity, we just call the function — the function is responsible
        # for returning a map as the newly constructed object.
        [func_reg, first_arg_reg, argc | rest] = instr.operands
        slot = List.first(rest)
        func_val = elem(frame.registers, func_reg)

        new_fv = if slot != nil,
          do: Feedback.record_call_site(advanced.feedback_vector, slot, func_val),
          else: advanced.feedback_vector

        _ = {first_arg_reg, argc}
        resumed_frame = %{advanced | feedback_vector: new_fv}

        case func_val do
          {:function, callee_code, _closure_ctx} ->
            {:call, callee_code, resumed_frame, globals, output}

          _ ->
            err = %VMError{
              message: "TypeError: #{inspect(func_val)} is not a constructor",
              instruction_index: frame.ip,
              opcode: opcode
            }
            {:error, err}
        end

      opcode == Opcodes.suspend_generator() ->
        # Generators are not fully implemented; for now, suspend returns the accumulator
        {:return, frame.accumulator, frame.caller_frame, output, frame.feedback_vector}

      opcode == Opcodes.resume_generator() ->
        # No-op in this interpreter
        {:continue, advanced, globals, output}

      opcode == Opcodes.call_with_spread() ->
        # Simplified: treat like call_any_receiver
        [func_reg | rest] = instr.operands
        slot = List.last(rest)
        func_val = elem(frame.registers, func_reg)

        new_fv = if slot != nil,
          do: Feedback.record_call_site(advanced.feedback_vector, slot, func_val),
          else: advanced.feedback_vector

        resumed_frame = %{advanced | feedback_vector: new_fv}

        case func_val do
          {:function, callee_code, _} ->
            {:call, callee_code, resumed_frame, globals, output}

          _ ->
            err = %VMError{
              message: "TypeError: not a function",
              instruction_index: frame.ip,
              opcode: opcode
            }
            {:error, err}
        end

      opcode == Opcodes.construct_with_spread() ->
        [func_reg | rest] = instr.operands
        slot = List.last(rest)
        func_val = elem(frame.registers, func_reg)

        new_fv = if slot != nil,
          do: Feedback.record_call_site(advanced.feedback_vector, slot, func_val),
          else: advanced.feedback_vector

        resumed_frame = %{advanced | feedback_vector: new_fv}

        case func_val do
          {:function, callee_code, _} ->
            {:call, callee_code, resumed_frame, globals, output}

          _ ->
            err = %VMError{
              message: "TypeError: not a constructor",
              instruction_index: frame.ip,
              opcode: opcode
            }
            {:error, err}
        end

      # ------------------------------------------------------------------
      # 0x7_ Property access
      # ------------------------------------------------------------------

      opcode == Opcodes.lda_named_property() ->
        # Load a property by name: acc = registers[obj_reg][names[name_idx]]
        # Also records the object's hidden class for polymorphism tracking.
        [obj_reg, name_idx | rest] = instr.operands
        slot = List.first(rest)
        obj = elem(frame.registers, obj_reg)
        name = Enum.at(frame.code.names, name_idx)

        new_fv = if slot != nil and is_map(obj),
          do: Feedback.record_property_load(advanced.feedback_vector, slot, obj),
          else: advanced.feedback_vector

        value = if is_map(obj), do: Map.get(obj, name), else: nil
        {:continue, %{advanced | accumulator: value, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.sta_named_property() ->
        # Store acc to an object property: registers[obj_reg][names[name_idx]] = acc
        [obj_reg, name_idx | rest] = instr.operands
        slot = List.first(rest)
        obj = elem(frame.registers, obj_reg)
        name = Enum.at(frame.code.names, name_idx)

        new_fv = if slot != nil and is_map(obj),
          do: Feedback.record_property_load(advanced.feedback_vector, slot, obj),
          else: advanced.feedback_vector

        new_obj = if is_map(obj), do: Map.put(obj, name, frame.accumulator), else: obj
        new_regs = put_elem(advanced.registers, obj_reg, new_obj)
        {:continue, %{advanced | registers: new_regs, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.lda_keyed_property() ->
        # Load a property by key (computed): key = acc; acc = registers[obj_reg][key]
        [obj_reg | rest] = instr.operands
        slot = List.first(rest)
        obj = elem(frame.registers, obj_reg)
        key = frame.accumulator

        new_fv = if slot != nil and is_map(obj),
          do: Feedback.record_property_load(advanced.feedback_vector, slot, obj),
          else: advanced.feedback_vector

        value = cond do
          is_map(obj) -> Map.get(obj, key)
          is_list(obj) and is_integer(key) -> Enum.at(obj, key)
          true -> nil
        end

        {:continue, %{advanced | accumulator: value, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.sta_keyed_property() ->
        # Store a keyed property: registers[obj_reg][registers[key_reg]] = acc
        [obj_reg, key_reg | rest] = instr.operands
        slot = List.first(rest)
        obj = elem(frame.registers, obj_reg)
        key = elem(frame.registers, key_reg)

        new_fv = if slot != nil and is_map(obj),
          do: Feedback.record_property_load(advanced.feedback_vector, slot, obj),
          else: advanced.feedback_vector

        new_obj = cond do
          is_map(obj) -> Map.put(obj, key, frame.accumulator)
          true -> obj
        end

        new_regs = put_elem(advanced.registers, obj_reg, new_obj)
        {:continue, %{advanced | registers: new_regs, feedback_vector: new_fv}, globals, output}

      opcode == Opcodes.lda_named_property_no_feedback() ->
        # Same as lda_named_property but no feedback recording (fast-path variant)
        [obj_reg, name_idx | _] = instr.operands
        obj = elem(frame.registers, obj_reg)
        name = Enum.at(frame.code.names, name_idx)
        value = if is_map(obj), do: Map.get(obj, name), else: nil
        {:continue, %{advanced | accumulator: value}, globals, output}

      opcode == Opcodes.sta_named_property_no_feedback() ->
        [obj_reg, name_idx | _] = instr.operands
        obj = elem(frame.registers, obj_reg)
        name = Enum.at(frame.code.names, name_idx)
        new_obj = if is_map(obj), do: Map.put(obj, name, frame.accumulator), else: obj
        new_regs = put_elem(advanced.registers, obj_reg, new_obj)
        {:continue, %{advanced | registers: new_regs}, globals, output}

      opcode == Opcodes.delete_property_strict() or opcode == Opcodes.delete_property_sloppy() ->
        # Delete a property from an object in a register.
        # acc = key to delete; operands = [obj_reg]
        [obj_reg | _] = instr.operands
        obj = elem(frame.registers, obj_reg)
        key = frame.accumulator
        new_obj = if is_map(obj), do: Map.delete(obj, key), else: obj
        new_regs = put_elem(advanced.registers, obj_reg, new_obj)
        {:continue, %{advanced | registers: new_regs, accumulator: true}, globals, output}

      # ------------------------------------------------------------------
      # 0x8_ Object and array creation
      # ------------------------------------------------------------------

      opcode == Opcodes.create_object_literal() ->
        # Create an empty map — the Elixir representation of a JS object.
        # The template_idx and flags operands are ignored in this interpreter;
        # a more sophisticated implementation would pre-populate fields.
        {:continue, %{advanced | accumulator: %{}}, globals, output}

      opcode == Opcodes.create_array_literal() ->
        # Create an empty list — Elixir's representation of a JS array.
        {:continue, %{advanced | accumulator: []}, globals, output}

      opcode == Opcodes.create_regexp_literal() ->
        # Create a regex placeholder (just store the pattern string).
        [pattern_idx | _] = instr.operands
        pattern = Enum.at(frame.code.constants, pattern_idx)
        {:continue, %{advanced | accumulator: {:regexp, pattern}}, globals, output}

      opcode == Opcodes.create_closure() ->
        # Wrap a CodeObject as a callable function value, capturing the current context.
        # Result: {:function, code_object, closure_context}
        # The closure_context links the new function to any variables it captures
        # from the enclosing scope.
        [code_idx | _] = instr.operands
        callee_code = Enum.at(frame.code.constants, code_idx)
        closure = {:function, callee_code, frame.context}
        {:continue, %{advanced | accumulator: closure}, globals, output}

      opcode == Opcodes.create_context() ->
        # Push a new block-scope context frame.
        [slot_count | _] = instr.operands
        new_ctx = Scope.new_context(frame.context, slot_count)
        {:continue, %{advanced | context: new_ctx}, globals, output}

      opcode == Opcodes.clone_object() ->
        # Shallow clone an object: registers[src_reg] into acc.
        [src_reg | _] = instr.operands
        obj = elem(frame.registers, src_reg)
        clone = if is_map(obj), do: Map.merge(%{}, obj), else: obj
        {:continue, %{advanced | accumulator: clone}, globals, output}

      # ------------------------------------------------------------------
      # 0x9_ Iteration
      # ------------------------------------------------------------------

      opcode == Opcodes.get_iterator() ->
        # Convert the accumulator into an iterator.
        # For a list: {:iterator, list, 0}  (index-based)
        # For a map: {:iterator, map_list, 0}  (key-value pairs)
        iter = case frame.accumulator do
          list when is_list(list) -> {:iterator, list, false}
          map when is_map(map) -> {:iterator, Map.to_list(map), false}
          _ -> {:iterator, [], false}
        end
        {:continue, %{advanced | accumulator: iter}, globals, output}

      opcode == Opcodes.call_iterator_step() ->
        # Advance the iterator. Updates the iterator in the accumulator.
        # Returns {:iterator, remaining, done_flag}
        case frame.accumulator do
          {:iterator, [], _} ->
            {:continue, %{advanced | accumulator: {:iterator, [], true}}, globals, output}

          {:iterator, [_ | rest], _} ->
            {:continue, %{advanced | accumulator: {:iterator, rest, false}}, globals, output}

          _ ->
            {:continue, %{advanced | accumulator: {:iterator, [], true}}, globals, output}
        end

      opcode == Opcodes.get_iterator_done() ->
        # Test if iteration is complete.
        result = case frame.accumulator do
          {:iterator, _, done} -> done
          _ -> true
        end
        {:continue, %{advanced | accumulator: result}, globals, output}

      opcode == Opcodes.get_iterator_value() ->
        # Get the current value from the iterator.
        result = case frame.accumulator do
          {:iterator, [head | _], false} -> head
          _ -> nil
        end
        {:continue, %{advanced | accumulator: result}, globals, output}

      # ------------------------------------------------------------------
      # 0xA_ Exceptions
      # ------------------------------------------------------------------

      opcode == Opcodes.throw() ->
        # Throw the accumulator value as a runtime error.
        err = %VMError{
          message: "Thrown: #{inspect(frame.accumulator)}",
          instruction_index: frame.ip,
          opcode: opcode
        }
        {:error, err}

      opcode == Opcodes.rethrow() ->
        # Re-throw: same as throw but semantics differ for catch blocks.
        # Here we just produce the same error.
        err = %VMError{
          message: "Rethrown: #{inspect(frame.accumulator)}",
          instruction_index: frame.ip,
          opcode: opcode
        }
        {:error, err}

      # ------------------------------------------------------------------
      # 0xB_ Context / scope management
      # ------------------------------------------------------------------

      opcode == Opcodes.push_context() ->
        # Push a new scope context (alias for create_context with slot count from operand)
        [slot_count | _] = instr.operands
        new_ctx = Scope.new_context(frame.context, slot_count)
        {:continue, %{advanced | context: new_ctx}, globals, output}

      opcode == Opcodes.pop_context() ->
        # Pop the current scope context (restore the parent).
        parent_ctx = if frame.context, do: frame.context.parent, else: nil
        {:continue, %{advanced | context: parent_ctx}, globals, output}

      opcode == Opcodes.lda_module_variable() ->
        # Load a module-level variable by index (stored in globals under a synthetic name)
        [idx | _] = instr.operands
        name = "__module_var_#{idx}__"

        case Scope.get_global(globals, name) do
          {:ok, value} -> {:continue, %{advanced | accumulator: value}, globals, output}
          :error -> {:continue, %{advanced | accumulator: :undefined}, globals, output}
        end

      opcode == Opcodes.sta_module_variable() ->
        [idx | _] = instr.operands
        name = "__module_var_#{idx}__"
        new_globals = Scope.set_global(globals, name, frame.accumulator)
        {:continue, advanced, new_globals, output}

      # ------------------------------------------------------------------
      # 0xF_ VM control
      # ------------------------------------------------------------------

      opcode == Opcodes.stack_check() ->
        # Check if the call stack is too deep. This is called at function entry
        # to catch runaway recursion before it exhausts real process memory.
        if depth >= @max_call_depth do
          err = %VMError{
            message: "RangeError: Maximum call stack size exceeded",
            instruction_index: frame.ip,
            opcode: opcode
          }
          {:error, err}
        else
          {:continue, advanced, globals, output}
        end

      opcode == Opcodes.debugger() ->
        # Debugger breakpoint — no-op in the interpreter.
        # A real debugger would pause execution here, allowing inspection of
        # registers, the call stack, and the feedback vector.
        {:continue, advanced, globals, output}

      opcode == Opcodes.halt() ->
        # Unconditional halt: stop execution and return the current accumulator.
        # Used as the final instruction in top-level scripts.
        {:return, frame.accumulator, frame.caller_frame, output, frame.feedback_vector}

      # ------------------------------------------------------------------
      # Unknown opcode
      # ------------------------------------------------------------------
      true ->
        err = %VMError{
          message: "Unknown opcode: 0x#{Integer.to_string(opcode, 16)}",
          instruction_index: frame.ip,
          opcode: opcode
        }
        {:error, err}
    end
  end

  # ---------------------------------------------------------------------------
  # Helper: add_values
  # ---------------------------------------------------------------------------
  # JavaScript's + operator does either numeric addition or string concatenation
  # depending on the types of the operands. If either is a string, both are
  # coerced to strings and concatenated.
  defp add_values(left, right) when is_binary(left) or is_binary(right) do
    to_string_val(left) <> to_string_val(right)
  end

  defp add_values(left, right), do: left + right

  # ---------------------------------------------------------------------------
  # Helper: truthy?
  # ---------------------------------------------------------------------------
  # JavaScript's definition of "truthy": everything EXCEPT these falsy values:
  #   false, null (nil), undefined (:undefined), 0, "" (empty string), [] (empty list)
  #
  # Note: In JavaScript, 0.0 is also falsy. Empty lists are truthy in JS
  # (arrays are always truthy), but we use [] as our array representation
  # and choose to make empty lists truthy for consistency with standard
  # Elixir/Ruby conventions. This is a deliberate simplification.
  defp truthy?(false), do: false
  defp truthy?(nil), do: false
  defp truthy?(:undefined), do: false
  defp truthy?(0), do: false
  defp truthy?(""), do: false
  defp truthy?(f) when is_float(f) and f == 0.0, do: false
  defp truthy?(_), do: true

  # ---------------------------------------------------------------------------
  # Helper: abstract_equal?
  # ---------------------------------------------------------------------------
  # JavaScript's == (abstract equality) performs type coercion.
  # Key rules:
  #   - null == undefined is true
  #   - number == string: parse string to number, compare
  #   - everything else: use strict equality
  defp abstract_equal?(nil, :undefined), do: true
  defp abstract_equal?(:undefined, nil), do: true
  defp abstract_equal?(nil, nil), do: true
  defp abstract_equal?(:undefined, :undefined), do: true

  defp abstract_equal?(num, str) when is_number(num) and is_binary(str) do
    case Float.parse(str) do
      {f, ""} -> num == f
      _ ->
        case Integer.parse(str) do
          {i, ""} -> num == i
          _ -> false
        end
    end
  end

  defp abstract_equal?(str, num) when is_binary(str) and is_number(num) do
    abstract_equal?(num, str)
  end

  defp abstract_equal?(a, b), do: a === b

  # ---------------------------------------------------------------------------
  # Helper: type_string
  # ---------------------------------------------------------------------------
  # Returns the JavaScript typeof string for a value.
  defp type_string(value) do
    cond do
      is_integer(value) or is_float(value) -> "number"
      is_binary(value) -> "string"
      is_boolean(value) -> "boolean"
      is_nil(value) -> "object"  # famous JS quirk: typeof null === "object"
      value == :undefined -> "undefined"
      is_map(value) -> "object"
      is_list(value) -> "object"
      is_tuple(value) and tuple_size(value) == 3 and elem(value, 0) == :function -> "function"
      is_tuple(value) and tuple_size(value) == 2 and elem(value, 0) == :builtin -> "function"
      true -> "unknown"
    end
  end

  # ---------------------------------------------------------------------------
  # Helper: to_string_val
  # ---------------------------------------------------------------------------
  # Converts any VM value to its string representation (for concatenation/print).
  defp to_string_val(value) do
    cond do
      is_binary(value) -> value
      is_integer(value) -> Integer.to_string(value)
      is_float(value) -> Float.to_string(value)
      is_boolean(value) -> if value, do: "true", else: "false"
      is_nil(value) -> "null"
      value == :undefined -> "undefined"
      is_map(value) -> "[object Object]"
      is_list(value) -> "[object Array]"
      true -> inspect(value)
    end
  end
end
