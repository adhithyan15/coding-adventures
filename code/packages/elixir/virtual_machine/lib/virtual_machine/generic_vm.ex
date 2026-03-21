defmodule CodingAdventures.VirtualMachine.GenericVM do
  @moduledoc """
  A pluggable stack-based bytecode interpreter for Elixir.

  ## What is a GenericVM?

  The GenericVM is a bytecode interpreter that knows HOW to execute instructions
  but does not know WHICH instructions exist. You "teach" it new instructions by
  registering handler functions for each opcode. This makes it reusable across
  completely different instruction sets — you can build a Python-like VM, a
  Brainfuck interpreter, or a custom language VM all using the same core.

  ## Immutable Functional Design

  Unlike imperative VMs (Python, Java) that mutate state, the Elixir GenericVM
  passes immutable state through handler functions. Each handler receives the
  current VM state and returns a tuple:

      {output_or_nil, updated_vm}

  - The first element is an optional string output (for PRINT-like instructions).
    If the instruction does not produce output, this is `nil`.
  - The second element is the new VM state with any changes applied (stack
    modified, variables updated, program counter advanced, etc.)

  This design means:
  - **No side effects**: every handler is a pure function of its inputs
  - **Time travel**: keep references to old states for debugging
  - **Easy testing**: feed in a state, check what comes out
  - **Concurrency-safe**: no shared mutable state

  ## Architecture

      ┌─────────────────────────────────────────────────────────┐
      │                      GenericVM                          │
      │                                                         │
      │  ┌─────────┐  ┌──────────┐  ┌───────────┐             │
      │  │  Stack   │  │Variables │  │Call Stack  │             │
      │  │ [7,3,1] │  │ %{x: 5}  │  │ [frame1]  │             │
      │  └─────────┘  └──────────┘  └───────────┘             │
      │                                                         │
      │  ┌─────────────────────────────────────────┐           │
      │  │          Handler Registry                │           │
      │  │  0x01 → handle_push                      │           │
      │  │  0x02 → handle_add                       │           │
      │  │  0x03 → handle_print                     │           │
      │  │  0xFF → handle_halt                      │           │
      │  └─────────────────────────────────────────┘           │
      │                                                         │
      │  PC: 3    Halted: false    Output: ["7"]               │
      └─────────────────────────────────────────────────────────┘

  ## Usage Example

      # 1. Create a VM
      vm = GenericVM.new()

      # 2. Register handlers for your instruction set
      vm = GenericVM.register_opcode(vm, 0x01, fn vm, instr, code ->
        value = Enum.at(code.constants, instr.operand)
        vm = GenericVM.push(vm, value)
        vm = GenericVM.advance_pc(vm)
        {nil, vm}
      end)

      # 3. Build a program (CodeObject)
      code = %CodeObject{
        instructions: [%Instruction{opcode: 0x01, operand: 0}],
        constants: [42]
      }

      # 4. Execute
      {traces, final_vm} = GenericVM.execute(vm, code)

  ## Extra State

  Some VMs need language-specific state beyond the standard stack/variables.
  For example, a Brainfuck interpreter needs a tape and a data pointer.
  Rather than adding Brainfuck-specific fields to the struct, use the
  `extra` map:

      vm = GenericVM.put_extra(vm, :tape, List.duplicate(0, 30000))
      vm = GenericVM.put_extra(vm, :data_pointer, 0)
      tape = GenericVM.get_extra(vm, :tape)
  """

  alias CodingAdventures.VirtualMachine.Types.{Instruction, CodeObject, VMTrace, CallFrame, BuiltinFunction}
  alias CodingAdventures.VirtualMachine.Errors

  # ---------------------------------------------------------------------------
  # Struct Definition
  # ---------------------------------------------------------------------------

  @typedoc """
  The complete state of a GenericVM at any point in time.

  ## Fields

  - `stack` — the operand stack (head of list = top of stack)
  - `variables` — named variable bindings (string keys → any values)
  - `locals` — ordered list of local variables (for frame-based access)
  - `pc` — program counter (index of next instruction to execute)
  - `halted` — whether the VM has stopped execution
  - `output` — accumulated output strings (newest first for O(1) prepend)
  - `call_stack` — saved frames for function call/return
  - `handlers` — opcode (integer) → handler function registry
  - `builtins` — name (string) → BuiltinFunction registry
  - `max_recursion_depth` — optional limit on call stack depth
  - `frozen` — if true, the VM is in a read-only state
  - `extra` — open map for language-specific extensions
  """
  defstruct stack: [],
            variables: %{},
            locals: [],
            pc: 0,
            halted: false,
            output: [],
            call_stack: [],
            handlers: %{},
            builtins: %{},
            max_recursion_depth: nil,
            frozen: false,
            extra: %{}

  @type handler :: (t(), Instruction.t(), CodeObject.t() -> {String.t() | nil, t()})

  @type t :: %__MODULE__{
          stack: [any()],
          variables: map(),
          locals: [any()],
          pc: non_neg_integer(),
          halted: boolean(),
          output: [String.t()],
          call_stack: [CallFrame.t()],
          handlers: %{integer() => handler()},
          builtins: %{String.t() => BuiltinFunction.t()},
          max_recursion_depth: non_neg_integer() | nil,
          frozen: boolean(),
          extra: map()
        }

  # ===========================================================================
  # Construction
  # ===========================================================================

  @doc """
  Create a new, empty GenericVM.

  The VM starts with an empty stack, no variables, program counter at 0,
  and no registered handlers. You must register opcode handlers before
  executing any program.

  ## Example

      vm = GenericVM.new()
      vm.stack   #=> []
      vm.pc      #=> 0
      vm.halted  #=> false
  """
  @spec new() :: t()
  def new, do: %__MODULE__{}

  # ===========================================================================
  # Plugin Registration
  # ===========================================================================

  @doc """
  Register a handler function for a specific opcode.

  ## What is a handler?

  A handler is a function that implements one instruction. It receives three
  arguments:

  1. `vm` — the current VM state
  2. `instruction` — the `%Instruction{}` being executed
  3. `code` — the `%CodeObject{}` being run (for accessing constants/names)

  It must return `{output_or_nil, updated_vm}`.

  ## Example

      vm = GenericVM.register_opcode(vm, 0x01, fn vm, instr, code ->
        value = Enum.at(code.constants, instr.operand)
        vm = GenericVM.push(vm, value)
        vm = GenericVM.advance_pc(vm)
        {nil, vm}
      end)

  ## Overwriting

  Registering an opcode that already has a handler replaces the old handler.
  This is intentional — it allows language-specific VMs to override default
  behavior.
  """
  @spec register_opcode(t(), integer(), handler()) :: t()
  def register_opcode(%__MODULE__{} = vm, opcode, handler)
      when is_integer(opcode) and is_function(handler, 3) do
    %{vm | handlers: Map.put(vm.handlers, opcode, handler)}
  end

  @doc """
  Register a built-in function by name.

  Built-in functions are host-language (Elixir) functions that VM programs
  can call by name. They are stored in the builtins registry and looked up
  when the VM encounters a CALL_BUILTIN-style instruction.

  ## Example

      vm = GenericVM.register_builtin(vm, "print", fn args, vm ->
        IO.puts(inspect(args))
        {nil, vm}
      end)
  """
  @spec register_builtin(t(), String.t(), function()) :: t()
  def register_builtin(%__MODULE__{} = vm, name, implementation)
      when is_binary(name) do
    builtin = %BuiltinFunction{name: name, implementation: implementation}
    %{vm | builtins: Map.put(vm.builtins, name, builtin)}
  end

  @doc """
  Look up a built-in function by name.

  Returns the `%BuiltinFunction{}` struct if found, or `nil` if no builtin
  with that name is registered.

  ## Example

      builtin = GenericVM.get_builtin(vm, "print")
      if builtin, do: builtin.implementation.(args, vm)
  """
  @spec get_builtin(t(), String.t()) :: BuiltinFunction.t() | nil
  def get_builtin(%__MODULE__{} = vm, name) do
    Map.get(vm.builtins, name)
  end

  # ===========================================================================
  # Configuration
  # ===========================================================================

  @doc """
  Set the maximum recursion depth (call stack limit).

  When the call stack reaches this depth, `push_frame/2` will raise
  `MaxRecursionError`. Set to `nil` for no limit (dangerous for untrusted code).

  ## Example

      vm = GenericVM.set_max_recursion_depth(vm, 100)
  """
  @spec set_max_recursion_depth(t(), non_neg_integer() | nil) :: t()
  def set_max_recursion_depth(%__MODULE__{} = vm, depth) do
    %{vm | max_recursion_depth: depth}
  end

  @doc """
  Set the frozen flag.

  A frozen VM is in a read-only state. This can be used by language-specific
  VMs to prevent further modifications after certain operations.

  ## Example

      vm = GenericVM.set_frozen(vm, true)
      vm.frozen  #=> true
  """
  @spec set_frozen(t(), boolean()) :: t()
  def set_frozen(%__MODULE__{} = vm, frozen) when is_boolean(frozen) do
    %{vm | frozen: frozen}
  end

  # ===========================================================================
  # Stack Operations
  # ===========================================================================

  @doc """
  Push a value onto the stack.

  ## How the Stack Works

  The VM stack is a Last-In-First-Out (LIFO) data structure. Values are
  pushed onto the top and popped from the top. We implement it as an
  Elixir list where the HEAD is the top of the stack. This gives us
  O(1) push and pop operations.

      Before push(42):  stack = [3, 1]       (3 is on top)
      After push(42):   stack = [42, 3, 1]   (42 is now on top)

  ## Example

      vm = GenericVM.push(vm, 42)
      GenericVM.peek(vm)  #=> 42
  """
  @spec push(t(), any()) :: t()
  def push(%__MODULE__{} = vm, value) do
    %{vm | stack: [value | vm.stack]}
  end

  @doc """
  Pop the top value from the stack.

  Returns `{value, updated_vm}` where the value is the former top of
  the stack and the VM has the value removed.

  Raises `StackUnderflowError` if the stack is empty.

  ## Example

      vm = GenericVM.push(GenericVM.new(), 42)
      {value, vm} = GenericVM.pop(vm)
      value  #=> 42
  """
  @spec pop(t()) :: {any(), t()}
  def pop(%__MODULE__{stack: []} = _vm) do
    raise Errors.StackUnderflowError, "Cannot pop from an empty stack."
  end

  def pop(%__MODULE__{stack: [top | rest]} = vm) do
    {top, %{vm | stack: rest}}
  end

  @doc """
  Peek at the top value without removing it.

  Raises `StackUnderflowError` if the stack is empty.

  ## Example

      vm = GenericVM.push(GenericVM.new(), 42)
      GenericVM.peek(vm)  #=> 42
      # Stack is unchanged — 42 is still there
  """
  @spec peek(t()) :: any()
  def peek(%__MODULE__{stack: []}) do
    raise Errors.StackUnderflowError, "Cannot peek at an empty stack."
  end

  def peek(%__MODULE__{stack: [top | _]}) do
    top
  end

  # ===========================================================================
  # Call Stack
  # ===========================================================================

  @doc """
  Push a call frame onto the call stack.

  This is called when the VM enters a function. The frame saves the
  return address and the caller's variable bindings so they can be
  restored when the function returns.

  If a max recursion depth is set and the call stack is already at
  that depth, raises `MaxRecursionError`.

  ## Example

      frame = %CallFrame{return_address: 5, saved_variables: vm.variables, saved_locals: vm.locals}
      vm = GenericVM.push_frame(vm, frame)
  """
  @spec push_frame(t(), CallFrame.t()) :: t()
  def push_frame(%__MODULE__{} = vm, %CallFrame{} = frame) do
    if vm.max_recursion_depth != nil and length(vm.call_stack) >= vm.max_recursion_depth do
      raise Errors.MaxRecursionError,
            "Maximum recursion depth exceeded (limit: #{vm.max_recursion_depth})"
    end

    %{vm | call_stack: [frame | vm.call_stack]}
  end

  @doc """
  Pop a call frame from the call stack.

  This is called when the VM returns from a function. Returns
  `{frame, updated_vm}` where the frame contains the saved return
  address and variable bindings.

  Raises `VMError` if the call stack is empty (returning without
  a matching call).

  ## Example

      {frame, vm} = GenericVM.pop_frame(vm)
      vm = GenericVM.jump_to(vm, frame.return_address)
  """
  @spec pop_frame(t()) :: {CallFrame.t(), t()}
  def pop_frame(%__MODULE__{call_stack: []}) do
    raise Errors.VMError, "Cannot return — call stack is empty"
  end

  def pop_frame(%__MODULE__{call_stack: [frame | rest]} = vm) do
    {frame, %{vm | call_stack: rest}}
  end

  # ===========================================================================
  # Program Counter
  # ===========================================================================

  @doc """
  Advance the program counter by one.

  Most instructions call this at the end of their handler to move to the
  next instruction. Jump instructions use `jump_to/2` instead.

  ## Example

      vm = GenericVM.advance_pc(vm)
      # vm.pc is now one higher than before
  """
  @spec advance_pc(t()) :: t()
  def advance_pc(%__MODULE__{} = vm) do
    %{vm | pc: vm.pc + 1}
  end

  @doc """
  Set the program counter to a specific address.

  Used by jump instructions (unconditional jump, conditional jump,
  function call, etc.) to move execution to a different point in
  the program.

  ## Example

      vm = GenericVM.jump_to(vm, 0)   # Jump back to the beginning
  """
  @spec jump_to(t(), non_neg_integer()) :: t()
  def jump_to(%__MODULE__{} = vm, target) when is_integer(target) do
    %{vm | pc: target}
  end

  # ===========================================================================
  # Execution — Running Programs
  # ===========================================================================

  @doc """
  Execute a complete program from the current PC until halted or out of instructions.

  Returns `{traces, final_vm}` where `traces` is a list of `%VMTrace{}` structs
  (one per instruction executed) and `final_vm` is the VM state after the last
  instruction.

  ## How Execution Works

  The VM runs a fetch-decode-execute loop:

  1. **Fetch**: read the instruction at `vm.pc`
  2. **Decode**: look up the handler for that instruction's opcode
  3. **Execute**: call the handler, which returns `{output, new_vm}`
  4. **Trace**: record a `VMTrace` snapshot of the step
  5. **Repeat**: go back to step 1 with the new VM state

  Execution stops when:
  - The VM is halted (`vm.halted == true`)
  - The program counter goes past the end of the instruction list

  ## Example

      {traces, vm} = GenericVM.execute(vm, code)
      Enum.each(traces, fn t ->
        IO.puts("PC=\#{t.pc}: \#{t.description}")
      end)
  """
  @spec execute(t(), CodeObject.t()) :: {[VMTrace.t()], t()}
  def execute(%__MODULE__{} = vm, %CodeObject{} = code) do
    do_execute(vm, code, [])
  end

  # --- Private recursive execution loop ---
  #
  # We use tail recursion with an accumulator for the traces list.
  # Traces are accumulated in reverse (newest first) and reversed
  # at the end for correct chronological order.

  defp do_execute(%__MODULE__{halted: true} = vm, _code, traces) do
    {Enum.reverse(traces), vm}
  end

  defp do_execute(%__MODULE__{pc: pc} = vm, %CodeObject{instructions: instrs} = _code, traces)
       when pc >= length(instrs) do
    {Enum.reverse(traces), vm}
  end

  defp do_execute(%__MODULE__{} = vm, %CodeObject{} = code, traces) do
    {trace, vm} = step(vm, code)
    do_execute(vm, code, [trace | traces])
  end

  @doc """
  Execute a single instruction and return the trace.

  Returns `{trace, updated_vm}`. This is useful for debugger-style
  step-by-step execution where you want to inspect the VM state
  between each instruction.

  ## How a Single Step Works

  1. Read the instruction at `vm.pc`
  2. Snapshot the stack (before state)
  3. Look up and call the handler
  4. Snapshot the stack again (after state)
  5. Build a `VMTrace` record
  6. Return `{trace, new_vm}`

  Raises `InvalidOpcodeError` if no handler is registered for the
  instruction's opcode.

  ## Example

      {trace, vm} = GenericVM.step(vm, code)
      IO.inspect(trace.stack_before)
      IO.inspect(trace.stack_after)
  """
  @spec step(t(), CodeObject.t()) :: {VMTrace.t(), t()}
  def step(%__MODULE__{} = vm, %CodeObject{} = code) do
    instruction = Enum.at(code.instructions, vm.pc)
    pc_before = vm.pc

    # Snapshot the stack BEFORE execution.
    # We store the stack reversed (head = top) for O(1) push/pop,
    # but traces show it in natural order (bottom → top) for readability.
    stack_before = Enum.reverse(vm.stack)

    handler = Map.get(vm.handlers, instruction.opcode)

    if handler == nil do
      raise Errors.InvalidOpcodeError,
            "Unknown opcode: #{inspect(instruction.opcode)}. No handler registered."
    end

    {output_value, vm} = handler.(vm, instruction, code)

    # Build the trace record capturing what happened
    trace = %VMTrace{
      pc: pc_before,
      instruction: instruction,
      stack_before: stack_before,
      stack_after: Enum.reverse(vm.stack),
      variables: vm.variables,
      output: output_value,
      description: describe_step(instruction)
    }

    {trace, vm}
  end

  # ===========================================================================
  # Reset
  # ===========================================================================

  @doc """
  Reset the VM's runtime state while preserving registered handlers and builtins.

  This clears the stack, variables, locals, program counter, output, call stack,
  halted flag, frozen flag, and extra state. The handler registry and builtin
  registry are preserved, so the VM is ready to run a new program with the
  same instruction set.

  ## When to use this

  After executing one program, reset the VM before running another. This is
  more efficient than creating a new VM and re-registering all handlers.

  ## Example

      {_traces, vm} = GenericVM.execute(vm, program1)
      vm = GenericVM.reset(vm)
      {_traces, vm} = GenericVM.execute(vm, program2)
  """
  @spec reset(t()) :: t()
  def reset(%__MODULE__{} = vm) do
    %{vm |
      stack: [],
      variables: %{},
      locals: [],
      pc: 0,
      halted: false,
      output: [],
      call_stack: [],
      frozen: false,
      extra: %{}
    }
  end

  # ===========================================================================
  # Extra State — Language-Specific Extensions
  # ===========================================================================

  @doc """
  Store a value in the extra state map.

  The extra map is an open-ended storage area for language-specific state
  that does not fit into the standard VM fields. For example:

  - A Brainfuck VM uses `extra` for the tape and data pointer
  - A Forth VM might use it for a return stack
  - A regex VM might use it for capture groups

  ## Example

      vm = GenericVM.put_extra(vm, :tape, List.duplicate(0, 30000))
      vm = GenericVM.put_extra(vm, :data_pointer, 0)
  """
  @spec put_extra(t(), any(), any()) :: t()
  def put_extra(%__MODULE__{} = vm, key, value) do
    %{vm | extra: Map.put(vm.extra, key, value)}
  end

  @doc """
  Retrieve a value from the extra state map.

  Returns the value if found, or `default` (defaults to `nil`) if the
  key is not present.

  ## Example

      tape = GenericVM.get_extra(vm, :tape)
      pointer = GenericVM.get_extra(vm, :data_pointer, 0)
  """
  @spec get_extra(t(), any(), any()) :: any()
  def get_extra(%__MODULE__{} = vm, key, default \\ nil) do
    Map.get(vm.extra, key, default)
  end

  # ===========================================================================
  # Internal Helpers
  # ===========================================================================

  # Generate a human-readable description of an instruction step.
  # This appears in VMTrace records to help users understand what happened.

  defp describe_step(%Instruction{opcode: op, operand: nil}) do
    hex = op |> Integer.to_string(16) |> String.upcase() |> String.pad_leading(2, "0")
    "Execute 0x#{hex}"
  end

  defp describe_step(%Instruction{opcode: op, operand: operand}) do
    hex = op |> Integer.to_string(16) |> String.upcase() |> String.pad_leading(2, "0")
    "Execute 0x#{hex} with operand #{inspect(operand)}"
  end
end
