defmodule Ca.Sha1 do
  @moduledoc """
  SHA-1 cryptographic hash function (FIPS 180-4) implemented from scratch.

  ## What Is SHA-1?

  SHA-1 (Secure Hash Algorithm 1) takes any sequence of bytes and produces a
  fixed-size 20-byte (160-bit) "fingerprint" called a digest. The same input
  always produces the same digest. Change even one bit of input and the digest
  changes completely — the "avalanche effect". You cannot reverse a digest back
  to the original input.

  We implement SHA-1 from scratch rather than using `:crypto.hash(:sha, data)`
  so that every step is visible and explained.

  ## The Merkle-Damgård Construction

  SHA-1 processes data in 512-bit (64-byte) blocks:

      message ──► [pad] ──► block₀ ──► block₁ ──► ... ──► 20-byte digest
                                 │           │
                         [H₀..H₄]──►compress──►compress──►...

  The "state" is five 32-bit words (H₀..H₄), initialized to fixed constants.
  For each block, 80 rounds of bit mixing fold the block into the state.
  The final state is the digest.

  Analogy: a blender. Start with a base liquid (the initial constants). Add
  ingredients one chunk at a time (message blocks). Each blend mixes the new
  ingredient with everything before it. You cannot un-blend.

  ## Elixir Bitstring Pattern Matching

  Elixir's pattern matching shines in binary protocols. We use patterns like:

      <<word::big-32, rest::binary>>

  This destructures a binary into a 32-bit big-endian integer `word` and the
  remaining bytes `rest`. It is more expressive than manual byte manipulation.

  For 32-bit arithmetic (additions, etc.) we mask with `band(x, 0xFFFFFFFF)`.
  Elixir integers have arbitrary precision, so we must keep them in the 32-bit
  range manually — just like Python.

  ## FIPS 180-4 Test Vectors

      sha1("") |> Base.encode16(case: :lower)
      # → "da39a3ee5e6b4b0d3255bfef95601890afd80709"

      sha1("abc") |> Base.encode16(case: :lower)
      # → "a9993e364706816aba3e25717850c26c9cd0d89d"
  """

  import Bitwise

  # ─── Initialization Constants ─────────────────────────────────────────────
  #
  # SHA-1 starts with these five 32-bit words as its initial state. They are
  # "nothing up my sleeve" numbers — their obvious counting-sequence structure
  # (01234567, 89ABCDEF, … reversed) proves no backdoor is baked in.
  #
  #   H₀ = 0x67452301 → bytes 67 45 23 01 → reverse: 01 23 45 67
  #   H₁ = 0xEFCDAB89 → bytes EF CD AB 89 → reverse: 89 AB CD EF

  @init {0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0}

  # Round constants — one per 20-round stage, derived from square roots:
  #   K₀ = floor(sqrt(2)  × 2^30) = 0x5A827999  (rounds 0–19)
  #   K₁ = floor(sqrt(3)  × 2^30) = 0x6ED9EBA1  (rounds 20–39)
  #   K₂ = floor(sqrt(5)  × 2^30) = 0x8F1BBCDC  (rounds 40–59)
  #   K₃ = floor(sqrt(10) × 2^30) = 0xCA62C1D6  (rounds 60–79)

  @k {0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC, 0xCA62C1D6}

  # ─── Helper: Circular Left Shift ──────────────────────────────────────────
  #
  # rotl(n, x) rotates x left by n bit positions within a 32-bit word.
  # Bits that "fall off" the left end reappear on the right.
  #
  # Example: n=2, x=0b01101001 (8-bit for clarity)
  #   Regular:  01101001 <<< 2 = 10100100  (01 on the left is lost)
  #   Circular: 01101001 ROTL 2 = 10100110  (01 wraps around)
  #
  # We mask to 32 bits because Elixir integers have arbitrary precision.

  defp rotl(n, x) do
    band(bor(x <<< n, x >>> (32 - n)), 0xFFFFFFFF)
  end

  # ─── Padding ──────────────────────────────────────────────────────────────
  #
  # The compression function needs exactly 64-byte blocks. Padding extends
  # the message per FIPS 180-4 §5.1.1:
  #
  #   1. Append 0x80 (the '1' bit followed by seven '0' bits).
  #   2. Append 0x00 bytes until length ≡ 56 (mod 64).
  #   3. Append original bit length as a 64-bit big-endian integer.
  #
  # Example — "abc" (3 bytes = 24 bits):
  #   61 62 63 80 [52 zero bytes] 00 00 00 00 00 00 00 18
  #                                                   ^^ 24 in hex
  #
  # Elixir binaries make padding elegant: we build the padding as a bitstring
  # using the <<...>> constructor.
  #
  # `byte_size` returns the byte length. `rem/2` is remainder (like % in C).

  defp pad(data) do
    bit_len = byte_size(data) * 8
    # How many zero bytes do we need? Solve: (n + 1 + z) mod 64 = 56
    z = rem(rem(56 - rem(byte_size(data) + 1, 64), 64) + 64, 64)
    padding = <<0x80, 0::size(z * 8), bit_len::big-64>>
    data <> padding
  end

  # ─── Message Schedule ─────────────────────────────────────────────────────
  #
  # Each 64-byte block is parsed as 16 big-endian 32-bit words, then expanded
  # to 80 words:
  #
  #   W[i] = ROTL(1, W[i-3] XOR W[i-8] XOR W[i-14] XOR W[i-16])  for i ≥ 16
  #
  # Why expand to 80? More words → more mixing → better avalanche effect.
  #
  # We parse 16 words using recursive pattern matching on the binary, then
  # expand via a recursive list operation. The result is stored as a tuple
  # (indexed in O(1)) rather than a list (indexed in O(n)) for performance.

  defp parse_16_words(<<>>, acc), do: Enum.reverse(acc)

  defp parse_16_words(<<w::big-32, rest::binary>>, acc) do
    parse_16_words(rest, [w | acc])
  end

  defp schedule(block) do
    initial = parse_16_words(block, [])

    expanded =
      Enum.reduce(16..79, initial, fn i, w ->
        # XOR the four previous words
        xored =
          bxor(
            bxor(Enum.at(w, i - 3), Enum.at(w, i - 8)),
            bxor(Enum.at(w, i - 14), Enum.at(w, i - 16))
          )

        w ++ [rotl(1, xored)]
      end)

    List.to_tuple(expanded)
  end

  # ─── Compression Function ─────────────────────────────────────────────────
  #
  # 80 rounds of mixing fold one 64-byte block into the five-word state.
  #
  # Four stages of 20 rounds each, each using a different auxiliary function:
  #
  #   Stage  Rounds  f(b,c,d)                    Purpose
  #   ─────  ──────  ──────────────────────────  ─────────────────
  #     1    0–19    (b AND c) OR (NOT b AND d)  Selector / mux
  #     2    20–39   b XOR c XOR d               Parity
  #     3    40–59   (b&c)|(b&d)|(c&d)           Majority vote
  #     4    60–79   b XOR c XOR d               Parity again
  #
  # Each round:
  #   temp = ROTL(5, a) + f(b,c,d) + e + K + W[t]  (mod 2^32)
  #   shift: e=d, d=c, c=ROTL(30,b), b=a, a=temp
  #
  # Davies-Meyer feed-forward: after all 80 rounds, add the compressed
  # output back to the original state to prevent invertibility.

  defp f(t, b, c, d) when t < 20 do
    # Selector: if b=1 → c; if b=0 → d
    band(bor(band(b, c), band(bnot(b), d)), 0xFFFFFFFF)
  end

  defp f(t, b, c, d) when t < 40 do
    # Parity: 1 if an odd number of inputs are 1
    band(bxor(bxor(b, c), d), 0xFFFFFFFF)
  end

  defp f(t, b, c, d) when t < 60 do
    # Majority: 1 if at least 2 of the 3 inputs are 1
    band(bor(bor(band(b, c), band(b, d)), band(c, d)), 0xFFFFFFFF)
  end

  defp f(_t, b, c, d) do
    # Parity again (same formula, different constant)
    band(bxor(bxor(b, c), d), 0xFFFFFFFF)
  end

  defp k_const(t) when t < 20, do: elem(@k, 0)
  defp k_const(t) when t < 40, do: elem(@k, 1)
  defp k_const(t) when t < 60, do: elem(@k, 2)
  defp k_const(_t), do: elem(@k, 3)

  defp compress({h0, h1, h2, h3, h4}, block) do
    w = schedule(block)
    {a, b, c, d, e} = {h0, h1, h2, h3, h4}

    {a, b, c, d, e} =
      Enum.reduce(0..79, {a, b, c, d, e}, fn t, {a, b, c, d, e} ->
        ft = f(t, b, c, d)
        kt = k_const(t)
        wt = elem(w, t)
        temp = band(rotl(5, a) + ft + e + kt + wt, 0xFFFFFFFF)
        {temp, a, rotl(30, b), c, d}
      end)

    {
      band(h0 + a, 0xFFFFFFFF),
      band(h1 + b, 0xFFFFFFFF),
      band(h2 + c, 0xFFFFFFFF),
      band(h3 + d, 0xFFFFFFFF),
      band(h4 + e, 0xFFFFFFFF),
    }
  end

  # ─── Finalization ─────────────────────────────────────────────────────────
  #
  # Convert the five 32-bit state words to 20 bytes in big-endian order.
  # The bitstring constructor <<h0::big-32, ...>> does this compactly.

  defp finalize({h0, h1, h2, h3, h4}) do
    <<h0::big-32, h1::big-32, h2::big-32, h3::big-32, h4::big-32>>
  end

  # ─── Block Processing ─────────────────────────────────────────────────────

  defp process_blocks(<<>>, state), do: state

  defp process_blocks(<<block::binary-64, rest::binary>>, state) do
    process_blocks(rest, compress(state, block))
  end

  # ─── Public API ───────────────────────────────────────────────────────────

  @doc """
  Compute the SHA-1 digest of `data`. Returns a 20-byte binary.

  This is the one-shot API: hash a complete message in a single call.

  ## Examples

      iex> CodingAdventures.Sha1.sha1("abc") |> Base.encode16(case: :lower)
      "a9993e364706816aba3e25717850c26c9cd0d89d"

      iex> CodingAdventures.Sha1.sha1("") |> Base.encode16(case: :lower)
      "da39a3ee5e6b4b0d3255bfef95601890afd80709"
  """
  def sha1(data) when is_binary(data) do
    data
    |> pad()
    |> process_blocks(@init)
    |> finalize()
  end

  @doc """
  Compute SHA-1 and return the 40-character lowercase hex string.

  ## Examples

      iex> CodingAdventures.Sha1.sha1_hex("abc")
      "a9993e364706816aba3e25717850c26c9cd0d89d"
  """
  def sha1_hex(data) when is_binary(data) do
    data |> sha1() |> Base.encode16(case: :lower)
  end
end
