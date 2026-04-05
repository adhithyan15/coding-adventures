# ============================================================================
# CodingAdventures.VigenereCipher
# ============================================================================
#
# The Vigenere Cipher (1553)
# ==========================
#
# The Vigenere cipher is a *polyalphabetic substitution* cipher that uses a
# repeating keyword to apply different Caesar shifts at each position. Unlike
# Caesar (single shift) or Atbash (fixed mapping), each letter is shifted by
# a different amount determined by the corresponding letter of the keyword.
#
# How Encryption Works
# --------------------
#
# Given plaintext and keyword:
#
#     Plaintext:  A  T  T  A  C  K  A  T  D  A  W  N
#     Key cycle:  L  E  M  O  N  L  E  M  O  N  L  E
#     Shift:      11 4  12 14 13 11 4  12 14 13 11 4
#     Ciphertext: L  X  F  O  P  V  E  F  R  N  H  R
#
# Character Handling
# ------------------
#
# - Uppercase stays uppercase, lowercase stays lowercase.
# - Non-alphabetic characters pass through unchanged.
# - Key position advances only on alphabetic characters.
# - Key must be non-empty, alphabetic only.
#
# Cryptanalysis
# -------------
#
# The cipher was broken by Kasiski (1863) and Friedman (1920s) using:
# 1. Index of Coincidence (IC) to find the key length
# 2. Chi-squared statistic to find each key letter
#
# English text has IC ~0.0667 (letters are unevenly distributed).
# Random text has IC ~0.0385 (uniform = 1/26).
# When ciphertext is split by the correct key length, each group
# is a Caesar cipher with IC near the English value.

