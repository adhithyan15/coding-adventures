defmodule CodingAdventures.Rng do
  @moduledoc """
  Three classic pseudorandom number generators.

  ## Algorithms

  - `CodingAdventures.Rng.LCG` — Linear Congruential Generator (Knuth 1948).
    State: `state = (state × a + c) mod 2^64`. Fast, full period, but
    consecutive outputs are linearly correlated.

  - `CodingAdventures.Rng.Xorshift64` — Marsaglia (2003) XOR-shift generator.
    Three XOR-shift operations on 64-bit state; no multiplication. Period
    `2^64 − 1`. Seed 0 is replaced with 1.

  - `CodingAdventures.Rng.PCG32` — Permuted Congruential Generator (O'Neill 2014).
    Same LCG recurrence plus an XSH RR output permutation. Passes all known
    statistical test suites with only 8 bytes of state.

  ## API

  Every generator module exposes the same functional interface.  Functions
  return `{value, new_generator}` tuples so that generators compose cleanly
  in pipelines and remain pure (no mutable state, no process dictionary).

      {:ok, g} = CodingAdventures.Rng.LCG.new(42)
      {v, g}   = CodingAdventures.Rng.LCG.next_u32(g)    # v in [0, 2^32)
      {u, g}   = CodingAdventures.Rng.LCG.next_u64(g)    # u in [0, 2^64)
      {f, g}   = CodingAdventures.Rng.LCG.next_float(g)  # f in [0.0, 1.0)
      {n, g}   = CodingAdventures.Rng.LCG.next_int_in_range(g, 1, 6) # n in [1,6]

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.
  """

  # Shared constants macro ─────────────────────────────────────────────────────
  #
  # Using `__using__` lets every submodule inherit the constants without
  # duplicating them.  A caller does `use CodingAdventures.Rng` (or, from
  # within the file, just references the parent module's constants directly via
  # the fully-qualified name).
  #
  # Constants:
  #   @multiplier   — Knuth / Numerical Recipes LCG multiplier (64-bit)
  #   @increment    — LCG additive increment (must be odd for full period)
  #   @mask64       — 64-bit all-ones mask; used to simulate uint64 overflow
  #   @mask32       — 32-bit all-ones mask; used to extract lower 32 bits
  #   @float_div    — 2^32 as a float; divides u32 into [0.0, 1.0)

  defmacro __using__(_opts) do
    quote do
      import Bitwise

      @multiplier 6_364_136_223_846_793_005
      @increment  1_442_695_040_888_963_407
      @mask64     0xFFFF_FFFF_FFFF_FFFF
      @mask32     0xFFFF_FFFF
      @float_div  4_294_967_296.0
    end
  end
end

# ── LCG ───────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.Rng.LCG do
  @moduledoc """
  Linear Congruential Generator (Knuth 1948).

  State recurrence (mod 2^64):

      state = (state × a + c) mod 2^64

  where `a = 6_364_136_223_846_793_005` and `c = 1_442_695_040_888_963_407`.
  These satisfy the Hull-Dobell theorem: full period 2^64.

  Output: upper 32 bits of state.  Lower bits have shorter sub-periods so
  taking the top half gives better statistical quality.

  Reference values for seed=1 (first three `next_u32` outputs):

      [1_412_771_199, 1_791_099_446, 124_312_908]  # PCG32
      [1_817_669_548, 2_187_888_307, 2_784_682_393] # LCG

  """

  use CodingAdventures.Rng

  # Struct ─────────────────────────────────────────────────────────────────────

  @typedoc "LCG generator state"
  @type t :: %__MODULE__{state: non_neg_integer()}

  defstruct [:state]

  # Constructor ─────────────────────────────────────────────────────────────────

  @doc """
  Create a new LCG seeded with `seed`.  Any non-negative integer is valid.

      {:ok, g} = CodingAdventures.Rng.LCG.new(42)
  """
  @spec new(non_neg_integer()) :: {:ok, t()}
  def new(seed) do
    {:ok, %__MODULE__{state: band(seed, @mask64)}}
  end

  # next_u32 ────────────────────────────────────────────────────────────────────

  @doc """
  Advance the LCG state and return `{upper_32_bits, new_lcg}`.

  The multiply-add is masked to 64 bits after each step to emulate unsigned
  64-bit wraparound (Elixir integers are arbitrary precision).
  """
  @spec next_u32(t()) :: {non_neg_integer(), t()}
  def next_u32(%__MODULE__{state: s} = g) do
    new_state = band(s * @multiplier + @increment, @mask64)
    value     = bsr(new_state, 32)
    {value, %__MODULE__{g | state: new_state}}
  end

  # next_u64 ────────────────────────────────────────────────────────────────────

  @doc """
  Return a 64-bit value composed of two consecutive `next_u32` calls.

  The high word goes into bits [63:32] and the low word into [31:0].
  """
  @spec next_u64(t()) :: {non_neg_integer(), t()}
  def next_u64(g) do
    {hi, g1} = next_u32(g)
    {lo, g2} = next_u32(g1)
    {band(bor(bsl(hi, 32), lo), @mask64), g2}
  end

  # next_float ──────────────────────────────────────────────────────────────────

  @doc """
  Return a `Float` uniformly distributed in `[0.0, 1.0)`.
  """
  @spec next_float(t()) :: {float(), t()}
  def next_float(g) do
    {u, g1} = next_u32(g)
    {u / @float_div, g1}
  end

  # next_int_in_range ───────────────────────────────────────────────────────────

  @doc """
  Return a uniform random integer in `[min, max]` inclusive.

  Uses rejection sampling to eliminate modulo bias.  The threshold is:

      threshold = rem(rem(-range_size, 2^32) + 2^32, 2^32)

  Any draw below `threshold` is discarded.  Expected extra draws per call
  is less than 2 for all range sizes.
  """
  @spec next_int_in_range(t(), integer(), integer()) :: {integer(), t()}
  def next_int_in_range(g, min_val, max_val) when min_val <= max_val do
    range_size = max_val - min_val + 1
    threshold  = rem(rem(-range_size, 1 <<< 32) + (1 <<< 32), 1 <<< 32)
    threshold  = rem(threshold, range_size)
    do_sample(g, min_val, range_size, threshold)
  end

  def next_int_in_range(_gen, min_val, max_val) do
    raise ArgumentError, "next_int_in_range requires min_val <= max_val, got #{min_val} > #{max_val}"
  end

  defp do_sample(g, min, range_size, threshold) do
    {r, g1} = next_u32(g)
    if r >= threshold do
      {min + rem(r, range_size), g1}
    else
      do_sample(g1, min, range_size, threshold)
    end
  end
