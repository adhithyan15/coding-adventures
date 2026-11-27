defmodule Boom do
  @moduledoc false
  # A caller struct whose protocol implementations record every call.  The
  # package must never run any of them.
  defstruct [:tag]
end

defimpl Inspect, for: Boom do
  def inspect(_, _) do
    :counters.add(:persistent_term.get(:boom_calls), 1, 1)
    "boom"
  end
end

defimpl String.Chars, for: Boom do
  def to_string(_) do
    :counters.add(:persistent_term.get(:boom_calls), 1, 1)
    "boom"
  end
end

defimpl Enumerable, for: Boom do
  defp bump, do: :counters.add(:persistent_term.get(:boom_calls), 1, 1)
  def count(_), do: (bump(); {:error, __MODULE__})
  def member?(_, _), do: (bump(); {:error, __MODULE__})
  def slice(_), do: (bump(); {:error, __MODULE__})
  def reduce(_, _, _), do: (bump(); {:done, []})
end

defmodule CodingAdventures.LogicGates.HardeningTest do
  use ExUnit.Case, async: false

  alias CodingAdventures.LogicGates.{Combinational, Gates, InvalidBitError, Sequential}

  @ff %{master_q: 0, master_q_bar: 1, slave_q: 0, slave_q_bar: 1}

  setup do
    :persistent_term.put(:boom_calls, :counters.new(1, []))
    :ok
  end

  defp calls, do: :counters.get(:persistent_term.get(:boom_calls), 1)

  defp raises_package_error(fun) do
    try do
      fun.()
      flunk("expected an exception")
    rescue
      e in [ArgumentError, InvalidBitError] ->
        assert byte_size(Exception.message(e)) < 200
        e
    end
  end

  defp states(n), do: List.duplicate(@ff, n)

  describe "caller structs never run caller code" do
    test "every list-taking public function" do
      boom = %Boom{tag: 1}

      fns = [
        fn -> Gates.and_gate(boom, 1) end,
        fn -> Gates.nand_gate(1, boom) end,
        fn -> Gates.nand_and(boom, 1) end,
        fn -> Sequential.sr_latch(boom, 0, 0, 1) end,
        fn -> Gates.and_n(boom) end,
        fn -> Gates.or_n(boom) end,
        fn -> Gates.and_n([1, boom]) end,
        fn -> Combinational.mux4(0, 0, 0, 0, boom) end,
        fn -> Combinational.mux_n(boom, [0]) end,
        fn -> Combinational.mux_n([0, 1], boom) end,
        fn -> Combinational.mux_n([0, boom], [0]) end,
        fn -> Combinational.demux(1, boom, 2) end,
        fn -> Combinational.demux(1, [0], boom) end,
        fn -> Combinational.decoder(boom) end,
        fn -> Combinational.encoder(boom) end,
        fn -> Combinational.priority_encoder(boom) end,
        fn -> Combinational.tri_state(boom, 1) end,
        fn -> Sequential.d_flip_flop(1, 0, boom) end,
        fn -> Sequential.register(boom, 1, []) end,
        fn -> Sequential.register([1], 1, boom) end,
        fn -> Sequential.register([1], 1, [boom]) end,
        fn -> Sequential.shift_register(1, 1, boom) end,
        fn -> Sequential.shift_register(1, 1, [boom]) end,
        fn -> Sequential.shift_register(1, 1, states(2), boom) end,
        fn -> Sequential.counter(1, 0, boom) end,
        fn -> Sequential.counter(1, 1, boom) end,
        fn -> Sequential.counter(1, 0, [boom]) end
      ]

      for f <- fns, do: raises_package_error(f)
      assert calls() == 0
    end

    test "stream and non-list states are rejected promptly" do
      for bad <- [Stream.cycle([@ff]), %{}, %{a: 1}, 5, :x, {1}] do
        raises_package_error(fn -> Sequential.shift_register(1, 1, bad) end)
        raises_package_error(fn -> Sequential.counter(1, 0, bad) end)
        raises_package_error(fn -> Sequential.register([], 1, bad) end)
      end
    end
  end

  describe "improper lists" do
    test "every list function raises the package error" do
      imp = [1 | 1]
      raises_package_error(fn -> Gates.and_n(imp) end)
      raises_package_error(fn -> Gates.or_n(imp) end)
      raises_package_error(fn -> Combinational.decoder(imp) end)
      raises_package_error(fn -> Combinational.encoder(imp) end)
      raises_package_error(fn -> Combinational.priority_encoder(imp) end)
      raises_package_error(fn -> Combinational.mux_n(imp, [0]) end)
      raises_package_error(fn -> Combinational.mux_n([0, 1], imp) end)
      raises_package_error(fn -> Combinational.demux(1, imp, 2) end)
      raises_package_error(fn -> Sequential.register(imp, 1, [@ff]) end)
      raises_package_error(fn -> Sequential.register([1], 1, [@ff | 1]) end)
      raises_package_error(fn -> Sequential.shift_register(1, 1, [@ff | 1]) end)
      raises_package_error(fn -> Sequential.counter(1, 0, [@ff | 1]) end)
    end
  end

  describe "caps" do
    test "and_n/or_n at and beyond the cap" do
      cap = Gates.max_inputs()
      assert Gates.and_n(List.duplicate(1, cap)) == 1
      assert Gates.or_n(List.duplicate(0, cap)) == 0
      raises_package_error(fn -> Gates.and_n(List.duplicate(1, cap + 1)) end)
      raises_package_error(fn -> Gates.or_n(List.duplicate(1, cap + 1)) end)
    end

    test "very long lists are rejected quickly" do
      long = List.duplicate(1, 5_000_000)
      {us, _} = :timer.tc(fn -> raises_package_error(fn -> Gates.and_n(long) end) end)
      assert us < 500_000
    end

    test "select-bit caps" do
      cap = Gates.max_select_bits()
      raises_package_error(fn -> Combinational.decoder(List.duplicate(0, cap + 1)) end)
      raises_package_error(fn -> Combinational.demux(1, List.duplicate(0, cap + 1), 2) end)
      raises_package_error(fn -> Combinational.mux_n([0, 1], List.duplicate(0, cap + 1)) end)
      assert length(Combinational.decoder(List.duplicate(0, 10))) == 1024
    end

    test "decoder at the cap completes in sane time" do
      {us, out} = :timer.tc(fn -> Combinational.decoder(List.duplicate(0, 16)) end)
      assert length(out) == 65_536
      assert us < 5_000_000
    end

    test "mux_n at cap and over" do
      cap = Gates.max_inputs()
      inputs = List.duplicate(0, cap) |> List.replace_at(5, 1)
      sel = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1]
      assert Combinational.mux_n(inputs, sel) == 1
      raises_package_error(fn -> Combinational.mux_n(List.duplicate(0, cap + 1), sel) end)
    end

    test "encoders and register widths" do
      cap = Gates.max_inputs()
      raises_package_error(fn -> Combinational.encoder(List.duplicate(0, cap + 1)) end)
      raises_package_error(fn -> Combinational.priority_encoder(List.duplicate(0, cap + 1)) end)
      w = Gates.max_width()

      raises_package_error(fn ->
        Sequential.register(List.duplicate(0, w + 1), 1, states(w + 1))
      end)

      raises_package_error(fn -> Sequential.shift_register(1, 1, states(w + 1)) end)
      {bits, st} = Sequential.register(List.duplicate(1, 1000), 1, states(1000))
      assert length(bits) == 1000 and length(st) == 1000
    end

    test "counter width cap" do
      c = Gates.max_counter_width()
      {bits, _} = Sequential.counter(1, 0, states(c))
      assert length(bits) == c
      raises_package_error(fn -> Sequential.counter(1, 0, states(c + 1)) end)
      raises_package_error(fn -> Sequential.counter(1, 1, states(c + 1)) end)
    end

    test "shift is linear time on wide registers" do
      {us, {bits, _}} = :timer.tc(fn -> Sequential.shift_register(1, 1, states(20_000)) end)
      assert length(bits) == 20_000
      assert us < 2_000_000

      {us, _} =
        :timer.tc(fn -> Sequential.shift_register(1, 1, states(20_000), direction: :right) end)

      assert us < 2_000_000
    end
  end

  describe "former crash identities" do
    test "mux_n shape errors" do
      raises_package_error(fn -> Combinational.mux_n([0, 1, 0], [0, 1]) end)
      raises_package_error(fn -> Combinational.mux_n([0, 1, 0, 1], [0]) end)
      raises_package_error(fn -> Combinational.mux_n(5, [0]) end)
      raises_package_error(fn -> Combinational.mux_n([0, 1], :x) end)
    end

    test "and_n / or_n non-lists" do
      raises_package_error(fn -> Gates.and_n(5) end)
      raises_package_error(fn -> Gates.or_n(nil) end)
      raises_package_error(fn -> Gates.and_n([1]) end)
    end

    test "bad sequential state" do
      raises_package_error(fn -> Sequential.d_flip_flop(1, 0, %{}) end)
      raises_package_error(fn -> Sequential.d_flip_flop(1, 0, nil) end)
      raises_package_error(fn -> Sequential.d_flip_flop(1, 0, [1]) end)
      assert_raise InvalidBitError, fn -> Sequential.d_flip_flop(1, 0, %{@ff | slave_q: 7}) end
    end

    test "unknown shift direction and bad opts" do
      raises_package_error(fn -> Sequential.shift_register(1, 1, states(2), direction: :up) end)
      raises_package_error(fn -> Sequential.shift_register(1, 1, states(2), :left) end)
      raises_package_error(fn -> Sequential.shift_register(1, 1, states(2), [:left]) end)
      raises_package_error(fn -> Sequential.shift_register(1, 1, states(2), foo: 1) end)
    end

    test "counter reset with garbage state" do
      raises_package_error(fn -> Sequential.counter(0, 1, [:garbage]) end)
      raises_package_error(fn -> Sequential.counter(0, 1, [%{}]) end)
      raises_package_error(fn -> Sequential.counter(0, 1, :nope) end)
    end

    test "mux4 message is fixed text" do
      e = raises_package_error(fn -> Combinational.mux4(0, 0, 0, 0, [0]) end)
      assert Exception.message(e) == "sel must be a list of 2 bits"
    end
  end

  describe "demux / decoder regressions" do
    test "non-positive, non-power-of-two and non-integer counts rejected" do
      for n <- [0, 1, -1, -4, 3, 6, 5.0, nil, :x, 65_537, 131_072] do
        raises_package_error(fn -> Combinational.demux(1, [], n) end)
        raises_package_error(fn -> Combinational.demux(1, [0], n) end)
      end

      raises_package_error(fn -> Combinational.demux(1, [0, 0], 0) end)
    end

    test "bignum n_outputs is rejected instantly" do
      big = Integer.pow(10, 80_000)

      {us, _} =
        :timer.tc(fn -> raises_package_error(fn -> Combinational.demux(1, [0], big) end) end)

      assert us < 50_000
      raises_package_error(fn -> Combinational.demux(1, [0], -big) end)
    end

    test "valid counts still work" do
      assert Combinational.demux(1, [1], 2) == [0, 1]
      assert length(Combinational.demux(1, List.duplicate(0, 8), 256)) == 256
    end
  end

  describe "shift order preserved" do
    # Reference: the original quadratic implementation (append at the tail).
    defp ref_shift(serial_in, clock, state, :left) do
      {_, acc} =
        Enum.reduce(state, {serial_in, []}, fn ff, {inp, acc} ->
          {q, _, nf} = Sequential.d_flip_flop(inp, clock, ff)
          {q, acc ++ [nf]}
        end)

      {Enum.map(acc, & &1.slave_q), acc}
    end

    defp ref_shift(serial_in, clock, state, :right) do
      {bits, rev} = ref_shift(serial_in, clock, Enum.reverse(state), :left)
      _ = bits
      new = Enum.reverse(rev)
      {Enum.map(new, & &1.slave_q), new}
    end

    test "matches the reference implementation in both directions" do
      for dir <- [:left, :right] do
        steps = for i <- 0..11, do: {rem(i * 7 + 1, 2), rem(i, 2)}

        {_, _} =
          Enum.reduce(steps, {states(5), states(5)}, fn {d, clk}, {fast, ref} ->
            {fb, fs} = Sequential.shift_register(d, clk, fast, direction: dir)
            {rb, rs} = ref_shift(d, clk, ref, dir)
            assert fb == rb
            assert fs == rs
            {fs, rs}
          end)
      end
    end
  end
end
