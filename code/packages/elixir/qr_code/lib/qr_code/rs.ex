defmodule CodingAdventures.QrCode.RS do
  @moduledoc """
  Reed-Solomon error correction for QR Code (b=0 convention).

  ## Why does QR Code need Reed-Solomon?

  QR Codes are often printed on paper, displayed on screens, or affixed to
  products in hostile environments. They can be:
    - Partially obscured by dirt, stickers, or damage
    - Partially destroyed by wear
    - Printed with low contrast

  Reed-Solomon error correction allows scanners to recover the original data
  even when up to T codewords are corrupt, where T depends on the ECC level:
    - L: recovers ~7% of codewords
    - M: recovers ~15% of codewords
    - Q: recovers ~25% of codewords
    - H: recovers ~30% of codewords

  ## The math

  QR Code uses Reed-Solomon over GF(256) — the Galois Field with 256 elements
  (integers 0–255) and arithmetic modulo the irreducible polynomial
  `x^8 + x^4 + x^3 + x^2 + 1 = 0x11D`.

  The encoder computes a **remainder polynomial** R(x):

      D(x) is the data polynomial of degree k-1 (k = data codewords).
      G(x) is the generator of degree n (n = ECC codewords).
      R(x) = D(x) * x^n mod G(x)

  This is pure polynomial division over GF(256).

  ## The b=0 vs b=1 distinction

  Most Reed-Solomon implementations (including MA02 in this repo) use the
  **b=1 convention**: the generator polynomial's roots are α^1, α^2, ..., α^n.
  ISO/IEC 18004 specifies the **b=0 convention** for QR Code: roots are
  α^0, α^1, ..., α^{n-1}.

  Concretely, for n=7 ECC codewords:
    b=1:  g(x) = (x+α)(x+α²)(x+α³)(x+α⁴)(x+α⁵)(x+α⁶)(x+α⁷)
    b=0:  g(x) = (x+α⁰)(x+α¹)(x+α²)(x+α³)(x+α⁴)(x+α⁵)(x+α⁶)

  The ECC bytes produced by each convention are different, and a QR Code
  decoder expects b=0 bytes. Using MA02's b=1 encoder would produce wrong ECC.

  ## Block structure

  For versions with multiple blocks, the data codewords are split into groups:
    - Group 1: `g1_count` blocks of `short_len` data codewords each.
    - Group 2: `g2_count` blocks of `short_len + 1` data codewords each.

  Each block is RS-encoded independently. The blocks are then interleaved:
  round-robin data bytes, then round-robin ECC bytes. This spreads a burst
  error across all blocks so each block only loses a small fraction.

  ## LFSR shift-register encoder

  The standard polynomial-division approach can be implemented with a linear
  feedback shift register (LFSR) — an array of n registers that get updated
  for each incoming data byte:

      ecc = [0] * n
      for each data byte b:
        feedback = b XOR ecc[0]
        shift ecc left: ecc[i] = ecc[i+1] for i in 0..n-2
        ecc[n-1] = 0
        for i in 0..n-1:
          ecc[i] ^= gf_mul(generator[i+1], feedback)

  The result is the ECC codeword sequence (the polynomial remainder).
  """

  import Bitwise
  alias CodingAdventures.QrCode.Tables
  alias CodingAdventures.GF256

  # ---------------------------------------------------------------------------
  # RS encoding — LFSR division
  # ---------------------------------------------------------------------------

  @doc """
  Compute the Reed-Solomon ECC bytes for one block of data.

  `data` is a list of byte-integers (the data codewords for this block).
  `generator` is the generator polynomial (list of n+1 GF(256) values,
  as returned by `Tables.generator/1`).

  Returns a list of `n_ecc` byte-integers (the ECC codewords for this block).

  ## Algorithm

  We use LFSR-based polynomial division over GF(256):

      ecc = [0] * n_ecc
      for each data byte b:
        feedback = b XOR ecc[0]
        ecc = [ecc[1], ..., ecc[n-1], 0]   (shift left)
        for i in 0..n-1:
          ecc[i] ^= generator[i+1] * feedback   (GF multiply)

  This is equivalent to computing `D(x) * x^n mod G(x)`.
  """
  @spec encode_block([byte()], [byte()]) :: [byte()]
  def encode_block(data, generator) do
    n_ecc = length(generator) - 1
    initial_ecc = List.duplicate(0, n_ecc)

    ecc =
      Enum.reduce(data, initial_ecc, fn b, ecc ->
        feedback = bxor(b, hd(ecc))
        # Shift left: drop first element, append 0.
        shifted = tl(ecc) ++ [0]

        if feedback == 0 do
          shifted
        else
          # XOR each register with generator[i+1] * feedback.
          # generator is [g0, g1, ..., gn], we use g1..gn.
          gen_tail = tl(generator)

          Enum.zip(shifted, gen_tail)
          |> Enum.map(fn {reg, gk} ->
            bxor(reg, GF256.multiply(gk, feedback))
          end)
        end
      end)

    ecc
  end

  # ---------------------------------------------------------------------------
  # Block splitting and ECC computation
  # ---------------------------------------------------------------------------

  @doc """
  Split data codewords into blocks and compute RS ECC for each block.

  Returns a list of `{data_block, ecc_block}` tuples, one per block, in order.

  ## Block structure

  Given `total_blocks` blocks and `total_data` codewords:
    - `short_len = total_data div total_blocks`
    - `num_long = total_data rem total_blocks`  (blocks with one extra codeword)

  Group 1: `total_blocks - num_long` blocks of `short_len` each.
  Group 2: `num_long` blocks of `short_len + 1` each.

  This matches the ISO 18004 block interleaving specification.
  """
  @spec compute_blocks([byte()], pos_integer(), atom()) :: [{[byte()], [byte()]}]
  def compute_blocks(data, version, ecc) do
    total_blocks = Tables.num_blocks(ecc, version)
    ecc_len = Tables.ecc_codewords_per_block(ecc, version)
    total_data = Tables.num_data_codewords(version, ecc)
    short_len = div(total_data, total_blocks)
    num_long = rem(total_data, total_blocks)
    generator = Tables.generator(ecc_len)

    {blocks, _} =
      Enum.reduce(0..(total_blocks - 1), {[], data}, fn block_idx, {blocks_acc, remaining} ->
        # Blocks 0..(g1_count-1) are "short" (short_len data codewords).
        # Blocks g1_count..(total_blocks-1) are "long" (short_len+1 each).
        g1_count = total_blocks - num_long
        block_len = if block_idx >= g1_count, do: short_len + 1, else: short_len

        {block_data, rest} = Enum.split(remaining, block_len)
        block_ecc = encode_block(block_data, generator)
        {blocks_acc ++ [{block_data, block_ecc}], rest}
      end)

    blocks
  end

  # ---------------------------------------------------------------------------
  # Interleaving
  # ---------------------------------------------------------------------------

  @doc """
  Interleave codewords from all blocks into a single flat list.

  The interleaving rule:
  1. Round-robin data codewords: byte 0 from block 0, byte 0 from block 1, ...
     then byte 1 from block 0, byte 1 from block 1, ..., etc.
  2. Round-robin ECC codewords: ECC byte 0 from block 0, ECC byte 0 from block 1, ...

  This ensures that a burst error affecting a contiguous module region only
  corrupts one or two codewords from each block, well within each block's
  correction budget.
  """
  @spec interleave_blocks([{[byte()], [byte()]}]) :: [byte()]
  def interleave_blocks(blocks) do
    all_data = Enum.map(blocks, &elem(&1, 0))
    all_ecc = Enum.map(blocks, &elem(&1, 1))

    max_data = Enum.max_by(all_data, &length/1) |> length()
    max_ecc = Enum.max_by(all_ecc, &length/1) |> length()

    # Interleave data codewords.
    interleaved_data =
      for i <- 0..(max_data - 1),
          block <- all_data,
          i < length(block),
          do: Enum.at(block, i)

    # Interleave ECC codewords.
    interleaved_ecc =
      for i <- 0..(max_ecc - 1),
          block <- all_ecc,
          i < length(block),
          do: Enum.at(block, i)

    interleaved_data ++ interleaved_ecc
  end
end
