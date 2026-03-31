defmodule CodingAdventures.AtbashCipherTest do
  @moduledoc """
  Comprehensive tests for the Atbash cipher implementation.

  These tests verify that the Atbash cipher correctly reverses the alphabet
  for both uppercase and lowercase letters, preserves non-alphabetic
  characters, and satisfies the self-inverse property.
  """
  use ExUnit.Case
  doctest CodingAdventures.AtbashCipher

  alias CodingAdventures.AtbashCipher

  # --- Basic Encryption ---

  test "encrypts HELLO to SVOOL" do
    # H(7)->S(18), E(4)->V(21), L(11)->O(14), L(11)->O(14), O(14)->L(11)
    assert AtbashCipher.encrypt("HELLO") == "SVOOL"
  end

  test "encrypts hello to svool (case preservation)" do
    assert AtbashCipher.encrypt("hello") == "svool"
  end

  test "encrypts mixed case with punctuation" do
    assert AtbashCipher.encrypt("Hello, World! 123") == "Svool, Dliow! 123"
  end

  test "reverses full uppercase alphabet" do
    assert AtbashCipher.encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ") ==
             "ZYXWVUTSRQPONMLKJIHGFEDCBA"
  end

  test "reverses full lowercase alphabet" do
    assert AtbashCipher.encrypt("abcdefghijklmnopqrstuvwxyz") ==
             "zyxwvutsrqponmlkjihgfedcba"
  end

  # --- Case Preservation ---

  test "uppercase stays uppercase" do
    assert AtbashCipher.encrypt("ABC") == "ZYX"
  end

  test "lowercase stays lowercase" do
    assert AtbashCipher.encrypt("abc") == "zyx"
  end

  test "mixed case is preserved" do
    assert AtbashCipher.encrypt("AbCdEf") == "ZyXwVu"
  end

  # --- Non-Alpha Passthrough ---

  test "digits pass through unchanged" do
    assert AtbashCipher.encrypt("12345") == "12345"
  end

  test "punctuation passes through unchanged" do
    assert AtbashCipher.encrypt("!@#$%") == "!@#$%"
  end

  test "spaces pass through unchanged" do
    assert AtbashCipher.encrypt("   ") == "   "
  end

  test "mixed alpha and digits" do
    assert AtbashCipher.encrypt("A1B2C3") == "Z1Y2X3"
  end

  test "newlines and tabs pass through" do
    assert AtbashCipher.encrypt("A\nB\tC") == "Z\nY\tX"
  end

  # --- Self-Inverse Property ---

  test "self-inverse for HELLO" do
    assert AtbashCipher.encrypt(AtbashCipher.encrypt("HELLO")) == "HELLO"
  end

  test "self-inverse for lowercase" do
    assert AtbashCipher.encrypt(AtbashCipher.encrypt("hello")) == "hello"
  end

  test "self-inverse for mixed input" do
    input = "Hello, World! 123"
    assert AtbashCipher.encrypt(AtbashCipher.encrypt(input)) == input
  end

  test "self-inverse for full alphabet" do
    alpha = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    assert AtbashCipher.encrypt(AtbashCipher.encrypt(alpha)) == alpha
  end

  test "self-inverse for empty string" do
    assert AtbashCipher.encrypt(AtbashCipher.encrypt("")) == ""
  end

  test "self-inverse for long text" do
    text = "The quick brown fox jumps over the lazy dog! 42"
    assert AtbashCipher.encrypt(AtbashCipher.encrypt(text)) == text
  end

  # --- Edge Cases ---

  test "empty string" do
    assert AtbashCipher.encrypt("") == ""
  end

  test "single uppercase letters" do
    assert AtbashCipher.encrypt("A") == "Z"
    assert AtbashCipher.encrypt("Z") == "A"
    assert AtbashCipher.encrypt("M") == "N"
    assert AtbashCipher.encrypt("N") == "M"
  end

  test "single lowercase letters" do
    assert AtbashCipher.encrypt("a") == "z"
    assert AtbashCipher.encrypt("z") == "a"
  end

  test "single digit" do
    assert AtbashCipher.encrypt("5") == "5"
  end

  test "no letter maps to itself" do
    # 25 - p == p only when p == 12.5, which is not an integer
    for offset <- 0..25 do
      upper = <<(?A + offset)::utf8>>
      assert AtbashCipher.encrypt(upper) != upper, "#{upper} maps to itself!"

      lower = <<(?a + offset)::utf8>>
      assert AtbashCipher.encrypt(lower) != lower, "#{lower} maps to itself!"
    end
  end

  # --- Decrypt ---

  test "decrypts SVOOL to HELLO" do
    assert AtbashCipher.decrypt("SVOOL") == "HELLO"
  end

  test "decrypts svool to hello" do
    assert AtbashCipher.decrypt("svool") == "hello"
  end

  test "decrypt is the inverse of encrypt" do
    texts = ["HELLO", "hello", "Hello, World! 123", "", "42"]

    for text <- texts do
      assert AtbashCipher.decrypt(AtbashCipher.encrypt(text)) == text
    end
  end

  test "encrypt and decrypt produce identical output" do
    texts = ["HELLO", "svool", "Test!", ""]

    for text <- texts do
      assert AtbashCipher.encrypt(text) == AtbashCipher.decrypt(text)
    end
  end
end
