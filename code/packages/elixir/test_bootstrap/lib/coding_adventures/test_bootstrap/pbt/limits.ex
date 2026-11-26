defmodule CodingAdventures.TestBootstrap.Pbt.Limits do
  @moduledoc """
  Bounds that keep every loop and allocation finite (CWE-400).  A caller or a
  fixture can ask for more only by being refused.
  """
  import Bitwise

  def max_cases, do: 100_000
  def max_shrink_steps, do: 10_000
  def max_input_bytes, do: 1 <<< 20
  def max_list_len, do: 100_000
  def max_gen_count, do: 1_000_000
  # Attempt budget of one shrink = 16 * max_steps + 64: a rejected attempt copies
  # its candidate but is not an accepted step.
  def attempt_factor, do: 16
  def attempt_slack, do: 64
  def max_gen_bytes_total, do: 1 <<< 20
  def max_frequency_total, do: 1 <<< 32
  # Most elements one whole shrink may copy, summed over every attempt of every
  # nested shrinker.
  def max_shrink_work, do: 64 * 1024 * 1024
  def max_retries, do: 10_000
  def default_max_retries, do: 100
  def max_tuple_arity, do: 64

  def attempt_budget(max_steps), do: attempt_factor() * max_steps + attempt_slack()

  @doc "Raises unless `value` is an integer within `[lo, hi]`."
  def check_range!(value, lo, hi, name) do
    unless is_integer(value), do: raise(ArgumentError, "#{name} must be an integer")

    unless value >= lo and value <= hi,
      do: raise(ArgumentError, "#{name} #{value} outside [#{lo}, #{hi}]")

    value
  end

  def check_budget!(max_steps), do: check_range!(max_steps, 0, max_shrink_steps(), "max_steps")
  def check_count!(count), do: check_range!(count, 0, max_gen_count(), "count")
end
