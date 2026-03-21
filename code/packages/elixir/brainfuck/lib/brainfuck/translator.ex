defmodule CodingAdventures.Brainfuck.Translator do
  @moduledoc """
  Brainfuck Translator — Source Code to Bytecode in One Pass.

  ## Why "Translator" and not "Compiler"?

  A compiler transforms a high-level *structured* representation (an AST)
  into lower-level instructions. It handles scoping, type checking, operator
  precedence, and all the complexity that comes with real languages.

  Brainfuck does not have any of that. There is no AST, no scoping, no types.
  Each source character maps directly to one instruction. The only non-trivial
  step is **bracket matching** — connecting `[` to its matching `]` so the
  VM knows where to jump.

  So we call this a "translator" rather than a "compiler": it translates
  characters to opcodes, with bracket matching as the sole transformation.

  ## How Bracket Matching Works

  Bracket matching is a classic stack problem:

  1. Scan the source left to right.
  2. When we see `[`, emit a `LOOP_START` with a placeholder target (0),
     and push its instruction index onto a stack.
  3. When we see `]`, pop the matching `[` index from the stack.
     - Patch the `[` instruction to jump to one past the current `]`.
     - Emit a `LOOP_END` that jumps back to the `[`.
  4. After scanning, if the stack is not empty, we have unmatched `[`.

  ## Example

  Source: `++[>+<-]`

  Translation:

      Index  Opcode       Operand   Source
      ─────────────────────────────────────
      0      INC          —         +
      1      INC          —         +
      2      LOOP_START   8         [  (jump to 8 if cell==0)
      3      RIGHT        —         >
      4      INC          —         +
      5      LEFT         —         <
      6      DEC          —         -
      7      LOOP_END     2         ]  (jump to 2 if cell!=0)
      8      HALT         —         (end)

  When cell 0 reaches zero, LOOP_START at index 2 jumps to index 8
  (one past the LOOP_END). When the cell is still nonzero, LOOP_END
  at index 7 jumps back to index 2.
  """

  alias CodingAdventures.VirtualMachine.Types.{Instruction, CodeObject}
  alias CodingAdventures.Brainfuck.Opcodes

  # =========================================================================
  # Error type
  # =========================================================================

  defmodule TranslationError do
    @moduledoc """
    Raised when the Brainfuck source has mismatched brackets.

    Brainfuck requires every `[` to have a matching `]` and vice versa.
    This error indicates a structural problem in the source code that
    prevents translation to bytecode.
    """
    defexception [:message]
  end

  # =========================================================================
  # Public API
  # =========================================================================

  @doc """
  Translate Brainfuck source code into a CodeObject.

  Single-pass translation: each source character becomes one instruction.
  Non-command characters are ignored (they are comments). Brackets are
  matched using a stack, and their operands are patched to point to the
  correct jump targets.

  ## Parameters

  - `source` — the Brainfuck program as a string

  ## Returns

  A `%CodeObject{}` with:
  - `instructions` — the bytecode with a trailing HALT
  - `constants` — empty list (Brainfuck has no constants)
  - `names` — empty list (Brainfuck has no variables)

  ## Raises

  - `TranslationError` — if brackets are mismatched

  ## Examples

      iex> code = CodingAdventures.Brainfuck.Translator.translate("+++.")
      iex> length(code.instructions)
      5
  """
  def translate(source) do
    char_to_op = Opcodes.char_to_op()

    # -- Single-pass translation with bracket matching via a stack ----------
    #
    # We accumulate instructions in a list (appending is O(n) in Elixir,
    # but Brainfuck programs are small enough that this is fine). The
    # bracket_stack tracks unmatched '[' positions for patching.

    loop_start_op = Opcodes.loop_start()
    loop_end_op = Opcodes.loop_end()

    {instructions, bracket_stack} =
      source
      |> String.graphemes()
      |> Enum.reduce({[], []}, fn char, {instrs, stack} ->
        case Map.get(char_to_op, char) do
          # Not a Brainfuck command — it is a comment, skip it
          nil ->
            {instrs, stack}

          # '[' — emit LOOP_START with placeholder, push index onto stack
          ^loop_start_op ->
            index = length(instrs)
            instr = %Instruction{opcode: loop_start_op, operand: 0}
            {instrs ++ [instr], [index | stack]}

          # ']' — pop matching '[', patch both jump targets
          ^loop_end_op ->
            case stack do
              [] ->
                raise TranslationError, "Unmatched ']' — no matching '[' found"

              [start_index | rest_stack] ->
                end_index = length(instrs)

                # Patch LOOP_START to jump past LOOP_END (end_index + 1)
                patched = %Instruction{
                  opcode: loop_start_op,
                  operand: end_index + 1
                }

                instrs = List.replace_at(instrs, start_index, patched)

                # Emit LOOP_END that jumps back to LOOP_START
                instr = %Instruction{opcode: loop_end_op, operand: start_index}
                {instrs ++ [instr], rest_stack}
            end

          # Simple command — no operand needed
          op ->
            {instrs ++ [%Instruction{opcode: op, operand: nil}], stack}
        end
      end)

    # -- Validate: all brackets must be matched ----------------------------
    if bracket_stack != [] do
      raise TranslationError,
            "Unmatched '[' — #{length(bracket_stack)} unclosed bracket(s)"
    end

    # -- Append HALT to mark end of program --------------------------------
    instructions = instructions ++ [%Instruction{opcode: Opcodes.halt(), operand: nil}]

    %CodeObject{instructions: instructions, constants: [], names: []}
  end
end
