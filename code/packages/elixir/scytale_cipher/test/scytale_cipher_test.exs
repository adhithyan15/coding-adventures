defmodule CodingAdventures.ScytaleCipherTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.ScytaleCipher

  # --- Encryption Tests ---

  test "encrypt HELLO WORLD with key=3" do
    assert ScytaleCipher.encrypt("HELLO WORLD", 3) == "HLWLEOODL R "
  end

  test "encrypt ABCDEF with key=2" do
    assert ScytaleCipher.encrypt("ABCDEF", 2) == "ACEBDF"
  end

  test "encrypt ABCDEF with key=3" do
    assert ScytaleCipher.encrypt("ABCDEF", 3) == "ADBECF"
  end

  test "encrypt with key equal to text length" do
    assert ScytaleCipher.encrypt("ABCD", 4) == "ABCD"
  end

  test "encrypt empty string" do
    assert ScytaleCipher.encrypt("", 2) == ""
  end

  test "encrypt raises on key < 2" do
    assert_raise ArgumentError, fn -> ScytaleCipher.encrypt("HELLO", 1) end
  end

  test "encrypt raises on key > text length" do
    assert_raise ArgumentError, fn -> ScytaleCipher.encrypt("HI", 3) end
  end

  # --- Decryption Tests ---

  test "decrypt HELLO WORLD with key=3" do
    assert ScytaleCipher.decrypt("HLWLEOODL R ", 3) == "HELLO WORLD"
  end

  test "decrypt ACEBDF with key=2" do
    assert ScytaleCipher.decrypt("ACEBDF", 2) == "ABCDEF"
  end

  test "decrypt empty string" do
    assert ScytaleCipher.decrypt("", 2) == ""
  end

  test "decrypt raises on invalid key" do
    assert_raise ArgumentError, fn -> ScytaleCipher.decrypt("HELLO", 0) end
    assert_raise ArgumentError, fn -> ScytaleCipher.decrypt("HI", 3) end
  end

  # --- Round Trip Tests ---

  test "round trip HELLO WORLD" do
    text = "HELLO WORLD"
    assert text == ScytaleCipher.decrypt(ScytaleCipher.encrypt(text, 3), 3)
  end

  test "round trip with various keys" do
    text = "The quick brown fox jumps over the lazy dog!"
    n = String.length(text)

    for key <- 2..div(n, 2) do
      ct = ScytaleCipher.encrypt(text, key)
      pt = ScytaleCipher.decrypt(ct, key)
      assert pt == text, "Round trip failed for key=#{key}"
    end
  end

  # --- Brute Force Tests ---

  test "brute force finds original text" do
    original = "HELLO WORLD"
    ct = ScytaleCipher.encrypt(original, 3)
    results = ScytaleCipher.brute_force(ct)
    found = Enum.find(results, fn r -> r.key == 3 end)
    assert found != nil
    assert found.text == original
  end

  test "brute force returns all keys 2 to n/2" do
    results = ScytaleCipher.brute_force("ABCDEFGHIJ")
    keys = Enum.map(results, fn r -> r.key end)
    assert keys == [2, 3, 4, 5]
  end

  test "brute force short text returns empty" do
    assert ScytaleCipher.brute_force("AB") == []
  end

  # --- Padding Tests ---

  test "padding stripped on decrypt" do
    ct = ScytaleCipher.encrypt("HELLO", 3)
    assert ScytaleCipher.decrypt(ct, 3) == "HELLO"
  end
end
