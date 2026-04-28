defmodule CodingAdventures.HashFunctionsTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.HashFunctions

  test "FNV-1a matches source-of-truth vectors" do
    assert HashFunctions.fnv1a32("") == 2_166_136_261
    assert HashFunctions.fnv1a32("a") == 3_826_002_220
    assert HashFunctions.fnv1a32("abc") == 440_920_331
    assert HashFunctions.fnv1a32("hello") == 1_335_831_723
    assert HashFunctions.fnv1a32("foobar") == 3_214_735_720

    assert HashFunctions.fnv1a64("") == 14_695_981_039_346_656_037
    assert HashFunctions.fnv1a64("a") == 12_638_187_200_555_641_996
    assert HashFunctions.fnv1a64("abc") == 16_654_208_175_385_433_931
    assert HashFunctions.fnv1a64("hello") == 11_831_194_018_420_276_491
  end

  test "DJB2 matches known vectors" do
    assert HashFunctions.djb2("") == 5381
    assert HashFunctions.djb2("a") == 177_670
    assert HashFunctions.djb2("abc") == 193_485_963
    assert HashFunctions.djb2("hello") == 210_714_636_441
  end

  test "polynomial rolling hash matches manual computations" do
    assert HashFunctions.polynomial_rolling("") == 0
    assert HashFunctions.polynomial_rolling("a") == 97
    assert HashFunctions.polynomial_rolling("ab") == 3105
    assert HashFunctions.polynomial_rolling("abc") == 96_354
    assert HashFunctions.polynomial_rolling("hello world", 31, 100) < 100
    assert_raise ArgumentError, fn -> HashFunctions.polynomial_rolling("x", 31, 0) end
  end

  test "MurmurHash3 matches source-of-truth vectors" do
    assert HashFunctions.murmur3_32("", 0) == 0
    assert HashFunctions.murmur3_32("", 1) == 0x514E28B7
    assert HashFunctions.murmur3_32("a") == 0x3C2569B2
    assert HashFunctions.murmur3_32("abc") == 0xB3DD93FA
  end

  test "MurmurHash3 covers tail paths and seeds" do
    for input <- ["abcd", "abcde", "abcdef", "abcdefg"] do
      assert is_integer(HashFunctions.murmur3_32(input))
    end

    refute HashFunctions.murmur3_32("hello", 0) == HashFunctions.murmur3_32("hello", 1)
  end

  test "analysis helpers return bounded values" do
    score = HashFunctions.avalanche_score(&HashFunctions.fnv1a32/1, 32, 8)
    assert score >= 0.0
    assert score <= 1.0

    chi2 = HashFunctions.distribution_test(fn _data -> 0 end, ["a", "b", "c", "d"], 4)
    assert chi2 == 12.0
  end

  test "analysis helpers reject invalid inputs" do
    assert_raise ArgumentError, fn ->
      HashFunctions.avalanche_score(&HashFunctions.fnv1a32/1, 0, 1)
    end

    assert_raise ArgumentError, fn ->
      HashFunctions.avalanche_score(&HashFunctions.fnv1a32/1, 32, 0)
    end

    assert_raise ArgumentError, fn ->
      HashFunctions.distribution_test(&HashFunctions.fnv1a32/1, [], 4)
    end

    assert_raise ArgumentError, fn ->
      HashFunctions.distribution_test(&HashFunctions.fnv1a32/1, ["x"], 0)
    end
  end
end
