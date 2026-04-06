defmodule CodingAdventures.RegisterVM.Types.VMResult do
  @moduledoc """
  The result of a complete VM execution run.

  ## Fields

  - `output` — list of strings printed by the program (in order). The VM
    does not write to stdout directly; instead, programs use a `Print`
    convention (calling a built-in that appends to this list). This makes
    the VM easy to test without capturing IO.

  - `return_value` — the final value in the accumulator when the top-level
    function returned or halted. For a script-level execution this is the
    value of the last expression evaluated.

  - `error` — a `%VMError{}` if execution terminated abnormally, or `nil`
    for a successful run. Check this before using `return_value`.

  - `final_feedback_vector` — the feedback vector of the top-level frame
    after execution completes. Useful for testing that type observations
    were recorded correctly at specific instruction sites.
  """

  defstruct [
    output: [],
    return_value: nil,
    error: nil,
    final_feedback_vector: []
  ]
end
