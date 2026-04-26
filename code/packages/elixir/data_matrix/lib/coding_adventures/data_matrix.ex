defmodule CodingAdventures.DataMatrix do
  @moduledoc """
  Data Matrix ECC200 encoder — ISO/IEC 16022:2006 compliant.

  Data Matrix is a two-dimensional matrix barcode invented in 1989 and
  standardised as ISO/IEC 16022:2006. Unlike QR Code, which needs three
  finder squares scattered in the corners, Data Matrix surrounds the entire
  symbol with a single "L + clock" border:

      D D D D D D D D D D   ← timing row (alternating dark/light from left)
      D · · · · · · · · D   ← right column timing
      D ·   data area   · D
      D · · · · · · · · D
      D D D D D D D D D D   ← L-finder bottom row (all dark)
      ↑ L-finder left column (all dark)

  The "L-shape" (left column + bottom row all dark) is the finder pattern.
  The alternating top-row / right-column is the timing pattern (clock).
  Together they form the "L + clock" border that a scanner uses to locate
  and orient the symbol.

  ## Key differences from QR Code

  | Property          | QR Code              | Data Matrix              |
  |-------------------|----------------------|--------------------------|
  | GF polynomial     | 0x11D                | **0x12D**                |
  | RS root conv.     | b=0 (α^0..α^{n−1})  | **b=1 (α^1..α^n)**       |
  | Finder pattern    | Three 7×7 squares    | **L + clock border**     |
  | Data placement    | Two-column zigzag    | **Utah diagonal zigzag** |
  | Masking           | 8 patterns evaluated | **No masking**           |

  ## Encoding pipeline

      input string
        → ASCII encoding   (chars+1; consecutive digit pairs packed)
        → symbol selection (smallest size whose capacity ≥ codeword count)
        → pad to capacity  (scrambled-pad codewords fill unused slots)
        → RS blocks + ECC  (GF(256)/0x12D, b=1 convention, interleaved)
        → grid init        (L-finder + timing + alignment borders)
        → Utah placement   (diagonal codeword placement, no masking)
        → ModuleGrid

  ## IMPORTANT: Elixir reserved words

  Elixir reserves `after`, `rescue`, `catch`, `else`, `end`, `do`, `fn`,
  `when`, `cond`, `try`, `receive`, `true`, `false`, `nil`.  These CANNOT be
  used as variable names.  Throughout this module we use:
  - `data_cw` instead of `data` (avoids confusion with `do`)
  - `ecc_cw` / `ecc_block` instead of `ecc`
  - `rest_bytes` instead of `after`
  """

  import Bitwise

  @version "0.1.0"
  def version, do: @version

  # ============================================================================
  # Public types
  # ============================================================================

  @typedoc "Symbol shape preference for auto-selection."
  @type symbol_shape :: :square | :rectangular | :any

  @typedoc "Options for `encode/2`."
  @type data_matrix_options :: %{
    optional(:shape) => symbol_shape()
  }

  # ============================================================================
  # GF(256) with primitive polynomial 0x12D
  # ============================================================================
  #
  # Data Matrix uses GF(256) with the irreducible polynomial:
  #
  #   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D  =  301 decimal
  #
  # This is DIFFERENT from QR Code's polynomial (0x11D = x^8+x^4+x^3+x^2+1).
  # Both produce valid GF(256) fields, but the fields are non-isomorphic.
  # In particular, the primitive element α has different powers in each.
  #
  # How GF(256) arithmetic works:
  #   - Elements are polynomials of degree < 8 over GF(2) (i.e., bytes 0-255).
  #   - Addition is XOR.
  #   - Multiplication: polynomial multiply, then reduce mod p(x).
  #   - The trick: represent elements as powers of a generator α (= x = 2).
  #     α^0=1, α^1=2, α^2=4, ..., α^7=128, α^8 = reduce(256, 0x12D) = 0x2D.
  #   - With the exp/log tables: gf_mul(a,b) = exp[(log[a]+log[b]) mod 255].

  # Build the GF(256)/0x12D exp and log tables at compile time as a
  # module attribute.  Module attributes are evaluated at compile time;
  # we compute the tables inline here without calling any function.
  #
  # gf_exp[i] = α^i mod 0x12D   (for i = 0..254; gf_exp[255] = 1)
  # gf_log[v] = k such that α^k = v  (for v = 1..255; gf_log[0] = undefined)

  @gf_tables (
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

    # exp table: 255 elements (index 0..254) + wrap: index 255 = 1
    exp_list = Enum.reverse(exp_rev) ++ [1]

    # log table as a map {byte_value => log_index}
    log_map = Map.new(log_pairs)

    %{exp: List.to_tuple(exp_list), log: log_map}
  )

  @doc """
  Multiply two GF(256)/0x12D field elements.

  Uses the log/exp table trick:
    a × b = α^{(log(a) + log(b)) mod 255}

  Zero is an absorbing element: 0 × anything = 0.

  ## Examples

      iex> CodingAdventures.DataMatrix.gf_mul(2, 2)
      4

      iex> CodingAdventures.DataMatrix.gf_mul(0x80, 2)
      0x2D

      iex> CodingAdventures.DataMatrix.gf_mul(0, 255)
      0
  """
  @compile {:inline, gf_mul: 2}
  def gf_mul(a, b) when a == 0 or b == 0, do: 0
  def gf_mul(a, b) do
    log_a = Map.fetch!(@gf_tables[:log], a)
    log_b = Map.fetch!(@gf_tables[:log], b)
    idx = rem(log_a + log_b, 255)
    elem(@gf_tables[:exp], idx)
  end

  @doc "Return the GF(256)/0x12D exp table as a tuple (index 0..255)."
  def gf_exp_table, do: @gf_tables[:exp]

  @doc "Return the GF(256)/0x12D log table as a map (value => log index)."
  def gf_log_table, do: @gf_tables[:log]

  # ============================================================================
  # Symbol size table
  # ============================================================================
  #
  # Each entry describes one Data Matrix ECC200 symbol size:
  #
  #   symbol_rows, symbol_cols — total size including outer border
  #   region_rows, region_cols — number of data sub-regions (rr × rc)
  #   data_region_height, data_region_width — size of each sub-region's interior
  #   data_cw   — total data codeword capacity
  #   ecc_cw    — total ECC codeword count
  #   num_blocks — number of interleaved RS blocks
  #   ecc_per_block — ECC codewords per block
  #
  # For small symbols (≤ 26×26), there is one data region (1×1) whose interior
  # exactly fills the area between the outer borders.
  # For large symbols, multiple regions are separated by 2-module alignment
  # borders (one all-dark bar + one alternating bar = 2 modules wide).
  #
  # Source: ISO/IEC 16022:2006, Table 7.

  @type symbol_entry :: %{
    symbol_rows: non_neg_integer(),
    symbol_cols: non_neg_integer(),
    region_rows: non_neg_integer(),
    region_cols: non_neg_integer(),
    data_region_height: non_neg_integer(),
    data_region_width: non_neg_integer(),
    data_cw: non_neg_integer(),
    ecc_cw: non_neg_integer(),
    num_blocks: non_neg_integer(),
    ecc_per_block: non_neg_integer()
  }

  # 24 square symbol sizes (10×10 .. 144×144)
  @square_sizes [
    %{symbol_rows: 10,  symbol_cols: 10,  region_rows: 1, region_cols: 1, data_region_height:  8, data_region_width:  8, data_cw:    3, ecc_cw:   5, num_blocks:  1, ecc_per_block:  5},
    %{symbol_rows: 12,  symbol_cols: 12,  region_rows: 1, region_cols: 1, data_region_height: 10, data_region_width: 10, data_cw:    5, ecc_cw:   7, num_blocks:  1, ecc_per_block:  7},
    %{symbol_rows: 14,  symbol_cols: 14,  region_rows: 1, region_cols: 1, data_region_height: 12, data_region_width: 12, data_cw:    8, ecc_cw:  10, num_blocks:  1, ecc_per_block: 10},
    %{symbol_rows: 16,  symbol_cols: 16,  region_rows: 1, region_cols: 1, data_region_height: 14, data_region_width: 14, data_cw:   12, ecc_cw:  12, num_blocks:  1, ecc_per_block: 12},
    %{symbol_rows: 18,  symbol_cols: 18,  region_rows: 1, region_cols: 1, data_region_height: 16, data_region_width: 16, data_cw:   18, ecc_cw:  14, num_blocks:  1, ecc_per_block: 14},
    %{symbol_rows: 20,  symbol_cols: 20,  region_rows: 1, region_cols: 1, data_region_height: 18, data_region_width: 18, data_cw:   22, ecc_cw:  18, num_blocks:  1, ecc_per_block: 18},
    %{symbol_rows: 22,  symbol_cols: 22,  region_rows: 1, region_cols: 1, data_region_height: 20, data_region_width: 20, data_cw:   30, ecc_cw:  20, num_blocks:  1, ecc_per_block: 20},
    %{symbol_rows: 24,  symbol_cols: 24,  region_rows: 1, region_cols: 1, data_region_height: 22, data_region_width: 22, data_cw:   36, ecc_cw:  24, num_blocks:  1, ecc_per_block: 24},
    %{symbol_rows: 26,  symbol_cols: 26,  region_rows: 1, region_cols: 1, data_region_height: 24, data_region_width: 24, data_cw:   44, ecc_cw:  28, num_blocks:  1, ecc_per_block: 28},
    %{symbol_rows: 32,  symbol_cols: 32,  region_rows: 2, region_cols: 2, data_region_height: 14, data_region_width: 14, data_cw:   62, ecc_cw:  36, num_blocks:  2, ecc_per_block: 18},
    %{symbol_rows: 36,  symbol_cols: 36,  region_rows: 2, region_cols: 2, data_region_height: 16, data_region_width: 16, data_cw:   86, ecc_cw:  42, num_blocks:  2, ecc_per_block: 21},
    %{symbol_rows: 40,  symbol_cols: 40,  region_rows: 2, region_cols: 2, data_region_height: 18, data_region_width: 18, data_cw:  114, ecc_cw:  48, num_blocks:  2, ecc_per_block: 24},
    %{symbol_rows: 44,  symbol_cols: 44,  region_rows: 2, region_cols: 2, data_region_height: 20, data_region_width: 20, data_cw:  144, ecc_cw:  56, num_blocks:  4, ecc_per_block: 14},
    %{symbol_rows: 48,  symbol_cols: 48,  region_rows: 2, region_cols: 2, data_region_height: 22, data_region_width: 22, data_cw:  174, ecc_cw:  68, num_blocks:  4, ecc_per_block: 17},
    %{symbol_rows: 52,  symbol_cols: 52,  region_rows: 2, region_cols: 2, data_region_height: 24, data_region_width: 24, data_cw:  204, ecc_cw:  84, num_blocks:  4, ecc_per_block: 21},
    %{symbol_rows: 64,  symbol_cols: 64,  region_rows: 4, region_cols: 4, data_region_height: 14, data_region_width: 14, data_cw:  280, ecc_cw: 112, num_blocks:  4, ecc_per_block: 28},
    %{symbol_rows: 72,  symbol_cols: 72,  region_rows: 4, region_cols: 4, data_region_height: 16, data_region_width: 16, data_cw:  368, ecc_cw: 144, num_blocks:  4, ecc_per_block: 36},
    %{symbol_rows: 80,  symbol_cols: 80,  region_rows: 4, region_cols: 4, data_region_height: 18, data_region_width: 18, data_cw:  456, ecc_cw: 192, num_blocks:  4, ecc_per_block: 48},
    %{symbol_rows: 88,  symbol_cols: 88,  region_rows: 4, region_cols: 4, data_region_height: 20, data_region_width: 20, data_cw:  576, ecc_cw: 224, num_blocks:  4, ecc_per_block: 56},
    %{symbol_rows: 96,  symbol_cols: 96,  region_rows: 4, region_cols: 4, data_region_height: 22, data_region_width: 22, data_cw:  696, ecc_cw: 272, num_blocks:  4, ecc_per_block: 68},
    %{symbol_rows: 104, symbol_cols: 104, region_rows: 4, region_cols: 4, data_region_height: 24, data_region_width: 24, data_cw:  816, ecc_cw: 336, num_blocks:  6, ecc_per_block: 56},
    %{symbol_rows: 120, symbol_cols: 120, region_rows: 6, region_cols: 6, data_region_height: 18, data_region_width: 18, data_cw: 1050, ecc_cw: 408, num_blocks:  6, ecc_per_block: 68},
    %{symbol_rows: 132, symbol_cols: 132, region_rows: 6, region_cols: 6, data_region_height: 20, data_region_width: 20, data_cw: 1304, ecc_cw: 496, num_blocks:  8, ecc_per_block: 62},
    %{symbol_rows: 144, symbol_cols: 144, region_rows: 6, region_cols: 6, data_region_height: 22, data_region_width: 22, data_cw: 1558, ecc_cw: 620, num_blocks: 10, ecc_per_block: 62}
  ]

  # 6 rectangular symbol sizes (8×18 .. 16×48)
  @rect_sizes [
    %{symbol_rows:  8, symbol_cols: 18, region_rows: 1, region_cols: 1, data_region_height:  6, data_region_width: 16, data_cw:  5, ecc_cw:  7, num_blocks: 1, ecc_per_block:  7},
    %{symbol_rows:  8, symbol_cols: 32, region_rows: 1, region_cols: 2, data_region_height:  6, data_region_width: 14, data_cw: 10, ecc_cw: 11, num_blocks: 1, ecc_per_block: 11},
    %{symbol_rows: 12, symbol_cols: 26, region_rows: 1, region_cols: 1, data_region_height: 10, data_region_width: 24, data_cw: 16, ecc_cw: 14, num_blocks: 1, ecc_per_block: 14},
    %{symbol_rows: 12, symbol_cols: 36, region_rows: 1, region_cols: 2, data_region_height: 10, data_region_width: 16, data_cw: 22, ecc_cw: 18, num_blocks: 1, ecc_per_block: 18},
    %{symbol_rows: 16, symbol_cols: 36, region_rows: 1, region_cols: 2, data_region_height: 14, data_region_width: 16, data_cw: 32, ecc_cw: 24, num_blocks: 1, ecc_per_block: 24},
    %{symbol_rows: 16, symbol_cols: 48, region_rows: 1, region_cols: 2, data_region_height: 14, data_region_width: 22, data_cw: 49, ecc_cw: 28, num_blocks: 1, ecc_per_block: 28}
  ]

  @doc "Return all 24 square symbol size entries."
  def square_sizes, do: @square_sizes

  @doc "Return all 6 rectangular symbol size entries."
  def rect_sizes, do: @rect_sizes

  # ============================================================================
  # Symbol selection
  # ============================================================================

  @doc """
  Select the smallest symbol size that can hold `codeword_count` data codewords.

  ## Options

  - `:shape` — `:square` (default), `:rectangular`, or `:any`

  Returns `{:ok, entry}` or `{:error, :input_too_long}`.

  ## Examples

      iex> {:ok, entry} = CodingAdventures.DataMatrix.select_symbol(1, :square)
      iex> entry.symbol_rows
      10

      iex> {:ok, entry} = CodingAdventures.DataMatrix.select_symbol(4, :square)
      iex> entry.symbol_rows
      12
  """
  def select_symbol(codeword_count, shape \\ :square) do
    candidates =
      case shape do
        :square -> @square_sizes
        :rectangular -> @rect_sizes
        :any -> @square_sizes ++ @rect_sizes
      end

    # Sort by data capacity, then by total area for tie-breaking
    sorted =
      Enum.sort_by(candidates, fn e ->
        {e.data_cw, e.symbol_rows * e.symbol_cols}
      end)

    case Enum.find(sorted, fn e -> e.data_cw >= codeword_count end) do
      nil ->
        {:error,
         {:input_too_long,
          "Encoded data requires #{codeword_count} codewords, exceeds maximum 1558 (144×144 symbol)."}}

      entry ->
        {:ok, entry}
    end
  end

  # ============================================================================
  # ASCII encoding
  # ============================================================================
  #
  # Data Matrix uses a codeword vocabulary of 256 values.  In ASCII mode
  # (the default):
  #
  #   - Single ASCII char c (0–127)     → codeword = c + 1        (range 1..128)
  #   - Two consecutive ASCII digits    → codeword = 130 + (d1×10 + d2)  (range 130..229)
  #   - Extended ASCII c (128–255)      → UPPER_SHIFT (235), c − 127
  #   - Pad (fill unused capacity)      → 129
  #
  # The digit-pair optimization is crucial for numeric input:
  #   "12345678" → [142, 174, 196, 208]   (4 codewords for 8 chars)
  # vs.
  #   single-char → [50, 51, 52, 53, 54, 55, 56, 57]  (8 codewords)
  # Saving: 50% for purely numeric input.

  @doc """
  Encode a binary/string in Data Matrix ASCII mode.

  Rules:
  - Two consecutive ASCII digits → one codeword: `130 + (d1×10 + d2)`
  - Single ASCII char (0–127)   → one codeword: `ascii_value + 1`
  - Extended ASCII (128–255)    → two codewords: `235` then `value - 127`

  ## Examples

      iex> CodingAdventures.DataMatrix.encode_ascii("A")
      [66]

      iex> CodingAdventures.DataMatrix.encode_ascii("12")
      [142]

      iex> CodingAdventures.DataMatrix.encode_ascii("1234")
      [142, 164]

      iex> CodingAdventures.DataMatrix.encode_ascii("1A")
      [50, 66]

      iex> CodingAdventures.DataMatrix.encode_ascii("00")
      [130]

      iex> CodingAdventures.DataMatrix.encode_ascii("99")
      [229]
  """
  def encode_ascii(input) when is_binary(input) do
    encode_ascii(:binary.bin_to_list(input))
  end

  def encode_ascii([]) do
    []
  end

  def encode_ascii([c1, c2 | rest_bytes])
      when c1 >= 0x30 and c1 <= 0x39 and c2 >= 0x30 and c2 <= 0x39 do
    # Two consecutive ASCII digits: pack as one codeword
    # 130 + (d1 * 10 + d2) where d1 = c1 - 0x30, d2 = c2 - 0x30
    d1 = c1 - 0x30
    d2 = c2 - 0x30
    [130 + d1 * 10 + d2 | encode_ascii(rest_bytes)]
  end

  def encode_ascii([c | rest_bytes]) when c <= 127 do
    # Single ASCII character (0–127): codeword = value + 1
    [c + 1 | encode_ascii(rest_bytes)]
  end

  def encode_ascii([c | rest_bytes]) when c >= 128 do
    # Extended ASCII (128–255): UPPER_SHIFT (235) followed by value - 127
    [235, c - 127 | encode_ascii(rest_bytes)]
  end

  # ============================================================================
  # Pad codewords
  # ============================================================================
  #
  # After encoding data, the codeword stream must be padded to exactly
  # `data_cw` bytes (the symbol's full capacity).  ISO/IEC 16022 §5.2.3:
  #
  #   1. First pad byte is always 129 (the pad codeword).
  #   2. Subsequent pads use a scrambling formula to avoid long runs of 129:
  #        scrambled = 129 + (149 × k mod 253) + 1
  #        if scrambled > 254: scrambled -= 254
  #      where k is the 1-indexed position in the full codeword stream.
  #
  # Example: "A" encodes to [66], padded to 3 (10×10 symbol capacity):
  #   position k=2: first pad → 129
  #   position k=3: 129 + (149×3 mod 253) + 1 = 129 + 194 + 1 = 324 > 254 → 70
  #   result: [66, 129, 70]

  @doc """
  Pad `codewords` to exactly `data_cw` length using the ISO scrambled-pad rule.

  ## Examples

      iex> CodingAdventures.DataMatrix.pad_codewords([66], 3)
      [66, 129, 70]
  """
  def pad_codewords(codewords, data_cw) do
    current_len = length(codewords)

    if current_len >= data_cw do
      Enum.take(codewords, data_cw)
    else
      # k is 1-indexed position of the next pad byte in the full codeword stream
      # The first pad is at position (current_len + 1)
      pad_bytes =
        Enum.map((current_len + 1)..data_cw//1, fn k ->
          if k == current_len + 1 do
            # First pad byte is always 129
            129
          else
            # Subsequent pads: scrambled
            scrambled = 129 + rem(149 * k, 253) + 1
            if scrambled > 254, do: scrambled - 254, else: scrambled
          end
        end)

      codewords ++ pad_bytes
    end
  end

  # ============================================================================
  # Reed-Solomon generator polynomial
  # ============================================================================
  #
  # The RS generator for n ECC symbols uses the b=1 convention:
  #   g(x) = (x + α^1)(x + α^2)···(x + α^n)
  #
  # This is exactly what Data Matrix requires, and differs from QR Code which
  # uses b=0: g(x) = (x + α^0)(x + α^1)···(x + α^{n-1}).
  #
  # Building the generator iteratively:
  #   Start with g = [1]  (constant polynomial = 1)
  #   For k = 1 to n:
  #     Multiply g(x) by (x + α^k)
  #     i.e., for each coefficient gj: new[j] ^= gj; new[j+1] ^= gf_mul(gj, α^k)

  @doc """
  Build the RS generator polynomial for `n_ecc` ECC codewords (b=1 convention).

  Returns a list of `n_ecc + 1` GF(256) bytes, highest degree first, with
  the leading coefficient equal to 1 (monic polynomial).

  Each root α^k (k = 1..n_ecc) satisfies g(α^k) = 0 over GF(256)/0x12D.

  ## Examples

      iex> gen = CodingAdventures.DataMatrix.build_generator(5)
      iex> length(gen)
      6
      iex> hd(gen)
      1
  """
  def build_generator(n_ecc) do
    Enum.reduce(1..n_ecc//1, [1], fn k, g ->
      # α^k in GF(256)/0x12D
      alpha_k = elem(@gf_tables.exp, k)

      # Multiply g(x) by (x + alpha_k)
      # Shift g to get x*g(x), XOR with alpha_k * g(x)
      n = length(g)
      next = List.duplicate(0, n + 1)

      {next, _} =
        Enum.reduce(0..(n - 1)//1, {next, g}, fn j, {acc, [gj | rest]} ->
          acc = List.update_at(acc, j, &bxor(&1, gj))
          acc = List.update_at(acc, j + 1, &bxor(&1, gf_mul(gj, alpha_k)))
          {acc, rest}
        end)

      next
    end)
  end

  # ============================================================================
  # Reed-Solomon block encoding
  # ============================================================================
  #
  # Each RS block's ECC bytes are computed as the polynomial remainder:
  #   R(x) = D(x) × x^n_ecc  mod  G(x)
  #
  # The LFSR (linear feedback shift register) approach implements this
  # efficiently without explicitly constructing D(x) × x^n_ecc:
  #
  #   Initialize rem = [0, 0, ..., 0]  (n_ecc zeros)
  #   For each data byte d:
  #     feedback = d XOR rem[0]
  #     Shift rem left: rem[i] ← rem[i+1]
  #     For each position i: rem[i] ^= generator[i+1] × feedback
  #
  # The `generator` parameter includes the leading 1 coefficient, so
  # generator[0] = 1 (not used in the feedback loop) and generator[1..n_ecc]
  # are the coefficients multiplied by feedback.

  @doc """
  Compute `n_ecc` ECC bytes for one RS block using LFSR polynomial division.

  `generator` is the output of `build_generator/1` (n_ecc+1 coefficients,
  leading 1 at index 0).

  ## Examples

      iex> gen = CodingAdventures.DataMatrix.build_generator(5)
      iex> ecc = CodingAdventures.DataMatrix.rs_encode_block([66, 129, 70], gen)
      iex> length(ecc)
      5
  """
  def rs_encode_block(data_bytes, generator) do
    n_ecc = length(generator) - 1
    # Generator without the leading 1: gen_tail = generator[1..n_ecc]
    gen_tail = tl(generator)

    rem_final =
      Enum.reduce(data_bytes, List.duplicate(0, n_ecc), fn byte, rem ->
        feedback = bxor(byte, hd(rem))

        # Shift rem left (drop head, append 0 at end)
        shifted = tl(rem) ++ [0]

        # XOR each position with gen_tail[i] * feedback
        if feedback == 0 do
          shifted
        else
          Enum.zip(shifted, gen_tail)
          |> Enum.map(fn {r, g} -> bxor(r, gf_mul(g, feedback)) end)
        end
      end)

    rem_final
  end

  # ============================================================================
  # Block splitting and interleaving
  # ============================================================================
  #
  # Large symbols split the data stream across multiple independent RS blocks.
  # Interleaving means the codewords from different blocks are interwoven in
  # the final stream placed into the symbol.  This distributes burst errors
  # (physical scratches/contamination) across multiple blocks, so each block
  # only loses a few codewords from a scratch that destroys many consecutive
  # modules.
  #
  # Interleaving convention:
  #   - Data codewords: round-robin across blocks
  #     [d0_0, d1_0, d2_0, d0_1, d1_1, d2_1, ...]
  #   - ECC codewords: round-robin across blocks after all data
  #     [e0_0, e1_0, e2_0, e0_1, e1_1, e2_1, ...]
  #
  # Block size distribution:
  #   - If data_cw mod num_blocks == 0: all blocks have data_cw/num_blocks
  #   - Else: first (data_cw mod num_blocks) blocks get one extra codeword

  @doc false
  def compute_interleaved(data_bytes, entry) do
    %{data_cw: data_cw, num_blocks: num_blocks, ecc_per_block: ecc_per_block} = entry
    generator = build_generator(ecc_per_block)

    base_len = div(data_cw, num_blocks)
    extra_blocks = rem(data_cw, num_blocks)

    # Split data into blocks: earlier blocks get ceiling, later get floor
    {data_blocks, _} =
      Enum.reduce(0..(num_blocks - 1)//1, {[], data_bytes}, fn b, {acc, remaining} ->
        block_len = if b < extra_blocks, do: base_len + 1, else: base_len
        {block, rest} = Enum.split(remaining, block_len)
        {acc ++ [block], rest}
      end)

    # Compute ECC for each block
    ecc_blocks = Enum.map(data_blocks, fn blk -> rs_encode_block(blk, generator) end)

    # Max data block length (for interleaving)
    max_data_len = Enum.map(data_blocks, &length/1) |> Enum.max()

    # Interleave data codewords (round-robin)
    interleaved_data =
      for pos <- 0..(max_data_len - 1)//1,
          b <- 0..(num_blocks - 1)//1,
          pos < length(Enum.at(data_blocks, b)),
          do: Enum.at(Enum.at(data_blocks, b), pos)

    # Interleave ECC codewords (round-robin)
    interleaved_ecc =
      for pos <- 0..(ecc_per_block - 1)//1,
          b <- 0..(num_blocks - 1)//1,
          do: Enum.at(Enum.at(ecc_blocks, b), pos)

    interleaved_data ++ interleaved_ecc
  end

  # ============================================================================
  # Grid initialization
  # ============================================================================
  #
  # The physical module grid is a 2D array of booleans (true = dark, false = light).
  # Before placing data, the structural elements are painted:
  #
  # Writing order (last-write wins at shared positions):
  #   1. Alignment borders (for multi-region symbols)
  #   2. Top row timing (alternating: dark at even columns)
  #   3. Right column timing (alternating: dark at even rows)
  #   4. Left column L-finder (all dark)
  #   5. Bottom row L-finder (all dark — highest precedence)
  #
  # The L-finder is written last because it overrides the timing pattern at
  # corner intersections (e.g., the top-left corner is simultaneously the
  # start of the L-finder column AND the start of the timing row — it must
  # be dark, which the timing row agrees with since col 0 is even).

  @doc false
  def init_grid(entry) do
    %{
      symbol_rows: symbol_rows,
      symbol_cols: symbol_cols,
      region_rows: region_rows,
      region_cols: region_cols,
      data_region_height: drh,
      data_region_width: drw
    } = entry

    # Start with an all-light (false) grid
    grid = for _r <- 0..(symbol_rows - 1)//1, do: List.duplicate(false, symbol_cols)
    grid = List.to_tuple(Enum.map(grid, &List.to_tuple/1))

    # Step 1: Alignment borders between data regions
    # For each pair of adjacent region rows, place 2 horizontal alignment rows
    grid =
      Enum.reduce(0..(region_rows - 2)//1, grid, fn rr, g ->
        # After the rr-th region row ends, there is a 2-module alignment border
        ab_row0 = 1 + (rr + 1) * drh + rr * 2
        ab_row1 = ab_row0 + 1

        g =
          Enum.reduce(0..(symbol_cols - 1)//1, g, fn c, gg ->
            gg = grid_set(gg, ab_row0, c, true)            # all dark
            grid_set(gg, ab_row1, c, rem(c, 2) == 0)       # alternating
          end)

        g
      end)

    grid =
      Enum.reduce(0..(region_cols - 2)//1, grid, fn rc, g ->
        ab_col0 = 1 + (rc + 1) * drw + rc * 2
        ab_col1 = ab_col0 + 1

        Enum.reduce(0..(symbol_rows - 1)//1, g, fn r, gg ->
          gg = grid_set(gg, r, ab_col0, true)              # all dark
          grid_set(gg, r, ab_col1, rem(r, 2) == 0)         # alternating
        end)
      end)

    # Step 2: Top row timing (alternating, dark at even columns)
    grid =
      Enum.reduce(0..(symbol_cols - 1)//1, grid, fn c, g ->
        grid_set(g, 0, c, rem(c, 2) == 0)
      end)

    # Step 3: Right column timing (alternating, dark at even rows)
    grid =
      Enum.reduce(0..(symbol_rows - 1)//1, grid, fn r, g ->
        grid_set(g, r, symbol_cols - 1, rem(r, 2) == 0)
      end)

    # Step 4: Left column all dark (L-finder left leg)
    grid =
      Enum.reduce(0..(symbol_rows - 1)//1, grid, fn r, g ->
        grid_set(g, r, 0, true)
      end)

    # Step 5: Bottom row all dark (L-finder bottom leg — highest precedence)
    grid =
      Enum.reduce(0..(symbol_cols - 1)//1, grid, fn c, g ->
        grid_set(g, symbol_rows - 1, c, true)
      end)

    grid
  end

  # Helper: set grid[r][c] = value, returning updated grid (tuple of tuples)
  @compile {:inline, grid_set: 4}
  defp grid_set(grid, r, c, value) do
    row = elem(grid, r)
    put_elem(grid, r, put_elem(row, c, value))
  end

  # Helper: get grid[r][c]
  @compile {:inline, grid_get: 3}
  defp grid_get(grid, r, c) do
    elem(elem(grid, r), c)
  end

  # ============================================================================
  # Utah placement algorithm
  # ============================================================================
  #
  # The Utah algorithm is named for the US state whose shape the 8-bit module
  # placement pattern resembles.  The standard Utah shape for one codeword:
  #
  #     col: c-2  c-1   c
  #     r-2:  .   [1]  [2]    bit 1 = LSB, bit 8 = MSB
  #     r-1: [3]  [4]  [5]
  #     r  : [6]  [7]  [8]
  #
  # The algorithm starts at (row=4, col=0) and scans diagonally:
  #   - Upward-right diagonal: row-=2, col+=2
  #   - Then step: row+=1, col+=3
  #   - Downward-left diagonal: row+=2, col-=2
  #   - Then step: row+=3, col+=1
  #   - Repeat until all codewords placed or grid exhausted
  #
  # Four corner patterns handle edge cases where the standard Utah shape
  # would fall partially outside the grid boundaries.
  #
  # After all codewords are placed, any unset modules are filled with:
  #   dark if (r + c) mod 2 == 1  (the "fill" rule from ISO Annex F)

  # Boundary wrap rules (from ISO/IEC 16022 Annex F)
  # Applied when a Utah placement position falls outside the grid.
  defp apply_wrap(row, col, n_rows, n_cols) do
    cond do
      row < 0 and col == 0 ->
        {1, 3}

      row < 0 and col == n_cols ->
        {0, col - 2}

      row < 0 ->
        {row + n_rows, col - 4}

      col < 0 ->
        {row - 4, col + n_cols}

      true ->
        {row, col}
    end
  end

  # Place one 8-bit codeword using the standard Utah shape.
  # Offsets (dr, dc, bit_index) where bit 7 = MSB, bit 0 = LSB.
  # The pattern is:
  #   (0, 0, 7)   = bit 8 at (row, col)
  #   (0,-1, 6)   = bit 7 at (row, col-1)
  #   (0,-2, 5)   = bit 6 at (row, col-2)
  #   (-1, 0, 4)  = bit 5 at (row-1, col)
  #   (-1,-1, 3)  = bit 4 at (row-1, col-1)
  #   (-1,-2, 2)  = bit 3 at (row-1, col-2)
  #   (-2, 0, 1)  = bit 2 at (row-2, col)
  #   (-2,-1, 0)  = bit 1 at (row-2, col-1)
  @utah_offsets [
    { 0,  0, 7},
    { 0, -1, 6},
    { 0, -2, 5},
    {-1,  0, 4},
    {-1, -1, 3},
    {-1, -2, 2},
    {-2,  0, 1},
    {-2, -1, 0}
  ]

  defp place_utah(codeword, row, col, n_rows, n_cols, grid, used) do
    Enum.reduce(@utah_offsets, {grid, used}, fn {dr, dc, bit}, {g, u} ->
      {wr, wc} = apply_wrap(row + dr, col + dc, n_rows, n_cols)

      if wr >= 0 and wr < n_rows and wc >= 0 and wc < n_cols and
           not grid_get(u, wr, wc) do
        value = band(bsr(codeword, bit), 1) == 1
        {grid_set(g, wr, wc, value), grid_set(u, wr, wc, true)}
      else
        {g, u}
      end
    end)
  end

  # Corner pattern 1: top-left wrap
  # Triggered when (row == n_rows and col == 0) and (n_rows mod 4 == 0 or n_cols mod 4 == 0)
  defp place_corner1(codeword, n_rows, n_cols, grid, used) do
    positions = [
      {0,           n_cols - 2, 7},
      {0,           n_cols - 1, 6},
      {1,           0,          5},
      {2,           0,          4},
      {n_rows - 2,  0,          3},
      {n_rows - 1,  0,          2},
      {n_rows - 1,  1,          1},
      {n_rows - 1,  2,          0}
    ]

    Enum.reduce(positions, {grid, used}, fn {r, c, bit}, {g, u} ->
      if r >= 0 and r < n_rows and c >= 0 and c < n_cols and not grid_get(u, r, c) do
        value = band(bsr(codeword, bit), 1) == 1
        {grid_set(g, r, c, value), grid_set(u, r, c, true)}
      else
        {g, u}
      end
    end)
  end

  # Corner pattern 2: top-right wrap
  # Triggered when (row == n_rows - 2 and col == 0) and n_cols mod 4 != 0
  defp place_corner2(codeword, n_rows, n_cols, grid, used) do
    positions = [
      {0,           n_cols - 2, 7},
      {0,           n_cols - 1, 6},
      {1,           n_cols - 1, 5},
      {2,           n_cols - 1, 4},
      {n_rows - 1,  0,          3},
      {n_rows - 1,  1,          2},
      {n_rows - 1,  2,          1},
      {n_rows - 1,  3,          0}
    ]

    Enum.reduce(positions, {grid, used}, fn {r, c, bit}, {g, u} ->
      if r >= 0 and r < n_rows and c >= 0 and c < n_cols and not grid_get(u, r, c) do
        value = band(bsr(codeword, bit), 1) == 1
        {grid_set(g, r, c, value), grid_set(u, r, c, true)}
      else
        {g, u}
      end
    end)
  end

  # Corner pattern 3: bottom-left wrap
  # Triggered when (row == n_rows - 2 and col == 0) and n_cols mod 8 == 4
  defp place_corner3(codeword, n_rows, n_cols, grid, used) do
    positions = [
      {0,           n_cols - 1, 7},
      {1,           0,          6},
      {2,           0,          5},
      {n_rows - 2,  0,          4},
      {n_rows - 1,  0,          3},
      {n_rows - 1,  1,          2},
      {n_rows - 1,  2,          1},
      {n_rows - 1,  3,          0}
    ]

    Enum.reduce(positions, {grid, used}, fn {r, c, bit}, {g, u} ->
      if r >= 0 and r < n_rows and c >= 0 and c < n_cols and not grid_get(u, r, c) do
        value = band(bsr(codeword, bit), 1) == 1
        {grid_set(g, r, c, value), grid_set(u, r, c, true)}
      else
        {g, u}
      end
    end)
  end

  # Corner pattern 4: right-edge wrap for odd-dimension matrices
  # Triggered when (row == n_rows + 4 and col == 2) and n_cols mod 8 == 0
  defp place_corner4(codeword, n_rows, n_cols, grid, used) do
    positions = [
      {n_rows - 3,  n_cols - 1, 7},
      {n_rows - 2,  n_cols - 1, 6},
      {n_rows - 1,  n_cols - 3, 5},
      {n_rows - 1,  n_cols - 2, 4},
      {n_rows - 1,  n_cols - 1, 3},
      {0,           0,          2},
      {1,           0,          1},
      {2,           0,          0}
    ]

    Enum.reduce(positions, {grid, used}, fn {r, c, bit}, {g, u} ->
      if r >= 0 and r < n_rows and c >= 0 and c < n_cols and not grid_get(u, r, c) do
        value = band(bsr(codeword, bit), 1) == 1
        {grid_set(g, r, c, value), grid_set(u, r, c, true)}
      else
        {g, u}
      end
    end)
  end

  @doc """
  Run the Utah diagonal placement algorithm on the logical data matrix.

  `codewords` is the interleaved data+ECC byte list.
  `n_rows` × `n_cols` is the logical data matrix size
  (= region_rows × data_region_height × region_cols × data_region_width).

  Returns a tuple-of-tuples boolean grid (the logical data matrix, with
  residual modules filled by the (r+c) mod 2 == 1 rule).
  """
  def utah_placement(codewords, n_rows, n_cols) do
    # Initialize empty logical grid and "used" tracker
    init_row = List.to_tuple(List.duplicate(false, n_cols))
    init = List.to_tuple(List.duplicate(init_row, n_rows))
    grid = init
    used = init

    total_cw = length(codewords)
    cw_arr = List.to_tuple(codewords)

    {grid, final_used, _cw_idx} =
      utah_loop(grid, used, 0, total_cw, cw_arr, 4, 0, n_rows, n_cols)

    # Fill any residual unset modules: dark if (r + c) mod 2 == 1
    Enum.reduce(0..(n_rows - 1)//1, grid, fn r, g ->
      Enum.reduce(0..(n_cols - 1)//1, g, fn c, gg ->
        if not grid_get(final_used, r, c) do
          grid_set(gg, r, c, rem(r + c, 2) == 1)
        else
          gg
        end
      end)
    end)
  end

  # Recursive Utah scanning loop.
  # `row` and `col` are the current reference position.
  defp utah_loop(grid, used, cw_idx, total_cw, cw_arr, row, col, n_rows, n_cols) do
    # Termination: stepped past the grid, or all codewords placed
    if (row >= n_rows and col >= n_cols) or cw_idx >= total_cw do
      {grid, used, cw_idx}
    else
      # --- Corner pattern triggers (checked before diagonal scan) ---
      {grid, used, cw_idx} =
        if row == n_rows and col == 0 and
             (rem(n_rows, 4) == 0 or rem(n_cols, 4) == 0) and
             cw_idx < total_cw do
          {g, u} = place_corner1(elem(cw_arr, cw_idx), n_rows, n_cols, grid, used)
          {g, u, cw_idx + 1}
        else
          {grid, used, cw_idx}
        end

      {grid, used, cw_idx} =
        if row == n_rows - 2 and col == 0 and rem(n_cols, 4) != 0 and cw_idx < total_cw do
          {g, u} = place_corner2(elem(cw_arr, cw_idx), n_rows, n_cols, grid, used)
          {g, u, cw_idx + 1}
        else
          {grid, used, cw_idx}
        end

      {grid, used, cw_idx} =
        if row == n_rows - 2 and col == 0 and rem(n_cols, 8) == 4 and cw_idx < total_cw do
          {g, u} = place_corner3(elem(cw_arr, cw_idx), n_rows, n_cols, grid, used)
          {g, u, cw_idx + 1}
        else
          {grid, used, cw_idx}
        end

      {grid, used, cw_idx} =
        if row == n_rows + 4 and col == 2 and rem(n_cols, 8) == 0 and cw_idx < total_cw do
          {g, u} = place_corner4(elem(cw_arr, cw_idx), n_rows, n_cols, grid, used)
          {g, u, cw_idx + 1}
        else
          {grid, used, cw_idx}
        end

      # --- Upward-right diagonal scan ---
      {grid, used, cw_idx, row, col} =
        utah_up_right(grid, used, cw_idx, total_cw, cw_arr, row, col, n_rows, n_cols)

      # Step to next diagonal start
      row = row + 1
      col = col + 3

      # --- Downward-left diagonal scan ---
      {grid, used, cw_idx, row, col} =
        utah_down_left(grid, used, cw_idx, total_cw, cw_arr, row, col, n_rows, n_cols)

      # Step to next diagonal start
      row = row + 3
      col = col + 1

      utah_loop(grid, used, cw_idx, total_cw, cw_arr, row, col, n_rows, n_cols)
    end
  end

  # Upward-right diagonal scan: row -= 2, col += 2
  defp utah_up_right(grid, used, cw_idx, total_cw, cw_arr, row, col, n_rows, n_cols) do
    if row < 0 or col >= n_cols do
      {grid, used, cw_idx, row, col}
    else
      {grid, used, cw_idx} =
        if row >= 0 and row < n_rows and col >= 0 and col < n_cols and
             not grid_get(used, row, col) and cw_idx < total_cw do
          {g, u} = place_utah(elem(cw_arr, cw_idx), row, col, n_rows, n_cols, grid, used)
          {g, u, cw_idx + 1}
        else
          {grid, used, cw_idx}
        end

      utah_up_right(grid, used, cw_idx, total_cw, cw_arr, row - 2, col + 2, n_rows, n_cols)
    end
  end

  # Downward-left diagonal scan: row += 2, col -= 2
  defp utah_down_left(grid, used, cw_idx, total_cw, cw_arr, row, col, n_rows, n_cols) do
    if row >= n_rows or col < 0 do
      {grid, used, cw_idx, row, col}
    else
      {grid, used, cw_idx} =
        if row >= 0 and row < n_rows and col >= 0 and col < n_cols and
             not grid_get(used, row, col) and cw_idx < total_cw do
          {g, u} = place_utah(elem(cw_arr, cw_idx), row, col, n_rows, n_cols, grid, used)
          {g, u, cw_idx + 1}
        else
          {grid, used, cw_idx}
        end

      utah_down_left(grid, used, cw_idx, total_cw, cw_arr, row + 2, col - 2, n_rows, n_cols)
    end
  end

  # ============================================================================
  # Logical → Physical coordinate mapping
  # ============================================================================
  #
  # The Utah algorithm works on the "logical data matrix" — a virtual grid
  # that spans all data regions as one flat plane.  We then need to map each
  # logical (r, c) to its physical location in the full symbol grid.
  #
  # For a symbol with rr × rc data regions, each of size (rh × rw):
  #   physical_row = (r div rh) * (rh + 2) + (r mod rh) + 1
  #   physical_col = (c div rw) * (rw + 2) + (c mod rw) + 1
  #
  # The +2 accounts for the 2-module alignment border between regions.
  # The +1 accounts for the 1-module outer border (finder + timing).
  #
  # For a 1×1 region symbol, this simplifies to: physical = logical + 1.

  @compile {:inline, logical_to_physical: 3}
  defp logical_to_physical(r, c, entry) do
    rh = entry.data_region_height
    rw = entry.data_region_width
    phys_row = div(r, rh) * (rh + 2) + rem(r, rh) + 1
    phys_col = div(c, rw) * (rw + 2) + rem(c, rw) + 1
    {phys_row, phys_col}
  end

  # Convert the tuple-of-tuples grid to a list-of-lists grid
  defp tuples_to_lists(grid, rows, cols) do
    Enum.map(0..(rows - 1)//1, fn r ->
      Enum.map(0..(cols - 1)//1, fn c ->
        grid_get(grid, r, c)
      end)
    end)
  end

  # ============================================================================
  # Public API
  # ============================================================================

  @doc """
  Encode a string or binary into a Data Matrix ECC200 module grid.

  Returns `{:ok, %{rows: r, cols: c, modules: [[bool]]}}` where `modules`
  is a list of rows, each a list of booleans (`true` = dark, `false` = light).

  Returns `{:error, {:input_too_long, message}}` if the encoded codeword
  count exceeds 1558 (the 144×144 symbol capacity).

  ## Options

  - `:shape` — `:square` (default), `:rectangular`, `:any`

  ## Examples

      iex> {:ok, grid} = CodingAdventures.DataMatrix.encode("A")
      iex> grid.rows
      10
      iex> grid.cols
      10

      iex> {:ok, grid} = CodingAdventures.DataMatrix.encode("Hello World")
      iex> grid.rows
      16
  """
  def encode(input, opts \\ %{}) do
    shape = Map.get(opts, :shape, :square)
    input_bytes =
      cond do
        is_binary(input) -> :binary.bin_to_list(input)
        is_list(input) -> input
        true -> [input]
      end

    # Step 1: ASCII encode
    codewords = encode_ascii(input_bytes)

    # Step 2: Symbol selection
    with {:ok, entry} <- select_symbol(length(codewords), shape) do
      # Step 3: Pad to capacity
      padded = pad_codewords(codewords, entry.data_cw)

      # Step 4-6: Split into blocks, compute ECC, interleave
      interleaved = compute_interleaved(padded, entry)

      # Step 7: Initialize physical grid
      phys_grid = init_grid(entry)

      # Step 8: Utah placement on logical data matrix
      n_rows = entry.region_rows * entry.data_region_height
      n_cols = entry.region_cols * entry.data_region_width
      logical_grid = utah_placement(interleaved, n_rows, n_cols)

      # Step 9: Map logical → physical
      phys_grid =
        Enum.reduce(0..(n_rows - 1)//1, phys_grid, fn r, pg ->
          Enum.reduce(0..(n_cols - 1)//1, pg, fn c, pgg ->
            {pr, pc} = logical_to_physical(r, c, entry)
            value = grid_get(logical_grid, r, c)
            grid_set(pgg, pr, pc, value)
          end)
        end)

      # Step 10: Convert to list-of-lists (no masking — Data Matrix never masks)
      modules = tuples_to_lists(phys_grid, entry.symbol_rows, entry.symbol_cols)

      {:ok,
       %{
         rows: entry.symbol_rows,
         cols: entry.symbol_cols,
         modules: modules
       }}
    end
  end

  @doc """
  Encode a string or binary and return the module grid directly (raises on error).

  ## Examples

      iex> grid = CodingAdventures.DataMatrix.encode!("A")
      iex> grid.rows
      10
  """
  def encode!(input, opts \\ %{}) do
    case encode(input, opts) do
      {:ok, grid} -> grid
      {:error, {:input_too_long, msg}} -> raise ArgumentError, msg
    end
  end

  @doc """
  Render a Data Matrix symbol as an ASCII art string (for debugging).

  Dark modules are `█`, light modules are ` `.

  ## Examples

      iex> art = CodingAdventures.DataMatrix.render_ascii("A")
      iex> is_binary(art)
      true
  """
  def render_ascii(input, opts \\ %{}) do
    case encode(input, opts) do
      {:ok, grid} ->
        lines =
          Enum.map(grid.modules, fn row ->
            Enum.map(row, fn dark -> if dark, do: "█", else: " " end)
            |> Enum.join("")
          end)

        Enum.join(lines, "\n")

      {:error, reason} ->
        inspect(reason)
    end
  end
end