end

# ── Xorshift64 ────────────────────────────────────────────────────────────────

defmodule CodingAdventures.Rng.Xorshift64 do
  @moduledoc """
  Marsaglia (2003) XOR-shift generator.

  Three XOR-shift operations scramble 64-bit state with no multiplication:

      x ^= x <<< 13   (left shift, mask to 64 bits)
      x ^= x >>> 7    (logical right shift)
      x ^= x <<< 17   (left shift, mask to 64 bits)

  Period: `2^64 − 1`.  State 0 is a fixed point (XOR-ing zero never changes
  it), so seed 0 is replaced with 1.

  Output: lower 32 bits of state.

  Reference values for seed=1 (first three `next_u32` outputs):

      [1_082_269_761, 201_397_313, 1_854_285_353]
  """

  use CodingAdventures.Rng

  @typedoc "Xorshift64 generator state"
  @type t :: %__MODULE__{state: non_neg_integer()}

  defstruct [:state]

  @doc """
  Create a new Xorshift64 seeded with `seed`.  Seed 0 is replaced with 1.

      {:ok, g} = CodingAdventures.Rng.Xorshift64.new(1)
  """
  @spec new(non_neg_integer()) :: {:ok, t()}
  def new(0),    do: {:ok, %__MODULE__{state: 1}}
  def new(seed), do: {:ok, %__MODULE__{state: band(seed, @mask64)}}

  @doc """
  Apply the three XOR-shifts and return `{lower_32_bits, new_xorshift64}`.
  """
  @spec next_u32(t()) :: {non_neg_integer(), t()}
  def next_u32(%__MODULE__{state: x} = g) do
    x = band(bxor(x, bsl(x, 13)), @mask64)
    x = bxor(x, bsr(x, 7))
    x = band(bxor(x, bsl(x, 17)), @mask64)
    {band(x, @mask32), %__MODULE__{g | state: x}}
  end

  @doc "Return a 64-bit value from two consecutive `next_u32` calls."
  @spec next_u64(t()) :: {non_neg_integer(), t()}
  def next_u64(g) do
    {hi, g1} = next_u32(g)
    {lo, g2} = next_u32(g1)
    {band(bor(bsl(hi, 32), lo), @mask64), g2}
  end

  @doc "Return a `Float` in `[0.0, 1.0)`."
  @spec next_float(t()) :: {float(), t()}
  def next_float(g) do
    {u, g1} = next_u32(g)
    {u / @float_div, g1}
  end

  @doc "Return a uniform integer in `[min, max]` using rejection sampling."
  @spec next_int_in_range(t(), integer(), integer()) :: {integer(), t()}
  def next_int_in_range(g, min_val, max_val) when min_val <= max_val do
    range_size = max_val - min_val + 1
    threshold  = rem(rem(-range_size, 1 <<< 32) + (1 <<< 32), 1 <<< 32)
    threshold  = rem(threshold, range_size)
    do_sample(g, min_val, range_size, threshold)
  end

  def next_int_in_range(_gen, min_val, max_val) do
    raise ArgumentError, "next_int_in_range requires min_val <= max_val, got #{min_val} > #{max_val}"
  end

  defp do_sample(g, min, range_size, threshold) do
    {r, g1} = next_u32(g)
    if r >= threshold do
      {min + rem(r, range_size), g1}
    else
      do_sample(g1, min, range_size, threshold)
    end
  end
