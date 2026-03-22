defmodule CodingAdventures.LogicGates.Gates do
  @moduledoc """
  Logic Gates — the foundation of all digital computing.

  ## What is a logic gate?

  A logic gate takes one or two binary inputs (0 or 1) and produces one
  binary output (0 or 1). The output is determined entirely by the input —
  no state, no memory, no randomness.

  This module implements the seven fundamental gates, proves that all of them
  can be built from a single gate type (NAND), and provides multi-input variants.

  ## The Four Fundamental Gates

  These are the building blocks. NOT, AND, OR, and XOR are the four gates
  from which all other gates (and all of digital logic) can be constructed.

  Each gate is defined by its "truth table" — an exhaustive listing of
  every possible input combination and the corresponding output. Since each
  input can only be 0 or 1, a two-input gate has exactly 4 possible input
  combinations (2 x 2 = 4), making it easy to verify correctness.
  """

  # ---------------------------------------------------------------------------
  # Input validation
  # ---------------------------------------------------------------------------
  # Every gate checks that its inputs are valid binary values (0 or 1).
  # We use guard clauses to enforce this — Elixir's pattern matching and
  # guards are perfect for this kind of type-level constraint. If someone
  # passes true/false, a float, or an integer outside {0, 1}, we raise
  # an ArgumentError with a clear message.

  defguardp is_bit(value) when value === 0 or value === 1

  @doc false
  def validate_bit!(value, name \\ "input") do
    cond do
      is_boolean(value) ->
        raise ArgumentError, "#{name} must be an integer, got boolean #{inspect(value)}"

      not is_integer(value) ->
        raise ArgumentError, "#{name} must be an integer, got #{inspect(value)}"

      value not in [0, 1] ->
        raise ArgumentError, "#{name} must be 0 or 1, got #{value}"

      true ->
        :ok
    end
  end

  # ===========================================================================
  # THE FOUR FUNDAMENTAL GATES
  # ===========================================================================

  @doc """
  The NOT gate (also called an "inverter").

  NOT is the simplest gate — it has one input and flips it.
  If the input is 0, the output is 1. If the input is 1, the output is 0.

  Truth table:

      Input │ Output
      ──────┼───────
        0   │   1
        1   │   0

  In hardware, a NOT gate is a single CMOS inverter: one PMOS transistor
  and one NMOS transistor. When the input is HIGH, the NMOS conducts and
  pulls the output LOW. When the input is LOW, the PMOS conducts and
  pulls the output HIGH.

  Real-world analogy: a light switch that is wired "backwards" — when
  you flip it UP, the light turns OFF, and vice versa.

  ## Examples

      iex> CodingAdventures.LogicGates.Gates.not_gate(0)
      1
      iex> CodingAdventures.LogicGates.Gates.not_gate(1)
      0
  """
  @spec not_gate(0 | 1) :: 0 | 1
  def not_gate(a) when is_bit(a) do
    # Elixir's Bitwise XOR with 1 flips the bit:
    #   0 XOR 1 = 1
    #   1 XOR 1 = 0
    Bitwise.bxor(a, 1)
  end

  def not_gate(a), do: validate_bit!(a, "a") && raise("unreachable")

  @doc """
  The AND gate.

  AND outputs 1 only when BOTH inputs are 1. If either input is 0,
  the output is 0. Think of it as multiplication: 0 x anything = 0.

  Truth table:

      A  B │ Output
      ─────┼───────
      0  0 │   0
      0  1 │   0
      1  0 │   0
      1  1 │   1

  In hardware, a CMOS AND gate is actually a NAND gate followed by a
  NOT gate — it takes 6 transistors (4 for NAND + 2 for NOT). This is
  because NAND is the "natural" gate in CMOS technology.

  Real-world analogy: two light switches in SERIES. Both switches must
  be ON for the light to turn on. If either switch is off, no current
  flows and the light stays off.

  ## Examples

      iex> CodingAdventures.LogicGates.Gates.and_gate(1, 1)
      1
      iex> CodingAdventures.LogicGates.Gates.and_gate(1, 0)
      0
  """
  @spec and_gate(0 | 1, 0 | 1) :: 0 | 1
  def and_gate(a, b) when is_bit(a) and is_bit(b) do
    Bitwise.band(a, b)
  end

  def and_gate(a, b) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
  end

  @doc """
  The OR gate.

  OR outputs 1 if EITHER input (or both) is 1. The only way to get 0
  is if both inputs are 0.

  Truth table:

      A  B │ Output
      ─────┼───────
      0  0 │   0
      0  1 │   1
      1  0 │   1
      1  1 │   1

  In hardware, a CMOS OR gate is a NOR gate followed by a NOT gate —
  6 transistors total (4 for NOR + 2 for NOT).

  Real-world analogy: two light switches in PARALLEL. If either switch
  is ON, current can flow through that path and the light turns on.
  The light only goes off when both switches are OFF.

  ## Examples

      iex> CodingAdventures.LogicGates.Gates.or_gate(0, 0)
      0
      iex> CodingAdventures.LogicGates.Gates.or_gate(0, 1)
      1
  """
  @spec or_gate(0 | 1, 0 | 1) :: 0 | 1
  def or_gate(a, b) when is_bit(a) and is_bit(b) do
    Bitwise.bor(a, b)
  end

  def or_gate(a, b) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
  end

  @doc """
  The XOR gate (exclusive OR).

  XOR outputs 1 if the inputs are DIFFERENT. If both inputs are the
  same (both 0 or both 1), the output is 0.

  Truth table:

      A  B │ Output
      ─────┼───────
      0  0 │   0
      0  1 │   1
      1  0 │   1
      1  1 │   0

  Why XOR matters for arithmetic:
  In binary addition, 1 + 1 = 10 (that's "one-zero" in binary, which
  equals 2 in decimal). The sum digit is 0 and the carry is 1.
  Notice that the sum digit (0) is exactly what XOR(1, 1) produces!

  This is no coincidence — XOR is the "sum without carry" operation,
  which is why the half adder uses XOR for its sum output.

  ## Examples

      iex> CodingAdventures.LogicGates.Gates.xor_gate(0, 1)
      1
      iex> CodingAdventures.LogicGates.Gates.xor_gate(1, 1)
      0
  """
  @spec xor_gate(0 | 1, 0 | 1) :: 0 | 1
  def xor_gate(a, b) when is_bit(a) and is_bit(b) do
    Bitwise.bxor(a, b)
  end

  def xor_gate(a, b) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
  end

  # ===========================================================================
  # THE THREE COMPOSITE GATES
  # ===========================================================================
  #
  # These gates are each defined as the NOT of a fundamental gate.
  # They're important because NAND and NOR are each individually
  # "functionally complete" — you can build ANY other gate from
  # just NAND gates alone, or just NOR gates alone.

  @doc """
  The NAND gate (NOT-AND).

  NAND is the opposite of AND: it outputs 0 only when both inputs
  are 1, and outputs 1 for all other combinations.

  Truth table:

      A  B │ Output
      ─────┼───────
      0  0 │   1
      0  1 │   1
      1  0 │   1
      1  1 │   0

  NAND is special because it is "functionally complete" — every other
  gate can be built from NAND gates alone. In real hardware, chips are
  often built entirely from NAND gates because they are the cheapest
  to manufacture in CMOS technology (only 4 transistors).

  ## Examples

      iex> CodingAdventures.LogicGates.Gates.nand_gate(1, 1)
      0
      iex> CodingAdventures.LogicGates.Gates.nand_gate(0, 1)
      1
  """
  @spec nand_gate(0 | 1, 0 | 1) :: 0 | 1
  def nand_gate(a, b) when is_bit(a) and is_bit(b) do
    not_gate(and_gate(a, b))
  end

  def nand_gate(a, b) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
  end

  @doc """
  The NOR gate (NOT-OR).

  NOR is the opposite of OR: it outputs 1 only when both inputs are 0,
  and outputs 0 for all other combinations.

  Truth table:

      A  B │ Output
      ─────┼───────
      0  0 │   1
      0  1 │   0
      1  0 │   0
      1  1 │   0

  Like NAND, NOR is also functionally complete. Additionally, NOR gates
  are the basis of the SR latch — the simplest memory element.

  ## Examples

      iex> CodingAdventures.LogicGates.Gates.nor_gate(0, 0)
      1
      iex> CodingAdventures.LogicGates.Gates.nor_gate(1, 0)
      0
  """
  @spec nor_gate(0 | 1, 0 | 1) :: 0 | 1
  def nor_gate(a, b) when is_bit(a) and is_bit(b) do
    not_gate(or_gate(a, b))
  end

  def nor_gate(a, b) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
  end

  @doc """
  The XNOR gate (NOT-XOR, also called "equivalence gate").

  XNOR outputs 1 when both inputs are the SAME, and 0 when they
  are different. It is the opposite of XOR.

  Truth table:

      A  B │ Output
      ─────┼───────
      0  0 │   1
      0  1 │   0
      1  0 │   0
      1  1 │   1

  XNOR is sometimes called the "equality gate" because it outputs 1
  precisely when A equals B. This makes it useful for building
  comparators — circuits that check if two numbers are equal.

  ## Examples

      iex> CodingAdventures.LogicGates.Gates.xnor_gate(0, 0)
      1
      iex> CodingAdventures.LogicGates.Gates.xnor_gate(0, 1)
      0
  """
  @spec xnor_gate(0 | 1, 0 | 1) :: 0 | 1
  def xnor_gate(a, b) when is_bit(a) and is_bit(b) do
    not_gate(xor_gate(a, b))
  end

  def xnor_gate(a, b) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
  end

  # ===========================================================================
  # NAND-DERIVED GATES — Proving Functional Completeness
  # ===========================================================================
  #
  # The following functions build NOT, AND, OR, and XOR using ONLY NAND
  # gates. This proves that NAND is functionally complete — any Boolean
  # function can be implemented using nothing but NAND gates.
  #
  # In real chip design, this is not just theory. Many ASIC (Application-
  # Specific Integrated Circuit) designs use NAND-only standard cell
  # libraries because NAND gates have the best speed/area tradeoff in
  # CMOS technology.

  @doc """
  NOT built from NAND only.

      NOT(a) = NAND(a, a)

  When both inputs of a NAND gate are the same value:
    NAND(0, 0) = 1   (NOT 0 = 1 ✓)
    NAND(1, 1) = 0   (NOT 1 = 0 ✓)

  ## Examples

      iex> CodingAdventures.LogicGates.Gates.nand_not(0)
      1
      iex> CodingAdventures.LogicGates.Gates.nand_not(1)
      0
  """
  @spec nand_not(0 | 1) :: 0 | 1
  def nand_not(a) when is_bit(a), do: nand_gate(a, a)
  def nand_not(a), do: validate_bit!(a, "a") && raise("unreachable")

  @doc """
  AND built from NAND only.

      AND(a, b) = NOT(NAND(a, b)) = NAND(NAND(a, b), NAND(a, b))

  Uses 2 NAND gates. First we compute NAND(a, b), then we invert it
  using the NAND-NOT trick (feeding the same value to both inputs).

  ## Examples

      iex> CodingAdventures.LogicGates.Gates.nand_and(1, 1)
      1
      iex> CodingAdventures.LogicGates.Gates.nand_and(0, 1)
      0
  """
  @spec nand_and(0 | 1, 0 | 1) :: 0 | 1
  def nand_and(a, b) when is_bit(a) and is_bit(b) do
    nand_not(nand_gate(a, b))
  end

  def nand_and(a, b) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
  end

  @doc """
  OR built from NAND only.

      OR(a, b) = NAND(NOT(a), NOT(b)) = NAND(NAND(a, a), NAND(b, b))

  Uses 3 NAND gates. By De Morgan's law: NOT(NOT(a) AND NOT(b)) = a OR b.
  Since NAND = NOT(AND), we get: NAND(NOT(a), NOT(b)) = OR(a, b).

  ## Examples

      iex> CodingAdventures.LogicGates.Gates.nand_or(0, 0)
      0
      iex> CodingAdventures.LogicGates.Gates.nand_or(0, 1)
      1
  """
  @spec nand_or(0 | 1, 0 | 1) :: 0 | 1
  def nand_or(a, b) when is_bit(a) and is_bit(b) do
    nand_gate(nand_not(a), nand_not(b))
  end

  def nand_or(a, b) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
  end

  @doc """
  XOR built from NAND only.

      XOR(a, b) = NAND(NAND(a, NAND(a, b)), NAND(b, NAND(a, b)))

  Uses 4 NAND gates. This is the classic NAND-only XOR construction.

  ## Examples

      iex> CodingAdventures.LogicGates.Gates.nand_xor(0, 1)
      1
      iex> CodingAdventures.LogicGates.Gates.nand_xor(1, 1)
      0
  """
  @spec nand_xor(0 | 1, 0 | 1) :: 0 | 1
  def nand_xor(a, b) when is_bit(a) and is_bit(b) do
    nand_ab = nand_gate(a, b)
    nand_gate(nand_gate(a, nand_ab), nand_gate(b, nand_ab))
  end

  def nand_xor(a, b) do
    validate_bit!(a, "a")
    validate_bit!(b, "b")
  end

  # ===========================================================================
  # MULTI-INPUT VARIANTS
  # ===========================================================================
  #
  # Real circuits often need to AND or OR more than two signals together.
  # For example, a 4-input AND gate outputs 1 only when ALL four inputs
  # are 1. We implement these using Elixir's Enum.reduce, which chains
  # the two-input gate across all inputs.

  @doc """
  AND with N inputs.

  Returns 1 only if ALL inputs are 1. This is equivalent to chaining
  two-input AND gates:

      AND_N(a, b, c) = AND(AND(a, b), c)

  Raises ArgumentError if fewer than 2 inputs are provided.

  ## Examples

      iex> CodingAdventures.LogicGates.Gates.and_n([1, 1, 1])
      1
      iex> CodingAdventures.LogicGates.Gates.and_n([1, 1, 0])
      0
  """
  @spec and_n([0 | 1, ...]) :: 0 | 1
  def and_n(inputs) when is_list(inputs) and length(inputs) >= 2 do
    Enum.each(inputs, &validate_bit!(&1, "input"))
    Enum.reduce(inputs, &and_gate/2)
  end

  def and_n(inputs) when is_list(inputs) do
    raise ArgumentError, "and_n requires at least 2 inputs, got #{length(inputs)}"
  end

  @doc """
  OR with N inputs.

  Returns 1 if ANY input is 1. This is equivalent to chaining
  two-input OR gates:

      OR_N(a, b, c) = OR(OR(a, b), c)

  Raises ArgumentError if fewer than 2 inputs are provided.

  ## Examples

      iex> CodingAdventures.LogicGates.Gates.or_n([0, 0, 1])
      1
      iex> CodingAdventures.LogicGates.Gates.or_n([0, 0, 0])
      0
  """
  @spec or_n([0 | 1, ...]) :: 0 | 1
  def or_n(inputs) when is_list(inputs) and length(inputs) >= 2 do
    Enum.each(inputs, &validate_bit!(&1, "input"))
    Enum.reduce(inputs, &or_gate/2)
  end

  def or_n(inputs) when is_list(inputs) do
    raise ArgumentError, "or_n requires at least 2 inputs, got #{length(inputs)}"
  end
end
