defmodule CodingAdventures.TestBootstrap.Pbt.Shrink do
  @moduledoc """
  Integrated shrinking (VER05 4, VER05e 4, VER05m 3.1).

  A property that fails on `[913, -4, 77, 0, 2]` has told you nothing; one that
  fails on `[0]` has told you what is wrong.  Shrinking walks from the first
  failing candidate toward the smallest one that still fails.  It is
  "integrated" because the generator that made the value knows its own bounds,
  so a shrunk `Gens.integer(5, 10)` never becomes 0.

  The public shrinkers take a property that returns `true` when it HOLDS (the
  corpus convention) and return `{minimal, steps}`, where steps counts ACCEPTED
  moves only.  Internally `falsifies` is the inverse: `true` while a candidate
  still fails.
  """
  alias CodingAdventures.TestBootstrap.Pbt.{Budget, Hooks, Limits, NotFalsifyingError}
  alias CodingAdventures.TestBootstrap.Prng

  def in_bounds?(value, lo, hi),
    do: (is_nil(lo) or lo <= value) and (is_nil(hi) or value <= hi)

  @doc """
  Bisects then unit-steps `current` toward `target`.

  Each turn tries, in order, the bisection candidate `current - floor(diff / 2)`
  when `|diff| > 1`, then the unit candidate (one step toward the target).  A
  candidate is accepted only if inside `[lo, hi]` (nil = unbounded), different
  from current and still falsifying.  No acceptable candidate ends the walk.

  Worked example (corpus shrink_int/54321_le_100, property x <= 100): 54321 ->
  27161 -> 13581 -> ... -> 101 in 15 accepted moves; 100 would satisfy the
  property, so neither candidate from 101 is accepted.
  """
  def int_walk(current, target, lo, hi, falsifies, budget),
    do: walk(current, target, lo, hi, falsifies, budget, 0)

  defp walk(current, target, _lo, _hi, _f, budget, steps)
       when steps >= budget or current == target,
       do: {current, steps}

  defp walk(current, target, lo, hi, falsifies, budget, steps) do
    diff = current - target

    bisected =
      if abs(diff) > 1, do: accept(Hooks.get(:bisect_candidate).(current, diff), current, lo, hi, falsifies)

    case bisected || accept(Hooks.get(:unit_candidate).(current, diff), current, lo, hi, falsifies) do
      nil -> {current, steps}
      {:ok, cand} -> walk(cand, target, lo, hi, falsifies, budget, steps + 1)
    end
  end

  defp accept(cand, current, lo, hi, falsifies) do
    if cand != current and in_bounds?(cand, lo, hi) and falsifies.(cand) == true,
      do: {:ok, cand}
  end

  @doc "The target probe followed by the walk (VER05 4.2)."
  def shrink_int_core(initial, _target, _lo, _hi, _falsifies, 0), do: {initial, 0}

  def shrink_int_core(initial, target, lo, hi, falsifies, max_steps) do
    # the target itself fails: cannot do better
    if in_bounds?(target, lo, hi) and target != initial and falsifies.(target) == true do
      {target, 1}
    else
      int_walk(initial, target, lo, hi, falsifies, max_steps)
    end
  end

  @doc "A byte value shrunk toward 0 by the integer walk."
  def byte_walk(value, falsifies, budget), do: int_walk(value, 0, 0, 255, falsifies, budget)

  @doc """
  Shrinks a falsifying sequence (VER05 4.3).  `falsifies` always receives a
  FRESH list.

    * probe        `min_len == 0`: does the empty list fail?  Then it is the
                   answer, one step.  (`min_len > 0`: probe the length-`min_len`
                   prefix and carry on from it.)
    * bisection    keep the left half, else the right half, while one fails.
    * deletion     single elements, right to left.
    * element-wise left to right via `shrink_elem` (nil means none).

  Worked example (corpus list_sum_le_20, `[10, 5, 8, 4]`): the empty list and both
  halves satisfy the property; deleting index 3 leaves `[10, 5, 8]` (sum 23,
  still failing); element-wise 10 -> 8 gives the 1-minimal `[8, 5, 8]`.

  Every candidate is CHARGED to the budget tree BEFORE it is built; once refused,
  every loop ends.  With `max_steps == 0` or a spent budget the input is
  returned as is.
  """
  def shrink_sequence(initial, _falsifies, 0, _min_len, _shrink_elem), do: {initial, 0}

  def shrink_sequence(initial, falsifies, max_steps, min_len, shrink_elem) do
    Budget.enter(max_steps, fn budget ->
      if Budget.charge_copy(budget, length(initial)) do
        sequence(budget, initial, falsifies, max_steps, min_len, shrink_elem)
      else
        {initial, 0}
      end
    end)
  end

  defp sequence(budget, initial, falsifies, max_steps, min_len, shrink_elem) do
    probe =
      if length(initial) > min_len and Budget.charge(budget, min_len) and
           falsifies.(Enum.take(initial, min_len)) == true do
        if min_len == 0, do: :empty, else: {Enum.take(initial, min_len), 1}
      else
        {initial, 0}
      end

    case probe do
      :empty ->
        {[], 1}

      {current, steps} ->
        floor = max(1, min_len)
        {current, steps} = bisect(budget, current, steps, floor, max_steps, min_len, falsifies)

        {current, steps} =
          if Hooks.get(:skip_deletion_phase),
            do: {current, steps},
            else: delete(budget, current, length(current) - 1, steps, floor, max_steps, falsifies)

        if shrink_elem,
          do: elementwise(budget, current, 0, length(current), steps, max_steps, falsifies, shrink_elem),
          else: {current, steps}
    end
  end

  # Phase 1: bisection (left half first, then right half).
  defp bisect(budget, cur, steps, floor, max_steps, min_len, f) do
    size = length(cur)

    if size > floor and steps < max_steps do
      mid = div(size, 2)
      right_len = size - mid

      cond do
        mid >= min_len and Budget.charge(budget, mid) and f.(Enum.take(cur, mid)) == true ->
          bisect(budget, Enum.take(cur, mid), steps + 1, floor, max_steps, min_len, f)

        right_len >= min_len and Budget.charge(budget, right_len) and f.(Enum.drop(cur, mid)) == true ->
          bisect(budget, Enum.drop(cur, mid), steps + 1, floor, max_steps, min_len, f)

        true ->
          {cur, steps}
      end
    else
      {cur, steps}
    end
  end

  # Phase 2: single-element deletion, right to left.
  defp delete(budget, cur, i, steps, floor, max_steps, f) do
    size = length(cur)

    if i >= 0 and size > floor and steps < max_steps and Budget.room?(budget) do
      # charge BEFORE copying
      if Budget.charge(budget, size - 1) do
        cand = List.delete_at(cur, i)

        if f.(cand) == true,
          do: delete(budget, cand, min(i, size - 2), steps + 1, floor, max_steps, f),
          else: delete(budget, cur, i - 1, steps, floor, max_steps, f)
      else
        {cur, steps}
      end
    else
      {cur, steps}
    end
  end

  # Phase 3: element-wise, left to right.
  defp elementwise(_budget, cur, index, size, steps, _max, _f, _se) when index >= size,
    do: {cur, steps}

  defp elementwise(budget, cur, index, size, steps, max_steps, f, shrink_elem) do
    remaining = max_steps - steps

    if remaining <= 0 or not Budget.room?(budget) do
      {cur, steps}
    else
      with_element = fn candidate ->
        Budget.charge(budget, length(cur)) and f.(List.replace_at(cur, index, candidate)) == true
      end

      {value, used} = shrink_elem.(Enum.at(cur, index), with_element, remaining)

      if used > 0,
        do: elementwise(budget, List.replace_at(cur, index, value), index + 1, size, steps + used, max_steps, f, shrink_elem),
        else: elementwise(budget, cur, index + 1, size, steps, max_steps, f, shrink_elem)
    end
  end

  defp holds?(prop, value), do: prop.(value) == true

  @doc """
  Shrinks a falsifying integer toward `target` inside `[lo, hi]` (the corpus
  calls these min and max).  `prop` is true when the property holds.  Raises
  `NotFalsifyingError` when `initial` satisfies it, `ArgumentError` for
  `lo > hi`, an initial outside the bounds or a budget outside the limit.
  """
  def shrink_int(initial, target, lo, hi, max_steps \\ 1000, prop) when is_function(prop, 1) do
    for {v, name} <- [{initial, "initial"}, {target, "target"}, {lo, "min"}, {hi, "max"}] do
      unless is_integer(v), do: raise(ArgumentError, "#{name} must be an integer")
    end

    Limits.check_budget!(max_steps)
    if lo > hi, do: raise(ArgumentError, "min exceeds max")

    unless in_bounds?(initial, lo, hi),
      do: raise(ArgumentError, "initial value must lie within [min, max]")

    Budget.enter_child(max_steps, fn budget ->
      # starved by an enclosing shrink
      if Budget.charge(budget, 1) do
        if holds?(prop, initial), do: raise(NotFalsifyingError)
        falsifies = fn v -> Budget.charge(budget, 1) and not holds?(prop, v) end
        shrink_int_core(initial, target, lo, hi, falsifies, max_steps)
      else
        {initial, 0}
      end
    end)
  end

  @doc """
  Shrinks a falsifying list of integers: empty probe, halves, single deletion,
  then each element toward `elem_target` (unbounded).
  """
  def shrink_list(initial, elem_target, max_steps \\ 1000, prop) when is_function(prop, 1) do
    unless is_list(initial) and Enum.all?(initial, &is_integer/1),
      do: raise(ArgumentError, "initial must be a list of integers")

    unless is_integer(elem_target), do: raise(ArgumentError, "elem_target must be an integer")
    Limits.check_budget!(max_steps)

    Budget.enter_child(max_steps, fn budget ->
      # charged before the first copies and for the initial evaluation
      if Budget.charge_copy(budget, 2 * length(initial)) and Budget.charge(budget, 1) do
        if holds?(prop, initial), do: raise(NotFalsifyingError)
        walk = fn value, falsifies, steps -> int_walk(value, elem_target, nil, nil, falsifies, steps) end
        shrink_sequence(initial, fn c -> not holds?(prop, c) end, max_steps, 0, walk)
      else
        {initial, 0}
      end
    end)
  end

  @doc "Shrinks falsifying bytes (a binary): sequence shrinking over byte values toward 0x00."
  def shrink_bytes(initial, max_steps \\ 1000, prop) when is_function(prop, 1) do
    unless is_binary(initial), do: raise(ArgumentError, "initial must be a binary")
    Limits.check_budget!(max_steps)

    Budget.enter_child(max_steps, fn budget ->
      if Budget.charge_copy(budget, 3 * byte_size(initial)) and Budget.charge(budget, 1) do
        if holds?(prop, initial), do: raise(NotFalsifyingError)

        # each attempt also converts its candidate back to a binary: charge that copy too
        falsifies = fn c ->
          Budget.charge_copy(budget, length(c)) and not holds?(prop, :erlang.list_to_binary(c))
        end

        {list, steps} =
          shrink_sequence(:binary.bin_to_list(initial), falsifies, max_steps, 0, &byte_walk/3)

        {:erlang.list_to_binary(list), steps}
      else
        {initial, 0}
      end
    end)
  end

  @doc """
  Replays a failure from nothing but `seed`: draw up to 1000 values
  `bounded(1000)`; the first that falsifies is the initial failure, then shrunk
  toward 0 over `[0, 1000]`.  Same seed, same pair, forever.  Returns
  `{initial_fail, minimal}`.
  """
  def simulate_replay(seed, prop) when is_function(prop, 1) do
    Budget.enter_child(1000, fn budget ->
      first = draw_first(Hooks.get(:new_rng).(seed), budget, prop, 1000)

      if first == nil,
        do: raise(ArgumentError, "property was not falsified within 1000 draws")

      {minimal, _} = shrink_int(first, 0, 0, 1000, 1000, prop)
      {first, minimal}
    end)
  end

  defp draw_first(_rng, _budget, _prop, 0), do: nil

  defp draw_first(rng, budget, prop, left) do
    if Budget.charge(budget, 1) do
      {v, rng} = Prng.bounded(rng, 1000)
      if holds?(prop, v), do: draw_first(rng, budget, prop, left - 1), else: v
    end
  end
end
