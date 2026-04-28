defmodule CodingAdventures.PDF417.ModuleGrid do
  @moduledoc """
  The result of a PDF417 encode operation.

  `modules` is a list of rows, each row a list of booleans:
  - `true`  → dark module (bar)
  - `false` → light module (space)

  `rows` and `cols` are the total module dimensions of the symbol
  including start/stop patterns, row indicators, and quiet zone height
  (from `row_height` repetition).
  """

  defstruct [:rows, :cols, :modules]

  @type t :: %__MODULE__{
          rows: pos_integer(),
          cols: pos_integer(),
          modules: [[boolean()]]
        }
end

defmodule CodingAdventures.PDF417 do
  @moduledoc """
  PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.

  ## What is PDF417?

  PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
  Technologies in 1991. The name encodes its geometry: every codeword has
  exactly **4** bars and **4** spaces (8 elements), and every codeword
  occupies exactly **17** modules of horizontal space. 4 + 1 + 7 = "417".

  Unlike a true 2D matrix barcode (QR, Data Matrix), PDF417 is a *stack* of
  short 1D barcode rows. Each row is independently scannable by a laser, which
  is why PDF417 is the format of choice for driver's licences, boarding passes,
  and shipping labels — anywhere a one-shot laser scanner must read a lot of
  data quickly.

  ## Encoding pipeline

  ```
  raw bytes
    -> byte compaction     (codeword 924 latch + 6-bytes-to-5-codewords base-900)
    -> length descriptor   (first codeword = total codewords in symbol)
    -> RS ECC              (GF(929) Reed-Solomon, b=3 convention, alpha=3)
    -> dimension selection (auto: roughly square symbol)
    -> padding             (codeword 900 fills unused slots)
    -> row indicators      (LRI + RRI per row, encode R/C/ECC level)
    -> cluster table lookup (codeword -> 17-module bar/space pattern)
    -> start/stop patterns (fixed per row)
    -> ModuleGrid          (abstract boolean grid)
  ```

  ## v0.1.0 scope

  This release implements **byte compaction only** — every input byte is
  encoded directly without character-set translation. Text and numeric
  compaction are planned for v0.2.0. Byte mode handles arbitrary binary
  content correctly, so it is the safe default for general-purpose encoding.

  ## Quick start

  ```elixir
  {:ok, grid} = CodingAdventures.PDF417.encode("Hello, World!")
  # grid.rows, grid.cols — module dimensions of the symbol
  # grid.modules         — list of rows, each a list of booleans

  {:ok, grid} = CodingAdventures.PDF417.encode("Hello", ecc_level: 4, columns: 5)
  ```

  ## Important: Elixir reserved words

  Elixir reserves `after`, `rescue`, `catch`, `else`, `end`, `do`, `fn`,
  `when`, `cond`, `try`, `receive`, `true`, `false`, `nil`. Throughout this
  module we use:
  - `data_cws`   instead of `data`  (avoids confusion)
  - `ecc_cws`    instead of `ecc`
  - `tail_bytes` instead of `after`
  """

  import Bitwise

  alias CodingAdventures.PDF417.ClusterTables
  alias CodingAdventures.PDF417.ModuleGrid

  @version "0.1.0"
  def version, do: @version

  # ============================================================================
  # Constants
  # ============================================================================

  # GF(929): the integers mod 929.
  #
  # Why 929? The PDF417 codeword alphabet contains 929 values (0..928): 900 for
  # data codewords, plus 29 for control codewords (latches, shifts, padding,
  # and macro headers). A prime modulus guarantees every non-zero element has a
  # multiplicative inverse — a requirement for Reed-Solomon to work.
  # The primitive root (alpha) used is 3: 3^k covers all elements 1..928 exactly once.
  @gf929_prime 929
  @gf929_order 928 # multiplicative group order = prime - 1

  # Byte compaction latch codeword. Codeword 924 signals the decoder to switch
  # into byte compaction mode that accepts any number of bytes (as opposed to
  # 901/902 which have restrictions). We always use 924 for simplicity.
  @latch_byte 924

  # Neutral padding codeword. Codeword 900 is the "latch to text mode" command.
  # When the decoder is already in byte mode and sees 900, it is a no-op.
  # The standard uses it as a padding fill because decoders skip over it gracefully.
  @padding_cw 900

  # Symbol size limits from ISO/IEC 15438:2015.
  @min_rows 3
  @max_rows 90
  @min_cols 1
  @max_cols 30

  # ============================================================================
  # GF(929) exp/log tables — built at compile time
  # ============================================================================
  #
  # GF(929) is the integers mod 929. Since 929 is prime, every non-zero element
  # has a multiplicative inverse. We use log/antilog lookup tables for O(1)
  # multiplication, built once at compile time.
  #
  # gf_exp[i] = alpha^i mod 929   for i = 0..927  (gf_exp[928] = gf_exp[0] = 1)
  # gf_log[v] = i such that alpha^i = v  (for v = 1..928; 0 is undefined)
  #
  # We store gf_exp as a tuple for O(1) index access.
  # We store gf_log as a map {value => log_index} for O(1) lookup.
  #
  # These tables take ~56 KB total (929 integers each in two data structures)
  # and compile away into the beam binary.

  @gf_tables (
    import Bitwise

    prime = 929
    alpha = 3
    order = 928

    # Build the tables by iterating alpha^0, alpha^1, ..., alpha^(order-1).
    # Starting from val = 1 = alpha^0, each step multiplies by alpha (mod 929).
    # After `order` steps every non-zero element appears exactly once,
    # proving alpha = 3 is primitive for GF(929).
    {exp_rev, log_pairs, _} =
      Enum.reduce(0..(order - 1), {[], [], 1}, fn i, {exp_acc, log_acc, val} ->
        new_exp = [val | exp_acc]
        new_log = [{val, i} | log_acc]
        next_val = rem(val * alpha, prime)
        {new_exp, new_log, next_val}
      end)

    # Build exp tuple with wrap-around: gf_exp[928] = gf_exp[0] = 1.
    # This lets gf_mul compute ((log_a + log_b) mod 928) without clamping.
    exp_list = Enum.reverse(exp_rev) ++ [1]
    exp_tuple = List.to_tuple(exp_list)

    # Build log map.
    log_map = Map.new(log_pairs)

    %{exp: exp_tuple, log: log_map, prime: prime, order: order}
  )

  # Compile-time accessor for exp table.
  @doc false
  @compile {:inline, gf_exp: 1}
  defp gf_exp(i), do: elem(@gf_tables.exp, i)

  # Compile-time accessor for log table.
  @doc false
  @compile {:inline, gf_log: 1}
  defp gf_log(v), do: Map.fetch!(@gf_tables.log, v)

  @doc """
  Multiply two GF(929) field elements using log/antilog tables.

  For non-zero a, b:  a * b = alpha^{(log(a) + log(b)) mod 928}
  If either operand is 0, the product is 0 (zero absorbs multiplication).

  ## Examples

      iex> CodingAdventures.PDF417.gf_mul(3, 3)
      9

      iex> CodingAdventures.PDF417.gf_mul(0, 100)
      0

      iex> CodingAdventures.PDF417.gf_mul(1, 928)
      928
  """
  @compile {:inline, gf_mul: 2}
  def gf_mul(0, _b), do: 0
  def gf_mul(_a, 0), do: 0
  def gf_mul(a, b) do
    la = gf_log(a)
    lb = gf_log(b)
    gf_exp(rem(la + lb, @gf929_order))
  end

  @doc """
  Add two GF(929) field elements. Addition in this field is just integer
  addition modulo 929 — no XOR, unlike GF(2^8) fields.

  ## Examples

      iex> CodingAdventures.PDF417.gf_add(100, 900)
      71

      iex> CodingAdventures.PDF417.gf_add(0, 42)
      42
  """
  @compile {:inline, gf_add: 2}
  def gf_add(a, b), do: rem(a + b, @gf929_prime)

  # ============================================================================
  # Reed-Solomon generator polynomial
  # ============================================================================
  #
  # For ECC level L we need k = 2^(L+1) ECC codewords. The PDF417 generator
  # uses the b=3 convention from ISO/IEC 15438:
  #
  #   g(x) = (x - alpha^3)(x - alpha^4) ... (x - alpha^{k+2})
  #
  # Why b=3? The standard chose roots starting at alpha^3 rather than alpha^0
  # to gain implementation flexibility. The choice is arbitrary as long as
  # encoder and decoder agree.
  #
  # We build g iteratively by multiplying in each linear factor (x - alpha^j),
  # starting from g_0(x) = 1 and ending at g_k(x) of degree k.
  # The result is a big-endian list [g_k, g_{k-1}, ..., g_1, g_0] where g_k=1.

  @doc """
  Build the Reed-Solomon generator polynomial for the given ECC level.

  Returns `k+1` GF(929) coefficients in big-endian order (leading coefficient
  first, constant term last). Leading coefficient is always 1 (monic).

  `k = 2^(ecc_level+1)` is the number of ECC codewords.

  ## Examples

      iex> gen = CodingAdventures.PDF417.build_generator(0)
      iex> length(gen)
      3

      iex> hd(gen)
      1
  """
  def build_generator(ecc_level) do
    # k = 2^(ecc_level+1). We use Bitwise.bsl to keep integer arithmetic.
    k = bsl(1, ecc_level + 1)

    # Start with g(x) = 1 (constant polynomial).
    g_init = [1]

    # Multiply in each linear factor (x - alpha^j) for j = 3..k+2.
    Enum.reduce(3..(k + 2), g_init, fn j, g ->
      # alpha^j: use rem(j, order) to handle j >= 928 gracefully.
      root = gf_exp(rem(j, @gf929_order))
      # -alpha^j (mod 929): additive inverse in GF(929).
      neg_root = rem(@gf929_prime - root, @gf929_prime)

      # Multiply g(x) by (x + neg_root):
      #   new[i]   += g[i]          (coefficient of x)
      #   new[i+1] += g[i]*neg_root (constant term)
      n = length(g)
      new_g = List.duplicate(0, n + 1)

      {new_g, _} =
        Enum.reduce(0..(n - 1), {new_g, g}, fn i, {acc, [gi | rest]} ->
          acc = List.update_at(acc, i, &gf_add(&1, gi))
          acc = List.update_at(acc, i + 1, &gf_add(&1, gf_mul(gi, neg_root)))
          {acc, rest}
        end)

      new_g
    end)
  end

  # ============================================================================
  # Reed-Solomon encoder
  # ============================================================================
  #
  # Standard LFSR (linear feedback shift register) polynomial long-division.
  # No block interleaving — all data feeds a single RS encoder. This is simpler
  # than QR Code (which splits data into multiple blocks for burst-error
  # resilience). PDF417 instead relies on the row-cluster structure to spread
  # bursts across multiple codewords.
  #
  # For each input data codeword:
  #   feedback = (d + ecc[0]) mod 929
  #   shift register left by one
  #   for i = 0..k-1: ecc[i] += generator[k-i] * feedback   (mod 929)
  #
  # After processing all data, the register holds the k ECC codewords.
  #
  # The generator list is big-endian: g[0] = 1 (leading coeff, degree k).
  # Coefficient g[k-i] in the above is 0-indexed from the front of the list.

  @doc """
  Compute `k = 2^(ecc_level+1)` Reed-Solomon ECC codewords for `data` over
  GF(929) using the b=3 convention from ISO/IEC 15438.

  `data` is a list of integers 0..928 (the message codewords).
  Returns a list of k integers 0..928 (the ECC codewords).

  ## Examples

      iex> ecc = CodingAdventures.PDF417.rs_encode([3, 924, 72, 101, 108, 108, 111], 0)
      iex> length(ecc)
      2
  """
  def rs_encode(data_cws, ecc_level) do
    gen = build_generator(ecc_level)
    k = length(gen) - 1

    # Initialize the shift register with k zeros.
    ecc_init = List.duplicate(0, k)

    ecc_final =
      Enum.reduce(data_cws, ecc_init, fn d, ecc ->
        feedback = gf_add(d, hd(ecc))

        # Shift left: drop the first element, append a zero at the end.
        shifted = tl(ecc) ++ [0]

        if feedback == 0 do
          # No feedback contribution — just the shift.
          shifted
        else
          # Accumulate feedback * generator[k-i] into each cell.
          # gen is big-endian so gen[0]=1 (leading coeff). The coefficient
          # for position i in the register (after shifting) is gen[k-i].
          # We skip gen[0] (the leading 1) because it multiplies the shift
          # position that was already rotated off — we use gen[1..k] here.
          Enum.with_index(shifted)
          |> Enum.map(fn {r, i} ->
            # 0-indexed: position i in the shifted register uses gen[k - i].
            # gen is a list; Enum.at is fine for k up to 512.
            coeff = Enum.at(gen, k - i)
            gf_add(r, gf_mul(coeff, feedback))
          end)
        end
      end)

    ecc_final
  end

  # ============================================================================
  # Byte compaction
  # ============================================================================
  #
  # Byte compaction packs raw bytes by treating every group of 6 bytes as a
  # 48-bit big-endian integer and re-expressing it in base 900 (5 base-900
  # digits). The result fits in 5 codewords — 17% denser than encoding each
  # byte separately.
  #
  #   6 bytes (48 bits) -> integer N -> 5 base-900 digits -> 5 codewords
  #
  # Why does this work? 2^48 = 281,474,976,710,656 < 900^5 = 590,490,000,000,000
  # so every 48-bit value maps to exactly 5 base-900 digits with room to spare.
  #
  # Any leftover 1-5 bytes at the tail are emitted as one codeword each.
  # Byte values 0-255 fit trivially in codeword slots 0-928.
  #
  # We prefix the stream with the 924 latch so the decoder knows to switch into
  # byte compaction mode.
  #
  # Elixir integers are arbitrary precision, so 48-bit arithmetic is native.

  @doc """
  Encode a list of byte integers using PDF417 byte compaction mode.

  Returns `[924, c1, c2, ...]` where 924 is the byte-mode latch codeword and
  the remaining values are compacted codewords.

  6 input bytes become 5 output codewords via base-900 conversion.
  Remaining 1-5 bytes are emitted one-to-one.

  ## Examples

      iex> cws = CodingAdventures.PDF417.byte_compact([72, 101, 108, 108, 111, 44])
      iex> hd(cws)
      924
      iex> length(cws)
      6
  """
  def byte_compact(bytes) do
    # Start with the latch codeword.
    latch = [@latch_byte]
    compact_bytes(bytes, latch)
  end

  # Process 6-byte groups, then handle the tail.
  defp compact_bytes([], acc), do: Enum.reverse(acc)

  defp compact_bytes([b0, b1, b2, b3, b4, b5 | rest], acc) do
    # Pack 6 bytes into a 48-bit big-endian integer.
    v =
      b0 * 256 * 256 * 256 * 256 * 256 +
        b1 * 256 * 256 * 256 * 256 +
        b2 * 256 * 256 * 256 +
        b3 * 256 * 256 +
        b4 * 256 +
        b5

    # Express v in base 900, most-significant digit first.
    # 5 digits suffice because 900^5 > 2^48.
    {group, _} =
      Enum.reduce(1..5, {[], v}, fn _, {digits, remaining} ->
        {[rem(remaining, 900) | digits], div(remaining, 900)}
      end)

    # `group` is now [d4, d3, d2, d1, d0] (already big-endian because we
    # prepend each extracted digit — the last extracted is d0, which ends up
    # at the front of `group`, so we reverse to get big-endian order).
    # Wait — let's be precise:
    #   iteration 1: digit = rem(v, 900) = d0 (least significant); group = [d0]
    #   iteration 2: digit = rem(v//900, 900) = d1; group = [d1, d0]
    #   ...
    #   iteration 5: group = [d4, d3, d2, d1, d0]   <- big-endian already
    # We prepend each into the accumulator in reverse order, so Enum.reduce
    # with [digits | acc] gives us big-endian. Then we add these to the acc
    # in reverse — but acc is reversed at the end, so we need them in the
    # accumulator in the order d0, d1, d2, d3, d4 (most-sig last in acc
    # since acc is built reversed).
    #
    # Simplest correct approach: group is currently [d4, d3, d2, d1, d0]
    # (big-endian order). We want to push them into acc reversed so that
    # when we Enum.reverse(acc) at the end, d4 comes first.
    new_acc = Enum.reduce(group, acc, fn digit, a -> [digit | a] end)
    compact_bytes(rest, new_acc)
  end

  defp compact_bytes([byte | rest], acc) do
    # Tail bytes (fewer than 6 remaining): one codeword each, value = byte.
    compact_bytes(rest, [byte | acc])
  end

  # ============================================================================
  # Auto ECC level selection
  # ============================================================================
  #
  # These thresholds match the recommendation table from the PDF417 standard.
  # The idea is to pick a level whose ECC overhead is roughly proportional to
  # the data size: small symbols stay compact; large symbols still recover from
  # realistic damage (a scratch, a fold, a smear).
  #
  #   Level 0: 2   ECC codewords (k = 2)
  #   Level 1: 4   ECC codewords
  #   Level 2: 8   ECC codewords
  #   Level 3: 16  ECC codewords
  #   Level 4: 32  ECC codewords
  #   Level 5: 64  ECC codewords
  #   Level 6: 128 ECC codewords
  #   Level 7: 256 ECC codewords
  #   Level 8: 512 ECC codewords

  @doc """
  Auto-select the recommended ECC level based on data codeword count.

  ## Examples

      iex> CodingAdventures.PDF417.auto_ecc_level(10)
      2

      iex> CodingAdventures.PDF417.auto_ecc_level(100)
      3

      iex> CodingAdventures.PDF417.auto_ecc_level(500)
      5
  """
  def auto_ecc_level(data_count) do
    cond do
      data_count <= 40 -> 2
      data_count <= 160 -> 3
      data_count <= 320 -> 4
      data_count <= 863 -> 5
      true -> 6
    end
  end

  # ============================================================================
  # Dimension selection
  # ============================================================================
  #
  # Heuristic: aim for a roughly square *visual* symbol.
  #
  # Each PDF417 codeword is 17 modules wide. With the default row_height=3,
  # each logical row is 3 modules tall. So "square" in visual terms means
  # the total width ≈ total height:
  #
  #   module_width  = 69 + 17*cols
  #   module_height = rows * row_height
  #
  # Ignoring the fixed overhead, width/height ≈ (17*cols) / (rows * 3).
  # For a 1:1 ratio: rows ≈ (17/3) * cols ≈ 5.67 * cols.
  # Equivalently: cols ≈ rows / 5.67 ≈ sqrt(total / 5.67).
  # We use the approximation c = ceil(sqrt(total / 3)), which trades a slightly
  # wider-than-tall symbol for a simpler formula.

  @doc """
  Choose symbol dimensions (cols, rows) for a roughly square symbol holding
  `total` codewords.

  Returns `{cols, rows}` with cols in 1..30 and rows in 3..90.

  ## Examples

      iex> {cols, rows} = CodingAdventures.PDF417.choose_dimensions(20)
      iex> cols >= 1 and cols <= 30
      true
      iex> rows >= 3 and rows <= 90
      true
  """
  def choose_dimensions(total) do
    c = max(@min_cols, min(@max_cols, ceil(:math.sqrt(total / 3))))
    r = max(@min_rows, ceil_div(total, c))

    # If rows came out below minimum, recompute cols for 3-row minimum.
    {c, r} =
      if r < @min_rows do
        r2 = @min_rows
        c2 = max(@min_cols, min(@max_cols, ceil_div(total, r2)))
        r3 = max(@min_rows, ceil_div(total, c2))
        {c2, r3}
      else
        {c, r}
      end

    r = min(@max_rows, r)
    {c, r}
  end

  # Integer ceiling division: ceil(a / b) for positive integers.
  @compile {:inline, ceil_div: 2}
  defp ceil_div(a, b), do: div(a + b - 1, b)

  # ============================================================================
  # Row indicator computation
  # ============================================================================
  #
  # Each PDF417 row carries two row-indicator codewords, one on each side of
  # the data. Together LRI (Left Row Indicator) and RRI (Right Row Indicator)
  # encode the symbol's overall shape so the decoder can recover R, C, and the
  # ECC level even when only a partial row is read.
  #
  # Three derived quantities from the symbol geometry:
  #
  #   R_info = (R - 1) div 3       -- encodes total row count (R)
  #   C_info = C - 1               -- encodes column count (C)
  #   L_info = 3*L + (R - 1) rem 3 -- encodes ECC level + row-count parity
  #
  # The cluster of row r is r rem 3 (0, 1, or 2). Different clusters carry
  # different metadata so that any three consecutive rows together encode the
  # complete symbol description:
  #
  #   Cluster 0: LRI = 30*rowGroup + R_info,  RRI = 30*rowGroup + C_info
  #   Cluster 1: LRI = 30*rowGroup + L_info,  RRI = 30*rowGroup + R_info
  #   Cluster 2: LRI = 30*rowGroup + C_info,  RRI = 30*rowGroup + L_info
  #
  # where rowGroup = r div 3.
  #
  # The "30 * rowGroup" prefix tells the decoder which group of 3 rows it is
  # reading, so it can reconstruct the absolute row index from any one indicator.

  @doc """
  Compute the Left Row Indicator codeword for row `r` (0-indexed).

  Parameters:
  - `r`         — row index (0-indexed)
  - `rows`      — total number of logical PDF417 rows in the symbol
  - `cols`      — total number of data columns in the symbol
  - `ecc_level` — Reed-Solomon ECC level (0..8)

  ## Examples

      iex> CodingAdventures.PDF417.compute_lri(0, 9, 3, 2)
      2
  """
  def compute_lri(r, rows, cols, ecc_level) do
    r_info = div(rows - 1, 3)
    c_info = cols - 1
    l_info = 3 * ecc_level + rem(rows - 1, 3)
    row_group = div(r, 3)
    cluster = rem(r, 3)

    case cluster do
      0 -> 30 * row_group + r_info
      1 -> 30 * row_group + l_info
      _ -> 30 * row_group + c_info
    end
  end

  @doc """
  Compute the Right Row Indicator codeword for row `r` (0-indexed).

  Parameters mirror `compute_lri/4`.

  ## Examples

      iex> CodingAdventures.PDF417.compute_rri(0, 9, 3, 2)
      2
  """
  def compute_rri(r, rows, cols, ecc_level) do
    r_info = div(rows - 1, 3)
    c_info = cols - 1
    l_info = 3 * ecc_level + rem(rows - 1, 3)
    row_group = div(r, 3)
    cluster = rem(r, 3)

    case cluster do
      0 -> 30 * row_group + c_info
      1 -> 30 * row_group + r_info
      _ -> 30 * row_group + l_info
    end
  end

  # ============================================================================
  # Pattern expansion: width list -> boolean module list
  # ============================================================================
  #
  # Each codeword in the cluster tables is stored as a packed 32-bit integer
  # with 4 bits per bar/space width (8 widths total, alternating bar/space):
  #
  #   bits 31..28 = b1, bits 27..24 = s1, bits 23..20 = b2, bits 19..16 = s2,
  #   bits 15..12 = b3, bits 11..8  = s3, bits 7..4   = b4, bits 3..0   = s4
  #
  # We expand these widths into a list of booleans: true = dark (bar),
  # false = light (space). The 8 widths always sum to 17 — the defining
  # geometric invariant of a PDF417 codeword.
  #
  # The start and stop patterns are stored as plain width lists (not packed),
  # so we provide expand_widths/1 for those too.

  @doc """
  Expand a list of bar/space widths into a flat list of boolean module values.

  The first width is always a bar (dark = true), then alternating.
  Used for the start (8 widths, 17 modules) and stop (9 widths, 18 modules)
  patterns.

  ## Examples

      iex> CodingAdventures.PDF417.expand_widths([2, 1, 1])
      [true, true, false, true]
  """
  def expand_widths(widths) do
    {modules, _} =
      Enum.reduce(widths, {[], true}, fn w, {acc, dark} ->
        repeated = List.duplicate(dark, w)
        {acc ++ repeated, not dark}
      end)

    modules
  end

  # Expand a packed 32-bit codeword into 17 boolean modules.
  # Called only internally during rasterization; not exported.
  defp expand_packed(packed) do
    widths = [
      band(bsr(packed, 28), 0xF),
      band(bsr(packed, 24), 0xF),
      band(bsr(packed, 20), 0xF),
      band(bsr(packed, 16), 0xF),
      band(bsr(packed, 12), 0xF),
      band(bsr(packed, 8), 0xF),
      band(bsr(packed, 4), 0xF),
      band(packed, 0xF)
    ]

    expand_widths(widths)
  end

  # ============================================================================
  # Rasterization: codeword sequence -> ModuleGrid
  # ============================================================================
  #
  # Each PDF417 row has this layout:
  #
  #   [start 17] [LRI 17] [data * cols, 17 each] [RRI 17] [stop 18]
  #
  # Total module width = 17 + 17 + 17*cols + 17 + 18 = 69 + 17*cols.
  #
  # Vertically, each logical row is repeated `row_height` times to give the
  # symbol optical thickness. The default row_height=3 is the PDF417 standard
  # recommended minimum — it gives a laser scanner enough vertical range to
  # integrate over even at a skewed angle.

  @doc """
  Convert a flat codeword sequence into a `%ModuleGrid{}`.

  Parameters:
  - `sequence`   — flat list of codewords (data + padding + ECC) of length rows*cols + ecc_count
  - `rows`       — number of logical PDF417 rows (3..90)
  - `cols`       — number of data columns (1..30)
  - `ecc_level`  — ECC level (0..8), needed for row indicator computation
  - `row_height` — number of module rows per logical PDF417 row (>= 1)

  Returns `%ModuleGrid{rows: total_module_rows, cols: total_module_cols, modules: [[bool]]}`.
  """
  def rasterize(sequence, rows, cols, ecc_level, row_height) do
    module_width = 69 + 17 * cols
    module_height = rows * row_height

    # Pre-expand start and stop patterns (identical for every logical row).
    start_modules = expand_widths(ClusterTables.start_pattern())
    stop_modules = expand_widths(ClusterTables.stop_pattern())

    # All three cluster sub-tables.
    {cluster0, cluster1, cluster2} = ClusterTables.cluster_tables()

    # Build each logical row's module sequence, then repeat it row_height times.
    # We collect all physical rows in order, which is just a flat list of lists.
    all_module_rows =
      Enum.flat_map(0..(rows - 1), fn r ->
        cluster = rem(r, 3)

        cluster_table =
          case cluster do
            0 -> cluster0
            1 -> cluster1
            _ -> cluster2
          end

        # Compute this row's module list.
        lri = compute_lri(r, rows, cols, ecc_level)
        rri = compute_rri(r, rows, cols, ecc_level)

        # Look up codeword packed values from the cluster table.
        lri_modules = expand_packed(elem(cluster_table, lri))

        data_modules =
          Enum.flat_map(0..(cols - 1), fn j ->
            # sequence is 0-indexed: row r uses positions r*cols..(r*cols + cols - 1)
            cw = Enum.at(sequence, r * cols + j)
            expand_packed(elem(cluster_table, cw))
          end)

        rri_modules = expand_packed(elem(cluster_table, rri))

        row_modules = start_modules ++ lri_modules ++ data_modules ++ rri_modules ++ stop_modules

        # Sanity check: every row must be exactly module_width modules wide.
        if length(row_modules) != module_width do
          raise "PDF417 internal error: row #{r} has #{length(row_modules)} modules, expected #{module_width}"
        end

        # Repeat this 1D row row_height times into the physical grid.
        List.duplicate(row_modules, row_height)
      end)

    %ModuleGrid{
      rows: module_height,
      cols: module_width,
      modules: all_module_rows
    }
  end

  # ============================================================================
  # Public API: encode/1 and encode/2
  # ============================================================================

  @doc """
  Encode `data` as a PDF417 barcode symbol.

  `data` may be a binary string or a list of byte integers (0..255).

  Returns `{:ok, %ModuleGrid{}}` on success or `{:error, reason}` on failure.

  ## Options

  - `ecc_level: 0..8 | :auto` — Reed-Solomon ECC level. Default `:auto`.
    Higher levels use more ECC codewords, increasing resilience to damage at
    the cost of a larger symbol. Level 0 = 2 ECC codewords; level 8 = 512.
  - `columns: 1..30 | :auto` — Number of data columns. Default `:auto` (roughly square).
  - `row_height: pos_integer()` — Module rows per logical PDF417 row. Default `3`.

  ## Examples

      iex> {:ok, grid} = CodingAdventures.PDF417.encode("Hello")
      iex> is_struct(grid, CodingAdventures.PDF417.ModuleGrid)
      true

      iex> {:ok, grid} = CodingAdventures.PDF417.encode("Hi", ecc_level: 2)
      iex> grid.rows >= 3
      true

      iex> {:error, :invalid_ecc_level} = CodingAdventures.PDF417.encode("Hi", ecc_level: 9)
      {:error, :invalid_ecc_level}

      iex> {:error, :invalid_columns} = CodingAdventures.PDF417.encode("Hi", columns: 31)
      {:error, :invalid_columns}
  """
  def encode(data, opts \\ []) do
    with {:ok, bytes} <- to_byte_list(data),
         {:ok, ecc_level} <- validate_ecc_level(Keyword.get(opts, :ecc_level, :auto)),
         {:ok, columns_opt} <- validate_columns(Keyword.get(opts, :columns, :auto)),
         {:ok, row_height} <- validate_row_height(Keyword.get(opts, :row_height, 3)) do
      # Step 1: Byte compaction — convert raw bytes to codeword stream.
      data_cws = byte_compact(bytes)

      # Step 2: Auto-select ECC level if not specified.
      # The "+1" accounts for the length descriptor we prepend next.
      resolved_ecc_level =
        if ecc_level == :auto do
          auto_ecc_level(length(data_cws) + 1)
        else
          ecc_level
        end

      ecc_count = bsl(1, resolved_ecc_level + 1)

      # Step 3: Length descriptor.
      # The first codeword of every PDF417 symbol is a count: it equals
      # 1 (itself) + data_cw_count + ecc_count. Decoders use this to find the
      # boundary between data, padding, and ECC.
      length_desc = 1 + length(data_cws) + ecc_count

      # Full data sequence for RS encoding: [length_desc | data_cws].
      full_data = [length_desc | data_cws]

      # Step 4: Reed-Solomon ECC.
      ecc_cws = rs_encode(full_data, resolved_ecc_level)

      # Step 5: Choose symbol dimensions.
      total_cws = length(full_data) + length(ecc_cws)

      with {:ok, {cols, rows}} <- resolve_dimensions(columns_opt, total_cws) do
        # Step 6: Pad the data area to fill the grid exactly.
        # Padding codeword 900 is the "text mode latch" — harmless when the
        # decoder is already in byte mode, so it serves as a neutral filler.
        padding_count = cols * rows - total_cws
        padded_data = full_data ++ List.duplicate(@padding_cw, padding_count)

        # Final codeword sequence: padded data then ECC.
        sequence = padded_data ++ ecc_cws

        # Step 7: Rasterize.
        grid = rasterize(sequence, rows, cols, resolved_ecc_level, row_height)
        {:ok, grid}
      end
    end
  end

  # ============================================================================
  # Input conversion and validation helpers
  # ============================================================================

  # Convert input data to a list of byte integers 0..255.
  defp to_byte_list(data) when is_binary(data) do
    {:ok, :binary.bin_to_list(data)}
  end

  defp to_byte_list(data) when is_list(data) do
    # Validate that all elements are integers in 0..255.
    if Enum.all?(data, fn v -> is_integer(v) and v >= 0 and v <= 255 end) do
      {:ok, data}
    else
      {:error, :invalid_data}
    end
  end

  defp to_byte_list(_data), do: {:error, :invalid_data}

  # Validate the ecc_level option.
  defp validate_ecc_level(:auto), do: {:ok, :auto}
  defp validate_ecc_level(l) when is_integer(l) and l >= 0 and l <= 8, do: {:ok, l}
  defp validate_ecc_level(_), do: {:error, :invalid_ecc_level}

  # Validate the columns option.
  defp validate_columns(:auto), do: {:ok, :auto}
  defp validate_columns(c) when is_integer(c) and c >= @min_cols and c <= @max_cols, do: {:ok, c}
  defp validate_columns(_), do: {:error, :invalid_columns}

  # Validate the row_height option.
  defp validate_row_height(h) when is_integer(h) and h >= 1, do: {:ok, h}
  defp validate_row_height(_), do: {:ok, 3}

  # Resolve columns to {cols, rows}, checking capacity constraints.
  defp resolve_dimensions(:auto, total_cws) do
    {cols, rows} = choose_dimensions(total_cws)

    if cols * rows < total_cws do
      {:error, :input_too_long}
    else
      {:ok, {cols, rows}}
    end
  end

  defp resolve_dimensions(cols, total_cws) when is_integer(cols) do
    rows = max(@min_rows, ceil_div(total_cws, cols))

    cond do
      rows > @max_rows ->
        {:error, :input_too_long}

      cols * rows < total_cws ->
        {:error, :input_too_long}

      true ->
        {:ok, {cols, rows}}
    end
  end
end
