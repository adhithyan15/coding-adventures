# ============================================================================
# CodingAdventures.ScytaleCipher
# ============================================================================
#
# The Scytale (pronounced "SKIT-ah-lee") cipher is a *transposition* cipher
# from ancient Sparta (~700 BCE). Unlike substitution ciphers (Caesar, Atbash)
# which replace characters, the Scytale rearranges character positions using
# a columnar transposition.
#
# How Encryption Works
# --------------------
#
# 1. Write text row-by-row into a grid with `key` columns.
# 2. Pad the last row with spaces if needed.
# 3. Read column-by-column to produce ciphertext.
#
# Example: encrypt("HELLO WORLD", 3)
#
#     Grid (4 rows x 3 cols):
#         H E L
#         L O ' '
#         W O R
#         L D ' '
#
#     Columns: HLWL + EOOD + L R  = "HLWLEOODL R "
#
# How Decryption Works
# --------------------
#
# 1. Calculate rows = ceil(len / key).
# 2. Write ciphertext column-by-column.
# 3. Read row-by-row and strip trailing padding spaces.

defmodule CodingAdventures.ScytaleCipher do
  @moduledoc """
  Scytale transposition cipher from ancient Sparta (~700 BCE).

  Provides `encrypt/2`, `decrypt/2`, and `brute_force/1`.
  """

  @doc """
  Encrypt text using the Scytale transposition cipher.

  ## Examples

      iex> CodingAdventures.ScytaleCipher.encrypt("HELLO WORLD", 3)
      "HLWLEOODL R "

      iex> CodingAdventures.ScytaleCipher.encrypt("ABCDEF", 2)
      "ACEBDF"
  """
  @spec encrypt(String.t(), pos_integer()) :: String.t()
  def encrypt("", _key), do: ""

  def encrypt(text, key) when is_binary(text) and is_integer(key) do
    chars = String.graphemes(text)
    n = length(chars)

    if key < 2, do: raise(ArgumentError, "Key must be >= 2, got #{key}")
    if key > n, do: raise(ArgumentError, "Key must be <= text length (#{n}), got #{key}")

    # Pad to fill the grid
    num_rows = ceil_div(n, key)
    padded_len = num_rows * key
    pad_count = padded_len - n
    padded = chars ++ List.duplicate(" ", pad_count)

    # Read column-by-column
    0..(key - 1)//1
    |> Enum.flat_map(fn col ->
      0..(num_rows - 1)//1
      |> Enum.map(fn row -> Enum.at(padded, row * key + col) end)
    end)
    |> Enum.join()
  end

  @doc """
  Decrypt ciphertext encrypted with the Scytale cipher.

  Trailing padding spaces are stripped.

  ## Examples

      iex> CodingAdventures.ScytaleCipher.decrypt("HLWLEOODL R ", 3)
      "HELLO WORLD"
  """
  @spec decrypt(String.t(), pos_integer()) :: String.t()
  def decrypt("", _key), do: ""

  def decrypt(text, key) when is_binary(text) and is_integer(key) do
    chars = String.graphemes(text)
    n = length(chars)

    if key < 2, do: raise(ArgumentError, "Key must be >= 2, got #{key}")
    if key > n, do: raise(ArgumentError, "Key must be <= text length (#{n}), got #{key}")

    num_rows = ceil_div(n, key)

    # Handle uneven grids (when n % key != 0, e.g. during brute-force)
    full_cols = if rem(n, key) == 0, do: key, else: rem(n, key)

    # Compute column start indices and lengths
    {col_starts, col_lens, _} =
      Enum.reduce(0..(key - 1)//1, {[], [], 0}, fn c, {starts, lens, offset} ->
        col_len = if rem(n, key) == 0 or c < full_cols, do: num_rows, else: num_rows - 1
        {starts ++ [offset], lens ++ [col_len], offset + col_len}
      end)

    # Read row-by-row
    0..(num_rows - 1)//1
    |> Enum.flat_map(fn row ->
      0..(key - 1)//1
      |> Enum.filter(fn col -> row < Enum.at(col_lens, col) end)
      |> Enum.map(fn col -> Enum.at(chars, Enum.at(col_starts, col) + row) end)
    end)
    |> Enum.join()
    |> String.trim_trailing(" ")
  end

  @doc """
  Try all possible keys from 2 to len/2 and return decryption results.

  ## Examples

      iex> results = CodingAdventures.ScytaleCipher.brute_force("ACEBDF")
      iex> hd(results)
      %{key: 2, text: "ABCDEF"}
  """
  @spec brute_force(String.t()) :: [%{key: pos_integer(), text: String.t()}]
  def brute_force(text) when is_binary(text) do
    n = String.length(text)

    if n < 4 do
      []
    else
      max_key = div(n, 2)

      2..max_key//1
      |> Enum.map(fn candidate_key ->
        %{key: candidate_key, text: decrypt(text, candidate_key)}
      end)
    end
  end

  # Integer ceiling division: ceil(a / b)
  defp ceil_div(a, b), do: div(a + b - 1, b)
end
