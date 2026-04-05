defmodule CodingAdventures.VigenereCipherTest do
  use ExUnit.Case
  doctest CodingAdventures.VigenereCipher

  alias CodingAdventures.VigenereCipher

  # Long English text for cryptanalysis tests. IC analysis needs a
  # statistically significant sample to reliably detect the key length.
  @long_english_text (
    "The quick brown fox jumps over the lazy dog and then runs around the " <>
    "entire neighborhood looking for more adventures to embark upon while " <>
    "the sun slowly sets behind the distant mountains casting long shadows " <>
    "across the valley below where the river winds its way through ancient " <>
    "forests filled with towering oak trees and singing birds that herald " <>
    "the coming of spring with their melodious songs echoing through the " <>
    "canopy above where squirrels chase each other from branch to branch " <>
    "gathering acorns and other nuts for the long winter months ahead when " <>
    "the ground will be covered in a thick blanket of pristine white snow " <>
    "and the children will build snowmen and throw snowballs at each other " <>
    "laughing and playing until their parents call them inside for dinner " <>
    "where warm soup and fresh bread await them on the old wooden table"
  )

  # ---------------------------------------------------------------------------
  # Encrypt tests
  # ---------------------------------------------------------------------------

  describe "encrypt/2" do
    test "encrypts ATTACKATDAWN with key LEMON" do
      assert VigenereCipher.encrypt("ATTACKATDAWN", "LEMON") == "LXFOPVEFRNHR"
    end

    test "preserves case and punctuation" do
      assert VigenereCipher.encrypt("Hello, World!", "key") == "Rijvs, Uyvjn!"
    end

    test "handles all-lowercase text" do
      assert VigenereCipher.encrypt("attackatdawn", "lemon") == "lxfopvefrnhr"
    end

    test "handles mixed case key" do
      assert VigenereCipher.encrypt("ATTACKATDAWN", "LeMoN") == "LXFOPVEFRNHR"
    end

    test "handles single-char key" do
      assert VigenereCipher.encrypt("ABC", "B") == "BCD"
    end

    test "skips non-alpha for key advancement" do
      assert VigenereCipher.encrypt("A T", "LE") == "L X"
    end

    test "handles digits and special chars unchanged" do
      assert VigenereCipher.encrypt("Hello 123!", "key") == "Rijvs 123!"
    end

    test "handles empty plaintext" do
      assert VigenereCipher.encrypt("", "key") == ""
    end

    test "raises on empty key" do
      assert_raise ArgumentError, ~r/Key must not be empty/, fn ->
        VigenereCipher.encrypt("hello", "")
      end
    end

    test "raises on non-alpha key" do
      assert_raise ArgumentError, ~r/Key must contain only alphabetic/, fn ->
        VigenereCipher.encrypt("hello", "key1")
      end

      assert_raise ArgumentError, ~r/Key must contain only alphabetic/, fn ->
        VigenereCipher.encrypt("hello", "ke y")
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Decrypt tests
  # ---------------------------------------------------------------------------

  describe "decrypt/2" do
    test "decrypts LXFOPVEFRNHR with key LEMON" do
      assert VigenereCipher.decrypt("LXFOPVEFRNHR", "LEMON") == "ATTACKATDAWN"
    end

    test "preserves case and punctuation" do
      assert VigenereCipher.decrypt("Rijvs, Uyvjn!", "key") == "Hello, World!"
    end

    test "handles all-lowercase" do
      assert VigenereCipher.decrypt("lxfopvefrnhr", "lemon") == "attackatdawn"
    end

    test "handles empty ciphertext" do
      assert VigenereCipher.decrypt("", "key") == ""
    end

    test "raises on empty key" do
      assert_raise ArgumentError, ~r/Key must not be empty/, fn ->
        VigenereCipher.decrypt("hello", "")
      end
    end

    test "raises on non-alpha key" do
      assert_raise ArgumentError, ~r/Key must contain only alphabetic/, fn ->
        VigenereCipher.decrypt("hello", "123")
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Round-trip tests
  # ---------------------------------------------------------------------------

  describe "round-trip" do
    @round_trip_cases [
      {"ATTACKATDAWN", "LEMON"},
      {"Hello, World!", "key"},
      {"The quick brown fox!", "SECRET"},
      {"abc def ghi", "xyz"},
      {"MiXeD CaSe 123", "AbCdE"},
      {"a", "z"},
      {"ZZZZZZ", "A"}
    ]

    for {text, key_val} <- @round_trip_cases do
      test "decrypt(encrypt(#{inspect(text)}, #{inspect(key_val)})) == original" do
        text = unquote(text)
        key_val = unquote(key_val)
        encrypted = VigenereCipher.encrypt(text, key_val)
        assert VigenereCipher.decrypt(encrypted, key_val) == text
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Cryptanalysis: find_key_length
  # ---------------------------------------------------------------------------

  describe "find_key_length/1" do
    test "finds key length for text encrypted with a 5-letter key" do
      ct = VigenereCipher.encrypt(@long_english_text, "LEMON")
      assert VigenereCipher.find_key_length(ct) == 5
    end

    test "finds key length for text encrypted with a 6-letter key" do
      ct = VigenereCipher.encrypt(@long_english_text, "SECRET")
      assert VigenereCipher.find_key_length(ct) == 6
    end

    test "finds key length for text encrypted with a 3-letter key" do
      ct = VigenereCipher.encrypt(@long_english_text, "KEY")
      assert VigenereCipher.find_key_length(ct) == 3
    end

    test "returns 1 for very short text" do
      assert VigenereCipher.find_key_length("A") == 1
    end
  end

  # ---------------------------------------------------------------------------
  # Cryptanalysis: find_key
  # ---------------------------------------------------------------------------

  describe "find_key/2" do
    test "recovers LEMON from ciphertext with known key length 5" do
      ct = VigenereCipher.encrypt(@long_english_text, "LEMON")
      assert VigenereCipher.find_key(ct, 5) == "LEMON"
    end

    test "recovers SECRET from ciphertext with known key length 6" do
      ct = VigenereCipher.encrypt(@long_english_text, "SECRET")
      assert VigenereCipher.find_key(ct, 6) == "SECRET"
    end

    test "recovers KEY from ciphertext with known key length 3" do
      ct = VigenereCipher.encrypt(@long_english_text, "KEY")
      assert VigenereCipher.find_key(ct, 3) == "KEY"
    end
  end

  # ---------------------------------------------------------------------------
  # Cryptanalysis: break_cipher
  # ---------------------------------------------------------------------------

  describe "break_cipher/1" do
    test "automatically breaks cipher with key LEMON" do
      ct = VigenereCipher.encrypt(@long_english_text, "LEMON")
      result = VigenereCipher.break_cipher(ct)
      assert result.key == "LEMON"
      assert result.plaintext == @long_english_text
    end

    test "automatically breaks cipher with key SECRET" do
      ct = VigenereCipher.encrypt(@long_english_text, "SECRET")
      result = VigenereCipher.break_cipher(ct)
      assert result.key == "SECRET"
      assert result.plaintext == @long_english_text
    end

    test "recovered plaintext is self-consistent" do
      ct = VigenereCipher.encrypt(@long_english_text, "CIPHER")
      result = VigenereCipher.break_cipher(ct)
      rt = VigenereCipher.decrypt(VigenereCipher.encrypt(@long_english_text, result.key), result.key)
      assert rt == @long_english_text
    end
  end
end