defmodule CodingAdventures.VigenereCipher do
  @moduledoc """
  Vigenere cipher -- polyalphabetic substitution cipher with cryptanalysis.

  Provides `encrypt/2`, `decrypt/2`, `find_key_length/1`, `find_key/2`,
  and `break_cipher/1`.
  """

  # English letter frequencies (A-Z), used for chi-squared analysis.
  # E (~12.7%) is the most common letter, Z (~0.07%) the rarest.
  @english_frequencies [
    0.08167, 0.01492, 0.02782, 0.04253, 0.12702, 0.02228, 0.02015,
    0.06094, 0.06966, 0.00153, 0.00772, 0.04025, 0.02406, 0.06749,
    0.07507, 0.01929, 0.00095, 0.05987, 0.06327, 0.09056, 0.02758,
    0.00978, 0.02360, 0.00150, 0.01974, 0.00074
  ]

  # ---------------------------------------------------------------------------
  # Encrypt
  # ---------------------------------------------------------------------------

  @doc """
  Encrypt plaintext using the Vigenere cipher.

  Each alphabetic character is shifted forward by the corresponding key
  letter's value (A/a=0, B/b=1, ..., Z/z=25). Non-alphabetic characters
  pass through unchanged and do not advance the key position.

  ## Examples

      iex> CodingAdventures.VigenereCipher.encrypt("ATTACKATDAWN", "LEMON")
      "LXFOPVEFRNHR"

      iex> CodingAdventures.VigenereCipher.encrypt("Hello, World!", "key")
      "Rijvs, Uyvjn!"
  """
  @spec encrypt(String.t(), String.t()) :: String.t()
  def encrypt(plaintext, key_str) when is_binary(plaintext) and is_binary(key_str) do
    validate_key!(key_str)
    upper_key = String.upcase(key_str) |> String.to_charlist()
    key_len = length(upper_key)

    plaintext
    |> String.to_charlist()
    |> do_encrypt(upper_key, key_len, 0, [])
    |> Enum.reverse()
    |> List.to_string()
  end

  defp do_encrypt([], _key, _key_len, _key_idx, acc), do: acc

  defp do_encrypt([ch | rest], key_chars, key_len, key_idx, acc) when ch >= ?A and ch <= ?Z do
    shift = Enum.at(key_chars, rem(key_idx, key_len)) - ?A
    shifted = rem(ch - ?A + shift, 26) + ?A
    do_encrypt(rest, key_chars, key_len, key_idx + 1, [shifted | acc])
  end

  defp do_encrypt([ch | rest], key_chars, key_len, key_idx, acc) when ch >= ?a and ch <= ?z do
    shift = Enum.at(key_chars, rem(key_idx, key_len)) - ?A
    shifted = rem(ch - ?a + shift, 26) + ?a
    do_encrypt(rest, key_chars, key_len, key_idx + 1, [shifted | acc])
  end

  defp do_encrypt([ch | rest], key_chars, key_len, key_idx, acc) do
    do_encrypt(rest, key_chars, key_len, key_idx, [ch | acc])
  end

  # ---------------------------------------------------------------------------
  # Decrypt
  # ---------------------------------------------------------------------------

  @doc """
  Decrypt ciphertext using the Vigenere cipher.

  Shifts each letter backward by the key letter's value. Adding 26 before
  taking mod 26 handles negative arithmetic.

  ## Examples

      iex> CodingAdventures.VigenereCipher.decrypt("LXFOPVEFRNHR", "LEMON")
      "ATTACKATDAWN"

      iex> CodingAdventures.VigenereCipher.decrypt("Rijvs, Uyvjn!", "key")
      "Hello, World!"
  """
  @spec decrypt(String.t(), String.t()) :: String.t()
  def decrypt(ciphertext, key_str) when is_binary(ciphertext) and is_binary(key_str) do
    validate_key!(key_str)
    upper_key = String.upcase(key_str) |> String.to_charlist()
    key_len = length(upper_key)

    ciphertext
    |> String.to_charlist()
    |> do_decrypt(upper_key, key_len, 0, [])
    |> Enum.reverse()
    |> List.to_string()
  end

  defp do_decrypt([], _key, _key_len, _key_idx, acc), do: acc

  defp do_decrypt([ch | rest], key_chars, key_len, key_idx, acc) when ch >= ?A and ch <= ?Z do
    shift = Enum.at(key_chars, rem(key_idx, key_len)) - ?A
    shifted = rem(ch - ?A - shift + 26, 26) + ?A
    do_decrypt(rest, key_chars, key_len, key_idx + 1, [shifted | acc])
  end

  defp do_decrypt([ch | rest], key_chars, key_len, key_idx, acc) when ch >= ?a and ch <= ?z do
    shift = Enum.at(key_chars, rem(key_idx, key_len)) - ?A
    shifted = rem(ch - ?a - shift + 26, 26) + ?a
    do_decrypt(rest, key_chars, key_len, key_idx + 1, [shifted | acc])
  end

  defp do_decrypt([ch | rest], key_chars, key_len, key_idx, acc) do
    do_decrypt(rest, key_chars, key_len, key_idx, [ch | acc])
  end

  # ---------------------------------------------------------------------------
  # Cryptanalysis: find_key_length
  # ---------------------------------------------------------------------------

  @doc """
  Estimate the key length using Index of Coincidence (IC) analysis.

  For each candidate key length k, splits the ciphertext into k groups
  and calculates the average IC. The correct key length produces groups
  that are each a Caesar cipher on English (IC ~0.0667). To avoid
  selecting multiples of the true key length, picks the smallest k
  whose IC is within 90% of the best.

  """
  @spec find_key_length(String.t(), pos_integer()) :: pos_integer()
  def find_key_length(ciphertext, max_len \\ 20) do
    letters = extract_alpha_upper(ciphertext)
    n = length(letters)

    if n < 2 do
      1
    else
      upper = min(max_len, div(n, 2))

      # Calculate average IC for each candidate key length
      avg_ics =
        for k <- 2..upper//1 do
          {total_ic, group_count} =
            Enum.reduce(0..(k - 1)//1, {0.0, 0}, fn i, {total, cnt} ->
              group =
                letters
                |> Enum.drop(i)
                |> Enum.take_every(k)

              if length(group) > 1 do
                {total + index_of_coincidence(group), cnt + 1}
              else
                {total, cnt}
              end
            end)

          avg = if group_count > 0, do: total_ic / group_count, else: 0.0
          {k, avg}
        end

      # Find the best IC value
      best_ic = avg_ics |> Enum.map(fn {_k, ic} -> ic end) |> Enum.max(fn -> 0.0 end)

      if best_ic <= 0.0 do
        1
      else
        # Pick the smallest k whose IC is within 90% of the best
        ic_threshold = best_ic * 0.9

        case Enum.find(avg_ics, fn {_k, ic} -> ic >= ic_threshold end) do
          {k, _ic} -> k
          nil -> 1
        end
      end
    end
  end

  # ---------------------------------------------------------------------------
  # Cryptanalysis: find_key
  # ---------------------------------------------------------------------------

  @doc """
  Find the key letters given a known key length.

  For each position in the key, extracts the group of every k-th letter,
  tries all 26 shifts, and picks the one with the lowest chi-squared
  against English letter frequencies.

  """
  @spec find_key(String.t(), pos_integer()) :: String.t()
  def find_key(ciphertext, key_length) do
    letters = extract_alpha_upper(ciphertext)

    0..(key_length - 1)//1
    |> Enum.map(fn pos ->
      group =
        letters
        |> Enum.drop(pos)
        |> Enum.take_every(key_length)

      if group == [] do
        ?A
      else
        # Try all 26 shifts, pick lowest chi-squared
        {best_shift, _best_chi2} =
          Enum.reduce(0..25, {0, :infinity}, fn shift_val, {best_s, best_c} ->
            counts =
              Enum.reduce(group, :array.new(26, {:default, 0}), fn ch, arr ->
                decrypted = rem(ch - ?A - shift_val + 26, 26)
                :array.set(decrypted, :array.get(decrypted, arr) + 1, arr)
              end)

            chi2 = chi_squared(counts, length(group))

            if chi2 < best_c do
              {shift_val, chi2}
            else
              {best_s, best_c}
            end
          end)

        ?A + best_shift
      end
    end)
    |> List.to_string()
  end

  # ---------------------------------------------------------------------------
  # Cryptanalysis: break_cipher
  # ---------------------------------------------------------------------------

  @doc """
  Automatically break a Vigenere cipher.

  Combines IC-based key length detection with chi-squared key recovery:
    1. Find the key length
    2. Find the key letters
    3. Decrypt using the recovered key

  Requires sufficiently long ciphertext (~200+ characters of English)
  for reliable results.

  Returns a map with `:key` and `:plaintext`.

  """
  @spec break_cipher(String.t()) :: %{key: String.t(), plaintext: String.t()}
  def break_cipher(ciphertext) do
    key_length = find_key_length(ciphertext)
    recovered_key = find_key(ciphertext, key_length)
    plaintext = decrypt(ciphertext, recovered_key)

    %{key: recovered_key, plaintext: plaintext}
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Validate that key is non-empty and contains only ASCII letters.
  defp validate_key!(key_str) do
    if key_str == "" do
      raise ArgumentError, "Key must not be empty"
    end

    unless Regex.match?(~r/^[a-zA-Z]+$/, key_str) do
      raise ArgumentError, "Key must contain only alphabetic characters"
    end
  end

  # Extract only alphabetic characters, converted to uppercase charlists.
  defp extract_alpha_upper(text) do
    text
    |> String.to_charlist()
    |> Enum.filter(fn ch -> (ch >= ?A and ch <= ?Z) or (ch >= ?a and ch <= ?z) end)
    |> Enum.map(fn ch ->
      if ch >= ?a and ch <= ?z, do: ch - 32, else: ch
    end)
  end

  # Calculate the Index of Coincidence for a charlist of uppercase letters.
  #
  # IC = sum(count_i * (count_i - 1)) / (N * (N - 1))
  defp index_of_coincidence(letters) do
    n = length(letters)

    if n <= 1 do
      0.0
    else
      counts =
        Enum.reduce(letters, :array.new(26, {:default, 0}), fn ch, arr ->
          idx = ch - ?A
          :array.set(idx, :array.get(idx, arr) + 1, arr)
        end)

      numerator =
        Enum.reduce(0..25, 0, fn i, sum ->
          c = :array.get(i, counts)
          sum + c * (c - 1)
        end)

      numerator / (n * (n - 1))
    end
  end

  # Calculate chi-squared statistic comparing observed counts to English.
  defp chi_squared(counts, total) do
    @english_frequencies
    |> Enum.with_index()
    |> Enum.reduce(0.0, fn {freq, i}, chi2 ->
      expected = freq * total
      observed = :array.get(i, counts)
      diff = observed - expected
      chi2 + diff * diff / expected
    end)
  end
end
