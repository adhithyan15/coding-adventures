defmodule CodingAdventures.LZWHostileHeadersTest do
  # Security regression tests: hostile headers must not drive work or memory.
  #
  # Each case is a few bytes whose header claims an enormous count or length.
  # Elixir's binary pattern matching stops at the data that is actually present,
  # so these must return promptly with only that data.  Also round-trips every
  # input of length <= 2 and a seeded random sample.
  use ExUnit.Case, async: true

  alias CodingAdventures.LZW

  test "garbage headers do not hang" do
    for n <- [4, 8, 64] do
      result =
        try do
          LZW.decompress(:binary.copy(<<255>>, n))
        rescue
          _ -> :rejected
        end

      assert is_binary(result) or result == :rejected
    end
  end

  test "declared length larger than the stream" do
    <<_::binary-size(4), rest::binary>> = LZW.compress("hello hello hello")
    assert LZW.decompress(<<255, 255, 255, 255>> <> rest) == "hello hello hello"
  end

  test "declared length truncates output" do
    <<_::binary-size(4), rest::binary>> = LZW.compress("hello hello hello")
    assert LZW.decompress(<<0, 0, 0, 5>> <> rest) == "hello"
  end

  test "round trips every input of length <= 2" do
    inputs =
      [<<>>] ++
        for(a <- 0..255, do: <<a>>) ++
        for(a <- 0..255, b <- 0..255, do: <<a, b>>)

    for d <- inputs do
      assert LZW.decompress(LZW.compress(d)) == d
    end
  end

  test "round trips a seeded random sample" do
    :rand.seed(:exsss, {0x5EED, 0x5EED, 0x5EED})
    alphabets = [2, 4, 16, 256]

    for i <- 0..299 do
      n = 3 + :rand.uniform(597) - 1
      alpha = Enum.at(alphabets, rem(i, 4))
      d = for(_ <- 1..n, into: <<>>, do: <<:rand.uniform(alpha) - 1>>)
      assert LZW.decompress(LZW.compress(d)) == d
    end
  end
end
