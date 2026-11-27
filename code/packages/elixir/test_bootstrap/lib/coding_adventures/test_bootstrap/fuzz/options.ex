defmodule CodingAdventures.TestBootstrap.Fuzz.Options do
  @moduledoc """
  Bounds of a campaign; every field is validated (CWE-400).

  `minimize_budget` is the most target executions spent minimising ONE novel crash;
  on top of it the whole campaign spends at most `max_iterations` minimisation
  executions in total.  `step_budget` and `mem_budget` are the per-execution
  deterministic budgets of `Handle.tick/2` and `Handle.reserve/2`; `mem_budget` is
  also the process heap limit (in words) and `timeout_ms` the wall-clock kill.
  `ci` is `:on`, `:off` or `:unset`; `:unset` is treated as CI (fails closed).
  """
  alias CodingAdventures.TestBootstrap.Fuzz.Limits

  defstruct max_iterations: 1000,
            seed: 42,
            max_input_len: 4096,
            base_energy: 10,
            minimize_budget: 2000,
            step_budget: 200_000,
            mem_budget: 64 * 1024 * 1024,
            timeout_ms: 5000,
            ci: :unset

  defimpl Inspect do
    # The seed is private (VER06 section 6): never printed.
    def inspect(o, _opts), do: "#Fuzz.Options<max_iterations=#{o.max_iterations}>"
  end

  @doc "Validated options from a keyword list; raises `ArgumentError` on a bad or unknown field."
  def new(opts \\ []) do
    o = struct!(__MODULE__, opts)
    Limits.check_int!(o.seed, "seed")
    Limits.check_range!(o.max_iterations, 0, Limits.max_iterations(), "max_iterations")
    Limits.check_range!(o.max_input_len, 0, Limits.max_input_bytes(), "max_input_len")
    Limits.check_range!(o.base_energy, 0, Limits.max_iterations(), "base_energy")
    Limits.check_range!(o.minimize_budget, 0, Limits.max_minimize_budget(), "minimize_budget")
    Limits.check_range!(o.step_budget, 0, Limits.max_step_budget(), "step_budget")
    Limits.check_range!(o.mem_budget, 0, Limits.max_mem_budget(), "mem_budget")
    Limits.check_range!(o.timeout_ms, 1, Limits.max_timeout_ms(), "timeout_ms")

    unless o.ci in [:on, :off, :unset],
      do: raise(ArgumentError, "ci must be :on, :off or :unset")

    o
  end
end
