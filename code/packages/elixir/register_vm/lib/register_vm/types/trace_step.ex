defmodule CodingAdventures.RegisterVM.Types.TraceStep do
  @moduledoc """
  A snapshot of VM state before and after executing a single instruction.

  ## What is tracing?

  When you run `execute_with_trace/1`, the VM records a `TraceStep` for
  every instruction it executes. This gives you a complete audit trail:
  you can replay the execution, inspect how the accumulator changed at each
  step, and see exactly when feedback slots transitioned states.

  ## Analogy

  Imagine debugging a real CPU with a logic analyser: at each clock cycle,
  the analyser captures the values on every wire before and after the
  cycle. TraceStep is that per-cycle snapshot for our software VM.

  ## Fields

  - `frame_depth` — how deep in the call stack this instruction executed.
    0 = top-level script, 1 = first called function, etc. Useful for
    reconstructing nested call behaviour from a flat trace list.

  - `ip` — instruction pointer value when this step executed. Combined with
    `frame_depth` uniquely identifies the position in execution.

  - `instruction` — the `%RegisterInstruction{}` that was executed.

  - `acc_before` / `acc_after` — accumulator value immediately before and
    after executing the instruction. The most common delta to inspect.

  - `registers_before` / `registers_after` — full register file snapshot.
    Represented as a tuple matching the frame's register file layout.

  - `feedback_delta` — list of `{slot_idx, old_value, new_value}` tuples
    for every feedback slot that changed during this instruction. Empty list
    if no slot changed (the common case). Non-empty means the instruction
    observed a new type and updated its inline cache.
  """

  defstruct [
    :frame_depth,
    :ip,
    :instruction,
    :acc_before,
    :acc_after,
    :registers_before,
    :registers_after,
    feedback_delta: []
  ]
end
