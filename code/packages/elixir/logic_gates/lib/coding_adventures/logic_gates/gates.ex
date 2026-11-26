defmodule CodingAdventures.LogicGates.Gates do
  @moduledoc """
  Logic Gates — the foundation of all digital computing.

  ## What is a logic gate?

  A logic gate takes one or two binary inputs (0 or 1) and produces one
  binary output (0 or 1). The output is determined entirely by the input —
  no state, no memory, no randomness.

  This module implements the seven fundamental gates, proves that all of them
  can be built from a single gate type (NAND), and provides multi-input variants.

  ## Physical implementation via CMOS transistors

  The seven primitive gate functions (not_gate, and_gate, or_gate, xor_gate,
  nand_gate, nor_gate, xnor_gate) now delegate their digital evaluation to
  `CodingAdventures.Transistors.CMOSGates`. This reflects the physical reality
  that logic gates are built from transistor pairs:

      NOT  → inverter_evaluate_digital/1  (2 transistors)
      NAND → nand_evaluate_digital/2      (4 transistors)
      NOR  → nor_evaluate_digital/2       (4 transistors)
      AND  → and_evaluate_digital/2       (6 transistors = NAND + inverter)
      OR   → or_evaluate_digital/2        (6 transistors = NOR + inverter)
      XOR  → xor_evaluate_digital/2        (16 transistors = 4 NANDs)
      XNOR → xnor_evaluate_digital/2       (18 transistors = XOR + Inverter)

  The public API is unchanged — inputs and outputs are still 0/1 integers.

  ## The Four Fundamental Gates

  These are the building blocks. NOT, AND, OR, and XOR are the four gates
  from which all other gates (and all of digital logic) can be constructed.

  Each gate is defined by its "truth table" — an exhaustive listing of
  every possible input combination and the corresponding output. Since each
  input can only be 0 or 1, a two-input gate has exactly 4 possible input
  combinations (2 x 2 = 4), making it easy to verify correctness.
  """

  alias CodingAdventures.LogicGates.InvalidBitError
  alias CodingAdventures.Transistors.CMOSGates

  # ---------------------------------------------------------------------------
  # Input validation
  # ---------------------------------------------------------------------------
  # A bit is valid if and only if it is the integer 0 or the integer 1,
  # compared with strict `===`. Strictness matters in Elixir because
  # `1.0 == 1` is true while `1.0 === 1` is false: a float is never a bit.
  # Booleans are atoms (`true`/`false`), so they are rejected too, as are
  # nil, binaries, charlists, lists, tuples, maps, structs, PIDs, refs and
  # functions.
  #
  # Rejection raises `CodingAdventures.LogicGates.InvalidBitError`, which
  # carries the NAME of the offending parameter. Every gate validates each of
  # its own arguments (left to right) and reports the FIRST invalid one under
  # its own parameter name -- NAND-built gates never leak an inner gate's name.

  defguardp is_bit(value) when is_integer(value) and (value === 0 or value === 1)

  @doc false
  @spec validate_bit!(term(), String.t()) :: :ok
  def validate_bit!(value, param \\ "input")

  def validate_bit!(value, _param) when is_bit(value), do: :ok

  def validate_bit!(value, param) do
    raise InvalidBitError, param: param, value_description: describe(value)
  end

  # A bounded, side-effect-free description of a rejected value.
  #
  # We deliberately avoid `inspect/2` and `to_string/1`: both can dispatch to
  # user-defined protocol implementations (Inspect / String.Chars) that may
  # raise or run arbitrary code, and `limit:` does not bound integer digits.
  # Instead we describe by TYPE using guards only. Integers up to 2**64 in
  # magnitude print exactly; larger ones are described by bit length, so a
  # hostile `10 ** 1_000_000` yields a short string quickly.
  @max_exact 18_446_744_073_709_551_616

  defp describe(v) when is_integer(v) and v >= -@max_exact and v <= @max_exact,
    do: "integer #{v}"

  defp describe(v) when is_integer(v) do
    bits = byte_size(:binary.encode_unsigned(abs(v))) * 8
    "an integer of about #{bits} bits"
  end

  defp describe(v) when is_boolean(v), do: "boolean #{v}"
  defp describe(nil), do: "nil"
  defp describe(v) when is_atom(v), do: "an atom"
  defp describe(v) when is_float(v), do: "a float"
  defp describe(v) when is_binary(v), do: "a binary"
  defp describe(v) when is_list(v), do: "a list"
  defp describe(v) when is_tuple(v), do: "a tuple"
  defp describe(%_{}), do: "a struct"
  defp describe(v) when is_map(v), do: "a map"
  defp describe(v) when is_pid(v), do: "a pid"
  defp describe(v) when is_reference(v), do: "a reference"
  defp describe(v) when is_function(v), do: "a function"
  defp describe(v) when is_port(v), do: "a port"
  defp describe(_), do: "a non-bit value"

  # ── Resource caps and plain-list validation ─────────────────────────────
  #
  # Every list-taking function in this package validates its list arguments
  # with `checked_list!/3` BEFORE doing any work.  The caps mirror the Rust
  # implementation.  We walk the caller's list ourselves (at most cap + 1
  # cells) instead of calling `length/1` or `Enum.*`, because:
  #
  #   * `length/1` walks an arbitrarily long list before we can reject it,
  #   * `Enum.*` on a struct/stream dispatches the Enumerable protocol, i.e.
  #     runs caller code.  Only plain lists are accepted; anything else is
  #     rejected with a fixed-text error without being touched.

  @max_select_bits 16
  @max_inputs 65_536
  @max_width 65_536
  @max_counter_width 64

  @doc "Maximum number of select bits (mux/demux/decoder)."
  @spec max_select_bits() :: pos_integer()
  def max_select_bits, do: @max_select_bits

  @doc "Maximum number of inputs/outputs of a multi-input gate or circuit."
  @spec max_inputs() :: pos_integer()
  def max_inputs, do: @max_inputs

  @doc "Maximum register / shift-register width."
  @spec max_width() :: pos_integer()
  def max_width, do: @max_width

  @doc "Maximum counter width."
  @spec max_counter_width() :: pos_integer()
  def max_counter_width, do: @max_counter_width

  @doc false
  # Returns `{list, length}` for a proper list of at most `cap` elements.
  # Raises ArgumentError with fixed text otherwise.  `what` is always a
  # literal supplied by this package, never caller data.
  @spec checked_list!(term(), non_neg_integer(), String.t()) :: {list(), non_neg_integer()}
  def checked_list!(value, cap, what) when is_list(value) do
    case walk(value, cap, 0) do
      {:ok, n} -> {value, n}
      :improper -> raise ArgumentError, "#{what} must be a proper list"
      :too_long -> raise ArgumentError, "#{what} exceeds the maximum size of #{cap}"
    end
  end

  def checked_list!(_value, _cap, what) do
    raise ArgumentError, "#{what} must be a list"
  end

  defp walk([], _cap, n), do: {:ok, n}
  defp walk([_ | _], cap, n) when n >= cap, do: :too_long
  defp walk([_ | tail], cap, n), do: walk(tail, cap, n + 1)
  defp walk(_improper, _cap, _n), do: :improper

  @doc false
  # Validates every element of an already-checked plain list as a bit.
  @spec validate_bits!(list(), String.t()) :: :ok
  def validate_bits!([], _param), do: :ok

  def validate_bits!([head | tail], param) do
    validate_bit!(head, param)
    validate_bits!(tail, param)
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
    # Delegate to the CMOS inverter (2 transistors: 1 PMOS + 1 NMOS).
    # When input is HIGH, NMOS conducts and pulls output to GND (logic 0).
    # When input is LOW,  PMOS conducts and pulls output to Vdd (logic 1).
    CMOSGates.inverter_evaluate_digital(a)
  end

  def not_gate(a), do: validate_bit!(a, "a")

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
    # Delegate to the CMOS AND gate (NAND followed by an inverter = 6 transistors).
    # In CMOS, AND is NOT the primitive — NAND is. Adding an inverter stage gives AND.
    CMOSGates.and_evaluate_digital(a, b)
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
    # Delegate to the CMOS OR gate (NOR followed by an inverter = 6 transistors).
    # In CMOS, OR = NOT(NOR). The NOR is the natural gate; an inverter stage gives OR.
    CMOSGates.or_evaluate_digital(a, b)
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
    # Delegate to the CMOS XOR gate (4 NAND gates = 16 transistors).
    # Construction: let C = NAND(A,B); XOR = NAND(NAND(A,C), NAND(B,C))
    CMOSGates.xor_evaluate_digital(a, b)
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
    # Delegate to the CMOS NAND gate (4 transistors — the natural CMOS primitive).
    # NAND is cheaper than AND in silicon: 4 vs 6 transistors, no inverter stage.
    CMOSGates.nand_evaluate_digital(a, b)
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
    # Delegate to the CMOS NOR gate (4 transistors — the other natural CMOS primitive).
    # NOR is the gate used in SR latches; two cross-coupled NOR gates make a latch.
    CMOSGates.nor_evaluate_digital(a, b)
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
    # Delegate to the dedicated CMOS XNOR gate (XOR + Inverter) in the transistors package.
    # CMOSGates.xnor_evaluate_digital/2 computes NOT(XOR(A, B)) in a single call,
    # routing through the full transistor simulation.
    CMOSGates.xnor_evaluate_digital(a, b)
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
  def nand_not(a), do: validate_bit!(a, "a")

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
  def and_n(inputs) do
    {list, n} = checked_list!(inputs, @max_inputs, "and_n inputs")

    if n < 2 do
      raise ArgumentError, "and_n requires at least 2 inputs, got #{n}"
    end

    validate_bits!(list, "input")
    Enum.reduce(list, &and_gate/2)
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
  def or_n(inputs) do
    {list, n} = checked_list!(inputs, @max_inputs, "or_n inputs")

    if n < 2 do
      raise ArgumentError, "or_n requires at least 2 inputs, got #{n}"
    end

    validate_bits!(list, "input")
    Enum.reduce(list, &or_gate/2)
  end
end
