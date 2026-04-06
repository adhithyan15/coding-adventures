defmodule CodingAdventures.RegisterVM.Types.CallFrame do
  @moduledoc """
  A CallFrame holds all state for one active function invocation.

  ## What is a call frame?

  When a program calls a function, the runtime needs somewhere to store:
  - What the function is doing right now (instruction pointer)
  - Its private workspace (accumulator + registers)
  - Its "notes" about past executions (feedback vector)
  - How to get back to the caller (caller_frame pointer)
  - What variables it closes over (context/scope chain)

  All of this lives in a CallFrame. A call stack is a linked list of
  CallFrames, each pointing back to its caller via `caller_frame`.

  ## Analogy

  Imagine you are following a recipe (CodeObject). You have:
  - A bookmark showing where you are in the recipe (`ip`)
  - One "working hand" holding the current value (`accumulator`)
  - A row of labeled bowls for intermediate ingredients (`registers`)
  - A notepad where you jot down patterns you've noticed (`feedback_vector`)
  - A reference to whoever handed you this recipe (`caller_frame`)
  - A window into the pantry of shared variables (`context`)

  ## Fields

  - `code` — the `%CodeObject{}` being executed in this frame. Immutable
    for the lifetime of the frame.

  - `ip` — instruction pointer. An integer index into `code.instructions`.
    Starts at 0. Incremented by 1 after each instruction fetch (before
    executing the instruction). Jumps modify it further.

  - `accumulator` — the implicit "working register." Most instructions
    read from or write to this single value. Starting value is `:undefined`,
    mirroring JavaScript's uninitialized variable semantics.

  - `registers` — a fixed-size Elixir tuple of size `code.register_count`.
    Indexed by small non-negative integers (r0, r1, r2, ...). Initialized
    to `nil`. Used to hold intermediate values and function arguments.

    We use a tuple (not a list) because `elem/2` and `put_elem/3` give
    O(1) access by index, matching what a real CPU register file provides.

  - `feedback_vector` — a list of length `code.feedback_slot_count`. Each
    entry tracks what types the interpreter saw at the corresponding
    instruction site. Starts as a list of `:uninitialized` atoms.

  - `context` — a map representing the scope chain for variable capture.
    `nil` means no captured variables. See `Scope` module for structure.

  - `caller_frame` — the CallFrame that invoked this function. When the
    current function returns, execution resumes in `caller_frame` at
    `caller_frame.ip`. `nil` for the top-level script frame.
  """

  defstruct [
    :code,
    ip: 0,
    accumulator: :undefined,
    registers: {},
    feedback_vector: [],
    context: nil,
    caller_frame: nil
  ]
end
