defmodule CodingAdventures.Sha256 do
  @moduledoc """
  SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch.

  ## What Is SHA-256?

  SHA-256 is a member of the SHA-2 family designed by the NSA and published by
  NIST in 2001. It takes any sequence of bytes and produces a fixed-size 32-byte
  (256-bit) "fingerprint" called a digest. The same input always produces the
  same digest. Change even one bit and the digest changes completely — the
  "avalanche effect". You cannot reverse a digest back to the original input.

  SHA-256 is the workhorse of modern cryptography: TLS, Bitcoin, git, code
  signing, and password hashing all depend on it. Unlike MD5 (broken 2004) and
  SHA-1 (broken 2017), SHA-256 remains secure with no known practical attacks.

  ## How SHA-256 Differs from SHA-1

  Both use the Merkle-Damgard construction (pad, split into blocks, compress),
  but SHA-256 is stronger in every dimension:

  - 8 state words (vs 5), 64 rounds (vs 80), 64 unique constants (vs 4)
  - Non-linear message schedule using sigma functions with SHR (not just XOR)
  - Ch and Maj auxiliary functions (no parity rounds)

  ## Elixir Implementation Notes

  Elixir's pattern matching on binaries shines for protocol parsing:

      <<word::big-32, rest::binary>>

  This destructures a binary into a 32-bit big-endian integer and the rest.
  For 32-bit arithmetic we mask with `band(x, 0xFFFFFFFF)` since Elixir
  integers have arbitrary precision.

  ## FIPS 180-4 Test Vectors

      sha256("") |> Base.encode16(case: :lower)
      # => "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

      sha256("abc") |> Base.encode16(case: :lower)
      # => "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
  """

  import Bitwise

  # ─── Initial Hash Values ──────────────────────────────────────────────
  #
  # Eight 32-bit words: the first 32 bits of the fractional parts of the
  # square roots of the first 8 primes (2, 3, 5, 7, 11, 13, 17, 19).
  #
  # "Nothing up my sleeve" numbers — their mathematical origin is
  # transparent and verifiable, proving no backdoor is hidden.

  @init {0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
         0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19}

  # ─── Round Constants ──────────────────────────────────────────────────
  #
  # 64 constants: the first 32 bits of the fractional parts of the cube
  # roots of the first 64 primes (2, 3, 5, ..., 311).
  #
  # Stored as a tuple for O(1) indexed access via elem/2.

  @round_k {
    0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5,
    0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
    0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3,
    0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
    0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC,
    0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
    0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7,
    0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
    0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13,
    0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
    0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3,
    0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
    0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5,
    0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
    0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208,
    0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2
  }

  # ─── Bit Manipulation Helpers ─────────────────────────────────────────
  #
  # rotr(n, x) — right-rotate x by n bits within 32-bit word.
  # Unlike right shift (>>>), bits that fall off the right reappear on
  # the left. SHA-256 uses right rotation (ROTR) while SHA-1 uses left
  # rotation (ROTL) — the algebra is equivalent, just a convention.

  defp rotr(n, val) do
    band(bor(val >>> n, val <<< (32 - n)), 0xFFFFFFFF)
  end

  # ─── Auxiliary Functions ──────────────────────────────────────────────
  #
  # Ch(x, y, z) — "Choose": if bit of x is 1, pick y; else pick z.
  #   Truth table: x=0→z, x=1→y
  #   Formula: (x AND y) XOR (NOT x AND z)
  #
  # Maj(x, y, z) — "Majority": output 1 if >= 2 of the 3 inputs are 1.
  #   Formula: (x AND y) XOR (x AND z) XOR (y AND z)
  #
  # big_sigma0(x) — ROTR(2) XOR ROTR(13) XOR ROTR(22) — on `a` in rounds
  # big_sigma1(x) — ROTR(6) XOR ROTR(11) XOR ROTR(25) — on `e` in rounds
  # small_sigma0(x) — ROTR(7) XOR ROTR(18) XOR SHR(3)  — schedule
  # small_sigma1(x) — ROTR(17) XOR ROTR(19) XOR SHR(10) — schedule
  #
  # The SHR in the small sigma functions makes the schedule non-invertible,
  # a key improvement over SHA-1's linear (XOR-only) schedule.

  defp choose(x, y, z) do
    band(bxor(band(x, y), band(bnot(x), z)), 0xFFFFFFFF)
  end

  defp majority(x, y, z) do
    band(bxor(bxor(band(x, y), band(x, z)), band(y, z)), 0xFFFFFFFF)
  end

  defp big_sigma0(x), do: band(bxor(bxor(rotr(2, x), rotr(13, x)), rotr(22, x)), 0xFFFFFFFF)
  defp big_sigma1(x), do: band(bxor(bxor(rotr(6, x), rotr(11, x)), rotr(25, x)), 0xFFFFFFFF)

  defp small_sigma0(x), do: band(bxor(bxor(rotr(7, x), rotr(18, x)), x >>> 3), 0xFFFFFFFF)
  defp small_sigma1(x), do: band(bxor(bxor(rotr(17, x), rotr(19, x)), x >>> 10), 0xFFFFFFFF)

  # ─── Padding ──────────────────────────────────────────────────────────
  #
  # Extends the message to a multiple of 64 bytes per FIPS 180-4 §5.1.1:
  #   1. Append 0x80
  #   2. Append zeros until length ≡ 56 (mod 64)
  #   3. Append 64-bit big-endian original bit length

  defp pad(data) do
    bit_len = byte_size(data) * 8
    zero_count = rem(rem(56 - rem(byte_size(data) + 1, 64), 64) + 64, 64)
    padding = <<0x80, 0::size(zero_count * 8), bit_len::big-64>>
    data <> padding
  end

  # ─── Message Schedule ─────────────────────────────────────────────────
  #
  # Parse 16 big-endian 32-bit words from the 64-byte block, then expand
  # to 64 words:
  #   W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]
  #
  # We build the schedule as a list, then convert to a tuple for O(1)
  # element access during the 64 compression rounds.

  defp parse_16_words(<<>>, acc), do: Enum.reverse(acc)

  defp parse_16_words(<<word::big-32, rest::binary>>, acc) do
    parse_16_words(rest, [word | acc])
  end

  defp schedule(block) do
    initial = parse_16_words(block, [])

    expanded =
      Enum.reduce(16..63, initial, fn idx, words ->
        w2 = Enum.at(words, idx - 2)
        w7 = Enum.at(words, idx - 7)
        w15 = Enum.at(words, idx - 15)
        w16 = Enum.at(words, idx - 16)

        new_word = band(
          small_sigma1(w2) + w7 + small_sigma0(w15) + w16,
          0xFFFFFFFF
        )

        words ++ [new_word]
      end)

    List.to_tuple(expanded)
  end

  # ─── Compression Function ─────────────────────────────────────────────
  #
  # 64 rounds fold one 64-byte block into the eight-word state.
  #
  # Each round:
  #   T1 = h + Sigma1(e) + Ch(e,f,g) + K[t] + W[t]
  #   T2 = Sigma0(a) + Maj(a,b,c)
  #   Shift variables down; a = T1 + T2, e = d + T1
  #
  # Davies-Meyer feed-forward: add compressed output back to input state.

  defp compress({s0, s1, s2, s3, s4, s5, s6, s7}, block) do
    sched = schedule(block)

    {a, b, c, d, e, f, g, h} =
      Enum.reduce(0..63, {s0, s1, s2, s3, s4, s5, s6, s7}, fn round_idx, {a, b, c, d, e, f, g, h} ->
        t1 = band(
          h + big_sigma1(e) + choose(e, f, g) + elem(@round_k, round_idx) + elem(sched, round_idx),
          0xFFFFFFFF
        )

        t2 = band(big_sigma0(a) + majority(a, b, c), 0xFFFFFFFF)

        {
          band(t1 + t2, 0xFFFFFFFF),  # new a
          a,                            # new b
          b,                            # new c
          c,                            # new d
          band(d + t1, 0xFFFFFFFF),    # new e
          e,                            # new f
          f,                            # new g
          g                             # new h
        }
      end)

    {
      band(s0 + a, 0xFFFFFFFF),
      band(s1 + b, 0xFFFFFFFF),
      band(s2 + c, 0xFFFFFFFF),
      band(s3 + d, 0xFFFFFFFF),
      band(s4 + e, 0xFFFFFFFF),
      band(s5 + f, 0xFFFFFFFF),
      band(s6 + g, 0xFFFFFFFF),
      band(s7 + h, 0xFFFFFFFF)
    }
  end

  # ─── Finalization ─────────────────────────────────────────────────────
  #
  # Convert the eight 32-bit state words to 32 bytes in big-endian order.

  defp finalize({s0, s1, s2, s3, s4, s5, s6, s7}) do
    <<s0::big-32, s1::big-32, s2::big-32, s3::big-32,
      s4::big-32, s5::big-32, s6::big-32, s7::big-32>>
  end

  # ─── Block Processing ─────────────────────────────────────────────────

  defp process_blocks(<<>>, state), do: state

  defp process_blocks(<<block::binary-64, rest::binary>>, state) do
    process_blocks(rest, compress(state, block))
  end

  # ─── Public API ───────────────────────────────────────────────────────

  @doc """
  Compute the SHA-256 digest of `data`. Returns a 32-byte binary.

  This is the one-shot API: hash a complete message in a single call.

  ## Examples

      iex> CodingAdventures.Sha256.sha256("abc") |> Base.encode16(case: :lower)
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

      iex> CodingAdventures.Sha256.sha256("") |> Base.encode16(case: :lower)
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  """
  def sha256(data) when is_binary(data) do
    data
    |> pad()
    |> process_blocks(@init)
    |> finalize()
  end

  @doc """
  Compute SHA-256 and return the 64-character lowercase hex string.

  ## Examples

      iex> CodingAdventures.Sha256.sha256_hex("abc")
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
  """
  def sha256_hex(data) when is_binary(data) do
    data |> sha256() |> Base.encode16(case: :lower)
  end

  # ─── Streaming API ────────────────────────────────────────────────────
  #
  # The streaming hasher accumulates data in a buffer. When the buffer
  # reaches 64 bytes, we compress a block and keep the updated state.
  # On digest, we pad the remaining buffer using the total byte count.
  #
  # Since Elixir is immutable, the "hasher" is a struct that gets
  # returned with updated state after each operation.

  defmodule Hasher do
    @moduledoc """
    Streaming SHA-256 hasher for processing data in chunks.

    ## Examples

        iex> alias CodingAdventures.Sha256.Hasher
        iex> h = Hasher.new() |> Hasher.update("ab") |> Hasher.update("c")
        iex> Hasher.hex_digest(h)
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    """

    defstruct state: nil, buffer: <<>>, byte_count: 0

    @doc "Create a new streaming SHA-256 hasher."
    def new do
      %__MODULE__{
        state: {0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
                0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19},
        buffer: <<>>,
        byte_count: 0
      }
    end

    @doc "Feed more bytes into the hash. Returns the updated hasher."
    def update(%__MODULE__{} = hasher, data) when is_binary(data) do
      new_buffer = hasher.buffer <> data
      new_count = hasher.byte_count + byte_size(data)
      {new_state, remaining} = compress_full_blocks(hasher.state, new_buffer)

      %{hasher | state: new_state, buffer: remaining, byte_count: new_count}
    end

    @doc "Return the 32-byte digest. Non-destructive."
    def digest(%__MODULE__{} = hasher) do
      bit_len = hasher.byte_count * 8
      buf = hasher.buffer
      zero_count = rem(rem(56 - rem(byte_size(buf) + 1, 64), 64) + 64, 64)
      tail = buf <> <<0x80, 0::size(zero_count * 8), bit_len::big-64>>

      {final_state, <<>>} = compress_full_blocks(hasher.state, tail)
      {s0, s1, s2, s3, s4, s5, s6, s7} = final_state

      <<s0::big-32, s1::big-32, s2::big-32, s3::big-32,
        s4::big-32, s5::big-32, s6::big-32, s7::big-32>>
    end

    @doc "Return the 64-character hex string of the digest."
    def hex_digest(%__MODULE__{} = hasher) do
      hasher |> digest() |> Base.encode16(case: :lower)
    end

    @doc "Return an independent copy of this hasher."
    def copy(%__MODULE__{} = hasher) do
      %__MODULE__{
        state: hasher.state,
        buffer: hasher.buffer,
        byte_count: hasher.byte_count
      }
    end

    # Compress all complete 64-byte blocks in the buffer, return
    # updated state and any remaining bytes (< 64).
    defp compress_full_blocks(state, <<block::binary-64, rest::binary>>) do
      new_state = CodingAdventures.Sha256.do_compress(state, block)
      compress_full_blocks(new_state, rest)
    end

    defp compress_full_blocks(state, remaining) do
      {state, remaining}
    end
  end

  # Expose compress for the Hasher module (must be public for cross-module call).
  @doc false
  def do_compress(state, block), do: compress(state, block)
end
