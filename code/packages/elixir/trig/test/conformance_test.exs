defmodule TrigConformanceTest do
  @moduledoc """
  Bit-exact conformance of `Trig` against the shared TRIG-REF-1 vectors
  (PHY00a, `code/specs/fixtures/trig-reference-v1`).

  Every vector is one row

      op <TAB> x <TAB> y <TAB> cr <TAB> flags

  where `x` and `y` are IEEE binary64 BIT PATTERNS (16 lowercase hex digits),
  never decimals.  This test decodes `x` by a bit cast, calls the public
  function, encodes the result back to bits and compares the two INTEGERS
  (PHY00a 6.5).  Printed floats and `==` are never used to compare a result:
  `==` equates `0.0` and `-0.0`, and printing hides bits.

  The bit casts are the same eight bytes viewed two ways:

      <<f::float-64>> = <<bits::64>>      # 64-bit integer -> float
      <<bits::64>>    = <<f::float-64>>   # float -> 64-bit integer

  ## How the BEAM differs (PHY00a 5, 6.2, 6.5 (d))

  A BEAM float cannot be NaN or infinite; the arithmetic that would make one
  raises `ArithmeticError` instead.  So:

    * a row flagged `nonfinite=unrepresentable` has an INPUT (NaN or infinity)
      that cannot exist as a float.  It cannot even be constructed here, so it
      is COUNTED AND SKIPPED, by operation, and the counts are printed and
      asserted (never silently dropped);
    * a row flagged `nonfinite=rejects` (a non-finite result, including the
      `out-of-domain` NaN of `sin`/`cos`/`tan` beyond 2^29) or `beam-raises`
      (`atan2` whose `y / x` overflows although the result is finite)
      REQUIRES `ArithmeticError`.  A finite return is a failure, not a skip;
    * `y = error` (`sqrt` of a negative) requires `ArithmeticError` too.
  """

  use ExUnit.Case, async: true

  import Bitwise

  @fixture Path.expand("../../../../specs/fixtures/trig-reference-v1", __DIR__)
  @groups ~w(edge sweep-binade sweep-cw sweep-prng)
  @ops ~w(sin cos tan radians degrees sqrt atan atan2)
  @qnan 0x7FF8000000000000

  # Accuracy limits in representable steps (ulps) against the `cr` column.
  # PHY00a 3.8 makes NO accuracy claim for tan, so it is named as a no-claim
  # op rather than silently skipped.  Any other op that has a `cr` value but
  # no limit fails the test (a new op cannot be skipped unseen).
  @limits %{"sin" => 3, "cos" => 3, "radians" => 1, "degrees" => 1}
  @no_claim ["tan"]

  # --------------------------------------------------------------------------
  # Bit casts and row loading
  # --------------------------------------------------------------------------

  def bits_to_float(bits) do
    <<f::float-64>> = <<bits::64>>
    f
  end

  def float_to_bits(f) when is_float(f) do
    <<bits::64>> = <<f::float-64>>
    bits
  end

  def hex(s), do: String.to_integer(s, 16)

  # A bit pattern whose exponent field is all ones is a NaN or an infinity.
  def nonfinite_bits?(bits), do: (bits &&& 0x7FF0000000000000) == 0x7FF0000000000000

  def load_rows(group) do
    text = File.read!(Path.join(@fixture, "#{group}.tsv"))
    true = String.ends_with?(text, "\n")

    text
    |> String.split("\n", trim: true)
    |> Enum.map(fn line ->
      [op, x, y, cr, flags] = String.split(line, "\t")
      %{group: group, op: op, x: x, y: y, cr: cr, flags: String.split(flags, ",", trim: true)}
    end)
  end

  # The inputs as integers (one for most ops, two for atan2: y first).
  def input_bits(%{op: "atan2", x: x}), do: x |> String.split(",") |> Enum.map(&hex/1)
  def input_bits(%{x: x}), do: [hex(x)]

  def unrepresentable?(row) do
    "nonfinite=unrepresentable" in row.flags or Enum.any?(input_bits(row), &nonfinite_bits?/1)
  end

  def call_op(op, args) do
    floats = Enum.map(args, &bits_to_float/1)

    case {op, floats} do
      {"atan2", [y, x]} -> Trig.atan2(y, x)
      {_, [x]} -> apply(Trig, String.to_existing_atom(op), [x])
    end
  end

  # Calls the op and reports `{:ok, float}` or `:raised` (ArithmeticError).
  def try_op(op, args) do
    {:ok, call_op(op, args)}
  rescue
    ArithmeticError -> :raised
  end

  # --------------------------------------------------------------------------
  # Checking one row.  Returns :pass, :skipped, or {:fail, reason}.
  # --------------------------------------------------------------------------

  def check_row(row) do
    cond do
      row.op not in @ops ->
        {:fail, "unknown op #{inspect(row.op)}"}

      unrepresentable?(row) ->
        :skipped

      "nonfinite=rejects" in row.flags or "beam-raises" in row.flags or row.y == "error" ->
        case try_op(row.op, input_bits(row)) do
          :raised -> :pass
          {:ok, v} -> {:fail, "expected ArithmeticError, got #{hex16(float_to_bits(v))}"}
        end

      hex(row.y) == @qnan ->
        # A NaN expectation without a raise flag cannot be met on the BEAM.
        {:fail, "NaN expected but row carries no rejects flag"}

      true ->
        want = hex(row.y)

        case try_op(row.op, input_bits(row)) do
          :raised -> {:fail, "unexpected ArithmeticError"}
          {:ok, v} ->
            got = float_to_bits(v)

            if got == want,
              do: :pass,
              else: {:fail, "expected #{row.y}, got #{hex16(got)}"}
        end
    end
  end

  def hex16(bits), do: bits |> Integer.to_string(16) |> String.downcase() |> String.pad_leading(16, "0")

  # --------------------------------------------------------------------------
  # The fixture on disk is the one the manifest describes
  # --------------------------------------------------------------------------

  # The manifest is pretty-printed JSON with sorted keys.  It is read with
  # regular expressions, not a JSON library, so the package keeps working on
  # every Elixir the manifest requires (`JSON` only exists from Elixir 1.18).
  setup_all do
    text = File.read!(Path.join(@fixture, "manifest.json"))

    [_, block] = Regex.run(~r/"constants": \{(.*?)\}/s, text)

    constants =
      ~r/"(\w+)": "([0-9a-f]{16})"/
      |> Regex.scan(block)
      |> Map.new(fn [_, name, bits] -> {name, bits} end)

    groups =
      ~r/"file": "([^"]+)",\s*"name": "([^"]+)",\s*"rows": (\d+),\s*"sha256": "([0-9a-f]{64})"/
      |> Regex.scan(text)
      |> Enum.map(fn [_, file, name, rows, sha] ->
        %{file: file, name: name, rows: String.to_integer(rows), sha256: sha}
      end)

    assert text =~ ~s("algorithm": "TRIG-REF-1")
    %{manifest: %{constants: constants, groups: groups}}
  end

  test "manifest hashes and row counts", %{manifest: m} do
    assert Enum.map(m.groups, & &1.name) == @groups

    for g <- m.groups do
      data = File.read!(Path.join(@fixture, g.file))
      assert Base.encode16(:crypto.hash(:sha256, data), case: :lower) == g.sha256, g.file
      assert length(String.split(data, "\n", trim: true)) == g.rows, g.file
    end
  end

  test "row syntax: closed op set, 16-digit lower-case hex" do
    for group <- @groups, row <- load_rows(group) do
      assert row.op in @ops, "unknown op #{inspect(row.op)}"
      parts = String.split(row.x, ",")
      assert length(parts) == if(row.op == "atan2", do: 2, else: 1)

      fields = parts ++ if(row.y == "error", do: [], else: [row.y]) ++ if(row.cr == "-", do: [], else: [row.cr])

      for f <- fields, do: assert(f =~ ~r/\A[0-9a-f]{16}\z/, "bad field #{inspect(f)}")
    end
  end

  # --------------------------------------------------------------------------
  # Constants (PHY00a 6.5 (e)): every embedded constant, by bits
  # --------------------------------------------------------------------------

  test "constants match the manifest bit for bit", %{manifest: m} do
    consts = Trig.constants()
    assert Enum.sort(Map.keys(consts)) == Enum.sort(Map.keys(m.constants))

    for {name, hexbits} <- m.constants do
      assert float_to_bits(Map.fetch!(consts, name)) == hex(hexbits), name
    end
  end

  # Decimal literals written anywhere in the source must parse to the pinned
  # bits: "the literal is the nearest double" is a property of the Erlang
  # parser, so it is asserted rather than assumed.
  test "decimal literals parse to the pinned bits" do
    assert float_to_bits(1.0e-15) == 0x3CD203AF9EE75616
    assert float_to_bits(1.0e308) == 0x7FE1CCF385EBC8A0
    assert float_to_bits(1.0e-17) == 0x3C670EF54646D497
    assert float_to_bits(1.0e-300) == 0x01A56E1FC2F8F359
    assert float_to_bits(7.450580596923828e-9) == 0x3E40000000000000
  end

  test "degree constants equal the divisions" do
    c = Trig.constants()
    assert c["RAD_PER_DEG"] == c["PI"] / 180.0
    assert c["DEG_PER_RAD"] == 180.0 / c["PI"]
  end

  # --------------------------------------------------------------------------
  # Every vector, bit for bit
  # --------------------------------------------------------------------------

  for group <- @groups do
    test "every vector matches bit for bit: #{group}" do
      results = Enum.map(load_rows(unquote(group)), fn row -> {row, check_row(row)} end)
      assert results != []

      failures =
        for {row, {:fail, why}} <- results, do: {row.op, row.x, why}

      by_op = failures |> Enum.frequencies_by(&elem(&1, 0))

      assert failures == [],
             "failing rows per op: #{inspect(by_op)}; first: #{inspect(Enum.take(failures, 5))}"

      skipped = for {row, :skipped} <- results, do: row.op
      IO.puts("  [#{unquote(group)}] skipped unrepresentable rows: #{inspect(Enum.frequencies(skipped))}")
    end
  end

  # Unrepresentable rows are counted by name and the count is pinned, so a
  # skip can never grow silently (PHY00a 6.5 (d)).
  test "skipped unrepresentable rows are counted by operation" do
    rows = Enum.flat_map(@groups, &load_rows/1)
    flagged = for r <- rows, "nonfinite=unrepresentable" in r.flags, do: r
    detected = for r <- rows, unrepresentable?(r), do: r

    # The flag and the bit pattern agree in both directions.
    assert length(flagged) == length(detected)

    counts = flagged |> Enum.frequencies_by(& &1.op)

    assert counts == %{
             "atan" => 3,
             "atan2" => 19,
             "cos" => 12,
             "degrees" => 12,
             "radians" => 12,
             "sin" => 12,
             "sqrt" => 3,
             "tan" => 12
           }

    IO.puts("  total unrepresentable rows skipped: #{length(flagged)} #{inspect(counts)}")
  end

  # --------------------------------------------------------------------------
  # Accuracy against the correctly rounded column
  # --------------------------------------------------------------------------

  # Map a bit pattern onto a signed integer line on which adjacent doubles
  # differ by exactly one, so |ordered(a) - ordered(b)| is the distance in steps.
  def ordered(bits) do
    mag = bits &&& 0x7FFFFFFFFFFFFFFF
    if bits >>> 63 == 1, do: -mag, else: mag
  end

  test "accuracy against the correctly rounded column" do
    worst =
      for group <- @groups,
          row <- load_rows(group),
          row.cr != "-",
          not unrepresentable?(row),
          row.op not in @no_claim,
          reduce: %{} do
        acc ->
          # Fail closed: an op with a cr value but no limit is an error.
          assert Map.has_key?(@limits, row.op),
                 "op #{inspect(row.op)} has a cr value but no accuracy limit"

          got =
            case try_op(row.op, input_bits(row)) do
              {:ok, v} -> float_to_bits(v)
              :raised -> flunk("#{row.op} #{row.x} raised but has a cr value")
            end

          steps = abs(ordered(got) - ordered(hex(row.cr)))
          Map.update(acc, row.op, steps, &max(&1, steps))
      end

    for {op, bound} <- @limits do
      assert Map.get(worst, op, 0) <= bound, "#{op} worst=#{Map.get(worst, op)} > #{bound}"
    end
  end

  test "tan is a named no-claim op" do
    assert @no_claim == ["tan"]
    refute Map.has_key?(@limits, "tan")
  end

  test "unknown ops fail" do
    assert {:fail, _} = check_row(%{op: "sinh", x: "0", y: "0", cr: "-", flags: []})
    for g <- @groups, row <- load_rows(g), do: assert(row.op in @ops)
  end

  # --------------------------------------------------------------------------
  # The behaviours PHY00a 3.3 names, asserted directly
  # --------------------------------------------------------------------------

  test "sin of negative zero is negative zero" do
    assert float_to_bits(Trig.sin(-0.0)) == 0x8000000000000000
    assert float_to_bits(Trig.sin(0.0)) == 0
  end

  test "cos of zero is one" do
    assert float_to_bits(Trig.cos(0.0)) == 0x3FF0000000000000
    assert float_to_bits(Trig.cos(-0.0)) == 0x3FF0000000000000
  end

  test "tiny arguments are returned unchanged" do
    tiny = bits_to_float(0x3E3FFFFFFFFFFFFF)
    assert float_to_bits(Trig.sin(tiny)) == 0x3E3FFFFFFFFFFFFF
    assert float_to_bits(Trig.sin(5.0e-324)) == 1
    assert Trig.cos(tiny) == 1.0
  end

  test "beyond 2^29 sin, cos and tan raise ArithmeticError" do
    for x <- [1.0e20, 536_870_912.0 * (1 + :math.pow(2, -52)), 1.7976931348623157e308, -1.0e20],
        fun <- [&Trig.sin/1, &Trig.cos/1, &Trig.tan/1] do
      assert_raise ArithmeticError, fn -> fun.(x) end
    end

    # An integer beyond the limit is converted first, then refused.
    assert_raise ArithmeticError, fn -> Trig.sin(1_000_000_000) end
  end

  test "the last in-domain value is finite" do
    assert float_to_bits(Trig.sin(536_870_912.0)) == 0x3FD4E67C0E2622DF
    assert float_to_bits(Trig.cos(536_870_912.0)) == 0xBFEE3EDD2DFEEF90
  end

  test "integer arguments are converted once, as floats" do
    assert float_to_bits(Trig.sin(1)) == float_to_bits(Trig.sin(1.0))
    assert float_to_bits(Trig.cos(3)) == float_to_bits(Trig.cos(3.0))
    assert Trig.radians(180) == Trig.pi()
    assert is_float(Trig.sin(0))
    assert is_float(Trig.cos(0))
    assert is_float(Trig.radians(0))
    assert is_float(Trig.degrees(0))
    assert float_to_bits(Trig.radians(-0.0)) == 0x8000000000000000
    assert float_to_bits(Trig.degrees(-0.0)) == 0x8000000000000000
  end

  test "radians and degrees are single multiplies" do
    assert Trig.radians(180.0) == Trig.pi()
    assert Trig.degrees(Trig.pi()) == 180.0
    assert_raise ArithmeticError, fn -> Trig.degrees(1.7e308) end
  end

  test "tan keeps the pole sentinel" do
    assert Trig.tan(Trig.pi() / 2.0) == 1.0e308
    assert Trig.tan(-Trig.pi() / 2.0) == -1.0e308
  end

  test "atan2 pinned conventions" do
    assert Trig.atan2(-0.0, -1.0) == Trig.pi()
    assert float_to_bits(Trig.atan2(0.0, 0.0)) == 0
    # |y / x| overflows: the division raises on the BEAM (flag beam-raises).
    assert_raise ArithmeticError, fn -> Trig.atan2(1.0e300, 1.0e-300) end
  end

  test "atan keeps the sign of zero" do
    assert float_to_bits(Trig.atan(-0.0)) == 0x8000000000000000
  end

  test "sqrt of a negative raises ArithmeticError" do
    assert_raise ArithmeticError, fn -> Trig.sqrt(-1.0) end
    assert float_to_bits(Trig.sqrt(-0.0)) == 0x8000000000000000
  end

  test "negative quadrant arithmetic: k = -1, -5 map to quadrant 3" do
    h = Trig.pi() / 2.0
    assert Trig.sin(-h) == -Trig.sin(h)
    assert abs(Trig.cos(-5 * h)) < 1.0e-15
  end
end
