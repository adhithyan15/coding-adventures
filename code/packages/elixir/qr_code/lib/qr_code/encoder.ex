defmodule CodingAdventures.QrCode.Encoder do
  @moduledoc """
  Data encoding: numeric, alphanumeric, and byte modes.

  This module turns a raw UTF-8 string into the sequence of data codewords
  (bytes) that QR Code stores in the symbol. The pipeline is:

  ```
  input string
    → mode selection    (numeric / alphanumeric / byte)
    → version selection (smallest version at chosen ECC level)
    → bit stream        (mode indicator + char count + data + terminator + padding)
    → data codewords    (exactly num_data_codewords(version, ecc) bytes)
  ```

  ## Encoding modes

  QR Code defines four encoding modes. This module implements three:

  | Mode         | Characters          | Bits/char (approx) |
  |--------------|---------------------|---------------------|
  | Numeric      | 0–9                 | 3.33 (10/3)         |
  | Alphanumeric | 0–9, A–Z, 9 symbols | 5.5 (11/2)          |
  | Byte         | Any UTF-8 byte      | 8.0                 |

  Kanji mode (Shift-JIS 2-byte characters → 13 bits each) is deferred to v0.2.0.

  ## Mode indicator bits

  The first 4 bits of the bit stream identify the encoding mode:

      Numeric      → 0001
      Alphanumeric → 0010
      Byte         → 0100

  ## Character count field width

  The count field that follows the mode indicator has a width that depends on
  both the mode and the version group:

  | Mode         | Versions 1–9 | Versions 10–26 | Versions 27–40 |
  |--------------|-------------|----------------|----------------|
  | Numeric      | 10 bits     | 12 bits        | 14 bits        |
  | Alphanumeric |  9 bits     | 11 bits        | 13 bits        |
  | Byte         |  8 bits     | 16 bits        | 16 bits        |

  ## Bit stream padding

  After the data, the bit stream is padded to exactly `num_data_codewords × 8` bits:

  1. Terminator: up to 4 zero bits (fewer if at capacity).
  2. Byte alignment: zero bits to reach a byte boundary.
  3. Fill bytes: alternating 0xEC, 0x11 to fill remaining codewords.

  The alternating fill bytes (0b11101100, 0b00010001) have no special
  meaning — they are just chosen to be non-zero so fill isn't confused
  with data.
  """

  import Bitwise

  alias CodingAdventures.QrCode.Tables

  # 45-character alphanumeric set, in QR Code index order (0–44).
  # Pairs encode as: (first_idx × 45 + second_idx) → 11 bits.
  # Trailing single character → 6 bits.
  @alphanum_chars ~c"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"

  # Mode indicator bit patterns (4 bits each).
  @mode_indicator %{numeric: 0b0001, alphanumeric: 0b0010, byte: 0b0100}

  # Padding bytes used to fill remaining data codeword capacity.
  # Alternate 0xEC → 0x11 → 0xEC → ... The alternation prevents long
  # runs of the same byte, which helps mask evaluation.
  @pad1 0xEC
  @pad2 0x11

  # ---------------------------------------------------------------------------
  # Mode selection
  # ---------------------------------------------------------------------------

  @doc """
  Select the most compact encoding mode that covers the entire input.

  Preference order (most compact to least): numeric > alphanumeric > byte.

  A mode is selected only if it can encode EVERY character in the input:
  - Numeric: all characters must be ASCII digits '0'–'9'.
  - Alphanumeric: all characters must be in the 45-char QR alphanumeric set.
  - Byte: always valid (UTF-8 bytes).

  This is the v0.1.0 heuristic — mixed-mode segmentation (e.g., numeric
  run inside a byte string) is a v0.2.0 enhancement.
  """
  @spec select_mode(String.t()) :: :numeric | :alphanumeric | :byte
  def select_mode(input) do
    cond do
      numeric?(input) -> :numeric
      alphanumeric?(input) -> :alphanumeric
      true -> :byte
    end
  end

  @doc """
  Test whether all characters in the input are ASCII digits (0–9).
  Empty string returns true (numeric mode is valid).
  """
  @spec numeric?(String.t()) :: boolean()
  def numeric?(input) do
    String.match?(input, ~r/^\d*$/)
  end

  @doc """
  Test whether all characters are in the QR alphanumeric set (0–9, A–Z,
  space, $, %, *, +, -, ., /, :).
  """
  @spec alphanumeric?(String.t()) :: boolean()
  def alphanumeric?(input) do
    input
    |> :unicode.characters_to_list()
    |> Enum.all?(fn ch -> ch in @alphanum_chars end)
  end

  # ---------------------------------------------------------------------------
  # Character count field width
  # ---------------------------------------------------------------------------

  @doc """
  Width in bits of the character count field for a given mode and version.

  The width grows with version number to accommodate larger symbol capacities:
  - Versions 1–9: narrow counts (8–10 bits).
  - Versions 10–26: wider counts (11–16 bits).
  - Versions 27–40: widest counts (13–16 bits).
  """
  @spec char_count_bits(:numeric | :alphanumeric | :byte, pos_integer()) :: pos_integer()
  def char_count_bits(:numeric, version) when version <= 9, do: 10
  def char_count_bits(:numeric, version) when version <= 26, do: 12
  def char_count_bits(:numeric, _version), do: 14

  def char_count_bits(:alphanumeric, version) when version <= 9, do: 9
  def char_count_bits(:alphanumeric, version) when version <= 26, do: 11
  def char_count_bits(:alphanumeric, _version), do: 13

  def char_count_bits(:byte, version) when version <= 9, do: 8
  def char_count_bits(:byte, _version), do: 16

  # ---------------------------------------------------------------------------
  # Version selection
  # ---------------------------------------------------------------------------

  @doc """
  Find the minimum QR version (1–40) that fits the input at the chosen ECC level.

  Returns `{:ok, version}` on success.
  Returns `{:error, :input_too_long}` if the input exceeds version-40 capacity.

  ## How it works

  For each candidate version (1, 2, ..., 40), compute how many bits are
  needed:
    - 4 bits for the mode indicator
    - char_count_bits(mode, version) for the character count
    - data bits (mode-specific)

  Compare to `num_data_codewords(version, ecc) × 8` bits available. Choose
  the first version where the bits needed ≤ bits available.

  Note: the char count field width changes at version boundaries (9→10 and
  26→27), so we must recompute needed bits for each candidate version.
  """
  @spec select_version(String.t(), atom()) :: {:ok, pos_integer()} | {:error, :input_too_long}
  def select_version(input, ecc) do
    mode = select_mode(input)
    byte_len = byte_size(input)
    char_len = String.length(input)

    Enum.find_value(1..40, {:error, :input_too_long}, fn v ->
      capacity_bits = Tables.num_data_codewords(v, ecc) * 8

      data_bits =
        case mode do
          :byte -> byte_len * 8
          :numeric -> numeric_bit_count(char_len)
          :alphanumeric -> alphanumeric_bit_count(char_len)
        end

      bits_needed = 4 + char_count_bits(mode, v) + data_bits
      # Check: if ceil(bits_needed / 8) <= capacity_codewords, this version fits.
      # Equivalently: bits_needed <= capacity_bits (capacity_bits is already ×8).
      if bits_needed <= capacity_bits do
        {:ok, v}
      else
        # Check with ceiling: partial last byte counts
        if div(bits_needed + 7, 8) <= div(capacity_bits, 8), do: {:ok, v}, else: nil
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Numeric encoding bit count helpers
  # ---------------------------------------------------------------------------

  # Numeric mode: groups of 3 digits → 10 bits, pairs → 7 bits, singles → 4 bits.
  # Total bits = 10*(n÷3) + 7*(rem(n,3)==2) + 4*(rem(n,3)==1)
  defp numeric_bit_count(n) do
    full = div(n, 3)
    leftover = rem(n, 3)
    extra = if leftover == 2, do: 7, else: if(leftover == 1, do: 4, else: 0)
    full * 10 + extra
  end

  # Alphanumeric mode: pairs → 11 bits, single trailing → 6 bits.
  defp alphanumeric_bit_count(n) do
    pairs = div(n, 2)
    extra = if rem(n, 2) == 1, do: 6, else: 0
    pairs * 11 + extra
  end

  # ---------------------------------------------------------------------------
  # Bit stream assembly
  # ---------------------------------------------------------------------------

  @doc """
  Build the complete data codeword sequence for encoding `input` at the given
  version and ECC level.

  Returns a list of exactly `num_data_codewords(version, ecc)` bytes.

  The format is:
  ```
  [mode indicator 4 bits]
  [character count field (width depends on mode and version)]
  [encoded data bits (mode-specific)]
  [terminator: up to 4 zero bits, fewer if at capacity]
  [zero-padding to the next byte boundary]
  [alternating 0xEC / 0x11 fill bytes to capacity]
  ```
  """
  @spec build_data_codewords(String.t(), pos_integer(), atom()) :: [byte()]
  def build_data_codewords(input, version, ecc) do
    mode = select_mode(input)
    capacity = Tables.num_data_codewords(version, ecc)

    # Write mode indicator (4 bits).
    bits = write_bits([], @mode_indicator[mode], 4)

    # Write character count.
    char_count =
      case mode do
        :byte -> byte_size(input)
        _ -> String.length(input)
      end

    bits = write_bits(bits, char_count, char_count_bits(mode, version))

    # Write encoded data bits.
    bits =
      case mode do
        :numeric -> encode_numeric(input, bits)
        :alphanumeric -> encode_alphanumeric(input, bits)
        :byte -> encode_byte(input, bits)
      end

    # Terminator: up to 4 zero bits.
    capacity_bits = capacity * 8
    term_len = min(4, capacity_bits - length(bits))
    bits = if term_len > 0, do: write_bits(bits, 0, term_len), else: bits

    # Zero-pad to byte boundary.
    remainder = rem(length(bits), 8)
    bits = if remainder != 0, do: write_bits(bits, 0, 8 - remainder), else: bits

    # Convert to bytes.
    bytes = bits_to_bytes(bits)

    # Fill with alternating 0xEC / 0x11.
    fill(bytes, capacity)
  end

  # ---------------------------------------------------------------------------
  # Bit writer helpers
  # ---------------------------------------------------------------------------

  # Append `count` bits from `value` (MSB first) to the bit list.
  # A bit list is a list of 0s and 1s (integers), LSB of the list is the most
  # recently appended bit. We prepend in reversed order then reverse at the end.
  #
  # Actually we use a simpler approach: we build a flat list in order.
  defp write_bits(bits, value, count) do
    new_bits =
      for i <- (count - 1)..0//-1 do
        (value >>> i) &&& 1
      end

    bits ++ new_bits
  end

  # Convert a flat bit list (MSB first) to a list of bytes.
  defp bits_to_bytes(bits) do
    bits
    |> Enum.chunk_every(8)
    |> Enum.map(fn chunk ->
      # Pad chunk to 8 if it's shorter (should not happen after byte alignment).
      padded = chunk ++ List.duplicate(0, 8 - length(chunk))
      Enum.reduce(padded, 0, fn bit, acc -> (acc <<< 1) ||| bit end)
    end)
  end

  # Fill bytes with alternating 0xEC / 0x11 until we reach capacity.
  defp fill(bytes, capacity) when length(bytes) >= capacity, do: Enum.take(bytes, capacity)

  defp fill(bytes, capacity) do
    to_add = capacity - length(bytes)

    pad_bytes =
      Enum.map(0..(to_add - 1), fn i ->
        if rem(i, 2) == 0, do: @pad1, else: @pad2
      end)

    bytes ++ pad_bytes
  end

  # ---------------------------------------------------------------------------
  # Numeric mode encoding
  # ---------------------------------------------------------------------------

  @doc """
  Encode a numeric-only string into bit groups.

  Groups of 3 digits → 10 bits (range 0–999).
  Pairs of 2 digits  →  7 bits (range 0–99).
  Single trailing digit → 4 bits (range 0–9).

  Example: "01234567" → ["012", "345", "67"]
             → 10 bits, 10 bits, 7 bits = 27 bits.
  """
  @spec encode_numeric(String.t(), [0 | 1]) :: [0 | 1]
  def encode_numeric(input, bits \\ []) do
    chars = String.to_charlist(input)
    encode_numeric_chars(chars, bits)
  end

  defp encode_numeric_chars([], bits), do: bits

  defp encode_numeric_chars([a, b, c | rest], bits) do
    value = (a - ?0) * 100 + (b - ?0) * 10 + (c - ?0)
    encode_numeric_chars(rest, write_bits(bits, value, 10))
  end

  defp encode_numeric_chars([a, b], bits) do
    value = (a - ?0) * 10 + (b - ?0)
    write_bits(bits, value, 7)
  end

  defp encode_numeric_chars([a], bits) do
    write_bits(bits, a - ?0, 4)
  end

  # ---------------------------------------------------------------------------
  # Alphanumeric mode encoding
  # ---------------------------------------------------------------------------

  @doc """
  Encode an alphanumeric string into bit groups.

  Each character maps to an index 0–44 in the 45-char QR alphabet:
      '0'–'9' → 0–9
      'A'–'Z' → 10–35
      ' '     → 36
      '$'     → 37
      '%'     → 38
      '*'     → 39
      '+'     → 40
      '-'     → 41
      '.'     → 42
      '/'     → 43
      ':'     → 44

  Pairs of characters → 11 bits: (first_idx × 45 + second_idx).
  Trailing single character → 6 bits.
  """
  @spec encode_alphanumeric(String.t(), [0 | 1]) :: [0 | 1]
  def encode_alphanumeric(input, bits \\ []) do
    chars = String.to_charlist(input)
    encode_alphanum_chars(chars, bits)
  end

  defp encode_alphanum_chars([], bits), do: bits

  defp encode_alphanum_chars([a, b | rest], bits) do
    idx_a = alphanum_index(a)
    idx_b = alphanum_index(b)
    value = idx_a * 45 + idx_b
    encode_alphanum_chars(rest, write_bits(bits, value, 11))
  end

  defp encode_alphanum_chars([a], bits) do
    write_bits(bits, alphanum_index(a), 6)
  end

  # Return the 0-based index of a character in the QR alphanumeric set.
  # Raises if the character is not in the set (callers must have validated).
  defp alphanum_index(ch) do
    case :lists.member(ch, @alphanum_chars) do
      true -> :lists.search(fn x -> x == ch end, @alphanum_chars) |> elem(1) |> then(fn _ ->
        find_index(ch, @alphanum_chars, 0)
      end)
      false -> raise ArgumentError, "Character #{<<ch::utf8>>} not in QR alphanumeric set"
    end
  end

  defp find_index(ch, [ch | _], idx), do: idx
  defp find_index(ch, [_ | rest], idx), do: find_index(ch, rest, idx + 1)

  # ---------------------------------------------------------------------------
  # Byte mode encoding
  # ---------------------------------------------------------------------------

  @doc """
  Encode an arbitrary string as raw UTF-8 bytes.

  Each byte in the UTF-8 encoding of the input contributes 8 bits to the
  bit stream. This is the universal fallback — any input can be encoded in
  byte mode.

  Modern QR scanners default to UTF-8 interpretation. For maximum scanner
  compatibility, ECI mode with assignment 26 (= UTF-8) can be prepended,
  but v0.1.0 omits the ECI header.
  """
  @spec encode_byte(String.t(), [0 | 1]) :: [0 | 1]
  def encode_byte(input, bits \\ []) do
    input
    |> :binary.bin_to_list()
    |> Enum.reduce(bits, fn byte_val, acc ->
      write_bits(acc, byte_val, 8)
    end)
  end
end
