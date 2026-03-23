defmodule CodingAdventures.Md5 do
  @moduledoc """
  MD5 message digest algorithm (RFC 1321) implemented from scratch.

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.

  ## What Is MD5?

  MD5 (Message Digest 5) takes any sequence of bytes and produces a fixed-size
  16-byte (128-bit) "fingerprint" called a digest. The same input always produces
  the same digest. Change even one bit of input and the digest changes completely.

  Created by Ron Rivest in 1991 as an improvement over MD4. Standardized in
  RFC 1321. MD5 is cryptographically broken (collision attacks since 2004) and
  should NOT be used for security purposes (digital signatures, password hashing,
  TLS certificates). It remains valid for: non-security checksums, UUID v3, and
  legacy systems that already use it.

  ## The Critical Difference: Little-Endian

  The most important difference between MD5 and SHA-1 is byte order:

  ```
  Property     SHA-1       MD5
  ──────────   ─────────   ─────────────
  Output size  20 bytes    16 bytes
  State words  5 (H₀..H₄)  4 (A,B,C,D)
  Rounds       80          64
  Block size   512 bits    512 bits
  Word order   Big-endian  LITTLE-ENDIAN ← key difference!
  ```

  Big-endian (SHA-1): most significant byte first.  0x0A0B0C0D → 0A 0B 0C 0D
  Little-endian (MD5): LEAST significant byte first. 0x0A0B0C0D → 0D 0C 0B 0A

  In Elixir bitstring syntax, this means we write:
  - `<<word::little-32>>` to parse a little-endian 32-bit word from binary
  - `<<a::little-32, b::little-32, c::little-32, d::little-32>>` to produce
    the little-endian output digest

  This is the #1 source of MD5 implementation bugs.

  ## RFC 1321 Test Vectors

  ```
  md5("")              → "d41d8cd98f00b204e9800998ecf8427e"
  md5("a")             → "0cc175b9c0f1b6a831c399e269772661"
  md5("abc")           → "900150983cd24fb0d6963f7d28e17f72"
  md5("message digest") → "f96b697d7cb7938d525a2f31aaf161d0"
  ```

  ## Public API

  - `md5/1` — returns 16-byte binary digest
  - `md5_hex/1` — returns 32-character lowercase hex string
  """

  import Bitwise

  # ─── T-Table: 64 Constants Derived From Sine ────────────────────────────────
  #
  # T[i] = floor(abs(sin(i+1)) × 2^32)  for i in 0..63
  #
  # These constants are called "nothing up my sleeve" numbers. Ron Rivest chose
  # sin() because it is a standard mathematical function — anyone can verify that
  # no secret trapdoor was hidden in the constants. The derivation is:
  #
  #   sin(1) ≈ 0.8414709848...
  #   abs(sin(1)) × 2^32 = 0.8414709848 × 4,294,967,296 ≈ 3,614,090,360
  #   floor(3,614,090,360) = 0xD76AA478 = T[0]
  #
  # We compute the full table at compile time using `@t` module attribute.
  # `trunc/1` truncates toward zero (same as floor for positive numbers).
  # `&&& 0xFFFFFFFF` ensures the result fits in 32 bits.
  #
  # The `i+1` offset is because the RFC numbers T from 1, but we index from 0.

  @t (for i <- 0..63 do
    trunc(abs(:math.sin(i + 1)) * 4_294_967_296) &&& 0xFFFFFFFF
  end)

  # ─── Round Shift Amounts ─────────────────────────────────────────────────────
  #
  # Each of the 64 rounds rotates its intermediate value by a specific number of
  # bits. These shifts are grouped in 4 sets of 16, one set per stage:
  #
  #   Stage 1 (rounds 0–15):  [7, 12, 17, 22] × 4
  #   Stage 2 (rounds 16–31): [5,  9, 14, 20] × 4
  #   Stage 3 (rounds 32–47): [4, 11, 16, 23] × 4
  #   Stage 4 (rounds 48–63): [6, 10, 15, 21] × 4
  #
  # These shift amounts were chosen empirically by Rivest for good "avalanche"
  # effect — small input changes cascade to large output changes quickly.

  @s [
    7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,
    5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,
    4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,
    6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21
  ]

  # ─── Initialization Constants ────────────────────────────────────────────────
  #
  # The four 32-bit words A, B, C, D that form the initial state.
  # Also "nothing up my sleeve" numbers — the pattern is 0x01234567,
  # 0x89ABCDEF, 0xFEDCBA98, 0x76543210 with bytes reversed (little-endian view):
  #
  #   A = 0x67452301 → bytes: 01 23 45 67 (reverse of 67 45 23 01)
  #   B = 0xEFCDAB89 → bytes: 89 AB CD EF
  #   C = 0x98BADCFE → bytes: FE DC BA 98
  #   D = 0x10325476 → bytes: 76 54 32 10
  #
  # Reading these in order: 01 23 45 67 89 AB CD EF FE DC BA 98 76 54 32 10
  # — a recognizable "counting" sequence in little-endian byte order.

  @init_a 0x67452301
  @init_b 0xEFCDAB89
  @init_c 0x98BADCFE
  @init_d 0x10325476

  # ─── Public API ──────────────────────────────────────────────────────────────

  @doc """
  Compute the MD5 digest of `data`. Returns a 16-byte binary.

  NOTE: MD5 is cryptographically broken. Do NOT use for passwords, digital
  signatures, or security-sensitive checksums. Valid uses: UUID v3, legacy
  checksums, non-security fingerprinting.

  ## Examples

      iex> CodingAdventures.Md5.md5("") |> Base.encode16(case: :lower)
      "d41d8cd98f00b204e9800998ecf8427e"

      iex> CodingAdventures.Md5.md5("abc") |> Base.encode16(case: :lower)
      "900150983cd24fb0d6963f7d28e17f72"

  """
  @spec md5(binary()) :: binary()
  def md5(data) when is_binary(data) do
    padded = pad(data)
    {a, b, c, d} = compress_all(padded, {@init_a, @init_b, @init_c, @init_d})
    # Finalize: write each state word as a little-endian 32-bit integer.
    # The `<<x::little-32>>` syntax tells Elixir to lay out the bytes of x
    # in least-significant-byte-first order (little-endian).
    # For a = 0x01234567:
    #   big-endian:    01 23 45 67
    #   little-endian: 67 45 23 01  ← what <<a::little-32>> produces
    <<a::little-32, b::little-32, c::little-32, d::little-32>>
  end

  @doc """
  Compute MD5 and return the 32-character lowercase hex string.

  Uses `Base.encode16(case: :lower)` which is the standard Elixir way to
  produce lowercase hex. This matches all RFC 1321 test vectors.

  ## Examples

      iex> CodingAdventures.Md5.md5_hex("")
      "d41d8cd98f00b204e9800998ecf8427e"

      iex> CodingAdventures.Md5.md5_hex("abc")
      "900150983cd24fb0d6963f7d28e17f72"

  """
  @spec md5_hex(binary()) :: String.t()
  def md5_hex(data) when is_binary(data) do
    data |> md5() |> Base.encode16(case: :lower)
  end

  # ─── Padding ─────────────────────────────────────────────────────────────────
  #
  # MD5 processes messages in 512-bit (64-byte) blocks. Before processing,
  # the message must be padded to a multiple of 64 bytes.
  #
  # Padding rules (RFC 1321 §3.1):
  #
  #   1. Append byte 0x80 (the bit "1" followed by seven "0" bits).
  #   2. Append 0x00 bytes until total length ≡ 56 (mod 64).
  #   3. Append the original bit length as a 64-bit LITTLE-ENDIAN integer.
  #
  # This leaves exactly 8 bytes (= 64-bit length field) at the end of the block.
  #
  # Example — "abc" (3 bytes = 24 bits):
  #   61 62 63 80 [52 zero bytes] 18 00 00 00 00 00 00 00
  #                               ^^
  #   24 = 0x18, stored little-endian: 18 00 00 00 00 00 00 00
  #
  # CRITICAL: The length field is LITTLE-ENDIAN. SHA-1 uses big-endian here.
  # In Elixir: <<bit_len::little-64>> produces the correct little-endian bytes.
  #
  # Minimum padding: 1 byte (the 0x80). Maximum: 64 bytes (when original length
  # is already 55 mod 64 — we need a full extra block to fit the length field).

  defp pad(data) do
    byte_len = byte_size(data)
    bit_len = byte_len * 8

    # Number of zero bytes needed after the 0x80 byte.
    # Target: (byte_len + 1 + zeros) mod 64 = 56
    # Solving: zeros = (56 - byte_len - 1) mod 64 = (55 - byte_len) mod 64
    zeros = rem(55 - byte_len, 64) |> then(fn n -> if n < 0, do: n + 64, else: n end)

    # Build: original data + 0x80 + zero bytes + little-endian 64-bit length
    <<data::binary, 0x80, 0::size(zeros)-unit(8), bit_len::little-64>>
  end

  # ─── Block Processing ─────────────────────────────────────────────────────────
  #
  # Process all 64-byte blocks in the padded message, threading the state
  # through each compression call.

  defp compress_all(<<>>, state), do: state
  defp compress_all(<<block::binary-64, rest::binary>>, state) do
    compress_all(rest, compress(state, block))
  end

  # ─── Compression Function ────────────────────────────────────────────────────
  #
  # The heart of MD5. Takes a 4-word state {A, B, C, D} and a 64-byte block,
  # mixes them through 64 rounds, then adds the results back (Davies-Meyer).
  #
  # ## Parsing Block Words
  #
  # The 64-byte block is split into 16 × 32-bit words M[0..15].
  # Each word is parsed LITTLE-ENDIAN:
  #
  #   bytes: b0 b1 b2 b3
  #   word:  b0 + b1*256 + b2*65536 + b3*16777216
  #
  # In Elixir bitstring matching: `<<w::little-32, rest::binary>>`
  # This automatically reads w in little-endian order.
  #
  # Compare: `<<w::big-32, rest::binary>>` (default, or `<<w::32>>`) reads
  # big-endian. Using big-endian here would silently produce wrong results.
  #
  # ## Four Auxiliary Functions
  #
  # Each of the four stages uses a different Boolean function of {B, C, D}:
  #
  #   Stage 1 — F (selector):    (B & C) | (~B & D)
  #     Truth table: if B=1 → result=C, if B=0 → result=D
  #     B acts as a selector bit choosing between C and D.
  #
  #   Stage 2 — G (rotated F):   (D & B) | (~D & C)
  #     Same idea but D is the selector. Roles of B and D are swapped.
  #
  #   Stage 3 — H (parity):      B ^ C ^ D
  #     Result is 1 if an odd number of {B, C, D} are 1. Maximizes diffusion.
  #
  #   Stage 4 — I (unusual):     C ^ (B | ~D)
  #     The most unusual function. Analysis:
  #       When D=1: ~D=0, B|~D=B, result=C^B (parity of C and B)
  #       When D=0: ~D=1, B|~D=1, result=C^1 (inverts C)
  #     This provides a different mixing pattern than F, G, and H.
  #
  # ## Message Word Schedule
  #
  # Each stage accesses the 16 message words M[g] in a different order,
  # ensuring all 16 words are used 4 times with different interleaving:
  #
  #   Stage 1: g = i           (0,1,2,...,15 — sequential)
  #   Stage 2: g = (5i+1) % 16 (1,6,11,0,5,10,15,4,9,14,3,8,13,2,7,12)
  #   Stage 3: g = (3i+5) % 16 (5,8,11,14,1,4,7,10,13,0,3,6,9,12,15,2)
  #   Stage 4: g = (7i)   % 16 (0,7,14,5,12,3,10,1,8,15,6,13,4,11,2,9)
  #
  # ## Round Update Rule
  #
  #   tmp = A + f(B,C,D) + M[g] + T[i]   (all mod 2^32)
  #   new_A = B + rotl(S[i], tmp)         (rotl is left circular rotation)
  #   then {A, B, C, D} ← {D, new_A, B, C}
  #
  # Note: the rotation is on `tmp`, not on A. The result is then added to B.
  # This is sometimes written as: new_B = B + rotl(A + f + M[g] + T[i], S[i])
  #
  # ## Davies-Meyer Feed-Forward
  #
  # After all 64 rounds, add the original state back (mod 2^32):
  #   A_final = A_out + A_in
  #   B_final = B_out + B_in
  #   ...
  #
  # This prevents the compression from being invertible even if the round
  # function itself could be reversed. It's a standard construction from
  # symmetric cryptography.

  defp compress({a0, b0, c0, d0}, block) do
    # Parse the 64-byte block into 16 little-endian 32-bit words.
    # The `::little-32` specifier reads bytes in LSB-first order.
    m = parse_words(block, [])

    # Run all 64 rounds, threading {a, b, c, d} through each.
    {a, b, c, d} = run_rounds(0, a0, b0, c0, d0, m, @s, @t)

    # Davies-Meyer feed-forward: add the input state back, mod 2^32.
    {
      band(a0 + a, 0xFFFFFFFF),
      band(b0 + b, 0xFFFFFFFF),
      band(c0 + c, 0xFFFFFFFF),
      band(d0 + d, 0xFFFFFFFF)
    }
  end

  # Parse 64 bytes into a list of 16 little-endian 32-bit words.
  # Pattern `<<w::little-32, rest::binary>>` reads the next 4 bytes as a
  # little-endian integer, then recurses on the remaining bytes.
  defp parse_words(<<>>, acc), do: Enum.reverse(acc)
  defp parse_words(<<w::little-32, rest::binary>>, acc) do
    parse_words(rest, [w | acc])
  end

  # Run all 64 rounds. We unroll the loop by recursing with the round index i.
  # When i reaches 64, all rounds are complete and we return the state.
  defp run_rounds(64, a, b, c, d, _m, _s, _t), do: {a, b, c, d}

  defp run_rounds(i, a, b, c, d, m, s_table, t_table) do
    # Select the auxiliary function and message word index based on the stage.
    #
    # Stage boundaries: i<16, i<32, i<48, i<64
    # Each stage uses a different f() and a different message word schedule g.
    {f, g} = cond do
      i < 16 ->
        # Stage 1 — F function (selector: B chooses between C and D)
        # f = (B & C) | (~B & D), truncated to 32 bits
        f = band(bor(band(b, c), band(bnot(b), d)), 0xFFFFFFFF)
        {f, i}

      i < 32 ->
        # Stage 2 — G function (selector: D chooses between B and C)
        # f = (D & B) | (~D & C), truncated to 32 bits
        f = band(bor(band(d, b), band(bnot(d), c)), 0xFFFFFFFF)
        {f, rem(5 * i + 1, 16)}

      i < 48 ->
        # Stage 3 — H function (3-way XOR parity)
        # f = B ^ C ^ D, truncated to 32 bits
        f = band(bxor(bxor(b, c), d), 0xFFFFFFFF)
        {f, rem(3 * i + 5, 16)}

      true ->
        # Stage 4 — I function (C XOR (B OR NOT D))
        # This is the unusual "I" function. Analysis:
        #   D=1 → ~D=0 → B|~D=B → result = C^B (parity)
        #   D=0 → ~D=1 → B|~D=1 → result = C^1 (invert C)
        # Truncated to 32 bits with band.
        f = band(bxor(c, bor(b, bnot(d))), 0xFFFFFFFF)
        {f, rem(7 * i, 16)}
    end

    # Look up the shift amount and T constant for this round.
    s = Enum.at(s_table, i)
    t = Enum.at(t_table, i)
    mw = Enum.at(m, g)

    # The round update rule:
    #   tmp = A + f + M[g] + T[i]  (mod 2^32)
    #   new_A = B + rotl(tmp, S[i])  (mod 2^32)
    #   then rotate state: {A, B, C, D} ← {D, new_A, B, C}
    tmp = band(a + f + mw + t, 0xFFFFFFFF)
    new_a = band(b + rotl(tmp, s), 0xFFFFFFFF)

    # State rotation: A becomes D (shifted out), new_A replaces B,
    # B becomes C, and C becomes D.
    run_rounds(i + 1, d, new_a, b, c, m, s_table, t_table)
  end

  # ─── Left Circular Rotation ───────────────────────────────────────────────────
  #
  # ROTL(x, n) rotates the bits of x left by n positions (32-bit word).
  #
  # Example: rotl(0b10110001, 3) = 0b10001101
  #           ^^^                            ^^^
  #          top 3 bits wrap around to the bottom
  #
  # Implementation:
  #   left part:  (x << n) — shifts bits left, fills right with 0
  #   right part: (x >> (32-n)) — the bits that "wrapped around"
  #   combine:    left | right
  #   mask:       & 0xFFFFFFFF — keep only the lower 32 bits
  #
  # In Elixir, `bsr` is unsigned right shift (same as `>>>`). Since we mask
  # with 0xFFFFFFFF before calling rotl, x is always non-negative, so
  # `bsr` and signed right shift behave identically here.

  defp rotl(x, n) do
    band(bor(bsl(x, n), bsr(x, 32 - n)), 0xFFFFFFFF)
  end
end
