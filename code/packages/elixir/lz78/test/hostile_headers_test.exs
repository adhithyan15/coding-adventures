defmodule CodingAdventures.LZ78HostileHeadersTest do
  # Security regression tests: hostile headers must not drive work or memory.
  #
  # Each case is a few bytes whose header claims an enormous count or length.
  # Elixir's binary pattern matching stops at the data that is actually present,
  # so these must return promptly with only that data.  Also round-trips every
  # input of length <= 2 and a seeded random sample.
  use ExUnit.Case, async: true

  alias CodingAdventures.LZ78

  test "max header only" do
    assert LZ78.decompress(<<255, 255, 255, 255, 255, 255, 255, 255>>) == ""
  end

  test "count larger than remaining bytes yields the present tokens" do
    d = <<0, 0, 0, 2, 255, 255, 255, 255, 0, 0, 104, 0, 0, 0, 105, 0>>
    assert LZ78.decompress(d) == "hi"
  end

  test "length larger than the tokens supply" do
    d = <<255, 255, 255, 255, 0, 0, 0, 1, 0, 0, 104, 0>>
    assert LZ78.decompress(d) == "h"
  end

  test "round trips every input of length <= 2" do
    inputs =
      [<<>>] ++
        for(a <- 0..255, do: <<a>>) ++
        for(a <- 0..255, b <- 0..255, do: <<a, b>>)

    for d <- inputs do
      assert LZ78.decompress(LZ78.compress(d)) == d
    end
  end

  test "round trips a seeded random sample" do
    :rand.seed(:exsss, {0x5EED, 0x5EED, 0x5EED})
    alphabets = [2, 4, 16, 256]

    for i <- 0..299 do
      n = 3 + :rand.uniform(597) - 1
      alpha = Enum.at(alphabets, rem(i, 4))
      d = for(_ <- 1..n, into: <<>>, do: <<:rand.uniform(alpha) - 1>>)
      assert LZ78.decompress(LZ78.compress(d)) == d
    end
  end
end
