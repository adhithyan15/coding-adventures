defmodule CodingAdventures.TestBootstrap.Fuzz.FailureIds do
  @moduledoc """
  The closed failure vocabulary (VER06 section 6.2).  Nothing else is ever
  printed in a public failure line.
  """
  @all ["fuzz-crash", "fuzz-timeout", "fuzz-oom", "fuzz-leak", "fuzz-panic"]

  def crash, do: "fuzz-crash"
  def timeout, do: "fuzz-timeout"
  def oom, do: "fuzz-oom"
  def leak, do: "fuzz-leak"
  def panic, do: "fuzz-panic"
  def all, do: @all

  @doc """
  `valid?("fuzz-crash")` is true; `valid?("fuzz-arbitrary-error")` is false; a
  non-binary (an atom, `nil`, an integer) is false, never an error.
  """
  def valid?(candidate) when is_binary(candidate), do: candidate in @all
  def valid?(_), do: false
end
