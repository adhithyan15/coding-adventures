defmodule CodingAdventures.TestBootstrap.Fuzz.Limits do
  @moduledoc """
  CWE-400 ceilings (VER06n sections 3 and 6).  A fuzz campaign is
  attacker-reachable work (a CI job fed a hostile configuration or fixture), so
  every count and length that drives a loop or an allocation has a hard ceiling,
  and each one is checked BEFORE the allocation it bounds.
  """
  import Bitwise

  def max_iterations, do: 1_000_000
  def max_input_bytes, do: 1 <<< 20
  def max_crashes, do: 1000
  def max_initial_corpus, do: 10_000
  def max_step_budget, do: 100_000_000
  def max_mem_budget, do: 1 <<< 30
  def max_minimize_budget, do: 100_000
  def default_minimize_budget, do: 2000
  def default_step_budget, do: 200_000
  def default_mem_budget, do: 64 <<< 20
  def max_timeout_ms, do: 60_000
  def default_timeout_ms, do: 5000
  # Bytes of corpus seeds plus crash inputs one campaign keeps (256 MiB).
  def max_retained_bytes, do: 256 <<< 20
  # Bytes ONE mutate_sequence call may copy (initial copy + one buffer per step).
  def max_sequence_work, do: 1 <<< 28
  # The aggregate mutate_sequence work of one corpus run (16 MiB).
  def corpus_sequence_work, do: 1 <<< 24
  # Stack entries inspected per crash, however deep the recursion.
  def max_walk, do: 4096
  # Frames kept in a signature (the innermost survive).
  def max_frames, do: 64
  def max_kind_len, do: 64
  def max_target_len, do: 128
  # VER06 section 4.1: 8,192 bytes = 65,536 bits = 65,536 edges.
  def coverage_map_size, do: 65_536
  def coverage_bytes, do: 8192

  @doc "Raises `ArgumentError` unless `value` is an integer (booleans and floats are not)."
  def check_int!(value, _name) when is_integer(value), do: value

  def check_int!(value, name),
    do: raise(ArgumentError, "#{name} must be an integer, got #{kind(value)}")

  def check_range!(value, lo, hi, name) do
    check_int!(value, name)

    unless value >= lo and value <= hi,
      do: raise(ArgumentError, "#{name} must be in [#{lo}, #{hi}]")

    value
  end

  defp kind(v) when is_float(v), do: "a float"
  defp kind(v) when is_atom(v), do: "an atom"
  defp kind(v) when is_binary(v), do: "a binary"
  defp kind(_), do: "another term"
end
