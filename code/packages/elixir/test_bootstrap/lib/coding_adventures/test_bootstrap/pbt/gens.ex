defmodule CodingAdventures.TestBootstrap.Pbt.Gens do
  @moduledoc """
  The generator algebra (VER05 3, VER05e 3).

  DRAW ORDER IS THE COMPATIBILITY CONTRACT between language lanes: the pbt-v1
  vectors pin exact outputs per seed, so reordering a draw breaks conformance.
  Nested generators are length-bounded per level, not in total; shrinking, by
  contrast, shares one work budget across all levels.

  Values: integers are integers (u8, u64 and i128 included), bytes are binaries,
  lists are lists, `tuple/1` yields a tuple.  Every sampler is
  `(rng -> {value, rng})`.

  A predicate (`filter/3`) accepts only when it returns exactly `true`.
  """
  import Bitwise

  alias CodingAdventures.TestBootstrap.Pbt.{Budget, ExhaustedError, Gen, Hooks, Limits, Shrink}
  alias CodingAdventures.TestBootstrap.Prng

  @two127 1 <<< 127
  @two128 1 <<< 128
  @min_i128 -@two127
  @max_i128 @two127 - 1

  # The in-range point nearest zero: the integer shrink target.
  defp clamp_zero(lo, _hi) when lo > 0, do: lo
  defp clamp_zero(_lo, hi) when hi < 0, do: hi
  defp clamp_zero(_, _), do: 0

  defp int_shrinker(lo, hi) do
    target = clamp_zero(lo, hi)
    fn value, falsifies, max_steps -> Shrink.shrink_int_core(value, target, lo, hi, falsifies, max_steps) end
  end

  @doc "Always generates `value` and draws nothing."
  def constant(value), do: Gen.new(fn rng -> {value, rng} end)

  @doc "`bounded(2) == 1`.  Shrinks true to false."
  def bool do
    Gen.new(
      fn rng ->
        {v, rng} = Prng.bounded(rng, 2)
        {v == 1, rng}
      end,
      fn
        true, falsifies, max_steps when max_steps > 0 ->
          if falsifies.(false) == true, do: {false, 1}, else: {true, 0}

        value, _falsifies, _max_steps ->
          {value, 0}
      end
    )
  end

  @doc "`[0, 255]`: `bounded(256)`."
  def u8, do: Gen.new(fn rng -> Prng.bounded(rng, 256) end, int_shrinker(0, 255))

  @doc "A full 64-bit word: `next_u64`."
  def u64, do: Gen.new(&Prng.next_u64/1, int_shrinker(0, Prng.mask64()))

  @doc """
  Two stream words, high first, read as two's complement: words `0x8000...0, 0`
  give `-2**127`; `0x7fff...f, f...f` give `2**127 - 1`.
  """
  def sample_i128(rng) do
    {hi, rng} = Prng.next_u64(rng)
    {lo, rng} = Prng.next_u64(rng)
    v = (hi <<< 64) ||| lo
    {if(v >= @two127, do: v - @two128, else: v), rng}
  end

  def i128, do: Gen.new(&sample_i128/1, int_shrinker(@min_i128, @max_i128))

  @doc """
  A uniform integer over the inclusive range `[min, max]`, any width.  The
  offset from min is drawn by `bounded_big` (VER05 7.2).  Shrinks toward the
  in-range point nearest zero and never leaves `[min, max]`.
  """
  def integer(min, max) when is_integer(min) and is_integer(max) do
    if min > max, do: raise(ArgumentError, "integer needs min <= max")
    Gen.new(fn rng -> bounded_int(rng, min, max) end, int_shrinker(min, max))
  end

  def integer(_, _), do: raise(ArgumentError, "integer bounds must be integers")

  def bounded_int(rng, lo, hi) do
    {off, rng} = Prng.bounded_big(rng, hi - lo + 1)
    {lo + off, rng}
  end

  defp check_lengths!(min_len, max_len, limit, what) do
    unless is_integer(min_len) and is_integer(max_len),
      do: raise(ArgumentError, "#{what} lengths must be integers")

    if min_len < 0 or max_len < min_len,
      do: raise(ArgumentError, "#{what} needs 0 <= min_len <= max_len")

    if max_len > limit, do: raise(ArgumentError, "#{what} max_len #{max_len} exceeds #{limit}")
  end

  @doc """
  A byte string: `L = min_len + bounded(span)`, then `L` draws of `bounded(256)`.
  Shrinks by sequence shrinking, respecting `min_len`.
  """
  def bytes(min_len, max_len) do
    check_lengths!(min_len, max_len, Limits.max_input_bytes(), "bytes")
    span = max_len - min_len + 1

    sampler = fn rng ->
      {off, rng} = Prng.bounded(rng, span)
      {list, rng} = draw_n(rng, min_len + off, fn r -> Prng.bounded(r, 256) end)
      {:erlang.list_to_binary(list), rng}
    end

    shrinker = fn value, falsifies, max_steps ->
      # The list conversion (and the binary one on the way out) are copies too:
      # charge them first, and do nothing once the budget is spent.
      Budget.enter(max_steps, fn budget ->
        if max_steps == 0 or not Budget.charge_copy(budget, 2 * byte_size(value)) do
          {value, 0}
        else
          wrapped = fn c -> Budget.charge_copy(budget, length(c)) and falsifies.(:erlang.list_to_binary(c)) == true end

          {list, steps} =
            Shrink.shrink_sequence(:binary.bin_to_list(value), wrapped, max_steps, min_len, &Shrink.byte_walk/3)

          {:erlang.list_to_binary(list), steps}
        end
      end)
    end

    Gen.new(sampler, shrinker)
  end

  # n draws, in order; returns {list, rng}.
  defp draw_n(rng, n, draw) do
    {rev, rng} =
      Enum.reduce(1..n//1, {[], rng}, fn _, {acc, r} ->
        {v, r} = draw.(r)
        {[v | acc], r}
      end)

    {Enum.reverse(rev), rng}
  end

  @doc "Transforms generated values.  Drops shrinking: the function has no inverse."
  def map(%Gen{} = gen, fun) when is_function(fun, 1) do
    Gen.new(fn rng ->
      {v, rng} = Gen.sample(gen, rng)
      {fun.(v), rng}
    end)
  end

  @doc """
  Keeps only values for which `pred` returns `true`.  After exactly
  `max_retries` rejected samples sampling raises `ExhaustedError`.  Shrinking is
  preserved: a candidate failing the predicate is simply not a valid input.
  """
  def filter(%Gen{} = gen, max_retries \\ Limits.default_max_retries(), pred)
      when is_function(pred, 1) do
    Limits.check_range!(max_retries, 1, Limits.max_retries(), "max_retries")

    sampler = fn rng ->
      # Every retry inside a check is charged, so stacked filters (each retrying
      # 10,000 times) cannot multiply uncharged work: in a NESTED check a retry is
      # an attempt charged up the chain; in a top-level check it is one element of
      # work against the root's 64M allowance.
      budget = Budget.current()
      nested = budget != nil and Budget.nested?(budget)
      retry(gen, pred, rng, max_retries, max_retries, budget, nested)
    end

    shrinker = fn value, falsifies, max_steps ->
      # Charge BEFORE the user predicate runs: it is property code and may be costly.
      budget = Budget.current()

      Gen.shrink_nested(
        gen,
        value,
        fn c ->
          (budget == nil or Budget.charge(budget, 1)) and pred.(c) == true and falsifies.(c) == true
        end,
        max_steps
      )
    end

    Gen.new(sampler, shrinker)
  end

  defp retry(_gen, _pred, _rng, 0, max, _budget, _nested),
    do: raise(ExhaustedError, "filter rejected #{max} samples")

  defp retry(gen, pred, rng, left, max, budget, nested) do
    if budget != nil and not if(nested, do: Budget.charge(budget, 1), else: Budget.charge_copy(budget, 1)) do
      raise ExhaustedError, "the enclosing shrink budget was spent while filtering"
    end

    {v, rng} = Gen.sample(gen, rng)
    if pred.(v) == true, do: {v, rng}, else: retry(gen, pred, rng, left - 1, max, budget, nested)
  end

  @doc "Dependent generation: sample, then sample from the function's generator."
  def bind(%Gen{} = gen, fun) when is_function(fun, 1) do
    Gen.new(fn rng ->
      {v, rng} = Gen.sample(gen, rng)
      Gen.sample(fun.(v), rng)
    end)
  end

  def and_then(gen, fun), do: bind(gen, fun)

  @doc "Picks one generator uniformly (`bounded(len)`, then that arm's draws)."
  def one_of([]), do: raise(ArgumentError, "one_of needs at least one generator")

  def one_of(gens) when is_list(gens) do
    n = length(gens)

    Gen.new(fn rng ->
      {i, rng} = Prng.bounded(rng, n)
      Gen.sample(Enum.at(gens, i), rng)
    end)
  end

  @doc """
  A weighted choice among `{weight, gen}` pairs: `pick = bounded(total)`; arms
  are walked in order subtracting each weight, so a zero-weight arm can never
  be selected.  Empty, negative weight, zero total or total above 2**32 is
  refused.
  """
  def frequency([]), do: raise(ArgumentError, "frequency needs at least one arm")

  def frequency(arms) when is_list(arms) do
    total =
      Enum.reduce(arms, 0, fn {weight, _gen}, acc ->
        Limits.check_range!(weight, 0, Limits.max_frequency_total(), "frequency weight")
        sum = acc + weight

        if sum > Limits.max_frequency_total(),
          do: raise(ArgumentError, "frequency total weight exceeds 2**32")

        sum
      end)

    if total == 0, do: raise(ArgumentError, "frequency total weight must be > 0")

    Gen.new(fn rng ->
      {pick, rng} = Prng.bounded(rng, total)
      choose(arms, pick, rng)
    end)
  end

  defp choose([{weight, gen} | _], pick, rng) when pick < weight, do: Gen.sample(gen, rng)
  defp choose([{weight, _} | rest], pick, rng), do: choose(rest, pick - weight, rng)

  @doc """
  A list: `L = min_len + bounded(span)`, then `L` elements in order.  Shrinks by
  sequence shrinking with the element generator's shrinker for the element-wise
  phase.
  """
  def list(%Gen{} = elem, min_len, max_len) do
    check_lengths!(min_len, max_len, Limits.max_list_len(), "list")
    span = max_len - min_len + 1

    sampler = fn rng ->
      {off, rng} = Prng.bounded(rng, span)
      draw_n(rng, min_len + off, fn r -> Gen.sample(elem, r) end)
    end

    shrinker = fn value, falsifies, max_steps ->
      elem_shrinker = fn v, f, n -> Gen.shrink_nested(elem, v, f, n) end
      Shrink.shrink_sequence(value, falsifies, max_steps, min_len, elem_shrinker)
    end

    Gen.new(sampler, shrinker)
  end

  # One shrink-wide evaluation budget for a tuple: the repeat-until-no-progress
  # loop calls each component shrinker once per pass, so without a shared counter
  # the work is quadratic in max_steps.
  defp global_budget(falsifies, max_steps) do
    budget = Limits.attempt_budget(max_steps)
    spent = :atomics.new(1, signed: true)

    fn v -> :atomics.add_get(spent, 1, 1) <= budget and falsifies.(v) == true end
  end

  @doc """
  Samples each generator left to right (a tuple) and shrinks one coordinate at a
  time, in repeated passes until a pass makes no progress.
  """
  def tuple([]), do: raise(ArgumentError, "tuple needs at least one generator")

  def tuple(gens) when is_list(gens) do
    if length(gens) > Limits.max_tuple_arity(),
      do: raise(ArgumentError, "tuple arity exceeds #{Limits.max_tuple_arity()}")

    sampler = fn rng ->
      {list, rng} = draw_n_over(rng, gens)
      {List.to_tuple(list), rng}
    end

    shrinker = fn value, raw_falsifies, max_steps ->
      falsifies = global_budget(raw_falsifies, max_steps)
      {current, steps} = tuple_passes(gens, Tuple.to_list(value), 0, max_steps, falsifies)
      {List.to_tuple(current), steps}
    end

    Gen.new(sampler, shrinker)
  end

  defp draw_n_over(rng, gens) do
    {rev, rng} =
      Enum.reduce(gens, {[], rng}, fn g, {acc, r} ->
        {v, r} = Gen.sample(g, r)
        {[v | acc], r}
      end)

    {Enum.reverse(rev), rng}
  end

  defp tuple_passes(gens, current, steps, max_steps, falsifies) do
    if steps < max_steps do
      {current2, steps2, progress} =
        gens
        |> Enum.with_index()
        |> Enum.reduce({current, steps, false}, fn {gen, index}, {cur, st, prog} ->
          remaining = max_steps - st

          if remaining > 0 do
            slot = fn candidate -> falsifies.(List.to_tuple(List.replace_at(cur, index, candidate))) end
            {shrunk, used} = Gen.shrink_nested(gen, Enum.at(cur, index), slot, remaining)

            if used > 0,
              do: {List.replace_at(cur, index, shrunk), st + used, true},
              else: {cur, st, prog}
          else
            {cur, st, prog}
          end
        end)

      if progress, do: tuple_passes(gens, current2, steps2, max_steps, falsifies), else: {current2, steps2}
    else
      {current, steps}
    end
  end

  # -- corpus entry points (pbt-v1) ------------------------------------------------------

  @doc "`count` primitives of `type_name` (`\"bool\"`, `\"u8\"`, `\"u64\"`, `\"i128\"`) from `seed`."
  def gen_primitive(seed, type_name, count) do
    Limits.check_count!(count)

    sampler =
      case type_name do
        "bool" -> fn r -> Gen.sample(bool(), r) end
        "u8" -> fn r -> Prng.bounded(r, 256) end
        "u64" -> &Prng.next_u64/1
        "i128" -> &sample_i128/1
        _ -> raise ArgumentError, "unknown primitive type"
      end

    {list, _} = draw_n(Hooks.get(:new_rng).(seed), count, sampler)
    list
  end

  @doc "`count` uniform integers over `[lo, hi]` from `seed`."
  def gen_bounded(seed, lo, hi, count) do
    unless is_integer(lo) and is_integer(hi), do: raise(ArgumentError, "bounds must be integers")
    if lo > hi, do: raise(ArgumentError, "need lo <= hi")
    Limits.check_count!(count)

    # Each draw is about as wide as the span; count x width bounds what the
    # returned list can hold.
    width = max(1, div(Prng.bit_length(hi - lo) + 7, 8))

    if count * width > Limits.max_gen_bytes_total(),
      do: raise(ArgumentError, "count #{count} x #{width}-byte span exceeds the generation budget")

    {list, _} = draw_n(Hooks.get(:new_rng).(seed), count, fn r -> bounded_int(r, lo, hi) end)
    list
  end

  @doc "`count` byte strings with length in `[min_len, max_len]` from `seed`."
  def gen_bytes(seed, min_len, max_len, count) do
    check_lengths!(min_len, max_len, Limits.max_input_bytes(), "gen_bytes")
    # applies even when max_len == 0
    Limits.check_count!(count)

    if max_len > 0 and count > div(Limits.max_gen_bytes_total(), max_len),
      do: raise(ArgumentError, "count #{count} x max_len #{max_len} exceeds the byte budget")

    gen = bytes(min_len, max_len)
    {list, _} = draw_n(Hooks.get(:new_rng).(seed), count, fn r -> Gen.sample(gen, r) end)
    list
  end
end
