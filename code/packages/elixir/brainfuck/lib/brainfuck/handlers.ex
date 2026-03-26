defmodule CodingAdventures.Brainfuck.Handlers do
  @moduledoc """
  Brainfuck Opcode Handlers — Teaching the GenericVM a New Language.

  ## How Handlers Plug Into the GenericVM

  The GenericVM is a blank slate — it knows how to fetch-decode-execute
  instructions, but it does not know what any opcode *means*. That is where
  handlers come in.

  Each handler is a function with the signature:

      fn(vm, instruction, code) -> {output_or_nil, updated_vm}

  The handler receives:

  - **vm** — The GenericVM struct. We use `GenericVM.get_extra/2` and
    `GenericVM.put_extra/3` to access Brainfuck-specific state, and
    `GenericVM.advance_pc/1` / `GenericVM.jump_to/2` to control flow.
  - **instruction** — The current `%Instruction{}` being executed.
  - **code** — The `%CodeObject{}` (unused by most Brainfuck handlers,
    since Brainfuck has no constant or name pools).

  The handler returns `{output, updated_vm}` where output is a string
  if the instruction produces output (the `.` command), otherwise nil.

  ## Brainfuck's Extra State

  The GenericVM provides a stack, variables, and call frames — none of which
  Brainfuck uses. Instead, Brainfuck needs:

  - **:tape** — A list of 30,000 byte cells, initialized to 0.
  - **:dp** (data pointer) — Index into the tape, starts at 0.
  - **:input_buffer** — Input string to read from (simulates stdin).
  - **:input_pos** — Current read position in the input buffer.

  These are stored in `vm.extra` via `GenericVM.put_extra/3` and retrieved
  via `GenericVM.get_extra/2`. This is Elixir's equivalent of Python's
  dynamic attribute approach — but explicit and type-safe.

  ## Cell Wrapping

  Brainfuck cells are unsigned bytes: values 0-255. Incrementing 255
  wraps to 0; decrementing 0 wraps to 255. This is modular arithmetic:

      cell = rem(cell + 1, 256)           # INC
      cell = rem(cell - 1 + 256, 256)     # DEC

  Elixir's `rem/2` can return negative values for negative dividends
  (unlike Python's `%`), so we add 256 before taking the remainder
  for the DEC case to ensure correct wrapping.
  """

  alias CodingAdventures.VirtualMachine.GenericVM
  alias CodingAdventures.Brainfuck.Opcodes

  # =========================================================================
  # Constants
  # =========================================================================

  @tape_size 30_000
  @doc "The number of cells on the Brainfuck tape (#{@tape_size})."
  def tape_size, do: @tape_size

  # =========================================================================
  # Error type
  # =========================================================================

  defmodule BrainfuckError do
    @moduledoc """
    Runtime error during Brainfuck execution.

    Raised when the data pointer moves out of bounds (before index 0
    or past the end of the tape). This is a runtime error, not a
    translation error — the bytecode is valid, but the program's
    behavior is invalid.
    """
    defexception [:message]
  end

  # =========================================================================
  # Private helpers for tape access
  # =========================================================================
  # These helpers centralize tape/pointer access so each handler does not
  # need to repeat the GenericVM.get_extra/put_extra calls.

  defp get_tape(vm), do: GenericVM.get_extra(vm, :tape)
  defp get_dp(vm), do: GenericVM.get_extra(vm, :dp)
  defp set_dp(vm, dp), do: GenericVM.put_extra(vm, :dp, dp)

  defp set_tape_at(vm, index, value) do
    tape = get_tape(vm)
    tape = List.replace_at(tape, index, value)
    GenericVM.put_extra(vm, :tape, tape)
  end

  # =========================================================================
  # Pointer movement handlers
  # =========================================================================

  @doc """
  `>` — Move the data pointer one cell to the right.

  If the pointer is already at the last cell (index 29,999), this raises
  a `BrainfuckError`. Some Brainfuck implementations wrap around; we
  choose to error because silent wrapping hides bugs in BF programs.

  ## State Changes

  - `dp` incremented by 1
  - `pc` advanced by 1
  """
  def handle_right(vm, _instruction, _code) do
    dp = get_dp(vm) + 1

    if dp >= @tape_size do
      raise BrainfuckError, "Data pointer moved past end of tape (position #{dp})."
    end

    vm = set_dp(vm, dp) |> GenericVM.advance_pc()
    {nil, vm}
  end

  @doc """
  `<` — Move the data pointer one cell to the left.

  If the pointer is already at cell 0, this raises a `BrainfuckError`.

  ## State Changes

  - `dp` decremented by 1
  - `pc` advanced by 1
  """
  def handle_left(vm, _instruction, _code) do
    dp = get_dp(vm) - 1

    if dp < 0 do
      raise BrainfuckError, "Data pointer moved before start of tape (position -1)."
    end

    vm = set_dp(vm, dp) |> GenericVM.advance_pc()
    {nil, vm}
  end

  # =========================================================================
  # Cell modification handlers
  # =========================================================================

  @doc """
  `+` — Increment the byte at the data pointer.

  Wraps from 255 to 0 (unsigned byte arithmetic). This is the fundamental
  way to set cell values in Brainfuck — there is no "load immediate"
  instruction, so to set a cell to 65 ('A'), you need 65 `+` commands
  (or a clever loop).

  ## State Changes

  - `tape[dp]` incremented by 1 (mod 256)
  - `pc` advanced by 1
  """
  def handle_inc(vm, _instruction, _code) do
    dp = get_dp(vm)
    tape = get_tape(vm)
    value = rem(Enum.at(tape, dp) + 1, 256)
    vm = set_tape_at(vm, dp, value) |> GenericVM.advance_pc()
    {nil, vm}
  end

  @doc """
  `-` — Decrement the byte at the data pointer.

  Wraps from 0 to 255 (unsigned byte arithmetic). Combined with `+`,
  this provides full control over cell values.

  ## State Changes

  - `tape[dp]` decremented by 1 (mod 256)
  - `pc` advanced by 1
  """
  def handle_dec(vm, _instruction, _code) do
    dp = get_dp(vm)
    tape = get_tape(vm)
    # Add 256 before rem to handle 0-1 correctly: rem(-1+256, 256) = 255
    value = rem(Enum.at(tape, dp) - 1 + 256, 256)
    vm = set_tape_at(vm, dp, value) |> GenericVM.advance_pc()
    {nil, vm}
  end

  # =========================================================================
  # I/O handlers
  # =========================================================================

  @doc """
  `.` — Output the current cell's value as an ASCII character.

  The cell value (0-255) is converted to a single-byte binary string
  and appended to `vm.output`. The character is also returned as the
  handler's output (captured in the VMTrace).

  ## State Changes

  - Character appended to `vm.output`
  - `pc` advanced by 1
  """
  def handle_output(vm, _instruction, _code) do
    dp = get_dp(vm)
    tape = get_tape(vm)
    char = <<Enum.at(tape, dp)::utf8>>
    vm = %{vm | output: vm.output ++ [char]} |> GenericVM.advance_pc()
    {char, vm}
  end

  @doc """
  `,` — Read one byte of input into the current cell.

  Reads from the input buffer at the current input position.
  If the input is exhausted (EOF), the cell is set to 0.

  Different Brainfuck implementations handle EOF differently:
  - Set cell to 0 (our choice — clean and predictable)
  - Set cell to 255 (i.e., -1 in unsigned)
  - Leave cell unchanged

  We chose 0 because it makes "read until EOF" loops simple:
  `,[.,]` reads and echoes until input runs out, then the `,`
  sets the cell to 0 and the `[` skips the loop body.

  ## State Changes

  - `tape[dp]` set to input byte (or 0 on EOF)
  - `input_pos` incremented by 1 (if not EOF)
  - `pc` advanced by 1
  """
  def handle_input(vm, _instruction, _code) do
    dp = get_dp(vm)
    input_buffer = GenericVM.get_extra(vm, :input_buffer)
    input_pos = GenericVM.get_extra(vm, :input_pos)

    {value, new_pos} =
      if input_pos < byte_size(input_buffer) do
        <<_::binary-size(input_pos), byte, _::binary>> = input_buffer
        {byte, input_pos + 1}
      else
        # EOF: set cell to 0
        {0, input_pos}
      end

    vm = set_tape_at(vm, dp, value)
    vm = GenericVM.put_extra(vm, :input_pos, new_pos)
    vm = GenericVM.advance_pc(vm)
    {nil, vm}
  end

  # =========================================================================
  # Control flow handlers
  # =========================================================================

  @doc """
  `[` — Jump forward past the matching `]` if the current cell is zero.

  If the cell is **nonzero**, execution continues to the next instruction
  (entering the loop body). If the cell is **zero**, the VM jumps to the
  instruction index stored in the operand (one past the matching `]`),
  effectively skipping the loop entirely.

  This is the "while" test: `while (tape[dp] != 0) { ... }`

  ## State Changes

  - `pc` set to operand (if cell == 0) or advanced by 1 (if cell != 0)
  """
  def handle_loop_start(vm, instruction, _code) do
    dp = get_dp(vm)
    tape = get_tape(vm)

    if Enum.at(tape, dp) == 0 do
      # Cell is zero — skip the loop
      vm = GenericVM.jump_to(vm, instruction.operand)
      {nil, vm}
    else
      # Cell is nonzero — enter the loop
      vm = GenericVM.advance_pc(vm)
      {nil, vm}
    end
  end

  @doc """
  `]` — Jump backward to the matching `[` if the current cell is nonzero.

  If the cell is **nonzero**, jump back to the matching `[` (which will
  re-test the condition). If the cell is **zero**, fall through to the next
  instruction (exiting the loop).

  Together with LOOP_START, this implements:

      while tape[dp] != 0:
          <loop body>

  ## State Changes

  - `pc` set to operand (if cell != 0) or advanced by 1 (if cell == 0)
  """
  def handle_loop_end(vm, instruction, _code) do
    dp = get_dp(vm)
    tape = get_tape(vm)

    if Enum.at(tape, dp) != 0 do
      # Cell is nonzero — loop again
      vm = GenericVM.jump_to(vm, instruction.operand)
      {nil, vm}
    else
      # Cell is zero — exit loop
      vm = GenericVM.advance_pc(vm)
      {nil, vm}
    end
  end

  # =========================================================================
  # HALT handler
  # =========================================================================

  @doc """
  Stop the VM.

  Sets `vm.halted` to true. The execution loop checks this flag and
  stops when it is set.
  """
  def handle_halt(vm, _instruction, _code) do
    vm = %{vm | halted: true}
    {nil, vm}
  end

  # =========================================================================
  # Handler registry — maps opcode numbers to handler functions
  # =========================================================================

  @doc """
  Returns a map of all Brainfuck opcode handlers.

  Used by the VM factory (`CodingAdventures.Brainfuck.VM.create_brainfuck_vm/1`)
  to register all handlers at once.

  ## Example

      handlers = CodingAdventures.Brainfuck.Handlers.handlers()
      # => %{0x01 => &handle_right/3, 0x02 => &handle_left/3, ...}
  """
  def handlers do
    %{
      Opcodes.right() => &handle_right/3,
      Opcodes.left() => &handle_left/3,
      Opcodes.inc() => &handle_inc/3,
      Opcodes.dec() => &handle_dec/3,
      Opcodes.output_op() => &handle_output/3,
      Opcodes.input_op() => &handle_input/3,
      Opcodes.loop_start() => &handle_loop_start/3,
      Opcodes.loop_end() => &handle_loop_end/3,
      Opcodes.halt() => &handle_halt/3
    }
  end
end