end

# ── PCG32 ─────────────────────────────────────────────────────────────────────

defmodule CodingAdventures.Rng.PCG32 do
  @moduledoc """
  Permuted Congruential Generator, 32-bit output (O'Neill 2014).

  Uses the same LCG recurrence as `LCG` but applies an XSH RR output
  permutation before returning any bits.  The permutation breaks the linear
  correlation that makes plain LCG weak.

  ## Output permutation (XSH RR) applied to `old_state`:

  1. `xorshifted = ((old_state >>> 18) bxor old_state) >>> 27`
     Mix bits [63:18] down through [45:0], then keep the top 32 of the
     result by shifting right 27 more positions.

  2. `rot = old_state >>> 59`
     Take the top 5 bits as a rotation amount in `[0, 31]`.

  3. `output = rotr32(xorshifted, rot)`
     Right-rotate `xorshifted` by `rot` positions.

  ## Initialisation ("initseq" warm-up):

  1. Start from `state = 0`.
  2. Advance once (mixes the increment into state).
  3. `state = state + seed` (inject seed).
  4. Advance once more (scatters seed bits throughout state).

  Reference values for seed=1 (first three `next_u32` outputs):

      [1_412_771_199, 1_791_099_446, 124_312_908]
  """

  use CodingAdventures.Rng

  # PCG_INCREMENT must be odd for full period; @increment is already odd.
  @pcg_increment bor(1_442_695_040_888_963_407, 1)

  @typedoc "PCG32 generator state"
  @type t :: %__MODULE__{state: non_neg_integer(), increment: non_neg_integer()}

  defstruct [:state, :increment]

  @doc """
  Create a new PCG32 seeded with `seed` using the two-step warm-up.

      {:ok, g} = CodingAdventures.Rng.PCG32.new(1)
  """
  @spec new(non_neg_integer()) :: {:ok, t()}
  def new(seed) do
    g0 = %__MODULE__{state: 0, increment: @pcg_increment}
    # Step 1: advance once to mix the increment in.
    g1 = lcg_advance(g0)
    # Step 2: inject the seed.
    g2 = %__MODULE__{g1 | state: band(g1.state + band(seed, @mask64), @mask64)}
    # Step 3: advance once more to scatter seed bits.
    g3 = lcg_advance(g2)
    {:ok, g3}
  end

  @doc """
  Advance state and return the XSH RR permuted 32-bit output.

  The permutation is applied to `old_state` (before advancing), so the
  output depends on the state that was just consumed — not the next state.
  """
  @spec next_u32(t()) :: {non_neg_integer(), t()}
  def next_u32(%__MODULE__{state: old_state} = g) do
    g1 = lcg_advance(g)

    # XSH RR: mix high bits down, then rotate right by the top-5 rotation.
    xorshifted = band(bsr(bxor(bsr(old_state, 18), old_state), 27), @mask32)
    rot        = bsr(old_state, 59)
    output     = band(
      bor(bsr(xorshifted, rot), band(bsl(xorshifted, band(32 - rot, 31)), @mask32)),
      @mask32
    )

    {output, g1}
  end

  @doc "Return a 64-bit value from two consecutive `next_u32` calls."
  @spec next_u64(t()) :: {non_neg_integer(), t()}
  def next_u64(g) do
    {hi, g1} = next_u32(g)
    {lo, g2} = next_u32(g1)
    {band(bor(bsl(hi, 32), lo), @mask64), g2}
  end

  @doc "Return a `Float` in `[0.0, 1.0)`."
  @spec next_float(t()) :: {float(), t()}
  def next_float(g) do
    {u, g1} = next_u32(g)
    {u / @float_div, g1}
  end

  @doc "Return a uniform integer in `[min, max]` using rejection sampling."
  @spec next_int_in_range(t(), integer(), integer()) :: {integer(), t()}
  def next_int_in_range(g, min_val, max_val) when min_val <= max_val do
    range_size = max_val - min_val + 1
    threshold  = rem(rem(-range_size, 1 <<< 32) + (1 <<< 32), 1 <<< 32)
    threshold  = rem(threshold, range_size)
    do_sample(g, min_val, range_size, threshold)
  end

  def next_int_in_range(_gen, min_val, max_val) do
    raise ArgumentError, "next_int_in_range requires min_val <= max_val, got #{min_val} > #{max_val}"
  end

  defp do_sample(g, min, range_size, threshold) do
    {r, g1} = next_u32(g)
    if r >= threshold do
      {min + rem(r, range_size), g1}
    else
      do_sample(g1, min, range_size, threshold)
    end
  end

  # One step of the internal LCG recurrence that drives PCG32's state.
  defp lcg_advance(%__MODULE__{state: s, increment: inc} = g) do
    %__MODULE__{g | state: band(s * @multiplier + inc, @mask64)}
  end
end
