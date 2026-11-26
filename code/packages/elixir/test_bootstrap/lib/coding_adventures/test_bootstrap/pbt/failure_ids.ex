defmodule CodingAdventures.TestBootstrap.Pbt.FailureIds do
  @moduledoc """
  The closed failure vocabulary (VER05 5.1).  `property-timeout` is registered
  but never produced: the engine has no wall-clock timeout, so a property that
  loops forever hangs its process (run the check in its own process and bound it
  from outside).
  """
  def counterexample, do: "property-counterexample"
  def exhausted, do: "property-exhausted"
  def timeout, do: "property-timeout"
  def panic, do: "property-panic"
  def all, do: [counterexample(), exhausted(), timeout(), panic()]

  @doc "True only for a binary equal to one of the four registered ids."
  def valid?(candidate), do: is_binary(candidate) and candidate in all()
end
