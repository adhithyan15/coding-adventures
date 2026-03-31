defmodule CodingAdventures.CaesarCipherTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.CaesarCipher

  # ---------------------------------------------------------------------------
  # Encryption
  # ---------------------------------------------------------------------------

  describe "encrypt/2" do
    test "shifts uppercase letters forward" do
      assert CaesarCipher.encrypt("ABC", 1) == "BCD"
      assert CaesarCipher.encrypt("HELLO", 3) == "KHOOR"
    end

    test "shifts lowercase letters forward" do
      assert CaesarCipher.encrypt("abc", 1) == "bcd"
      assert CaesarCipher.encrypt("hello", 3) == "khoor"
    end

    test "wraps around at end of alphabet" do
      assert CaesarCipher.encrypt("XYZ", 3) == "ABC"
      assert CaesarCipher.encrypt("xyz", 3) == "abc"
    end

    test "preserves case in mixed-case text" do
      assert CaesarCipher.encrypt("Hello", 3) == "Khoor"
      assert CaesarCipher.encrypt("HeLLo", 1) == "IfMMp"
    end

    test "passes through non-alpha characters unchanged" do
      assert CaesarCipher.encrypt("Hello, World!", 3) == "Khoor, Zruog!"
      assert CaesarCipher.encrypt("123 ABC", 1) == "123 BCD"
      assert CaesarCipher.encrypt("a-b-c", 1) == "b-c-d"
    end

    test "handles empty string" do
      assert CaesarCipher.encrypt("", 5) == ""
    end

    test "shift of 0 returns original text" do
      assert CaesarCipher.encrypt("Hello", 0) == "Hello"
    end

    test "shift of 26 returns original text (full rotation)" do
      assert CaesarCipher.encrypt("Hello", 26) == "Hello"
    end

    test "shift of 52 returns original text (double rotation)" do
      assert CaesarCipher.encrypt("Hello", 52) == "Hello"
    end

    test "negative shift shifts backward" do
      assert CaesarCipher.encrypt("BCD", -1) == "ABC"
      assert CaesarCipher.encrypt("KHOOR", -3) == "HELLO"
    end

    test "shift of -26 returns original text" do
      assert CaesarCipher.encrypt("Hello", -26) == "Hello"
    end

    test "large positive shift wraps correctly" do
      # shift 27 == shift 1
      assert CaesarCipher.encrypt("A", 27) == "B"
    end

    test "large negative shift wraps correctly" do
      # shift -27 == shift -1 == shift 25
      assert CaesarCipher.encrypt("B", -27) == "A"
    end
  end

  # ---------------------------------------------------------------------------
  # Decryption
  # ---------------------------------------------------------------------------

  describe "decrypt/2" do
    test "reverses encryption" do
      assert CaesarCipher.decrypt("KHOOR", 3) == "HELLO"
      assert CaesarCipher.decrypt("bcd", 1) == "abc"
    end

    test "handles mixed case and non-alpha" do
      assert CaesarCipher.decrypt("Khoor, Zruog!", 3) == "Hello, World!"
    end

    test "handles empty string" do
      assert CaesarCipher.decrypt("", 5) == ""
    end

    test "shift of 0 returns original" do
      assert CaesarCipher.decrypt("HELLO", 0) == "HELLO"
    end
  end

  # ---------------------------------------------------------------------------
  # Round-trip (encrypt then decrypt)
  # ---------------------------------------------------------------------------

  describe "round-trip encrypt/decrypt" do
    test "decrypt(encrypt(text, s), s) == text for various shifts" do
      original = "The quick brown fox jumps over the lazy dog!"

      for shift <- [-100, -25, -1, 0, 1, 13, 25, 26, 100] do
        encrypted = CaesarCipher.encrypt(original, shift)
        decrypted = CaesarCipher.decrypt(encrypted, shift)
        assert decrypted == original, "Round-trip failed for shift #{shift}"
      end
    end

    test "round-trip preserves all character types" do
      original = "ABC abc 123 !@# \n\t"
      encrypted = CaesarCipher.encrypt(original, 7)
      assert CaesarCipher.decrypt(encrypted, 7) == original
    end
  end

  # ---------------------------------------------------------------------------
  # ROT13
  # ---------------------------------------------------------------------------

  describe "rot13/1" do
    test "encrypts with shift 13" do
      assert CaesarCipher.rot13("HELLO") == "URYYB"
      assert CaesarCipher.rot13("hello") == "uryyb"
    end

    test "is its own inverse (self-inverse property)" do
      original = "Hello, World!"
      assert CaesarCipher.rot13(CaesarCipher.rot13(original)) == original
    end

    test "no letter maps to itself" do
      # Every letter in the alphabet should change under ROT13
      alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
      rotated = CaesarCipher.rot13(alphabet)

      alphabet
      |> String.graphemes()
      |> Enum.zip(String.graphemes(rotated))
      |> Enum.each(fn {orig, shifted} ->
        assert orig != shifted, "Letter #{orig} maps to itself under ROT13"
      end)
    end

    test "passes through non-alpha characters" do
      assert CaesarCipher.rot13("123 !@#") == "123 !@#"
    end

    test "handles empty string" do
      assert CaesarCipher.rot13("") == ""
    end
  end

  # ---------------------------------------------------------------------------
  # Brute Force
  # ---------------------------------------------------------------------------

  describe "brute_force/1" do
    test "returns exactly 25 results (shifts 1 through 25)" do
      results = CaesarCipher.brute_force("KHOOR")
      assert length(results) == 25
    end

    test "each result is a {shift, plaintext} tuple" do
      results = CaesarCipher.brute_force("KHOOR")

      Enum.each(results, fn result ->
        assert {shift, text} = result
        assert is_integer(shift)
        assert is_binary(text)
      end)
    end

    test "shifts range from 1 to 25" do
      results = CaesarCipher.brute_force("KHOOR")
      shifts = Enum.map(results, fn {s, _text} -> s end)
      assert shifts == Enum.to_list(1..25)
    end

    test "contains the correct decryption" do
      # "KHOOR" was encrypted from "HELLO" with shift 3
      results = CaesarCipher.brute_force("KHOOR")
      match = Enum.find(results, fn {_s, text} -> text == "HELLO" end)
      assert match == {3, "HELLO"}
    end

    test "works with mixed case and non-alpha" do
      ciphertext = CaesarCipher.encrypt("Hello, World!", 5)
      results = CaesarCipher.brute_force(ciphertext)
      match = Enum.find(results, fn {_s, text} -> text == "Hello, World!" end)
      assert match == {5, "Hello, World!"}
    end

    test "handles empty string" do
      results = CaesarCipher.brute_force("")
      assert length(results) == 25
      assert Enum.all?(results, fn {_s, text} -> text == "" end)
    end
  end

  # ---------------------------------------------------------------------------
  # Frequency Analysis
  # ---------------------------------------------------------------------------

  describe "frequency_analysis/1" do
    test "identifies shift for a known English sentence" do
      plaintext = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG"
      ciphertext = CaesarCipher.encrypt(plaintext, 7)
      {detected_shift, decrypted} = CaesarCipher.frequency_analysis(ciphertext)
      assert detected_shift == 7
      assert decrypted == plaintext
    end

    test "identifies shift for lowercase text" do
      plaintext = "the quick brown fox jumps over the lazy dog"
      ciphertext = CaesarCipher.encrypt(plaintext, 13)
      {detected_shift, decrypted} = CaesarCipher.frequency_analysis(ciphertext)
      assert detected_shift == 13
      assert decrypted == plaintext
    end

    test "identifies shift 3 for longer text encrypted with shift 3" do
      plaintext = "HELLO WORLD THIS IS A LONGER SENTENCE FOR FREQUENCY ANALYSIS TO WORK PROPERLY"
      ciphertext = CaesarCipher.encrypt(plaintext, 3)
      {detected_shift, decrypted} = CaesarCipher.frequency_analysis(ciphertext)
      assert detected_shift == 3
      assert decrypted == plaintext
    end

    test "works with longer English text" do
      plaintext =
        "It was the best of times it was the worst of times " <>
        "it was the age of wisdom it was the age of foolishness"

      ciphertext = CaesarCipher.encrypt(plaintext, 19)
      {detected_shift, decrypted} = CaesarCipher.frequency_analysis(ciphertext)
      assert detected_shift == 19
      assert decrypted == plaintext
    end

    test "handles text with non-alpha characters" do
      plaintext = "To be, or not to be -- that is the question!"
      ciphertext = CaesarCipher.encrypt(plaintext, 10)
      {detected_shift, decrypted} = CaesarCipher.frequency_analysis(ciphertext)
      assert detected_shift == 10
      assert decrypted == plaintext
    end

    test "returns shift 0 for unencrypted English text" do
      plaintext = "The quick brown fox jumps over the lazy dog"
      {detected_shift, decrypted} = CaesarCipher.frequency_analysis(plaintext)
      assert detected_shift == 0
      assert decrypted == plaintext
    end
  end
end
