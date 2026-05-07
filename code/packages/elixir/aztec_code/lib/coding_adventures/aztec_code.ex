defmodule CodingAdventures.AztecCode do
  @moduledoc """
  Aztec Code encoder — ISO/IEC 24778:2008 compliant.

  Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
  published as a patent-free format. Unlike QR Code (three square finder
  patterns in the corners), Aztec Code places a single **bullseye finder
  pattern at the center** of the symbol. The scanner locates the center first,
  then reads outward in a clockwise spiral — no large quiet zone is needed.

  ## Where Aztec Code is used today

  - **IATA boarding passes** — the barcode on every airline boarding pass
  - **Eurostar and Amtrak rail tickets** — printed and on-screen tickets
  - **PostNL, Deutsche Post, La Poste** — European postal routing
  - **US driver's licences** (some states), **US military ID cards**

  ## Symbol variants

      Compact: 1–4 layers,  size = 11 + 4×layers  (15×15 to 27×27)
      Full:    1–32 layers, size = 15 + 4×layers  (19×19 to 143×143)

  ## Encoding pipeline (v0.1.0 — byte-mode only)

      input string / bytes
        → Binary-Shift codewords from Upper mode   (5-bit escape + length + raw bytes)
        → symbol size selection                    (smallest compact/full at 23% ECC)
        → pad to exact codeword count              (zero-fill; last codeword all-zero → 0xFF)
        → GF(256)/0x12D Reed-Solomon ECC           (same poly as Data Matrix, b=1 roots)
        → bit stuffing                             (insert complement after 4 identical bits)
        → GF(16) mode message                      (layers + cw-count + 5 or 6 RS nibbles)
        → grid initialization                      (bullseye + orientation + mode ring)
        → data spiral placement                    (clockwise from innermost layer outward)
        → ModuleGrid

  ## v0.1.0 simplifications

  1. **Byte-mode only** — all input encoded via Binary-Shift from Upper mode.
     Multi-mode (Digit/Upper/Lower/Mixed/Punct) optimization is v0.2.0.
  2. **8-bit codewords → GF(256) RS** (same polynomial as Data Matrix: 0x12D).
     GF(16) and GF(32) RS for 4-bit/5-bit codewords are v0.2.0.
  3. **Default ECC = 23%** — not yet user-configurable.
  4. **Auto-select compact vs full** — the encoder picks the smallest variant.

  ## IMPORTANT: Elixir reserved words

  Elixir reserves `after`, `rescue`, `catch`, `else`, `end`, `do`, `fn`,
  `when`, `cond`, `try`, `receive`, `true`, `false`, `nil`.  These CANNOT be
  used as variable names. Throughout this module we use:
  - `data_cws` instead of `data` (avoids shadowing `do`)
  - `ecc_cws` instead of `ecc`
  - `bit_val` instead of a bare `bit` which shadows keyword
  """

  import Bitwise

  @version "0.1.0"
  def version, do: @version

  # ============================================================================
  # GF(16) arithmetic — used exclusively for mode message Reed-Solomon
  # ============================================================================
  #
  # GF(16) is the finite field with 16 elements, built from the irreducible
  # polynomial:
  #
  #   p(x) = x^4 + x + 1   (binary 10011 = 0x13)
  #
  # Every non-zero element in GF(16) is a power of the primitive element α.
  # Since α is a root of p(x) we have:  α^4 = α + 1.
  #
  # The log table maps a field element (1..15) to its discrete logarithm
  # (0..14). The antilog (exponential) table maps a log value to the element.
  #
  # Field elements as powers of α:
  #   α^0  = 0b0001 = 1     α^5  = 0b0110 = 6     α^10 = 0b0111 = 7
  #   α^1  = 0b0010 = 2     α^6  = 0b1100 = 12    α^11 = 0b1110 = 14
  #   α^2  = 0b0100 = 4     α^7  = 0b1011 = 11    α^12 = 0b1111 = 15
  #   α^3  = 0b1000 = 8     α^8  = 0b0101 = 5     α^13 = 0b1101 = 13
  #   α^4  = 0b0011 = 3     α^9  = 0b1010 = 10    α^14 = 0b1001 = 9
  #
  # α^15 = α^0 = 1  (the period is 15, confirming α is primitive)
  #
  # LOG16[element] = discrete log (0..14); LOG16[0] = -1 (undefined)
  # ALOG16[log]    = element corresponding to that log value

  # Discrete logarithm table: LOG16[i] = k means α^k = i
  # Index  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15
  @gf16_log {-1,  0,  1,  4,  2,  8,  5, 10,  3, 14,  9,  7,  6, 13, 11, 12}

  # Antilogarithm table: ALOG16[k] = α^k  (index 0..14; index 15 = index 0 = 1)
  # Index  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15
  @gf16_alog {1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1}

  @doc """
  Multiply two GF(16) field elements using the log/antilog trick.

  a × b = ALOG16[(LOG16[a] + LOG16[b]) mod 15]

  Returns 0 if either operand is 0 (0 is the absorbing element).

  ## Examples

      iex> CodingAdventures.AztecCode.gf16_mul(2, 4)
      8

      iex> CodingAdventures.AztecCode.gf16_mul(3, 3)
      5

      iex> CodingAdventures.AztecCode.gf16_mul(0, 7)
      0
  """
  @compile {:inline, gf16_mul: 2}
  def gf16_mul(a, b) when a == 0 or b == 0, do: 0

  def gf16_mul(a, b) do
    log_a = elem(@gf16_log, a)
    log_b = elem(@gf16_log, b)
    elem(@gf16_alog, rem(log_a + log_b, 15))
  end

  @doc """
  Build the GF(16) RS generator polynomial with roots α^1 through α^n.

  Returns [g0, g1, ..., gn] where the list has n+1 coefficients,
  with gn = 1 (monic polynomial, lowest degree first).

  This is the same construction as for GF(256), but over the 15-element
  multiplicative group of GF(16).
  """
  def build_gf16_generator(n_ecc) do
    Enum.reduce(1..n_ecc//1, [1], fn k, g ->
      alpha_k = elem(@gf16_alog, rem(k, 15))
      len = length(g)
      next = List.duplicate(0, len + 1)

      {next, _} =
        Enum.reduce(0..(len - 1)//1, {next, g}, fn j, {acc, [gj | rest_g]} ->
          acc = List.update_at(acc, j + 1, &bxor(&1, gj))
          acc = List.update_at(acc, j, &bxor(&1, gf16_mul(gj, alpha_k)))
          {acc, rest_g}
        end)

      next
    end)
  end

  @doc """
  Compute n GF(16) RS check nibbles for the given data nibbles.

  Uses LFSR polynomial division (systematic encoding):
    R(x) = D(x) × x^n  mod  G(x)

  ## Examples

      iex> ecc = CodingAdventures.AztecCode.gf16_rs_encode([7, 2], 5)
      iex> length(ecc)
      5
  """
  def gf16_rs_encode(data_nibbles, n_ecc) do
    gen = build_gf16_generator(n_ecc)
    # gen[0] is constant term, gen[n_ecc] is leading (monic) coefficient.
    # The LFSR feedback uses gen[0..n_ecc-1] (all but the leading 1).
    gen_tail = Enum.drop(gen, 1)

    Enum.reduce(data_nibbles, List.duplicate(0, n_ecc), fn nibble, rem ->
      feedback = bxor(nibble, hd(rem))
      shifted = tl(rem) ++ [0]

      if feedback == 0 do
        shifted
      else
        Enum.zip(shifted, gen_tail)
        |> Enum.map(fn {r, gc} -> bxor(r, gf16_mul(gc, feedback)) end)
      end
    end)
  end

  # ============================================================================
  # GF(256)/0x12D arithmetic — used for 8-bit data codeword RS
  # ============================================================================
  #
  # Aztec Code uses GF(256) with the irreducible polynomial:
  #
  #   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D  =  301 decimal
  #
  # This is the SAME polynomial as Data Matrix ECC200, and DIFFERENT from
  # QR Code (0x11D). Both are valid GF(256) fields, but the primitive element
  # α has different discrete-log values in each.
  #
  # The generator polynomial convention is b=1: roots are α^1, α^2, ..., α^n.
  # This matches the Data Matrix convention (also called the MA02 convention
  # in this repo).
  #
  # Exp/log table construction:
  #   α^0 = 1, α^1 = 2, α^2 = 4, ...
  #   When 2×a overflows 8 bits: a = (2×a) XOR 0x12D

  @gf256_tables (
    import Bitwise

    {exp_rev, log_pairs, _} =
      Enum.reduce(0..254, {[], [], 1}, fn i, {exp_acc, log_acc, val} ->
        new_exp = [val | exp_acc]
        new_log = [{val, i} | log_acc]
        next = val <<< 1
        next = if (next &&& 0x100) != 0, do: bxor(next, 0x12D), else: next
        next = next &&& 0xFF
        {new_exp, new_log, next}
      end)

    exp_list = Enum.reverse(exp_rev) ++ [1]
    log_map = Map.new(log_pairs)
    %{exp: List.to_tuple(exp_list), log: log_map}
  )

  @doc """
  Multiply two GF(256)/0x12D field elements using log/antilog tables.

  a × b = EXP[(LOG[a] + LOG[b]) mod 255]

  Returns 0 if either operand is 0.

  ## Examples

      iex> CodingAdventures.AztecCode.gf256_mul(2, 2)
      4

      iex> CodingAdventures.AztecCode.gf256_mul(0x80, 2)
      0x2D

      iex> CodingAdventures.AztecCode.gf256_mul(0, 255)
      0
  """
  @compile {:inline, gf256_mul: 2}
  def gf256_mul(a, b) when a == 0 or b == 0, do: 0

  def gf256_mul(a, b) do
    log_a = Map.fetch!(@gf256_tables.log, a)
    log_b = Map.fetch!(@gf256_tables.log, b)
    idx = rem(log_a + log_b, 255)
    elem(@gf256_tables.exp, idx)
  end

  @doc "Return the GF(256)/0x12D exponent table as a tuple (index 0..255)."
  def gf256_exp_table, do: @gf256_tables.exp

  @doc "Return the GF(256)/0x12D log table as a map (value => log index)."
  def gf256_log_table, do: @gf256_tables.log

  @doc """
  Build the GF(256)/0x12D RS generator polynomial with roots α^1 through α^n.

  Returns a list of n+1 GF(256) bytes, lowest degree first, with
  the leading (degree-n) coefficient equal to 1 (monic polynomial).

  ## Examples

      iex> gen = CodingAdventures.AztecCode.build_gf256_generator(5)
      iex> length(gen)
      6
      iex> List.last(gen)
      1
  """
  def build_gf256_generator(n_ecc) do
    Enum.reduce(1..n_ecc//1, [1], fn k, g ->
      alpha_k = elem(@gf256_tables.exp, k)
      len = length(g)
      next = List.duplicate(0, len + 1)

      {next, _} =
        Enum.reduce(0..(len - 1)//1, {next, g}, fn j, {acc, [gj | rest_g]} ->
          acc = List.update_at(acc, j + 1, &bxor(&1, gj))
          acc = List.update_at(acc, j, &bxor(&1, gf256_mul(gj, alpha_k)))
          {acc, rest_g}
        end)

      next
    end)
  end

  @doc """
  Compute n_ecc GF(256)/0x12D RS check bytes for the given data bytes.

  Uses LFSR polynomial division. The generator polynomial is built with
  roots α^1 through α^n_ecc (b=1 convention, same as Data Matrix).

  ## Examples

      iex> ecc = CodingAdventures.AztecCode.gf256_rs_encode([0x48, 0x65, 0x6C], 4)
      iex> length(ecc)
      4
  """
  def gf256_rs_encode(data_bytes, n_ecc) do
    gen = build_gf256_generator(n_ecc)
    gen_tail = Enum.drop(gen, 1)

    Enum.reduce(data_bytes, List.duplicate(0, n_ecc), fn byte, rem ->
      feedback = bxor(byte, hd(rem))
      shifted = tl(rem) ++ [0]

      if feedback == 0 do
        shifted
      else
        Enum.zip(shifted, gen_tail)
        |> Enum.map(fn {r, gc} -> bxor(r, gf256_mul(gc, feedback)) end)
      end
    end)
  end

  # ============================================================================
  # Capacity tables
  # ============================================================================
  #
  # Derived from ISO/IEC 24778:2008 Table 1 and confirmed against the
  # TypeScript reference implementation.
  #
  # `total_bits` — total data + ECC bit positions in the data layers.
  # `max_bytes8` — maximum number of 8-bit codewords (data + ECC).
  #
  # A 23% ECC ratio means:
  #   ecc_cws  = ceil(0.23 × max_bytes8)
  #   data_cws = max_bytes8 − ecc_cws
  #
  # For input of `n` bytes, the stuffed bit count is approximately 1.2 × n × 8.
  # The encoder picks the smallest variant where ceil(1.2 × input_bytes) ≤ data_cws.

  # {total_bits, max_bytes8} for compact layers 1..4
  # Index 0 is a sentinel (unused); layers are 1-indexed.
  @compact_capacity {
    {0, 0},         # layer 0 — unused sentinel
    {72, 9},        # layer 1 — 15×15 symbol
    {200, 25},      # layer 2 — 19×19 symbol
    {392, 49},      # layer 3 — 23×23 symbol
    {648, 81}       # layer 4 — 27×27 symbol
  }

  # {total_bits, max_bytes8} for full layers 1..32
  @full_capacity {
    {0, 0},           # layer 0 — unused sentinel
    {88, 11},         # layer 1  — 19×19
    {216, 27},        # layer 2  — 23×23
    {360, 45},        # layer 3  — 27×27
    {520, 65},        # layer 4  — 31×31
    {696, 87},        # layer 5  — 35×35
    {888, 111},       # layer 6  — 39×39
    {1096, 137},      # layer 7  — 43×43
    {1320, 165},      # layer 8  — 47×47
    {1560, 195},      # layer 9  — 51×51
    {1816, 227},      # layer 10 — 55×55
    {2088, 261},      # layer 11 — 59×59
    {2376, 297},      # layer 12 — 63×63
    {2680, 335},      # layer 13 — 67×67
    {3000, 375},      # layer 14 — 71×71
    {3336, 417},      # layer 15 — 75×75
    {3688, 461},      # layer 16 — 79×79
    {4056, 507},      # layer 17 — 83×83
    {4440, 555},      # layer 18 — 87×87
    {4840, 605},      # layer 19 — 91×91
    {5256, 657},      # layer 20 — 95×95
    {5688, 711},      # layer 21 — 99×99
    {6136, 767},      # layer 22 — 103×103
    {6600, 825},      # layer 23 — 107×107
    {7080, 885},      # layer 24 — 111×111
    {7576, 947},      # layer 25 — 115×115
    {8088, 1011},     # layer 26 — 119×119
    {8616, 1077},     # layer 27 — 123×123
    {9160, 1145},     # layer 28 — 127×127
    {9720, 1215},     # layer 29 — 131×131
    {10296, 1287},    # layer 30 — 135×135
    {10888, 1361},    # layer 31 — 139×139
    {11496, 1437}     # layer 32 — 143×143
  }

  @doc "Return the compact capacity table as a tuple of {total_bits, max_bytes8} pairs."
  def compact_capacity, do: @compact_capacity

  @doc "Return the full capacity table as a tuple of {total_bits, max_bytes8} pairs."
  def full_capacity, do: @full_capacity

  # ============================================================================
  # Data encoding — Binary-Shift from Upper mode (v0.1.0 byte-mode path)
  # ============================================================================
  #
  # All input is wrapped in a single Binary-Shift block from Upper mode:
  #
  #   1. Emit 5 bits = 0b11111 = 31 (the Binary-Shift escape codeword in Upper mode)
  #   2. If len ≤ 31: emit len as 5 bits
  #      If len > 31:  emit 0b00000 (5 bits), then emit len as 11 bits
  #   3. Emit each byte as 8 bits, MSB first
  #
  # This is always a valid encoding. It is not maximally compact for text that
  # contains long runs of uppercase letters (which would benefit from Upper mode
  # codewords directly), but it handles all input uniformly and is the mandated
  # v0.1.0 approach.
  #
  # Example for "Hi" (2 bytes, 0x48 0x69):
  #   Binary-Shift:  11111         = 31
  #   Length 2:      00010         (5 bits, since 2 ≤ 31)
  #   'H' = 0x48:    01001000      (8 bits)
  #   'i' = 0x69:    01101001      (8 bits)
  #   Total:  5 + 5 + 8 + 8 = 26 bits

  @doc """
  Encode the input bytes as a flat 0/1 bit list using the Binary-Shift escape.

  Returns a list of integers, each 0 or 1, MSB first.

  ## Examples

      iex> CodingAdventures.AztecCode.encode_bytes_as_bits(<<65>>)
      [1, 1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 1]

      iex> length(CodingAdventures.AztecCode.encode_bytes_as_bits(<<1, 2, 3>>))
      5 + 5 + 24
  """
  def encode_bytes_as_bits(input) when is_binary(input) do
    encode_bytes_as_bits(:binary.bin_to_list(input))
  end

  def encode_bytes_as_bits(bytes) when is_list(bytes) do
    len = length(bytes)

    # Binary-Shift escape: codeword 31 in Upper mode, 5 bits
    escape_bits = int_to_bits(31, 5)

    # Length prefix: 5 bits if len ≤ 31, else 5×0 + 11-bit length
    length_bits =
      if len <= 31 do
        int_to_bits(len, 5)
      else
        int_to_bits(0, 5) ++ int_to_bits(len, 11)
      end

    # Raw bytes, MSB first
    byte_bits = Enum.flat_map(bytes, fn b -> int_to_bits(b, 8) end)

    escape_bits ++ length_bits ++ byte_bits
  end

  # Convert an integer `value` to a list of `n` bits, MSB first.
  @compile {:inline, int_to_bits: 2}
  defp int_to_bits(value, n) do
    Enum.map((n - 1)..0//-1, fn i -> band(bsr(value, i), 1) end)
  end

  # ============================================================================
  # Symbol size selection
  # ============================================================================
  #
  # The encoder picks the smallest symbol where:
  #   ceil(1.2 × input_bytes) ≤ data_cw_count
  #
  # The 1.2 factor is a conservative estimate for the bit-stuffing overhead
  # (worst case is 25%, but 20% covers all practical inputs).
  #
  # Selection order: compact layers 1, 2, 3, 4 → full layers 1..32.
  # If nothing fits, return {:error, :input_too_long}.

  defmodule SymbolSpec do
    @moduledoc "Describes a selected Aztec Code symbol configuration."
    @type t :: %__MODULE__{
            compact: boolean(),
            layers: pos_integer(),
            data_cw_count: pos_integer(),
            ecc_cw_count: pos_integer(),
            total_bits: pos_integer()
          }
    defstruct [:compact, :layers, :data_cw_count, :ecc_cw_count, :total_bits]
  end

  @doc """
  Select the smallest Aztec Code symbol that can hold `data_bit_count` bits
  at the given ECC percentage.

  Returns `{:ok, %SymbolSpec{}}` or `{:error, :input_too_long}`.

  ## Examples

      iex> {:ok, spec} = CodingAdventures.AztecCode.select_symbol(10, 23)
      iex> spec.compact
      true
      iex> spec.layers
      1
  """
  def select_symbol(data_bit_count, min_ecc_pct \\ 23) do
    # Conservative stuffing estimate: multiply by 1.2 to account for worst-case
    # bit stuffing overhead before comparing against symbol capacity.
    stuffed_bit_count = ceil(data_bit_count * 1.2)

    result =
      try_compact_layers(stuffed_bit_count, min_ecc_pct) ||
        try_full_layers(stuffed_bit_count, min_ecc_pct)

    case result do
      nil -> {:error, :input_too_long}
      spec -> {:ok, spec}
    end
  end

  defp try_compact_layers(stuffed_bits, min_ecc_pct) do
    Enum.find_value(1..4//1, fn layers ->
      {_total_bits, max_bytes8} = elem(@compact_capacity, layers)
      ecc_cw_count = ceil(min_ecc_pct / 100 * max_bytes8)
      data_cw_count = max_bytes8 - ecc_cw_count

      if data_cw_count > 0 and ceil(stuffed_bits / 8) <= data_cw_count do
        {total_bits, _} = elem(@compact_capacity, layers)

        %SymbolSpec{
          compact: true,
          layers: layers,
          data_cw_count: data_cw_count,
          ecc_cw_count: ecc_cw_count,
          total_bits: total_bits
        }
      end
    end)
  end

  defp try_full_layers(stuffed_bits, min_ecc_pct) do
    Enum.find_value(1..32//1, fn layers ->
      {_total_bits, max_bytes8} = elem(@full_capacity, layers)
      ecc_cw_count = ceil(min_ecc_pct / 100 * max_bytes8)
      data_cw_count = max_bytes8 - ecc_cw_count

      if data_cw_count > 0 and ceil(stuffed_bits / 8) <= data_cw_count do
        {total_bits, _} = elem(@full_capacity, layers)

        %SymbolSpec{
          compact: false,
          layers: layers,
          data_cw_count: data_cw_count,
          ecc_cw_count: ecc_cw_count,
          total_bits: total_bits
        }
      end
    end)
  end

  # ============================================================================
  # Padding
  # ============================================================================
  #
  # After encoding the data, the bit stream is padded to exactly
  # `data_cw_count × 8` bits:
  #   1. Zero-fill until the last partial byte is complete.
  #   2. Append zero bytes until target length is reached.
  #   3. If the very last byte is 0x00 (all zeros), replace it with 0xFF
  #      (the "all-zero codeword avoidance" rule — GF arithmetic with a zero
  #       leading codeword can cause RS edge cases).
  #
  # The padded bit stream is then split into data_cw_count byte values.

  @doc """
  Pad a bit list to exactly `target_bytes × 8` bits, applying the
  all-zero-codeword avoidance rule on the last byte.

  Returns a list of integers 0..255 (the byte-aligned codewords).

  ## Examples

      iex> CodingAdventures.AztecCode.pad_to_bytes([1, 0, 1], 2)
      [160, 0]
  """
  def pad_to_bytes(bits, target_bytes) do
    # Step 1: complete the current partial byte with zeros
    padded = bits ++ List.duplicate(0, rem(8 - rem(length(bits), 8), 8))
    # Step 2: extend to target with zero bytes
    padded = padded ++ List.duplicate(0, max(0, target_bytes * 8 - length(padded)))
    # Step 3: take exactly target_bytes * 8 bits
    padded = Enum.take(padded, target_bytes * 8)

    # Pack into bytes
    bytes =
      Enum.chunk_every(padded, 8)
      |> Enum.map(fn byte_bits ->
        Enum.reduce(byte_bits, 0, fn b, acc -> (acc <<< 1) ||| b end)
      end)

    # All-zero codeword avoidance on the last byte
    if length(bytes) > 0 and List.last(bytes) == 0 do
      List.replace_at(bytes, -1, 0xFF)
    else
      bytes
    end
  end

  # ============================================================================
  # Bit stuffing
  # ============================================================================
  #
  # Aztec Code applies a bit-stuffing rule to the final data + ECC bit stream
  # before placing bits into the symbol's data layers. The rule prevents long
  # runs of identical bits that could interfere with the scanner's reference
  # grid detection.
  #
  # Rule: after 4 consecutive identical bits (all 0 or all 1), insert one bit
  # of the OPPOSITE value.
  #
  # Example:
  #   Input:  1 1 1 1 0 0 0 0 1 0
  #   After 4×1: insert 0  →  [1, 1, 1, 1, 0]
  #   After 4×0: insert 1  →  [..., 0, 0, 0, 0, 1, 1, 0]
  #   Full:   1 1 1 1 0 0 0 0 0 1 1 0
  #
  # Note: the run counter RESETS after the stuffed bit. The stuffed bit starts
  # a new run of length 1.
  #
  # Bit stuffing does NOT apply to the bullseye, orientation marks, mode
  # message, or reference grid — only the data + ECC layer bits.

  @doc """
  Apply Aztec bit stuffing to a 0/1 bit list.

  Inserts a complement bit after every run of 4 identical bits.

  ## Examples

      iex> CodingAdventures.AztecCode.stuff_bits([1, 1, 1, 1, 0])
      [1, 1, 1, 1, 0, 0]

      iex> CodingAdventures.AztecCode.stuff_bits([0, 0, 0, 0, 0])
      [0, 0, 0, 0, 1, 0]

      iex> CodingAdventures.AztecCode.stuff_bits([1, 0, 1, 0])
      [1, 0, 1, 0]
  """
  def stuff_bits(bits) do
    {stuffed, _, _} =
      Enum.reduce(bits, {[], -1, 0}, fn bit_val, {acc, run_val, run_len} ->
        {run_val, run_len} =
          if bit_val == run_val do
            {run_val, run_len + 1}
          else
            {bit_val, 1}
          end

        acc = [bit_val | acc]

        if run_len == 4 do
          stuff = 1 - bit_val
          {[stuff | acc], stuff, 1}
        else
          {acc, run_val, run_len}
        end
      end)

    Enum.reverse(stuffed)
  end

  # ============================================================================
  # Mode message encoding
  # ============================================================================
  #
  # The mode message is Aztec Code's equivalent of QR Code's format information.
  # It encodes the symbol configuration (compact/full, layer count, codeword
  # count) and is protected by GF(16) Reed-Solomon.
  #
  # ## Compact mode message (28 bits = 7 nibbles)
  #
  #   m = ((layers - 1) << 6) | (data_cw_count - 1)  — an 8-bit value
  #   Split into 2 data nibbles (LSB first):
  #     nibble[0] = m[3:0]
  #     nibble[1] = m[7:4]
  #   Compute 5 ECC nibbles via GF(16) RS with roots α^1..α^5.
  #   Total: 7 nibbles × 4 bits = 28 bits.
  #
  # ## Full mode message (40 bits = 10 nibbles)
  #
  #   m = ((layers - 1) << 11) | (data_cw_count - 1)  — a 16-bit value
  #   Split into 4 data nibbles (LSB first):
  #     nibble[0] = m[3:0],  nibble[1] = m[7:4]
  #     nibble[2] = m[11:8], nibble[3] = m[15:12]
  #   Compute 6 ECC nibbles via GF(16) RS with roots α^1..α^6.
  #   Total: 10 nibbles × 4 bits = 40 bits.
  #
  # Each nibble is emitted as 4 bits, MSB of the nibble first.
  # (So the bit stream is: nibble[0] MSB..LSB, nibble[1] MSB..LSB, ...)

  @doc """
  Encode the mode message as a flat bit list (28 bits compact, 40 bits full).

  ## Examples

      iex> bits = CodingAdventures.AztecCode.encode_mode_message(true, 1, 7)
      iex> length(bits)
      28

      iex> bits = CodingAdventures.AztecCode.encode_mode_message(false, 2, 12)
      iex> length(bits)
      40
  """
  def encode_mode_message(compact, layers, data_cw_count) do
    {data_nibbles, n_ecc} =
      if compact do
        m = bsl(layers - 1, 6) ||| (data_cw_count - 1)
        nibbles = [band(m, 0xF), band(bsr(m, 4), 0xF)]
        {nibbles, 5}
      else
        m = bsl(layers - 1, 11) ||| (data_cw_count - 1)
        nibbles = [
          band(m, 0xF),
          band(bsr(m, 4), 0xF),
          band(bsr(m, 8), 0xF),
          band(bsr(m, 12), 0xF)
        ]
        {nibbles, 6}
      end

    ecc_nibbles = gf16_rs_encode(data_nibbles, n_ecc)
    all_nibbles = data_nibbles ++ ecc_nibbles

    # Emit each nibble MSB first (4 bits per nibble)
    Enum.flat_map(all_nibbles, fn nibble ->
      int_to_bits(nibble, 4)
    end)
  end

  # ============================================================================
  # Grid helpers (tuple-of-tuples for O(1) random access)
  # ============================================================================
  #
  # The module grid is represented as a tuple of rows, each row a tuple of
  # booleans. True = dark module, false = light module.
  #
  # A separate "reserved" grid tracks which modules have already been assigned
  # a structural role (bullseye, orientation marks, mode message, reference
  # grid) and must not be overwritten by the data placement algorithm.

  @compile {:inline, grid_get: 3, grid_set: 4}

  defp grid_get(grid, row, col) do
    elem(elem(grid, row), col)
  end

  defp grid_set(grid, row, col, value) do
    old_row = elem(grid, row)
    put_elem(grid, row, put_elem(old_row, col, value))
  end

  defp make_grid(size, init_val) do
    row = List.to_tuple(List.duplicate(init_val, size))
    List.to_tuple(List.duplicate(row, size))
  end

  # ============================================================================
  # Symbol structure initialization
  # ============================================================================
  #
  # The initialization steps, executed in order:
  #
  #   1. Reference grid lines (full symbols only) — placed BEFORE bullseye
  #      so bullseye overwrites any reference grid modules at the center.
  #   2. Bullseye finder pattern — concentric rings from center.
  #   3. Orientation marks — four dark corner modules of the mode message ring.
  #   4. Mode message bits — non-corner perimeter of the mode message ring.
  #
  # All placed modules are marked "reserved" so the data spiral skips them.
  #
  # ## Bullseye finder pattern
  #
  # The color at Chebyshev distance d from the center module:
  #   d = 0 or 1  → DARK  (solid 3×3 inner core)
  #   d even (d ≥ 2) → LIGHT
  #   d odd  (d ≥ 3) → DARK
  #
  # Compact bullseye radius = 5 (11×11 module square).
  # Full    bullseye radius = 7 (15×15 module square).
  #
  # The outermost ring of the bullseye is always DARK because the radius is
  # always odd (5 or 7). This means the transition from the bullseye (DARK) to
  # the orientation mark band is well defined.
  #
  # ## Reference grid (full symbols only)
  #
  # Horizontal and vertical lines at multiples of 16 from the center:
  #   rows / cols: cy, cy ± 16, cy ± 32, ...
  # Module color at (row, col) on a reference line:
  #   intersection of two reference lines → DARK
  #   horizontal reference line only      → DARK if (cx - col) mod 2 == 0
  #   vertical reference line only        → DARK if (cy - row) mod 2 == 0

  @doc """
  Draw the bullseye finder pattern and mark all modules as reserved.

  Returns `{modules, reserved}` with the bullseye filled in.
  """
  def draw_bullseye(modules, reserved, cx, cy, compact) do
    br = if compact, do: 5, else: 7

    Enum.reduce((cy - br)..(cy + br)//1, {modules, reserved}, fn row, {m, res} ->
      Enum.reduce((cx - br)..(cx + br)//1, {m, res}, fn col, {mm, rr} ->
        d = max(abs(col - cx), abs(row - cy))
        # d=0 or d=1: DARK (inner 3×3 core both dark)
        # d even (≥2): LIGHT  d odd (≥3): DARK
        dark = if d <= 1, do: true, else: rem(d, 2) == 1
        mm = grid_set(mm, row, col, dark)
        rr = grid_set(rr, row, col, true)
        {mm, rr}
      end)
    end)
  end

  @doc """
  Draw the reference grid for full Aztec symbols.

  Horizontal lines at rows (cy mod 16 == 0 in symbol coordinates, i.e. rows
  where (cy - row) mod 16 == 0), vertical lines likewise.

  Module color alternates dark/light from the center.
  """
  def draw_reference_grid(modules, reserved, cx, cy, size) do
    Enum.reduce(0..(size - 1)//1, {modules, reserved}, fn row, {m, res} ->
      Enum.reduce(0..(size - 1)//1, {m, res}, fn col, {mm, rr} ->
        on_h = rem(cy - row, 16) == 0
        on_v = rem(cx - col, 16) == 0

        if on_h or on_v do
          dark =
            cond do
              on_h and on_v -> true
              on_h -> rem(cx - col, 2) == 0
              true -> rem(cy - row, 2) == 0
            end

          mm = grid_set(mm, row, col, dark)
          rr = grid_set(rr, row, col, true)
          {mm, rr}
        else
          {mm, rr}
        end
      end)
    end)
  end

  @doc """
  Place orientation marks and mode message bits in the mode message ring.

  The mode message ring is the perimeter at Chebyshev distance
  `bullseye_radius + 1` from the center. The four corner modules of this ring
  are orientation marks (always DARK). The remaining non-corner modules carry
  the mode message bits in clockwise order from the top-left corner.

  Returns `{modules, reserved, remaining_positions}` where
  `remaining_positions` is the list of `{col, row}` positions after the mode
  message bits — these will be filled by the start of the data spiral.
  """
  def draw_orientation_and_mode_message(modules, reserved, cx, cy, compact, mode_msg_bits) do
    r = (if compact, do: 5, else: 7) + 1  # radius of mode message ring

    # Enumerate non-corner perimeter positions clockwise from (cx-r+1, cy-r):
    #   top edge: left to right (exclude corners)
    #   right edge: top to bottom (exclude corners)
    #   bottom edge: right to left (exclude corners)
    #   left edge: bottom to top (exclude corners)
    non_corner =
      # Top edge: col goes cx-r+1 .. cx+r-1, row = cy-r
      Enum.map((cx - r + 1)..(cx + r - 1)//1, fn col -> {col, cy - r} end) ++
      # Right edge: row goes cy-r+1 .. cy+r-1, col = cx+r
      Enum.map((cy - r + 1)..(cy + r - 1)//1, fn row -> {cx + r, row} end) ++
      # Bottom edge: col goes cx+r-1 .. cx-r+1 (reversed), row = cy+r
      Enum.map((cx + r - 1)..(cx - r + 1)//-1, fn col -> {col, cy + r} end) ++
      # Left edge: row goes cy+r-1 .. cy-r+1 (reversed), col = cx-r
      Enum.map((cy + r - 1)..(cy - r + 1)//-1, fn row -> {cx - r, row} end)

    # The four corner modules are orientation marks (always DARK)
    corners = [
      {cx - r, cy - r},
      {cx + r, cy - r},
      {cx + r, cy + r},
      {cx - r, cy + r}
    ]

    {modules, reserved} =
      Enum.reduce(corners, {modules, reserved}, fn {col, row}, {mm, rr} ->
        {grid_set(mm, row, col, true), grid_set(rr, row, col, true)}
      end)

    # Place mode message bits in the first positions; remainder goes to data
    n_msg = length(mode_msg_bits)

    {modules, reserved} =
      Enum.zip(Enum.take(non_corner, n_msg), mode_msg_bits)
      |> Enum.reduce({modules, reserved}, fn {{col, row}, bit_val}, {mm, rr} ->
        {grid_set(mm, row, col, bit_val == 1), grid_set(rr, row, col, true)}
      end)

    remaining = Enum.drop(non_corner, n_msg)
    {modules, reserved, remaining}
  end

  # ============================================================================
  # Data layer spiral placement
  # ============================================================================
  #
  # After structural initialization, the stuffed bit stream is placed into the
  # symbol's data layers. Placement proceeds from the innermost data layer
  # outward, one layer at a time. Within each layer, bits are placed in a
  # clockwise spiral in pairs (outer module first, then inner).
  #
  # ## Layer coordinate system
  #
  # For compact, the bullseye radius is 5 and the mode message ring is at
  # radius 6. The first data layer starts at inner radius d_inner = 7.
  # For full, bullseye radius is 7 and mode ring is at radius 8; d_inner = 9.
  #
  # For data layer L (1-indexed), d_inner = br + 1 + 2*L (where br is the
  # bullseye radius). Equivalently: d_inner = (br + 2) + 2*(L-1).
  #
  # ## One layer's clockwise spiral
  #
  # Each layer band is 2 modules wide (inner radius d_i, outer radius d_o = d_i + 1).
  # Within the band, pairs of bits are placed at (outer, inner) for each
  # edge position in clockwise order:
  #
  #   Top edge (L→R):    pairs at cols (cx-d_i+1)..(cx+d_i),   rows (cy-d_o, cy-d_i)
  #   Right edge (T→B):  pairs at rows (cy-d_i+1)..(cy+d_i),   cols (cx+d_o, cx+d_i)
  #   Bottom edge (R→L): pairs at cols (cx+d_i)..(cx-d_i+1),   rows (cy+d_o, cy+d_i)
  #   Left edge (B→T):   pairs at rows (cy+d_i)..(cy-d_i+1),   cols (cx-d_o, cx-d_i)
  #
  # ## Skipping reserved modules
  #
  # The reference grid modules (full symbols) are reserved. The data placement
  # algorithm skips any module that is reserved; the bit index only advances
  # when a bit is actually placed.

  @doc """
  Place all stuffed data bits into the module grid using the clockwise spiral.

  `mode_ring_remaining` is the list of `{col, row}` positions at the end of
  the mode message ring that were not used for mode message bits — the data
  spiral starts here before spiraling into the outer data layers.
  """
  def place_data_bits(modules, reserved, bits, cx, cy, compact, layers, mode_ring_remaining) do
    bits_tuple = List.to_tuple(bits)
    total_bits = length(bits)
    size = tuple_size(modules)

    place_bit = fn mm, col, row, bit_idx ->
      if row < 0 or row >= size or col < 0 or col >= size do
        {mm, bit_idx}
      else
        if grid_get(reserved, row, col) do
          {mm, bit_idx}
        else
          bit_val =
            if bit_idx < total_bits do
              elem(bits_tuple, bit_idx) == 1
            else
              false
            end

          {grid_set(mm, row, col, bit_val), bit_idx + 1}
        end
      end
    end

    # Fill the remaining positions in the mode ring first
    {modules, bit_idx} =
      Enum.reduce(mode_ring_remaining, {modules, 0}, fn {col, row}, {mm, idx} ->
        bit_val =
          if idx < total_bits do
            elem(bits_tuple, idx) == 1
          else
            false
          end

        {grid_set(mm, row, col, bit_val), idx + 1}
      end)

    # Spiral through data layers (innermost first)
    br = if compact, do: 5, else: 7
    d_start = br + 2  # mode ring at br+1, first data ring at br+2

    Enum.reduce(0..(layers - 1)//1, {modules, bit_idx}, fn layer_idx, {mm, idx} ->
      d_i = d_start + 2 * layer_idx  # inner radius
      d_o = d_i + 1                  # outer radius

      # Top edge: left to right
      {mm, idx} =
        Enum.reduce((cx - d_i + 1)..(cx + d_i)//1, {mm, idx}, fn col, {m2, i2} ->
          {m2, i2} = place_bit.(m2, col, cy - d_o, i2)
          place_bit.(m2, col, cy - d_i, i2)
        end)

      # Right edge: top to bottom
      {mm, idx} =
        Enum.reduce((cy - d_i + 1)..(cy + d_i)//1, {mm, idx}, fn row, {m2, i2} ->
          {m2, i2} = place_bit.(m2, cx + d_o, row, i2)
          place_bit.(m2, cx + d_i, row, i2)
        end)

      # Bottom edge: right to left
      {mm, idx} =
        Enum.reduce((cx + d_i)..(cx - d_i + 1)//-1, {mm, idx}, fn col, {m2, i2} ->
          {m2, i2} = place_bit.(m2, col, cy + d_o, i2)
          place_bit.(m2, col, cy + d_i, i2)
        end)

      # Left edge: bottom to top
      Enum.reduce((cy + d_i)..(cy - d_i + 1)//-1, {mm, idx}, fn row, {m2, i2} ->
        {m2, i2} = place_bit.(m2, cx - d_o, row, i2)
        place_bit.(m2, cx - d_i, row, i2)
      end)
    end)
    |> elem(0)
  end

  # ============================================================================
  # Public API
  # ============================================================================

  @typedoc "Options for `encode/2`."
  @type aztec_options :: %{
    optional(:min_ecc_percent) => 10..90
  }

  @doc """
  Encode a string or binary as an Aztec Code symbol.

  Returns `{:ok, %{rows: r, cols: c, modules: [[boolean()]]}}` where
  `modules` is a list of rows, each a list of booleans
  (`true` = dark module, `false` = light module).

  Returns `{:error, :input_too_long}` if the encoded data exceeds the maximum
  32-layer full symbol capacity.

  ## Options

  - `:min_ecc_percent` — minimum error-correction percentage (default: 23,
    range: 10..90). Higher values produce more robust (but larger) symbols.

  ## Encoding pipeline

  1. Encode input as Binary-Shift from Upper mode (byte mode only, v0.1.0).
  2. Select the smallest symbol at the requested ECC level.
  3. Pad data to the required codeword count.
  4. Compute GF(256)/0x12D Reed-Solomon ECC.
  5. Apply bit stuffing (insert complement after 4 identical bits).
  6. Compute GF(16) mode message (layer/codeword-count + RS protection).
  7. Initialize grid: bullseye → orientation marks → mode message → ref grid.
  8. Place stuffed data+ECC bits in the clockwise layer spiral.

  ## Examples

      iex> {:ok, grid} = CodingAdventures.AztecCode.encode("A")
      iex> grid.rows
      15
      iex> grid.cols
      15

      iex> {:ok, grid} = CodingAdventures.AztecCode.encode("Hello, World!")
      iex> grid.rows >= 15
      true
  """
  def encode(input, opts \\ %{}) do
    min_ecc_pct = Map.get(opts, :min_ecc_percent, 23)

    input_bytes =
      cond do
        is_binary(input) -> :binary.bin_to_list(input)
        is_list(input) -> input
        true -> [input]
      end

    # Step 1: Encode input as bits
    data_bits = encode_bytes_as_bits(input_bytes)

    # Step 2: Select symbol
    with {:ok, spec} <- select_symbol(length(data_bits), min_ecc_pct) do
      %SymbolSpec{
        compact: compact,
        layers: layers,
        data_cw_count: data_cw_count,
        ecc_cw_count: ecc_cw_count
      } = spec

      # Step 3: Pad to data_cw_count bytes
      data_bytes = pad_to_bytes(data_bits, data_cw_count)

      # Step 4: Compute RS ECC
      ecc_bytes = gf256_rs_encode(data_bytes, ecc_cw_count)

      # Step 5: Build combined bit stream and apply stuffing
      all_bytes = data_bytes ++ ecc_bytes

      raw_bits =
        Enum.flat_map(all_bytes, fn byte -> int_to_bits(byte, 8) end)

      stuffed_bits = stuff_bits(raw_bits)

      # Step 6: Compute mode message
      mode_msg_bits = encode_mode_message(compact, layers, data_cw_count)

      # Step 7: Initialize grid
      size = if compact, do: 11 + 4 * layers, else: 15 + 4 * layers
      cx = div(size, 2)
      cy = div(size, 2)

      modules = make_grid(size, false)
      reserved = make_grid(size, false)

      # Reference grid FIRST (full only), then bullseye overwrites center
      {modules, reserved} =
        if compact do
          {modules, reserved}
        else
          draw_reference_grid(modules, reserved, cx, cy, size)
        end

      {modules, reserved} = draw_bullseye(modules, reserved, cx, cy, compact)

      {modules, reserved, remaining_positions} =
        draw_orientation_and_mode_message(modules, reserved, cx, cy, compact, mode_msg_bits)

      # Step 8: Place data bits
      modules =
        place_data_bits(
          modules,
          reserved,
          stuffed_bits,
          cx, cy,
          compact,
          layers,
          remaining_positions
        )

      # Convert tuple-of-tuples to list-of-lists
      module_list =
        Enum.map(0..(size - 1)//1, fn row ->
          Enum.map(0..(size - 1)//1, fn col ->
            grid_get(modules, row, col)
          end)
        end)

      {:ok, %{rows: size, cols: size, modules: module_list}}
    end
  end

  @doc """
  Encode a string or binary and return the module grid directly (raises on error).

  ## Examples

      iex> grid = CodingAdventures.AztecCode.encode!("A")
      iex> grid.rows
      15
  """
  def encode!(input, opts \\ %{}) do
    case encode(input, opts) do
      {:ok, grid} -> grid
      {:error, :input_too_long} ->
        raise ArgumentError, "Input too long to fit in any Aztec Code symbol"
    end
  end

  @doc """
  Render an Aztec Code symbol as an ASCII art string (for debugging).

  Dark modules are rendered as `█`, light modules as space.

  ## Examples

      iex> art = CodingAdventures.AztecCode.render_ascii("A")
      iex> is_binary(art)
      true
  """
  def render_ascii(input, opts \\ %{}) do
    case encode(input, opts) do
      {:ok, grid} ->
        Enum.map_join(grid.modules, "\n", fn row ->
          Enum.map_join(row, "", fn dark -> if dark, do: "█", else: " " end)
        end)

      {:error, reason} ->
        inspect(reason)
    end
  end
end
