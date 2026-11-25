defmodule CodingAdventures.TestBootstrap.Pbt.Hooks do
  @moduledoc """
  Sabotage hooks: the only production indirections added for the negative
  controls (VER05m section 6).  A test swaps one with `with/2`, runs the corpus,
  and the runner must report problems.  They live in the process dictionary of
  the calling process, so they never leak across tests or into other processes.
  """
  import Bitwise

  alias CodingAdventures.TestBootstrap.{Prng, Pbt.Limits}

  @names [:bisect_candidate, :unit_candidate, :skip_deletion_phase, :new_rng, :default_max_work]

  defp default(:bisect_candidate), do: fn current, diff -> current - (diff >>> 1) end
  defp default(:unit_candidate), do: fn current, diff -> current - sign(diff) end
  defp default(:skip_deletion_phase), do: false
  defp default(:new_rng), do: &Prng.new/1
  defp default(:default_max_work), do: Limits.max_shrink_work()

  defp sign(d) when d > 0, do: 1
  defp sign(d) when d < 0, do: -1
  defp sign(_), do: 0

  @doc "The current value of a hook (its production default unless overridden)."
  def get(name) when name in @names do
    case Process.get({__MODULE__, name}, :unset) do
      :unset -> default(name)
      value -> value
    end
  end

  @doc "Runs `fun` with the given overrides (keyword list), restoring them afterwards."
  def with(overrides, fun) do
    Enum.each(overrides, fn {k, _} ->
      unless k in @names, do: raise(ArgumentError, "unknown hook #{k}")
    end)

    saved = Enum.map(overrides, fn {k, _} -> {k, Process.get({__MODULE__, k}, :unset)} end)
    Enum.each(overrides, fn {k, v} -> Process.put({__MODULE__, k}, v) end)

    try do
      fun.()
    after
      Enum.each(saved, fn
        {k, :unset} -> Process.delete({__MODULE__, k})
        {k, v} -> Process.put({__MODULE__, k}, v)
      end)
    end
  end
end
