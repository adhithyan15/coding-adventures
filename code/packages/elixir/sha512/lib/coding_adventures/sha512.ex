defmodule CodingAdventures.Sha512 do
  @moduledoc """
  SHA-512 cryptographic hash function (FIPS 180-4) implemented from scratch.

  ## What Is SHA-512?

  SHA-512 is the 64-bit sibling of SHA-256 in the SHA-2 family. It takes any
  sequence of bytes and produces a fixed-size 64-byte (512-bit) digest. The
  same input always produces the same digest. Change even one bit of input and
  the digest changes completely — the "avalanche effect".

  On 64-bit platforms, SHA-512 is often *faster* than SHA-256 because it
  processes 128-byte blocks (vs 64-byte) using native 64-bit arithmetic.

  ## How It Differs from SHA-256

      Property         SHA-256       SHA-512
      ────────         ───────       ───────
      Word size        32-bit        64-bit
      State words      8 × u32       8 × u64
      Block size       64 bytes      128 bytes
      Rounds           64            80
      Digest size      32 bytes      64 bytes
      Length field      64-bit       128-bit

  ## Elixir and 64-bit Arithmetic

  Elixir integers have arbitrary precision — no overflow, no wraparound.
  For SHA-512 we need modular 64-bit arithmetic, so we mask with
  `band(x, 0xFFFFFFFFFFFFFFFF)` after every addition.

  Elixir's binary pattern matching shines for parsing big-endian words:

      <<word::big-64, rest::binary>>

  This destructures a binary into a 64-bit big-endian integer and the rest.

  ## FIPS 180-4 Test Vectors

      sha512("") |> Base.encode16(case: :lower)
      # → "cf83e1357eefb8bd..."

      sha512("abc") |> Base.encode16(case: :lower)
      # → "ddaf35a193617aba..."
  """

  import Bitwise

  # ─── 64-bit Mask ─────────────────────────────────────────────────────────
  #
  # Elixir integers are arbitrary precision. After every addition we must
  # mask to 64 bits to simulate hardware wraparound behavior.

  @mask64 0xFFFFFFFFFFFFFFFF

  # ─── Initialization Constants ────────────────────────────────────────────
  #
  # Eight 64-bit words: first 64 bits of fractional parts of sqrt(2..19).
  #
  #   H₀ = frac(sqrt(2))  × 2^64 = 0x6a09e667f3bcc908
  #   H₁ = frac(sqrt(3))  × 2^64 = 0xbb67ae8584caa73b
  #   ...and so on

  @init {
    0x6A09E667F3BCC908, 0xBB67AE8584CAA73B,
    0x3C6EF372FE94F82B, 0xA54FF53A5F1D36F1,
    0x510E527FADE682D1, 0x9B05688C2B3E6C1F,
    0x1F83D9ABFB41BD6B, 0x5BE0CD19137E2179
  }

  # ─── Round Constants ─────────────────────────────────────────────────────
  #
  # 80 constants: first 64 bits of fractional parts of the cube roots of
  # the first 80 primes (2, 3, 5, ..., 409). Stored as a tuple for O(1)
  # indexed access via `elem/2`.

  @k {
    0x428A2F98D728AE22, 0x7137449123EF65CD, 0xB5C0FBCFEC4D3B2F, 0xE9B5DBA58189DBBC,
    0x3956C25BF348B538, 0x59F111F1B605D019, 0x923F82A4AF194F9B, 0xAB1C5ED5DA6D8118,
    0xD807AA98A3030242, 0x12835B0145706FBE, 0x243185BE4EE4B28C, 0x550C7DC3D5FFB4E2,
    0x72BE5D74F27B896F, 0x80DEB1FE3B1696B1, 0x9BDC06A725C71235, 0xC19BF174CF692694,
    0xE49B69C19EF14AD2, 0xEFBE4786384F25E3, 0x0FC19DC68B8CD5B5, 0x240CA1CC77AC9C65,
    0x2DE92C6F592B0275, 0x4A7484AA6EA6E483, 0x5CB0A9DCBD41FBD4, 0x76F988DA831153B5,
    0x983E5152EE66DFAB, 0xA831C66D2DB43210, 0xB00327C898FB213F, 0xBF597FC7BEEF0EE4,
    0xC6E00BF33DA88FC2, 0xD5A79147930AA725, 0x06CA6351E003826F, 0x142929670A0E6E70,
    0x27B70A8546D22FFC, 0x2E1B21385C26C926, 0x4D2C6DFC5AC42AED, 0x53380D139D95B3DF,
    0x650A73548BAF63DE, 0x766A0ABB3C77B2A8, 0x81C2C92E47EDAEE6, 0x92722C851482353B,
    0xA2BFE8A14CF10364, 0xA81A664BBC423001, 0xC24B8B70D0F89791, 0xC76C51A30654BE30,
    0xD192E819D6EF5218, 0xD69906245565A910, 0xF40E35855771202A, 0x106AA07032BBD1B8,
    0x19A4C116B8D2D0C8, 0x1E376C085141AB53, 0x2748774CDF8EEB99, 0x34B0BCB5E19B48A8,
    0x391C0CB3C5C95A63, 0x4ED8AA4AE3418ACB, 0x5B9CCA4F7763E373, 0x682E6FF3D6B2B8A3,
    0x748F82EE5DEFB2FC, 0x78A5636F43172F60, 0x84C87814A1F0AB72, 0x8CC702081A6439EC,
    0x90BEFFFA23631E28, 0xA4506CEBDE82BDE9, 0xBEF9A3F7B2C67915, 0xC67178F2E372532B,
    0xCA273ECEEA26619C, 0xD186B8C721C0C207, 0xEADA7DD6CDE0EB1E, 0xF57D4F7FEE6ED178,
    0x06F067AA72176FBA, 0x0A637DC5A2C898A6, 0x113F9804BEF90DAE, 0x1B710B35131C471B,
    0x28DB77F523047D84, 0x32CAAB7B40C72493, 0x3C9EBE0A15C9BEBC, 0x431D67C49C100D4C,
    0x4CC5D4BECB3E42B6, 0x597F299CFC657E2A, 0x5FCB6FAB3AD6FAEC, 0x6C44198C4A475817
  }

  # ─── Helper: 64-bit Right Rotation ───────────────────────────────────────
  #
  # rotr(n, x) rotates x right by n bit positions within a 64-bit word.
  # Bits that "fall off" the right end reappear on the left.
  #
  # We mask to 64 bits because Elixir integers have arbitrary precision.

  defp rotr(n, x) do
    band(bor(x >>> n, x <<< (64 - n)), @mask64)
  end

  # ─── SHA-512 Sigma Functions ─────────────────────────────────────────────
  #
  # Four mixing functions combining rotations and shifts:
  #
  #   Σ0(x) = ROTR(28,x) XOR ROTR(34,x) XOR ROTR(39,x)
  #   Σ1(x) = ROTR(14,x) XOR ROTR(18,x) XOR ROTR(41,x)
  #   σ0(x) = ROTR(1,x)  XOR ROTR(8,x)  XOR (x >>> 7)
  #   σ1(x) = ROTR(19,x) XOR ROTR(61,x) XOR (x >>> 6)
  #
  # Note: σ0 and σ1 use a right SHIFT (not rotation) for their third term.

  defp big_sigma0(x), do: bxor(bxor(rotr(28, x), rotr(34, x)), rotr(39, x))
  defp big_sigma1(x), do: bxor(bxor(rotr(14, x), rotr(18, x)), rotr(41, x))

  defp small_sigma0(x), do: bxor(bxor(rotr(1, x), rotr(8, x)), x >>> 7)
  defp small_sigma1(x), do: bxor(bxor(rotr(19, x), rotr(61, x)), x >>> 6)

  # ─── Choice and Majority ─────────────────────────────────────────────────
  #
  # Ch(x,y,z): for each bit, x chooses between y and z.
  #   x=1 → y, x=0 → z
  #
  # Maj(x,y,z): majority vote — output the bit appearing in ≥ 2 of 3 inputs.

  defp ch(x, y, z) do
    band(bxor(band(x, y), band(band(bnot(x), @mask64), z)), @mask64)
  end

  defp maj(x, y, z) do
    band(bxor(bxor(band(x, y), band(x, z)), band(y, z)), @mask64)
  end

  # ─── Padding ─────────────────────────────────────────────────────────────
  #
  # SHA-512 processes 128-byte blocks. Padding extends the message:
  #
  #   1. Append 0x80 (the '1' bit followed by seven '0' bits).
  #   2. Append 0x00 bytes until length ≡ 112 (mod 128).
  #   3. Append the original bit length as a 128-bit big-endian integer.
  #
  # Why 112? We need 16 bytes for the 128-bit length field: 112 + 16 = 128.

  defp pad(data) do
    bit_len = byte_size(data) * 8
    # How many zero bytes after 0x80? Solve: (n + 1 + z) mod 128 = 112
    z = rem(rem(112 - rem(byte_size(data) + 1, 128), 128) + 128, 128)
    # 128-bit big-endian length field
    padding = <<0x80, 0::size(z * 8), bit_len::big-128>>
    data <> padding
  end

  # ─── Message Schedule ────────────────────────────────────────────────────
  #
  # Each 128-byte block is parsed as 16 big-endian 64-bit words, then
  # expanded to 80 words using:
  #
  #   W[i] = σ1(W[i-2]) + W[i-7] + σ0(W[i-15]) + W[i-16]  (mod 2^64)

  defp parse_16_words(<<>>, acc), do: Enum.reverse(acc)

  defp parse_16_words(<<w::big-64, rest::binary>>, acc) do
    parse_16_words(rest, [w | acc])
  end

  defp schedule(block) do
    initial = parse_16_words(block, [])

    expanded =
      Enum.reduce(16..79, initial, fn i, w ->
        new_word =
          band(
            small_sigma1(Enum.at(w, i - 2)) +
              Enum.at(w, i - 7) +
              small_sigma0(Enum.at(w, i - 15)) +
              Enum.at(w, i - 16),
            @mask64
          )

        w ++ [new_word]
      end)

    List.to_tuple(expanded)
  end

  # ─── Compression Function ────────────────────────────────────────────────
  #
  # 80 rounds of mixing fold one 128-byte block into the eight-word state.
  #
  # Each round:
  #   T₁ = h + Σ1(e) + Ch(e,f,g) + K[t] + W[t]
  #   T₂ = Σ0(a) + Maj(a,b,c)
  #   h=g, g=f, f=e, e=d+T₁, d=c, c=b, b=a, a=T₁+T₂
  #
  # Davies-Meyer feed-forward after all 80 rounds.

  defp compress({s0, s1, s2, s3, s4, s5, s6, s7}, block) do
    w = schedule(block)
    {a, b, c, d, e, f, g, h} = {s0, s1, s2, s3, s4, s5, s6, s7}

    {a, b, c, d, e, f, g, h} =
      Enum.reduce(0..79, {a, b, c, d, e, f, g, h}, fn t, {a, b, c, d, e, f, g, h} ->
        t1 = band(h + big_sigma1(e) + ch(e, f, g) + elem(@k, t) + elem(w, t), @mask64)
        t2 = band(big_sigma0(a) + maj(a, b, c), @mask64)
        {band(t1 + t2, @mask64), a, b, c, band(d + t1, @mask64), e, f, g}
      end)

    {
      band(s0 + a, @mask64),
      band(s1 + b, @mask64),
      band(s2 + c, @mask64),
      band(s3 + d, @mask64),
      band(s4 + e, @mask64),
      band(s5 + f, @mask64),
      band(s6 + g, @mask64),
      band(s7 + h, @mask64)
    }
  end

  # ─── Finalization ────────────────────────────────────────────────────────
  #
  # Convert the eight 64-bit state words to 64 bytes in big-endian order.

  defp finalize({h0, h1, h2, h3, h4, h5, h6, h7}) do
    <<h0::big-64, h1::big-64, h2::big-64, h3::big-64,
      h4::big-64, h5::big-64, h6::big-64, h7::big-64>>
  end

  # ─── Block Processing ───────────────────────────────��────────────────────

  defp process_blocks(<<>>, state), do: state

  defp process_blocks(<<block::binary-128, rest::binary>>, state) do
    process_blocks(rest, compress(state, block))
  end

  # ─── Public API ──────────────────────────────────────────────────────────

  @doc """
  Compute the SHA-512 digest of `data`. Returns a 64-byte binary.

  This is the one-shot API: hash a complete message in a single call.

  ## Examples

      iex> CodingAdventures.Sha512.sha512("abc") |> Base.encode16(case: :lower) |> String.slice(0, 32)
      "ddaf35a193617abacc417349ae204131"

      iex> byte_size(CodingAdventures.Sha512.sha512(""))
      64
  """
  def sha512(data) when is_binary(data) do
    data
    |> pad()
    |> process_blocks(@init)
    |> finalize()
  end

  @doc """
  Compute SHA-512 and return the 128-character lowercase hex string.

  ## Examples

      iex> CodingAdventures.Sha512.sha512_hex("abc") |> String.slice(0, 16)
      "ddaf35a193617aba"
  """
  def sha512_hex(data) when is_binary(data) do
    data |> sha512() |> Base.encode16(case: :lower)
  end
end
