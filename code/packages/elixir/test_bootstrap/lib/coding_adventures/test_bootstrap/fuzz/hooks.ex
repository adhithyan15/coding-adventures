defmodule CodingAdventures.TestBootstrap.Fuzz.Hooks do
  @moduledoc """
  Sabotage hooks: the engine entry points the corpus runner calls, as swappable
  functions.  These are the only production indirections added for the negative
  controls (VER06n section 8): a test replaces one with a broken version, runs the
  corpus, and the runner must report problems.  They live in the process
  dictionary of the calling process (`with/2` restores them in `after`), so they
  never leak across tests or into other processes.  Production code never sets them.
  """
  alias CodingAdventures.TestBootstrap.Tfv
  alias CodingAdventures.TestBootstrap.Fuzz.{Bitmap, FailureIds, Mutate, Signature}

  @names [
    :bit_flip,
    :byte_flip,
    :byte_arithmetic,
    :boundary_insert,
    :block_delete,
    :block_insert,
    :block_duplicate,
    :splice,
    :mutate_sequence,
    :corpus_admit,
    :duplicate_crash,
    :failure_id,
    :format_tap,
    :tfv_valid
  ]

  def names, do: @names

  defp default(:bit_flip), do: &Mutate.bit_flip/3
  defp default(:byte_flip), do: &Mutate.byte_flip/3
  defp default(:byte_arithmetic), do: &Mutate.byte_arithmetic/5
  defp default(:boundary_insert), do: &Mutate.boundary_insert/5
  defp default(:block_delete), do: &Mutate.block_delete/3
  defp default(:block_insert), do: &Mutate.block_insert/3
  defp default(:block_duplicate), do: &Mutate.block_duplicate/4
  defp default(:splice), do: &Mutate.splice/4
  defp default(:mutate_sequence), do: &Mutate.mutate_sequence_counted/5
  defp default(:corpus_admit), do: &Bitmap.corpus_admit/2
  defp default(:duplicate_crash), do: &Signature.duplicate_crash?/4
  defp default(:failure_id), do: &FailureIds.valid?/1
  defp default(:format_tap), do: &Signature.format_tap_failure/4
  defp default(:tfv_valid), do: &Tfv.valid?/1

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
