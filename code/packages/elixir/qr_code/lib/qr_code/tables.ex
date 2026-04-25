defmodule CodingAdventures.QrCode.Tables do
  @moduledoc """
  ISO/IEC 18004:2015 lookup tables for QR Code encoding.

  All data in this module is constant — sourced directly from the ISO standard
  and never computed at runtime. Hard-coding these tables is both faster and
  more reliable than deriving them algorithmically.

  ## What lives here

  - **ECC codewords per block** — how many Reed-Solomon check bytes each block gets.
  - **Number of blocks** — how the message is split for burst-error resilience.
  - **Alignment pattern positions** — where the 5×5 alignment squares go (versions 2–40).
  - **Remainder bits** — zero-padding appended after interleaved codewords.
  - **Capacity table** — total data codewords per (version, ECC level).

  ## Why split into blocks?

  A burst error (e.g., a scratch across a QR code) destroys a contiguous
  region of modules. If the entire message were one RS block, a burst long
  enough to exceed the correction capacity of that block would be unrecoverable.

  By splitting the message into multiple shorter blocks, a burst error can
  only wipe out a fraction of each block. As long as each block's error count
  stays within its correction budget, the entire message is recovered.

  ## ECC level index convention

  Throughout all tables, ECC levels map to 0-based integer indices:

      :L → 0   (Low,      ~7% recovery)
      :M → 1   (Medium,  ~15% recovery)
      :Q → 2   (Quartile,~25% recovery)
      :H → 3   (High,    ~30% recovery)

  This matches the order in which the tables are laid out in ISO 18004 Table 9.
  """

  import Bitwise

  # ---------------------------------------------------------------------------
  # ECC level → integer index mapping
  # ---------------------------------------------------------------------------

  @doc """
  Convert an ECC level atom to its 0-based table index.

      :L → 0, :M → 1, :Q → 2, :H → 3

  Used by every table lookup that needs to select a row.
  """
  @spec ecc_index(atom()) :: 0..3
  def ecc_index(:L), do: 0
  def ecc_index(:l), do: 0
  def ecc_index(:M), do: 1
  def ecc_index(:m), do: 1
  def ecc_index(:Q), do: 2
  def ecc_index(:q), do: 2
  def ecc_index(:H), do: 3
  def ecc_index(:h), do: 3

  @doc """
  Return the 2-bit ECC level indicator placed in format information.

  ISO 18004 §7.9: the bits are deliberately not alphabetical:

      :L → 0b01 = 1
      :M → 0b00 = 0
      :Q → 0b11 = 3
      :H → 0b10 = 2

  These two bits go into the upper two bits of the 5-bit format data field.
  The XOR mask (0x5412) ensures the format information is never all-zero.
  """
  @spec ecc_indicator(atom()) :: 0..3
  def ecc_indicator(:L), do: 0b01
  def ecc_indicator(:l), do: 0b01
  def ecc_indicator(:M), do: 0b00
  def ecc_indicator(:m), do: 0b00
  def ecc_indicator(:Q), do: 0b11
  def ecc_indicator(:q), do: 0b11
  def ecc_indicator(:H), do: 0b10
  def ecc_indicator(:h), do: 0b10

  # ---------------------------------------------------------------------------
  # ECC codewords per block — ISO 18004:2015 Table 9
  # ---------------------------------------------------------------------------
  #
  # Each block gets this many Reed-Solomon check codewords.
  # Indexed [ecc_idx][version]. Index 0 is a placeholder (-1).
  #
  # Higher ECC level → more check bytes per block → more redundancy → less data.

  @ecc_codewords_per_block [
    # L:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    # M:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28],
    # Q:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    # H:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30]
  ]

  @doc """
  ECC codewords per block for the given ECC level and version.

  Example: `ecc_codewords_per_block(:M, 5)` returns 24, meaning each RS block
  for version 5 at ECC level M carries 24 Reed-Solomon check bytes.
  """
  @spec ecc_codewords_per_block(atom(), pos_integer()) :: pos_integer()
  def ecc_codewords_per_block(ecc, version) do
    @ecc_codewords_per_block
    |> Enum.at(ecc_index(ecc))
    |> Enum.at(version)
  end

  # ---------------------------------------------------------------------------
  # Number of ECC blocks — ISO 18004:2015 Table 9
  # ---------------------------------------------------------------------------
  #
  # Total number of RS blocks for this version/ECC combination.
  # More blocks → shorter blocks → better burst-error resilience.
  # But each block carries the same ecc_codewords_per_block overhead,
  # so more blocks also means proportionally fewer data codewords.

  @num_blocks [
    # L:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  1,  1,  1,  1,  1,  2,  2,  2,  2,  4,  4,  4,  4,  4,  6,  6,  6,  6,  7,  8,  8,  9,  9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25],
    # M:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  1,  1,  1,  2,  2,  4,  4,  4,  5,  5,  5,  8,  9,  9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49],
    # Q:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  1,  1,  2,  2,  4,  4,  6,  6,  8,  8,  8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68],
    # H:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  1,  1,  2,  4,  4,  4,  5,  6,  8,  8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 80]
  ]

  @doc """
  Total number of RS blocks for the given ECC level and version.

  More blocks means shorter individual blocks. Shorter blocks are faster to
  decode and better at surviving burst errors (a contiguous smear only hits
  each block once), at the cost of more total ECC overhead.
  """
  @spec num_blocks(atom(), pos_integer()) :: pos_integer()
  def num_blocks(ecc, version) do
    @num_blocks
    |> Enum.at(ecc_index(ecc))
    |> Enum.at(version)
  end

  # ---------------------------------------------------------------------------
  # Alignment pattern center coordinates — ISO 18004:2015 Annex E
  # ---------------------------------------------------------------------------
  #
  # Each list gives the set of row/column positions for alignment pattern centers.
  # The actual patterns are placed at ALL pairwise combinations of these values,
  # except any combination where the center falls on an already-reserved module
  # (finder pattern, timing strip, etc.).
  #
  # Version 1 has no alignment patterns.
  # Version 2 has one alignment pattern at the cross-product of [6, 18].
  # The three finder-overlap positions are automatically excluded by the
  # reserved-module check in the placer.

  @alignment_positions [
    [],                               # v1  — none
    [6, 18],                          # v2
    [6, 22],                          # v3
    [6, 26],                          # v4
    [6, 30],                          # v5
    [6, 34],                          # v6
    [6, 22, 38],                      # v7
    [6, 24, 42],                      # v8
    [6, 26, 46],                      # v9
    [6, 28, 50],                      # v10
    [6, 30, 54],                      # v11
    [6, 32, 58],                      # v12
    [6, 34, 62],                      # v13
    [6, 26, 46, 66],                  # v14
    [6, 26, 48, 70],                  # v15
    [6, 26, 50, 74],                  # v16
    [6, 30, 54, 78],                  # v17
    [6, 30, 56, 82],                  # v18
    [6, 30, 58, 86],                  # v19
    [6, 34, 62, 90],                  # v20
    [6, 28, 50, 72, 94],              # v21
    [6, 26, 50, 74, 98],              # v22
    [6, 30, 54, 78, 102],             # v23
    [6, 28, 54, 80, 106],             # v24
    [6, 32, 58, 84, 110],             # v25
    [6, 30, 58, 86, 114],             # v26
    [6, 34, 62, 90, 118],             # v27
    [6, 26, 50, 74, 98, 122],         # v28
    [6, 30, 54, 78, 102, 126],        # v29
    [6, 26, 52, 78, 104, 130],        # v30
    [6, 30, 56, 82, 108, 134],        # v31
    [6, 34, 60, 86, 112, 138],        # v32
    [6, 30, 58, 86, 114, 142],        # v33
    [6, 34, 62, 90, 118, 146],        # v34
    [6, 30, 54, 78, 102, 126, 150],   # v35
    [6, 24, 50, 76, 102, 128, 154],   # v36
    [6, 28, 54, 80, 106, 132, 158],   # v37
    [6, 32, 58, 84, 110, 136, 162],   # v38
    [6, 26, 54, 82, 110, 138, 166],   # v39
    [6, 30, 58, 86, 114, 142, 170]    # v40
  ]

  @doc """
  Alignment pattern center coordinates for a given version (1–40).

  Returns a list of integers. The actual pattern centers are at all pairwise
  `{row, col}` combinations of the returned values, minus any that would
  overlap with finder patterns or timing strips.

  Returns `[]` for version 1 (no alignment patterns).
  """
  @spec alignment_positions(pos_integer()) :: [non_neg_integer()]
  def alignment_positions(version) when version in 1..40 do
    Enum.at(@alignment_positions, version - 1)
  end

  # ---------------------------------------------------------------------------
  # Remainder bits — ISO 18004:2015 Table 1
  # ---------------------------------------------------------------------------
  #
  # After all interleaved codewords are placed into the grid, a few trailing
  # zero-bit modules may be needed to fill the symbol exactly.
  # This arises because the grid's raw module count is not always a multiple
  # of 8 bits (one codeword).
  #
  # Most versions need 0 remainder bits. Versions 2–6 need 7, etc.
  # See ISO 18004:2015 Table 1.

  @remainder_bits [
    # v: 1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20
         0, 7, 7, 7, 7, 7, 0, 0, 0, 0, 0, 0, 0, 3, 3, 3, 3, 3, 3, 3,
    # v:21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40
         4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 0, 0, 0, 0, 0, 0
  ]

  @doc """
  Number of remainder bits to append after the interleaved codewords.

  These are always zero-value bits. They are needed because the total number
  of modules in the data area may not be an exact multiple of 8.
  """
  @spec remainder_bits(pos_integer()) :: 0..7
  def remainder_bits(version) when version in 1..40 do
    Enum.at(@remainder_bits, version - 1)
  end

  # ---------------------------------------------------------------------------
  # Raw data modules — derived formula (Nayuki)
  # ---------------------------------------------------------------------------

  @doc """
  Total number of raw data + ECC module bits in the symbol.

  All modules minus all structural modules (finder, separator, timing,
  alignment, format, version, dark module).

  Formula from Nayuki's reference implementation.
  """
  @spec num_raw_data_modules(pos_integer()) :: pos_integer()
  def num_raw_data_modules(version) do
    result = (16 * version + 128) * version + 64

    result =
      if version >= 2 do
        num_align = div(version, 7) + 2
        result - (25 * num_align - 10) * num_align + 55
      else
        result
      end

    if version >= 7, do: result - 36, else: result
  end

  @doc """
  Total data codewords (message + padding, no ECC) for version and ECC level.

  This is the total capacity measured in bytes of user data (plus padding)
  that can be stored in the symbol. ECC bytes are *not* counted here.
  """
  @spec num_data_codewords(pos_integer(), atom()) :: pos_integer()
  def num_data_codewords(version, ecc) do
    div(num_raw_data_modules(version), 8) -
      num_blocks(ecc, version) * ecc_codewords_per_block(ecc, version)
  end

  # ---------------------------------------------------------------------------
  # RS generator polynomials — precomputed for QR (b=0 convention)
  # ---------------------------------------------------------------------------
  #
  # QR Code RS uses the b=0 convention: roots are α^0, α^1, ..., α^{n-1}.
  # The generator polynomial for n ECC codewords is:
  #
  #   g(x) = (x + α^0)(x + α^1)···(x + α^{n-1})
  #
  # This is DIFFERENT from the b=1 convention (MA02 reed-solomon package),
  # which starts from α^1. The b=0 convention shifts the generator by one step.
  #
  # These polynomials are precomputed here using GF(256) at module load time.
  # The generator for n codewords has degree n and n+1 coefficients.
  # Coefficients are stored in order [g0, g1, ..., gn] where g0 is the leading
  # (degree-n) coefficient (always 1 for monic polynomials).
  #
  # The QR RS encoding algorithm uses the LFSR (shift register) division:
  #
  #   remainder = [0] * n_ecc
  #   for each data byte b:
  #     feedback = b XOR remainder[0]
  #     remainder = [remainder[1], ..., remainder[n-1], 0]
  #     for i in 0..n-1:
  #       remainder[i] ^= gf_mul(generator[i+1], feedback)
  #
  # The ECC codewords are the final remainder.

  # Build the monic generator polynomial of degree n over GF(256), b=0 convention.
  # Returns a list [g0, g1, ..., gn] of n+1 coefficients, g0 = 1 (monic).
  #
  # Start with [1], then for each i in 0..n-1 multiply by (x + α^i):
  #   old poly g of degree k → new poly h of degree k+1 where
  #   h[j] = g[j-1] XOR (α^i * g[j])
  defp build_generator(n_ecc) do
    alog = CodingAdventures.GF256.alog_table()

    Enum.reduce(0..(n_ecc - 1), [1], fn i, g ->
      ai = Enum.at(alog, i)
      # Multiply current poly g (degree k) by (x + ai):
      # h has degree k+1 = length k+2
      old_len = length(g)
      next = List.duplicate(0, old_len + 1)

      {next, _} =
        Enum.reduce(0..(old_len - 1), {next, g}, fn j, {acc_next, _glist} ->
          gj = Enum.at(g, j)
          # Add g[j] * x^(k+1-j) term from shift
          acc_next = List.update_at(acc_next, j, &bxor(&1, gj))
          # Add g[j] * ai * x^(k-j) term from multiply by ai
          acc_next = List.update_at(acc_next, j + 1, &bxor(&1, CodingAdventures.GF256.multiply(gj, ai)))
          {acc_next, g}
        end)

      next
    end)
  end

  # All ECC lengths used by QR Code versions 1–40.
  # Precomputed at module load time (compile time in practice).
  @qr_ecc_lengths [7, 10, 13, 15, 16, 17, 18, 20, 22, 24, 26, 28, 30]

  # Generators are recomputed on each call. They are fast to compute (only
  # a handful of GF multiplications) and the results are small (at most 31
  # bytes). Caching via persistent_term is a premature optimization here.

  @doc """
  Return the RS generator polynomial for `n_ecc` error-correction codewords.

  Returns a list of `n_ecc + 1` GF(256) coefficients, MSB-first (leading
  coefficient is 1, the monic polynomial convention).

  These are the b=0 QR Code generators — roots are α^0, α^1, ..., α^{n-1}.
  """
  @spec generator(pos_integer()) :: [byte()]
  def generator(n_ecc) when n_ecc in @qr_ecc_lengths do
    build_generator(n_ecc)
  end

  def generator(n_ecc) do
    build_generator(n_ecc)
  end
end
