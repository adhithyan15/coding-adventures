defmodule CodingAdventures.AtbashCipher do
  @moduledoc """
  The Atbash cipher: a fixed reverse-alphabet substitution cipher.

  ## What is the Atbash Cipher?

  The Atbash cipher is one of the oldest known substitution ciphers,
  originally used with the Hebrew alphabet. The name "Atbash" comes from
  the first, last, second, and second-to-last letters of the Hebrew
  alphabet: Aleph-Tav-Beth-Shin.

  The cipher reverses the alphabet:

      Plain:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
      Cipher: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A

  ## The Formula

  Given a letter at position `p` (where A=0, B=1, ..., Z=25):

      encrypted_position = 25 - p

  For example, 'H' is at position 7: 25 - 7 = 18, which is 'S'.

  ## Self-Inverse Property

  The Atbash cipher is self-inverse: applying it twice returns the original.

      f(f(x)) = 25 - (25 - x) = x

  This means `encrypt/1` and `decrypt/1` are the same operation.

  ## Examples

      iex> CodingAdventures.AtbashCipher.encrypt("HELLO")
      "SVOOL"

      iex> CodingAdventures.AtbashCipher.encrypt("Hello, World! 123")
      "Svool, Dliow! 123"

      iex> CodingAdventures.AtbashCipher.decrypt("SVOOL")
      "HELLO"
  """

  @doc """
  Encrypt text using the Atbash cipher.

  Each letter is replaced by its reverse in the alphabet (A<->Z, B<->Y, etc.).
  Non-alphabetic characters pass through unchanged. Case is preserved.

  Because the Atbash cipher is self-inverse, this function is identical
  to `decrypt/1`. Both are provided for API clarity.

  ## Examples

      iex> CodingAdventures.AtbashCipher.encrypt("HELLO")
      "SVOOL"

      iex> CodingAdventures.AtbashCipher.encrypt("hello")
      "svool"

      iex> CodingAdventures.AtbashCipher.encrypt("Hello, World! 123")
      "Svool, Dliow! 123"
  """
  @spec encrypt(String.t()) :: String.t()
  def encrypt(text) when is_binary(text) do
    # Process the string as a charlist (list of codepoints), apply the
    # Atbash substitution to each character, then convert back to a string.
    text
    |> String.to_charlist()
    |> Enum.map(&atbash_char/1)
    |> List.to_string()
  end

  @doc """
  Decrypt text using the Atbash cipher.

  Because the Atbash cipher is self-inverse (applying it twice returns
  the original), decryption is identical to encryption. This function
  exists for API clarity.

  ## Examples

      iex> CodingAdventures.AtbashCipher.decrypt("SVOOL")
      "HELLO"
  """
  @spec decrypt(String.t()) :: String.t()
  def decrypt(text) when is_binary(text) do
    # Decryption IS encryption for Atbash.
    # Proof: f(f(x)) = 25 - (25 - x) = x
    encrypt(text)
  end

  # --- Private helper: apply Atbash to a single codepoint ---
  #
  # Elixir represents strings as UTF-8 binaries. When we convert to a
  # charlist, each element is an integer codepoint. We pattern match
  # on the ranges for uppercase (65-90) and lowercase (97-122) ASCII
  # letters.
  #
  # The algorithm for each letter:
  # 1. Compute position = codepoint - base (where base is ?A or ?a)
  # 2. Reverse: new_position = 25 - position
  # 3. Convert back: base + new_position

  # Uppercase letters: A(?A=65) through Z(?Z=90)
  defp atbash_char(codepoint) when codepoint >= ?A and codepoint <= ?Z do
    pos = codepoint - ?A        # A=0, B=1, ..., Z=25
    new_pos = 25 - pos          # Reverse the position
    ?A + new_pos                # Convert back to a codepoint
  end

  # Lowercase letters: a(?a=97) through z(?z=122)
  defp atbash_char(codepoint) when codepoint >= ?a and codepoint <= ?z do
    pos = codepoint - ?a        # a=0, b=1, ..., z=25
    new_pos = 25 - pos          # Reverse the position
    ?a + new_pos                # Convert back to a codepoint
  end

  # Non-alphabetic characters pass through unchanged
  defp atbash_char(codepoint), do: codepoint
end
