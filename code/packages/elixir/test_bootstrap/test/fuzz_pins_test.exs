defmodule FuzzPinsTest do
  @moduledoc """
  Pins what fuzz-v1 does not (VER06n section 9): every interest and delta constant,
  unequal splice splits with target and donor swapped, the power schedule
  (energy rows including exact half-way ties), real sig_hash values (cross-checked
  with the Python oracle and the Ruby lane), the rejection draw of bounded sampling
  and the minimiser's bisection halves.
  """
  use ExUnit.Case, async: true
  @moduletag timeout: 60_000

  import Bitwise
  alias CodingAdventures.TestBootstrap.Prng
  alias CodingAdventures.TestBootstrap.Fuzz
  alias CodingAdventures.TestBootstrap.Fuzz.{Bitmap, Engine, FailureIds, Limits, Mutate, Signature}

  defp hx(text), do: Fx.hex(text)
  defp hex(bytes), do: Fx.unhex(bytes)

  # -- constants ---------------------------------------------------------------------

  test "every interest value and delta is pinned" do
    assert Mutate.interest_8() == [0, 1, 127, 128, 255]
    assert Mutate.interest_16() == [0, 1, 255, 256, 32_767, 32_768, 65_535]
    assert Mutate.interest_32() == [0, 1, 65_535, 65_536, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF]

    assert Mutate.interest_64() ==
             [0, 1, 0xFFFFFFFF, 0x100000000, 0x7FFFFFFFFFFFFFFF, 0x8000000000000000, 0xFFFFFFFFFFFFFFFF]

    assert Mutate.deltas() == [-35, -8, -1, 1, 8, 35]
    assert Mutate.interest_by_width() == %{1 => Mutate.interest_8(), 2 => Mutate.interest_16(), 4 => Mutate.interest_32(), 8 => Mutate.interest_64()}
  end

  test "every interest value round-trips through boundary_insert in both endiannesses" do
    for {width, values} <- Mutate.interest_by_width(), val <- values do
      zero = :binary.copy(<<0>>, width + 2)
      le = Mutate.boundary_insert(zero, 1, width, val, "little")
      be = Mutate.boundary_insert(zero, 1, width, val, "big")
      assert :binary.decode_unsigned(binary_part(le, 1, width), :little) == val
      assert :binary.decode_unsigned(binary_part(be, 1, width), :big) == val
      assert binary_part(le, 0, 1) == <<0>> and binary_part(le, byte_size(le) - 1, 1) == <<0>>
    end
  end

  test "arithmetic wraps modulo the width in every width and endianness" do
    for w <- [1, 2, 4, 8], e <- ["little", "big"] do
      ff = :binary.copy(<<0xFF>>, w)
      zero = :binary.copy(<<0>>, w)
      assert hex(Mutate.byte_arithmetic(ff, 0, 1, w, e)) == String.duplicate("00", w)
      assert hex(Mutate.byte_arithmetic(zero, 0, -1, w, e)) == String.duplicate("ff", w)
      assert Mutate.byte_arithmetic(ff, 0, 1 <<< (8 * w), w, e) == ff
    end

    assert hex(Mutate.byte_arithmetic(hx("ff00"), 0, 1, 2, "little")) == "0001"
    assert hex(Mutate.byte_arithmetic(hx("ff00"), 0, 1, 2, "big")) == "ff01"
    assert hex(Mutate.byte_arithmetic(hx("01"), 0, 1, 1, :big)) == "02"
  end

  test "unsupported widths and short buffers return the input unchanged" do
    for w <- [0, 3, 5, 16, -1, 1 <<< 70] do
      assert hex(Mutate.byte_arithmetic(hx("0102"), 0, 1, w)) == "0102"
      assert hex(Mutate.boundary_insert(hx("0102"), 0, w, 9)) == "0102"
    end

    assert hex(Mutate.byte_arithmetic(hx("01"), 0, 1, 2)) == "01"
    assert Mutate.boundary_insert(<<>>, 0, 1, 9) == <<>>
  end

  # -- splice ------------------------------------------------------------------------

  test "unequal splits with target and donor swapped" do
    a = hx("01020304")
    b = hx("0506070809")
    assert hex(Mutate.splice(a, b, 1, 3)) == "010809"
    assert hex(Mutate.splice(b, a, 3, 1)) == "050607" <> "020304"
    assert hex(Mutate.splice(a, b, 4, 4)) == "0102030409"
    assert hex(Mutate.splice(b, a, 5, 2)) == "05060708090304"
    assert hex(Mutate.splice(a, b, 2, 2)) == "0102070809"
    refute Mutate.splice(a, b, 1, 3) == Mutate.splice(b, a, 3, 1)
  end

  test "splice clamps both splits" do
    a = hx("0102")
    b = hx("0304")
    assert hex(Mutate.splice(a, b, 99, 99)) == "0102"
    assert hex(Mutate.splice(a, b, -5, -5)) == "0304"
    assert hex(Mutate.splice(a, b, 1 <<< 80, -(1 <<< 80))) == "01020304"
    assert Mutate.splice(<<>>, <<>>, 1, 1) == <<>>
  end

  # -- flips -------------------------------------------------------------------------

  test "flips wrap and use parity for any count" do
    z = hx("0000")
    assert hex(Mutate.bit_flip(z, 9, 1)) == "0002"
    # floored: -7 mod 16 = 9 (BEAM rem/2 would give -7)
    assert hex(Mutate.bit_flip(z, -7, 1)) == "0002"
    assert hex(Mutate.bit_flip(z, 1, 15)) == "feff"
    assert hex(Mutate.bit_flip(z, 15, 15)) == "ffbf"
    assert hex(Mutate.bit_flip(z, 0, 17)) == "feff"
    assert hex(Mutate.bit_flip(z, 5, 32)) == "0000"
    assert hex(Mutate.bit_flip(z, 5, 16)) == "ffff"
    assert hex(Mutate.bit_flip(z, 15, 1)) == "0080"
    assert hex(Mutate.byte_flip(hx("123456"), 0, 1)) == "ed3456"
    assert hex(Mutate.byte_flip(hx("123456"), 1, 2)) == "12cba9"
    assert hex(Mutate.byte_flip(hx("123456"), -1, 2)) == "ed34a9"
    assert hex(Mutate.byte_flip(hx("123456"), 0, 3)) == "edcba9"
    assert hex(Mutate.byte_flip(hx("123456"), 1, 6)) == "123456"
    assert Mutate.bit_flip(<<>>, 3, 3) == <<>>
    assert hex(Mutate.byte_flip(hx("0102"), 0, 0)) == "0102"
    assert hex(Mutate.byte_flip(hx("0102"), 0, -4)) == "0102"
  end

  test "a huge flip count is cheap" do
    {us, _} =
      :timer.tc(fn ->
        out = Mutate.bit_flip(:binary.copy(<<0>>, 1000), 12_345, Integer.pow(10, 1000) + 7)
        assert byte_size(out) == 1000
        Mutate.byte_flip(:binary.copy(<<0>>, 1000), 12_345, Integer.pow(10, 1000) + 7)
      end)

    assert us < 2_000_000
  end

  test "flips are involutions" do
    data = hx("0a1b2c3d4e")

    for off <- [0, 1, 9, 39, 40, 1 <<< 65], count <- [1, 4, 17, (1 <<< 64) + 5] do
      assert Mutate.bit_flip(Mutate.bit_flip(data, off, count), off, count) == data
      assert Mutate.byte_flip(Mutate.byte_flip(data, off, count), off, count) == data
    end
  end

  test "block operations and their clamping" do
    assert hex(Mutate.block_delete(hx("0102030405"), 1, 2)) == "010405"
    assert hex(Mutate.block_delete(hx("0102030405"), 2, 99)) == "0102"
    assert hex(Mutate.block_delete(hx("0102030405"), 2, 0)) == "0102030405"
    assert Mutate.block_delete(<<>>, 2, 2) == <<>>
    assert hex(Mutate.block_insert(<<>>, 5, hx("0304"))) == "0304"
    assert hex(Mutate.block_insert(hx("010205"), 2, hx("0304"))) == "0102030405"
    assert hex(Mutate.block_insert(hx("0102"), 2, hx("03"))) == "010203"
    assert hex(Mutate.block_duplicate(hx("010203"), 0, 2, 3)) == "0102030102"
    assert hex(Mutate.block_duplicate(hx("010203"), 0, 0, 3)) == "010203"
    assert Mutate.block_duplicate(<<>>, 0, 1, 0) == <<>>
    assert hex(Mutate.block_duplicate(hx("0103"), 1, 9, 2)) == "010303"
  end

  test "wrong types are an ArgumentError (booleans and floats included)" do
    assert_raise ArgumentError, fn -> Mutate.bit_flip(1, 0) end
    assert_raise ArgumentError, fn -> Mutate.bit_flip(<<>>, true) end
    assert_raise ArgumentError, fn -> Mutate.bit_flip(<<>>, 1.0) end
    assert_raise ArgumentError, fn -> Mutate.byte_arithmetic(<<>>, 0, nil) end
    assert_raise ArgumentError, fn -> Mutate.block_insert("a", 0, 5) end
    assert_raise ArgumentError, fn -> Mutate.splice("a", nil, 0, 0) end
    assert_raise ArgumentError, fn -> Mutate.byte_arithmetic("ab", 0, 1, 1, "middle") end
    assert_raise ArgumentError, fn -> Mutate.boundary_insert("ab", 0, 1, 1, 7) end
    assert_raise ArgumentError, fn -> Mutate.byte_flip(<<1::3>>, 0, 1) end
  end

  # -- mutate_sequence ------------------------------------------------------------------

  test "sequence edges" do
    assert hex(Mutate.mutate_sequence(hx("0102"), 1, 0)) == "0102"
    assert hex(Mutate.mutate_sequence(hx("0102"), 1, -5)) == "0102"
    # an empty input is refilled to 00 before every step
    assert Mutate.mutate_sequence(<<>>, 7, 1) != <<>>
    assert Mutate.mutate_sequence(hx("00"), 9, 8) == Mutate.mutate_sequence(hx("00"), 9, 8)
    assert_raise ArgumentError, fn -> Mutate.mutate_sequence("a", 1, Limits.max_iterations() + 1) end
    assert_raise ArgumentError, fn -> Mutate.mutate_sequence(:binary.copy("a", Limits.max_input_bytes() + 1), 1, 1) end
    assert_raise ArgumentError, fn -> Mutate.mutate_sequence("a", "1", 1) end
  end

  test "the sequence respects max_len and keeps the stream aligned" do
    for seed <- 0..999 do
      assert byte_size(Mutate.mutate_sequence(hx("0001020304050607"), seed, 40, 8)) <= 8
    end

    assert byte_size(Mutate.mutate_sequence(hx("0001020304050607"), 3, 40, 1 <<< 20)) > 8
  end

  test "sequence work is the bytes actually copied, failed calls included" do
    {:ok, out, work} = Mutate.mutate_sequence_counted(hx("00" |> String.duplicate(10)), 5, 3, 1 <<< 20, 1 <<< 30)
    assert work >= 10 + 3
    assert work <= 10 + 3 * min(max(byte_size(out), 10), 100)
    assert {:ok, _, 2} = Mutate.mutate_sequence_counted(hx("0102"), 5, 0, 100, 100)

    # 100 initial + 100 for the first step; the second step would pass 250
    assert {:work_exhausted, 200} = Mutate.mutate_sequence_counted(:binary.copy("a", 100), 1, 10, 1 <<< 20, 250)
    # the initial copy alone is over the cap
    assert {:work_exhausted, 0} = Mutate.mutate_sequence_counted(:binary.copy("a", 100), 1, 10, 1 <<< 20, 99)

    # a failed call is still bounded: it stops as soon as the cap would be passed
    assert {:work_exhausted, w} = Mutate.mutate_sequence_counted(:binary.copy("a", 1 <<< 20), 1, 100, 1 <<< 21, 1 <<< 22)
    assert w <= 1 <<< 22

    err = assert_raise Mutate.WorkExhausted, fn -> Mutate.mutate_sequence(:binary.copy("a", 100), 1, 10, 1 <<< 20, 250) end
    assert err.work == 200
    assert Exception.message(err) =~ "work cap of 250"

    # flips of a 1 MiB input never build a million-bit integer
    big = :binary.copy(<<0>>, 1 <<< 20)
    assert byte_size(Mutate.bit_flip(big, 5, 1 <<< 40)) == 1 <<< 20
    assert Mutate.byte_flip(big, 3, 1) |> binary_part(3, 1) == <<255>>
  end

  # -- the rejection draw of bounded sampling (the corpus never reaches it) ---------------------

  test "the inverter forces the first SplitMix64 word" do
    for out <- [0, 1, 2, 12_345, 1 <<< 63, Prng.mask64()] do
      assert {^out, _} = Prng.next_u64(Prng.new(Fx.seed_for(out)))
    end
  end

  test "bounded(7) rejects a first word below the threshold, keeps one equal to it" do
    # 2^64 mod 7 = 2, so the threshold is rem(2^64 - 7, 7) = 2: raws 0 and 1 are redrawn
    assert rem((1 <<< 64) - 7, 7) == 2
    {w0, r1} = Prng.next_u64(Prng.new(Fx.seed_for(0)))
    {w1, r2} = Prng.next_u64(r1)
    assert w0 == 0
    {v, after_rng} = Prng.bounded(Prng.new(Fx.seed_for(0)), 7)
    assert v == rem(w1, 7)
    assert after_rng == r2

    {v, after_rng} = Prng.bounded(Prng.new(Fx.seed_for(2)), 7)
    assert v == 2
    assert after_rng == elem(Prng.next_u64(Prng.new(Fx.seed_for(2))), 1)
  end

  test "the rejection is visible through mutate_sequence: the real values, cross-checked with the Ruby lane" do
    input = hx("000102030405")
    # first word 0 (rejected, the second word picks the step), 1 (rejected), 2 (the
    # threshold: kept as step 2, byte arithmetic), 3
    assert hex(Mutate.mutate_sequence(input, 7_046_029_254_386_353_131, 3, 100)) == "fd030405"
    assert hex(Mutate.mutate_sequence(input, 17_885_559_969_949_501_885, 3, 100)) == "ff043605"
    assert hex(Mutate.mutate_sequence(input, 10_278_346_628_982_968_224, 3, 100)) == "000102b2020405"
    assert hex(Mutate.mutate_sequence(input, 9_697_084_067_704_644_217, 3, 100)) == "00010203a2fa"
    assert Fx.seed_for(0) == 7_046_029_254_386_353_131
    assert Fx.seed_for(1) == 17_885_559_969_949_501_885
    assert Fx.seed_for(2) == 10_278_346_628_982_968_224
    assert Fx.seed_for(3) == 9_697_084_067_704_644_217
  end

  test "the 2^63 + 1 threshold boundary" do
    n = (1 <<< 63) + 1
    threshold = rem((1 <<< 64) - n, n)
    assert threshold == (1 <<< 63) - 1

    below = Prng.new(Fx.seed_for(threshold - 1))
    {_, r1} = Prng.next_u64(below)
    {w1, r2} = Prng.next_u64(r1)
    {v, rng_after} = Prng.bounded(below, n)
    assert v == rem(w1, n)
    assert rng_after == r2

    at = Prng.new(Fx.seed_for(threshold))
    {v, _} = Prng.bounded(at, n)
    assert v == threshold
  end

  # -- bitmap, admission, energy ----------------------------------------------------------------

  test "bitmap" do
    b = Bitmap.new()
    assert Bitmap.count(b) == 0
    b = b |> Bitmap.record(3) |> Bitmap.record(3 + 65_536) |> Bitmap.record(-1)
    assert Bitmap.covered_edges(b) == [3, 65_535]
    assert Bitmap.has?(b, 65_539)
    refute Bitmap.has?(b, 4)
    assert {true, 2, b} = Bitmap.admit(b, [3, 4, 5, 5])
    assert {false, 0, b} = Bitmap.admit(b, [3, 4])
    assert byte_size(Bitmap.raw(b)) == 8192
    other = Bitmap.record(Bitmap.new(), 9)
    b = Bitmap.merge(b, other)
    assert Bitmap.has?(b, 9)
    raw = Bitmap.raw(Bitmap.record(Bitmap.new(), 10))
    assert binary_part(raw, 1, 1) == <<0b100>>
    b = Bitmap.reset(b)
    assert Bitmap.count(b) == 0
    assert Bitmap.raw(b) == :binary.copy(<<0>>, 8192)
    assert_raise ArgumentError, fn -> Bitmap.record(b, "1") end
    assert_raise ArgumentError, fn -> Bitmap.record(b, 1.5) end
    assert inspect(Bitmap.record(b, 77)) == "#Fuzz.Bitmap<edges=1>"
  end

  test "corpus_admit counts unique novel edges over unreduced ids" do
    assert Fuzz.corpus_admit([1, 2, 3], [2, 4]) == {true, 1}
    assert Fuzz.corpus_admit([1, 2, 3], [1, 2]) == {false, 0}
    assert Fuzz.corpus_admit([], [7, 7, 7]) == {true, 1}
    assert Fuzz.corpus_admit([1], [1 + 65_536]) == {true, 1}
    assert Fuzz.corpus_admit([1], []) == {false, 0}
    assert_raise ArgumentError, fn -> Fuzz.corpus_admit([1], [:x]) end
  end

  # [base, len, new, total, expected]: the power schedule, from the Python oracle formula
  @energy [
    [10, 4, 3, 6, 8], [10, 1, 0, 0, 10], [10, 4, 0, 0, 5], [1, 4, 0, 0, 1], [10, 16, 0, 0, 3],
    [10, 100, 0, 0, 1], [3, 4, 0, 0, 2], [5, 4, 0, 0, 3], [10, 9, 0, 0, 3], [10, 2, 1, 2, 11],
    [7, 1, 1, 3, 9], [0, 4, 0, 0, 1], [10, 4, 1, 1, 10], [25, 100, 0, 0, 3], [10, 25, 0, 0, 2],
    [15, 1, 0, 0, 15], [10, 1, 5, 5, 20]
  ]

  test "power schedule rows, including exact half-way ties" do
    for [base, len, new, total, want] <- @energy do
      assert Fuzz.calculate_energy(base, len, new, total) == want, inspect([base, len, new, total])
    end

    # exact .5 ties round UP (banker's rounding would give 2, 2, 8, 2):
    assert Fuzz.calculate_energy(3, 4, 0, 0) == 2
    assert Fuzz.calculate_energy(5, 4, 0, 0) == 3
    assert Fuzz.calculate_energy(10, 4, 3, 6) == 8
    assert Fuzz.calculate_energy(25, 100, 0, 0) == 3
    assert Fuzz.calculate_energy(1, 4, 0, 0) == 1
    assert Fuzz.calculate_energy(0, 1, 0, 0) == 1
    assert Fuzz.calculate_energy(10, 0, 0, 0) == 10
  end

  # -- signatures --------------------------------------------------------------------------------------

  @sigs [
    {"panic", ["eval", "step"], "4eca7ae8"},
    {"segv", ["alloc", "write"], "a0aa3aa2"},
    {"timeout", [], "4e61ea76"},
    {"oom", [], "7146a410"},
    {"RuntimeError", ["t.rb#Foo.bar", "t.rb#block_in_Foo.bar"], "c12b6a11"},
    {"ZeroDivisionError", ["target.rb#Object#parse"], "19321229"},
    {"stale-budget-sentinel", [], "8a20b3bf"},
    {"ArgumentError", ["a.rb#x"], "c93b2322"}
  ]

  test "real sig_hash values match the Python oracle and the Ruby lane" do
    for {kind, frames, want} <- @sigs, do: assert(Signature.crash_signature(kind, frames) == want, kind)
    assert Prng.fnv1a64("abc") == 16_654_208_175_385_433_931
  end

  test "the signature is documented as unescaped" do
    assert Signature.duplicate_crash?("k", ["x:y"], "k", ["x", "y"])
    refute Signature.duplicate_crash?("k", ["a"], "k", ["b"])
    assert_raise ArgumentError, fn -> Signature.crash_signature(:k, []) end
    assert_raise ArgumentError, fn -> Signature.crash_signature("k", "x") end
    assert_raise ArgumentError, fn -> Signature.crash_signature("k", [1]) end
  end

  test "frames and kinds are normalised without colons" do
    assert Signature.normalise_frame("Foo.bar/1") == "Foo.bar/1"
    assert Signature.normalise_frame("a:b c") == "a_b_c"
    refute Signature.normalise_frame("x::y") =~ ":"
    assert Signature.collapse(~w(a a b b a)) == ~w(a b a)
    assert length(Signature.collapse(Enum.map(1..500, &Integer.to_string/1))) == 64
    assert Signature.sanitize("A" <> String.duplicate("b", 500), 64) |> byte_size() == 64
  end

  test "the closed failure vocabulary" do
    assert FailureIds.all() == ~w(fuzz-crash fuzz-timeout fuzz-oom fuzz-leak fuzz-panic)
    for id <- FailureIds.all(), do: assert(FailureIds.valid?(id))
    for id <- [nil, 1, "fuzz-bogus", "FUZZ-CRASH", "fuzz-crash\n", :"fuzz-crash"], do: refute(FailureIds.valid?(id))
  end

  # -- the minimiser's bisection halves (the corpus never minimises) ------------------------------------

  defp trace_probe(pred) do
    {:ok, agent} = Agent.start_link(fn -> [] end)

    probe = fn candidate ->
      Agent.update(agent, &[hex(candidate) | &1])
      if pred.(candidate), do: {:crash, "K", ["f"]}, else: :pass
    end

    {probe, fn -> agent |> Agent.get(&Enum.reverse/1) end}
  end

  @sig Signature.crash_signature("K", ["f"])

  test "bisection halves: a 9-byte crasher needing one byte tries chunks 4, 2, 1 in that order" do
    {probe, seen} = trace_probe(&String.contains?(&1, <<5>>))
    assert {<<5>>, 8, true} = Engine.minimize(hx("000102030405060708"), @sig, probe, 1000)
    # Cross-checked with the Ruby lane: chunk 4 (delete 0..3 accepted, then 0..3 of the
    # rest rejected, then offset 4 accepted), chunk 2 (offset 0 rejected, offset 2
    # accepted), chunk 1 (offset 0 accepted, then the last byte rejected), then the
    # zeroing pass tries 05 -> 00 (rejected).
    assert seen.() == ["0405060708", "08", "04050607", "0607", "0405", "05", "", "00"]
  end

  test "bisection halves: a 1-byte crasher starts at chunk 1, so the empty input is reachable" do
    {probe, seen} = trace_probe(fn _ -> true end)
    assert {<<>>, 1, true} = Engine.minimize(hx("07"), @sig, probe, 1000)
    assert seen.() == [""]
  end

  test "bisection halves, then byte zeroing of non-essential bytes" do
    {probe, seen} = trace_probe(&(byte_size(&1) >= 2))
    assert {<<0, 0>>, 6, true} = Engine.minimize(hx("01020304"), @sig, probe, 1000)
    assert seen.() == ["0304", "", "04", "03", "0004", "0000"]
  end

  test "the budget counts rejected candidates too" do
    {probe, seen} = trace_probe(&String.contains?(&1, <<5>>))
    assert {bytes, 3, false} = Engine.minimize(hx("000102030405060708"), @sig, probe, 3)
    assert hex(bytes) == "04050607"
    assert seen.() == ["0405060708", "08", "04050607"]

    # all-rejected candidates still stop at the budget
    {probe, seen} = trace_probe(fn _ -> false end)
    assert {_, 4, false} = Engine.minimize(:binary.copy(<<9>>, 64), @sig, probe, 4)
    assert length(seen.()) == 4

    {probe, _} = trace_probe(fn _ -> true end)
    assert {<<>>, 0, true} = Engine.minimize(<<>>, @sig, probe, 0)
    assert {_, 0, false} = Engine.minimize(<<1>>, @sig, probe, 0)
  end

  test "the minimiser keeps the exact signature: another crash is never accepted" do
    probe = fn c -> if byte_size(c) >= 3, do: {:crash, "K", ["f"]}, else: {:crash, "OTHER", ["f"]} end
    assert {bytes, _, true} = Engine.minimize(hx("0a0b0c0d"), @sig, probe, 1000)
    assert byte_size(bytes) == 3
  end

  test "minimize_crashing_input is validated and bounded" do
    probe = fn _ -> :pass end
    assert Engine.minimize_crashing_input("abc", @sig, 10, probe) == "abc"
    assert_raise ArgumentError, fn -> Engine.minimize_crashing_input("abc", @sig, -1, probe) end
    assert_raise ArgumentError, fn -> Engine.minimize_crashing_input("abc", @sig, 100_001, probe) end
    assert_raise ArgumentError, fn -> Engine.minimize_crashing_input(:x, @sig, 1, probe) end
    assert_raise ArgumentError, fn -> Engine.minimize_crashing_input(:binary.copy("a", Limits.max_input_bytes() + 1), @sig, 1, probe) end
  end

  # -- TAP ------------------------------------------------------------------------------------------------

  test "tap format and closed ids" do
    for id <- FailureIds.all() do
      assert Signature.format_tap_failure(7, "ct_compare", id, "3a8f19b2") == "not ok 7 - ct_compare [#{id}:3a8f19b2]"
    end

    for id <- [nil, 1, "fuzz-bogus", "FUZZ-CRASH", "fuzz-crash\n"],
        do: assert_raise(ArgumentError, fn -> Signature.format_tap_failure(1, "t", id, "3a8f19b2") end)

    for sig <- ["3A8F19B2", "3a8f19b", "3a8f19b22", "", "3a8f19b2\n", "zzzzzzzz", nil],
        do: assert_raise(ArgumentError, fn -> Signature.format_tap_failure(1, "t", "fuzz-crash", sig) end)

    for n <- [0, 1 <<< 31, -1, "1", 1.0],
        do: assert_raise(ArgumentError, fn -> Signature.format_tap_failure(n, "t", "fuzz-crash", "3a8f19b2") end)

    assert_raise ArgumentError, fn -> Signature.format_tap_failure(1, :t, "fuzz-crash", "3a8f19b2") end
  end

  test "tap_failures for a report" do
    crash = %Fuzz.CrashRecord{signature: "4eca7ae8", kind: "panic", frames: ["eval", "step"], minimal_input: "x", hits: 2, minimized: true, input_dropped: false}
    report = %Fuzz.Report{iterations: 5, crashes: [crash], timeouts: 1, ooms: 2, seed: 7, corpus: ["a"]}

    assert Signature.tap_failures(report, "t", 3) ==
             ["not ok 3 - t [fuzz-crash:4eca7ae8]", "not ok 4 - t [fuzz-timeout:4e61ea76]", "not ok 5 - t [fuzz-oom:7146a410]"]

    clean = %Fuzz.Report{iterations: 5, seed: 7}
    assert Signature.tap_failures(clean, "t") == []
    assert Fuzz.Report.clean?(clean)
    refute Fuzz.Report.clean?(report)
    assert Fuzz.Report.unique_crashes(report) == 1
  end
end
