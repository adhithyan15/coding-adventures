defmodule CodingAdventures.TestBootstrap.Pbt.ExhaustedError do
  @moduledoc """
  A generator could not produce a value (a filter that never accepts, a spent
  budget).  The runner reports it as `property-exhausted`.
  """
  defexception message: "generator exhausted"
end

defmodule CodingAdventures.TestBootstrap.Pbt.NotFalsifyingError do
  @moduledoc """
  A shrink was asked to start from a value that does not falsify the property:
  a caller bug, loud by design.
  """
  defexception message: "the initial value must falsify the property"
end

defmodule CodingAdventures.TestBootstrap.Pbt.Gen do
  @moduledoc """
  A generator of values: it draws from a PRNG stream and knows how to shrink
  what it drew (VER05 3, 4).

  `sampler` is `(rng -> {value, rng})`.  `shrinker` is nil (values are reported
  unshrunk) or `(value, falsifies, max_steps -> {value, steps})`, where
  `falsifies` is a function returning `true` while a candidate still falsifies
  the property.
  """
  alias CodingAdventures.TestBootstrap.Pbt.{Budget, Limits}

  defstruct [:sampler, :shrinker]

  @type t :: %__MODULE__{}

  def new(sampler, shrinker \\ nil)
      when is_function(sampler, 1) and (is_nil(shrinker) or is_function(shrinker, 3)),
      do: %__MODULE__{sampler: sampler, shrinker: shrinker}

  @doc "Draws one value: `{value, rng}`."
  def sample(%__MODULE__{sampler: sampler}, rng), do: sampler.(rng)

  @doc """
  Shrinks `value` while `falsifies` holds; `{value, 0}` if there is no shrinker.
  A PUBLIC ENTRY POINT (VER05m 3.1): it opens a child of the ambient budget (a
  fresh root outside any shrink) with its own local cap, and charges every
  evaluation of `falsifies`, so however a property or a custom shrinker nests
  calls, the total work stays inside the outermost budget.
  """
  def shrink(%__MODULE__{} = gen, value, max_steps, falsifies) when is_function(falsifies, 1) do
    Limits.check_budget!(max_steps)

    Budget.enter_child(max_steps, fn budget ->
      run(gen, value, Budget.charged(budget, falsifies), max_steps)
    end)
  end

  @doc """
  For combinators shrinking their components: joins the enclosing budget instead
  of opening a new one (a fresh root only when none is open).  It still charges
  every evaluation of `falsifies` to the budget it joined, so even a leaf
  shrinker that charges nothing is bounded.
  """
  def shrink_nested(%__MODULE__{} = gen, value, falsifies, max_steps) do
    Limits.check_budget!(max_steps)

    Budget.enter(max_steps, fn budget ->
      run(gen, value, Budget.charged(budget, falsifies), max_steps)
    end)
  end

  defp run(%__MODULE__{shrinker: nil}, value, _falsifies, _max_steps), do: {value, 0}

  defp run(%__MODULE__{shrinker: shrinker}, value, falsifies, max_steps),
    do: shrinker.(value, falsifies, max_steps)
end
