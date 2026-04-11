defmodule CodingAdventures.Scrypt do
  @moduledoc """
  scrypt — Memory-Hard Password-Based Key Derivation Function (RFC 7914).

  ## What Is scrypt?

  scrypt is a password hashing function designed by Colin Percival in 2009. It
  was specifically engineered to be **memory-hard**: computing it requires not
  just CPU time, but also a large amount of RAM. This makes it very expensive
  to attack using ASICs (custom chips) or FPGAs, which can parallelize SHA-256
  cheaply but cannot cheaply parallelize access to large amounts of memory.

  Real-world use cases:
  - Litecoin (the cryptocurrency) uses scrypt as its proof-of-work hash
  - OpenBSD's `bcryptpbkdf` borrows ideas from scrypt
  - macOS File Vault key derivation
  - Tarsnap backup tool (scrypt's original application)

  ## Why Not PBKDF2?

  PBKDF2 (and bcrypt) are compute-bound — every iteration is purely CPU work.
  Modern GPUs can perform billions of SHA-256 hashes per second at low cost,
  so PBKDF2 passwords can be brute-forced efficiently on GPU clusters.

  scrypt changes the economics: each candidate password requires reading/writing
  a large block of memory (`N * 128 * r` bytes). Memory bandwidth is the
  bottleneck, and memory cannot be parallelized like compute. Even expensive GPU
  clusters lose their advantage when memory bandwidth per thread is the limit.

  ## Parameters

  | Parameter | Meaning                                      | RFC 7914 default |
  |-----------|----------------------------------------------|-----------------|
  | `n`       | CPU/memory cost (must be power of 2, ≥ 2)   | 16384           |
  | `r`       | Block size factor (typically 8)              | 8               |
  | `p`       | Parallelization factor (typically 1–4)       | 1               |
  | `dk_len`  | Output length in bytes                       | 32              |

  Memory usage = `128 * r * n` bytes (e.g. r=8, n=16384 → 16 MiB).

  ## Algorithm Overview (RFC 7914 §3)

  scrypt is built from three layers:

  ```
  scrypt(password, salt, N, r, p, dk_len)
  │
  ├─ Step 1: PBKDF2-HMAC-SHA256(password, salt, 1, p*128*r)
  │          Derive p independent 128*r-byte blocks.
  │
  ├─ Step 2: For each block, run ROMix(block, N)
  │          ROMix fills an N-entry lookup table (the "ROM"),
  │          then makes N pseudo-random lookups into it.
  │          This is the memory-hard step.
  │
  └─ Step 3: PBKDF2-HMAC-SHA256(password, mixed_blocks, 1, dk_len)
             Finalize the output key.
  ```

  ### Salsa20/8 Core

  The innermost primitive is a variant of the Salsa20 stream cipher, using
  only 8 rounds instead of the full 20 (hence "Salsa20/8"). It operates on
  64-byte (512-bit) blocks interpreted as 16 little-endian uint32 words.

  Each "double round" applies a quarter-round function to columns, then rows.
  After 4 double-rounds (= 8 total rounds), the final state is added word-by-word
  to the initial state (modulo 2^32).

  ### BlockMix

  BlockMix operates on a 128*r-byte block treated as 2r sequential 64-byte
  chunks. It XORs and feeds each chunk through Salsa20/8 sequentially, then
  deinterleaves: even-indexed outputs come first, odd-indexed outputs second.
  This deinterleaving is what gives scrypt its memory-access pattern.

  ### ROMix

  ROMix builds a random-access memory table of N BlockMix outputs (V[0]..V[N-1]),
  then makes N pseudo-random lookups: each step XORs the current block with a
  table entry chosen by the last 8 bytes of the current block (Integerify),
  then runs BlockMix again. Because the lookup indices are pseudo-random and
  depend on previous outputs, the full table must be kept in memory — it cannot
  be recomputed on-the-fly without revisiting earlier entries.

  ## Example

      iex> CodingAdventures.Scrypt.scrypt_hex("", "", 16, 1, 1, 64)
      "77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906"

  ## Security Notes

  - Use N ≥ 16384, r = 8, p = 1 for interactive logins (2013 recommendation).
  - Use N ≥ 1048576, r = 8, p = 1 for sensitive data at rest.
  - For new systems, Argon2id (RFC 9106) is preferred — it won the Password
    Hashing Competition and has better theoretical properties. But scrypt
    remains widely supported and is a solid choice.
  - Never use the same (password, salt) pair for two different purposes.
  """

  import Bitwise

  # ---------------------------------------------------------------------------
  # Internal PBKDF2-HMAC-SHA256
  #
  # We implement our own PBKDF2 core here instead of delegating to
  # CodingAdventures.PBKDF2, because the RFC 7914 test vectors include an
  # empty password ("") in test vector 1. Our PBKDF2 module (correctly) rejects
  # empty passwords for user-facing use, but scrypt's internal use of PBKDF2
  # is different: the "password" is the actual plaintext bytes fed into the KDF,
  # not a user-facing secret. The HMAC spec (RFC 2104) and the underlying hash
  # function handle empty inputs correctly.
  #
  # This private PBKDF2 core calls CodingAdventures.HMAC.hmac/4 directly,
  # bypassing the empty-key guard in the named HMAC variants.
  # ---------------------------------------------------------------------------

  alias CodingAdventures.Hmac
  alias CodingAdventures.Sha256

  # hmac_sha256_raw/2 — HMAC-SHA256 without the empty-key guard.
  #
  # Calls the generic Hmac.hmac/4 function directly, which uses the HMAC
  # construction from RFC 2104. An empty key is handled correctly by the HMAC
  # algorithm: it is zero-padded to the block size (64 bytes), just like any
  # other key shorter than 64 bytes. This is mathematically valid.
  defp hmac_sha256_raw(key, message) do
    Hmac.hmac(&Sha256.sha256/1, 64, key, message)
  end

  # pbkdf2_sha256_raw/4 — PBKDF2-HMAC-SHA256 without empty-password validation.
  #
  # This is the standard PBKDF2 algorithm (RFC 8018 §5.2) with h_len = 32.
  # We implement it here to allow empty passwords, which scrypt's RFC test
  # vectors require. The algorithm is identical to CodingAdventures.Pbkdf2:
  #
  #   DK = T_1 || T_2 || ... (first dk_len bytes)
  #   T_i = U_1 XOR U_2 XOR ... XOR U_c
  #   U_1 = HMAC(password, salt || INT32BE(i))
  #   U_j = HMAC(password, U_{j-1})
  #
  # The block index i is 1-based and encoded as 4-byte big-endian.
  defp pbkdf2_sha256_raw(password, salt, iterations, key_length) do
    h_len = 32
    num_blocks = ceil(key_length / h_len)

    dk =
      for i <- 1..num_blocks//1 do
        # Each block gets a unique seed: salt concatenated with the block index
        # encoded as a 4-byte big-endian integer. This ensures different blocks
        # produce different outputs even when salt is the same.
        seed = salt <> <<i::big-unsigned-integer-size(32)>>

        # U_1: first PRF application
        u1 = hmac_sha256_raw(password, seed)

        # Fold in U_2..U_c by XOR, carrying the previous U value forward.
        # iterations == 1 is handled specially to avoid an empty 2..1//1 range.
        {t, _last_u} =
          if iterations == 1 do
            {u1, u1}
          else
            Enum.reduce(2..iterations//1, {u1, u1}, fn _j, {acc, prev_u} ->
              next_u = hmac_sha256_raw(password, prev_u)
              new_acc = :crypto.exor(acc, next_u)
              {new_acc, next_u}
            end)
          end

        t
      end
      |> IO.iodata_to_binary()

    # Truncate to the exact requested length (the last block may have excess)
    binary_part(dk, 0, key_length)
  end

  # ---------------------------------------------------------------------------
  # Salsa20/8 Core (RFC 7914 §3)
  # ---------------------------------------------------------------------------
  #
  # Salsa20/8 is a 64-byte permutation. It interprets the input as 16 uint32
  # words in little-endian order, applies 8 rounds of a quarter-round function,
  # then adds the final state to the initial state (mod 2^32) word-by-word.
  #
  # "Salsa20" refers to the full cipher with 20 rounds. The "/8" suffix means
  # we use only 8 rounds — a deliberate tradeoff: 8 rounds is fast enough for
  # scrypt's internal use while still providing sufficient diffusion. Security
  # analysis shows 8 rounds is safe in the scrypt context where collisions are
  # not a concern.
  #
  # Quarter-round r(a, b, c, d) — this is the core mixing function:
  #
  #   b ^= rotl32(a + d, 7)
  #   c ^= rotl32(b + a, 9)
  #   d ^= rotl32(c + b, 13)
  #   a ^= rotl32(d + c, 18)
  #
  # The rotation distances (7, 9, 13, 18) were chosen by Bernstein to maximize
  # diffusion. Each input bit affects all output bits after a small number of
  # applications.
  #
  # A "double round" applies 4 quarter-rounds to columns, then 4 to rows.
  # Columns and rows refer to the 4×4 matrix interpretation of the 16 words:
  #
  #    x0   x1   x2   x3
  #    x4   x5   x6   x7
  #    x8   x9  x10  x11
  #   x12  x13  x14  x15
  #
  # Column rounds operate on vertical slices; row rounds on horizontal slices.
  # 4 double-rounds = 8 total quarter-rounds applied to columns + 8 to rows.

  # rotl32/2 — rotate a 32-bit word left by n positions.
  #
  # A left rotation by n is: (x << n) | (x >> (32 - n)).
  # We mask to 32 bits because Elixir integers are arbitrary-precision.
  defp rotl32(x, n), do: band(bsl(x, n) ||| bsr(x, 32 - n), 0xFFFFFFFF)

  # add32/2 — addition modulo 2^32.
  defp add32(a, b), do: band(a + b, 0xFFFFFFFF)

  # salsa20_8/1 — apply the Salsa20/8 permutation to a 64-byte binary.
  #
  # Step 1: Decode 16 little-endian uint32 words from the 64-byte input.
  # Step 2: Apply 4 double-rounds (= 8 rounds).
  # Step 3: Add each output word to the corresponding initial word (mod 2^32).
  # Step 4: Re-encode as 64 bytes in little-endian order.
  defp salsa20_8(block) when byte_size(block) == 64 do
    # Decode: pattern-match 16 consecutive 32-bit LE words from the binary.
    # <<w::little-unsigned-32>> reads exactly 4 bytes as a little-endian uint32.
    <<
      w0::little-unsigned-32,
      w1::little-unsigned-32,
      w2::little-unsigned-32,
      w3::little-unsigned-32,
      w4::little-unsigned-32,
      w5::little-unsigned-32,
      w6::little-unsigned-32,
      w7::little-unsigned-32,
      w8::little-unsigned-32,
      w9::little-unsigned-32,
      w10::little-unsigned-32,
      w11::little-unsigned-32,
      w12::little-unsigned-32,
      w13::little-unsigned-32,
      w14::little-unsigned-32,
      w15::little-unsigned-32
    >> = block

    # Save the initial state so we can add it back at the end.
    # This "addition at the end" is what makes Salsa20 a permutation rather
    # than just a hash: it prevents output collisions.
    i = {w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15}

    # Apply 4 double-rounds. Each double-round does:
    #   1. Column round: 4 quarter-rounds on the 4 columns of the 4×4 matrix
    #   2. Row round:    4 quarter-rounds on the 4 rows of the 4×4 matrix
    #
    # We inline all 8 double-rounds as direct assignments for performance.
    # Elixir/Erlang pattern matching makes this efficient.
    {x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15} =
      do_salsa_rounds(w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15, 4)

    # Step 3: Add initial state to final state, word by word, mod 2^32.
    # This prevents the permutation from being inverted even if an attacker
    # learns the round outputs — they'd still need to subtract the unknown
    # initial state.
    {i0, i1, i2, i3, i4, i5, i6, i7, i8, i9, i10, i11, i12, i13, i14, i15} = i

    r0 = add32(x0, i0)
    r1 = add32(x1, i1)
    r2 = add32(x2, i2)
    r3 = add32(x3, i3)
    r4 = add32(x4, i4)
    r5 = add32(x5, i5)
    r6 = add32(x6, i6)
    r7 = add32(x7, i7)
    r8 = add32(x8, i8)
    r9 = add32(x9, i9)
    r10 = add32(x10, i10)
    r11 = add32(x11, i11)
    r12 = add32(x12, i12)
    r13 = add32(x13, i13)
    r14 = add32(x14, i14)
    r15 = add32(x15, i15)

    # Step 4: Encode 16 words back to little-endian bytes.
    <<
      r0::little-unsigned-32,
      r1::little-unsigned-32,
      r2::little-unsigned-32,
      r3::little-unsigned-32,
      r4::little-unsigned-32,
      r5::little-unsigned-32,
      r6::little-unsigned-32,
      r7::little-unsigned-32,
      r8::little-unsigned-32,
      r9::little-unsigned-32,
      r10::little-unsigned-32,
      r11::little-unsigned-32,
      r12::little-unsigned-32,
      r13::little-unsigned-32,
      r14::little-unsigned-32,
      r15::little-unsigned-32
    >>
  end

  # do_salsa_rounds/17 — apply n double-rounds recursively.
  #
  # Each double-round consists of:
  #   Column round: r(x0,x4,x8,x12), r(x5,x9,x13,x1), r(x10,x14,x2,x6), r(x15,x3,x7,x11)
  #   Row round:    r(x0,x1,x2,x3),  r(x5,x6,x7,x4),  r(x10,x11,x8,x9), r(x15,x12,x13,x14)
  #
  # The quarter-round macro r(a, b, c, d) modifies its 4 inputs:
  #   b' = b XOR rotl32(a + d, 7)
  #   c' = c XOR rotl32(b' + a, 9)
  #   d' = d XOR rotl32(c' + b', 13)
  #   a' = a XOR rotl32(d' + c', 18)
  defp do_salsa_rounds(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15, 0) do
    {x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15}
  end

  defp do_salsa_rounds(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15, n) do
    # ── Column round ──────────────────────────────────────────────────────────
    # Column 0: r(x0, x4, x8, x12)
    x4 = bxor(x4, rotl32(add32(x0, x12), 7))
    x8 = bxor(x8, rotl32(add32(x4, x0), 9))
    x12 = bxor(x12, rotl32(add32(x8, x4), 13))
    x0 = bxor(x0, rotl32(add32(x12, x8), 18))
    # Column 1: r(x5, x9, x13, x1)
    x9 = bxor(x9, rotl32(add32(x5, x1), 7))
    x13 = bxor(x13, rotl32(add32(x9, x5), 9))
    x1 = bxor(x1, rotl32(add32(x13, x9), 13))
    x5 = bxor(x5, rotl32(add32(x1, x13), 18))
    # Column 2: r(x10, x14, x2, x6)
    x14 = bxor(x14, rotl32(add32(x10, x6), 7))
    x2 = bxor(x2, rotl32(add32(x14, x10), 9))
    x6 = bxor(x6, rotl32(add32(x2, x14), 13))
    x10 = bxor(x10, rotl32(add32(x6, x2), 18))
    # Column 3: r(x15, x3, x7, x11)
    x3 = bxor(x3, rotl32(add32(x15, x11), 7))
    x7 = bxor(x7, rotl32(add32(x3, x15), 9))
    x11 = bxor(x11, rotl32(add32(x7, x3), 13))
    x15 = bxor(x15, rotl32(add32(x11, x7), 18))

    # ── Row round ─────────────────────────────────────────────────────────────
    # Row 0: r(x0, x1, x2, x3)
    x1 = bxor(x1, rotl32(add32(x0, x3), 7))
    x2 = bxor(x2, rotl32(add32(x1, x0), 9))
    x3 = bxor(x3, rotl32(add32(x2, x1), 13))
    x0 = bxor(x0, rotl32(add32(x3, x2), 18))
    # Row 1: r(x5, x6, x7, x4)
    x6 = bxor(x6, rotl32(add32(x5, x4), 7))
    x7 = bxor(x7, rotl32(add32(x6, x5), 9))
    x4 = bxor(x4, rotl32(add32(x7, x6), 13))
    x5 = bxor(x5, rotl32(add32(x4, x7), 18))
    # Row 2: r(x10, x11, x8, x9)
    x11 = bxor(x11, rotl32(add32(x10, x9), 7))
    x8 = bxor(x8, rotl32(add32(x11, x10), 9))
    x9 = bxor(x9, rotl32(add32(x8, x11), 13))
    x10 = bxor(x10, rotl32(add32(x9, x8), 18))
    # Row 3: r(x15, x12, x13, x14)
    x12 = bxor(x12, rotl32(add32(x15, x14), 7))
    x13 = bxor(x13, rotl32(add32(x12, x15), 9))
    x14 = bxor(x14, rotl32(add32(x13, x12), 13))
    x15 = bxor(x15, rotl32(add32(x14, x13), 18))

    do_salsa_rounds(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15, n - 1)
  end

  # ---------------------------------------------------------------------------
  # XOR helpers
  # ---------------------------------------------------------------------------
  #
  # :crypto.exor/2 XORs two same-length binaries efficiently in native code.
  # We use it for 64-byte block XOR (inside BlockMix) and for full-block XOR
  # (inside ROMix when XORing the current block with a table entry).

  defp xor_bytes(a, b), do: :crypto.exor(a, b)

  # xor_blocks/2 — XOR two equal-length sequences of 64-byte blocks.
  #
  # Both arguments are binaries of the same total length (128*r bytes).
  # We process them as raw binaries: :crypto.exor handles arbitrary lengths.
  defp xor_blocks(a, b), do: :crypto.exor(a, b)

  # ---------------------------------------------------------------------------
  # BlockMix (RFC 7914 §3)
  # ---------------------------------------------------------------------------
  #
  # BlockMix takes a list of 2r 64-byte blocks and returns a new list of 2r
  # 64-byte blocks. It is the building block of ROMix.
  #
  # Algorithm:
  #   X = last block of B
  #   for i = 0 to 2r-1:
  #     X = Salsa20/8(X XOR B[i])
  #     Y[i] = X
  #   return Y[0], Y[2], Y[4], ..., Y[1], Y[3], Y[5], ...
  #
  # The deinterleaving (even first, then odd) is what creates the specific
  # memory access pattern that makes scrypt's security analysis work.
  # Without the deinterleaving, BlockMix would be a simple chained cipher.
  #
  # Input/output: binary of exactly 2*r*64 bytes.
  defp block_mix(b_bytes, r) do
    block_count = 2 * r
    block_size = 64

    # Split the input binary into a list of 64-byte blocks.
    # binary_part(bin, offset, length) is O(1) in Erlang — it creates a
    # sub-binary view into the original memory without copying.
    blocks =
      for i <- 0..(block_count - 1)//1 do
        binary_part(b_bytes, i * block_size, block_size)
      end

    # X starts as the last block.
    x_init = List.last(blocks)

    # Apply the sequential Salsa20/8 chain, collecting output blocks in Y.
    # We thread X through a fold, accumulating Y blocks in reverse.
    {_x, y_rev} =
      Enum.reduce(blocks, {x_init, []}, fn block_i, {x, y_acc} ->
        # XOR X with the current input block, then permute with Salsa20/8.
        new_x = salsa20_8(xor_bytes(x, block_i))
        {new_x, [new_x | y_acc]}
      end)

    # Reverse to get Y in forward order.
    y = Enum.reverse(y_rev)

    # Deinterleave: even-indexed blocks first, then odd-indexed blocks.
    # Elixir list indexing is 0-based. Even: 0, 2, 4, ...; Odd: 1, 3, 5, ...
    evens = for i <- 0..(block_count - 1)//1, rem(i, 2) == 0, do: Enum.at(y, i)
    odds = for i <- 0..(block_count - 1)//1, rem(i, 2) == 1, do: Enum.at(y, i)

    # Concatenate all blocks back into a single binary.
    IO.iodata_to_binary(evens ++ odds)
  end

  # ---------------------------------------------------------------------------
  # Integerify (RFC 7914 §3)
  # ---------------------------------------------------------------------------
  #
  # Integerify extracts a 64-bit little-endian integer from the last 64 bytes
  # of the current block. This integer is used as the lookup index into the V
  # table in ROMix.
  #
  # Why the last 64 bytes? Because after BlockMix, the last 64 bytes are the
  # most recently computed Salsa20/8 output — the most "mixed" part of the
  # block. Using these bytes as an index ensures that the lookup depends on
  # the full history of previous operations.
  #
  # The RFC specifies: j = Integerify(X) mod N
  # where Integerify takes the first 64-bit LE word from the last 2r 64-byte
  # blocks of X. In practice, that's the first 8 bytes of the last 64-byte block.
  defp integerify(x_bytes) do
    last_block_offset = byte_size(x_bytes) - 64
    last_block = binary_part(x_bytes, last_block_offset, 64)
    # Read the first 8 bytes as a little-endian unsigned 64-bit integer.
    <<j::little-unsigned-64, _rest::binary>> = last_block
    j
  end

  # ---------------------------------------------------------------------------
  # ROMix (RFC 7914 §3)
  # ---------------------------------------------------------------------------
  #
  # ROMix is the memory-hard core of scrypt. "ROM" stands for "Random-access
  # memory Operating Mode" — a term from the scrypt paper.
  #
  # Algorithm:
  #   V[0] = B
  #   V[i] = BlockMix(V[i-1])  for i = 1..N-1
  #   X = V[N-1]
  #   for i = 0..N-1:
  #     j = Integerify(X) rem N
  #     X = BlockMix(X XOR V[j])
  #   return X
  #
  # The first loop fills a table of N BlockMix outputs.
  # The second loop makes N pseudo-random lookups into that table.
  #
  # Memory usage: N * 128 * r bytes. For N=16384, r=8: 16 MiB.
  # An attacker who wants to skip storing V must recompute missing entries on
  # demand — but each entry depends on the previous, so recomputation is
  # expensive. This is the "memory-hard" property.
  #
  # Input: b_bytes — 128*r bytes (one "block" in scrypt terminology)
  #        n — table size (must be a power of 2, ≥ 2)
  defp ro_mix(b_bytes, n) do
    r_val = div(byte_size(b_bytes), 128)

    # ── Phase 1: Build the V table ────────────────────────────────────────────
    # V[i] stores a snapshot of the block BEFORE the i-th BlockMix.
    # The loop runs from i=0 to N-1:
    #   V[0] = B (initial)
    #   V[1] = BlockMix(V[0])
    #   ...
    #   V[N-1] = BlockMix(V[N-2])
    #
    # After the loop completes, x = BlockMix(V[N-1]), which is used to start
    # Phase 2. We must NOT reset x to V[N-1] — that would be wrong.
    #
    # We use a Map for O(1) random-access lookups in Phase 2.
    # (A production implementation would use :array or ETS for less GC pressure.)
    {v, x_after_fill} =
      Enum.reduce(0..(n - 1)//1, {%{}, b_bytes}, fn i, {v_acc, prev} ->
        # Store the snapshot BEFORE mixing: V[i] = prev
        v_new = Map.put(v_acc, i, prev)
        # Advance: next = BlockMix(prev)
        next = block_mix(prev, r_val)
        {v_new, next}
      end)

    # x_after_fill = BlockMix(V[N-1])  — this is the correct starting point
    # for Phase 2. Do NOT reset to V[N-1].
    x_init = x_after_fill

    # ── Phase 2: Pseudo-random lookups ────────────────────────────────────────
    x_final =
      Enum.reduce(0..(n - 1)//1, x_init, fn _i, x ->
        j = rem(integerify(x), n)
        vj = Map.get(v, j)
        block_mix(xor_blocks(x, vj), r_val)
      end)

    x_final
  end

  # ---------------------------------------------------------------------------
  # Validation helpers
  # ---------------------------------------------------------------------------

  defp power_of_two?(n) when n < 2, do: false
  defp power_of_two?(n), do: band(n, n - 1) == 0

  defp validate!(password, salt, n, r, p, dk_len) do
    # Note: scrypt does NOT reject empty password — RFC 7914 test vector 1
    # uses empty password and empty salt. The internal PBKDF2 handles this.
    _ = password
    _ = salt

    unless is_integer(n) and power_of_two?(n),
      do: raise(ArgumentError, "scrypt N must be a power of 2 and >= 2")

    unless is_integer(r) and r >= 1,
      do: raise(ArgumentError, "scrypt r must be a positive integer")

    unless is_integer(p) and p >= 1,
      do: raise(ArgumentError, "scrypt p must be a positive integer")

    unless is_integer(dk_len) and dk_len >= 1 and dk_len <= 1_048_576,
      do: raise(ArgumentError, "scrypt dk_len must be between 1 and 2^20")

    if n > 1_048_576,
      do: raise(ArgumentError, "scrypt N must not exceed 2^20")

    if p * r > 1_073_741_824,
      do: raise(ArgumentError, "scrypt p * r exceeds limit")
  end

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Derive a key using scrypt (RFC 7914).

  ## Parameters

  - `password` — the passphrase (binary, may be empty for RFC compatibility)
  - `salt`     — random salt (binary, typically 16–32 bytes)
  - `n`        — CPU/memory cost factor (power of 2, ≥ 2, ≤ 2^20)
  - `r`        — block size factor (positive integer)
  - `p`        — parallelization factor (positive integer)
  - `dk_len`   — desired output length in bytes (1–2^20)

  Returns a binary of `dk_len` bytes.

  ## Constraints

  - `n` must be a power of 2 and ≥ 2
  - `r` must be a positive integer
  - `p` must be a positive integer
  - `dk_len` must be between 1 and 2^20 (1 048 576)
  - `p * r` must not exceed 2^30
  - `n` must not exceed 2^20

  ## Examples

      # RFC 7914 test vector 1 (verified against OpenSSL, Python hashlib.scrypt, Go x/crypto/scrypt)
      iex> CodingAdventures.Scrypt.scrypt("", "", 16, 1, 1, 64) |> Base.encode16(case: :lower)
      "77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906"

  """
  @spec scrypt(binary, binary, pos_integer, pos_integer, pos_integer, pos_integer) :: binary
  def scrypt(password, salt, n, r, p, dk_len)
      when is_binary(password) and is_binary(salt) do
    validate!(password, salt, n, r, p, dk_len)

    # ── Step 1: Derive p blocks of 128*r bytes using PBKDF2-HMAC-SHA256 ──────
    # Each block will independently go through ROMix. The salt ties the output
    # to this specific (password, salt) pair. We use 1 iteration of PBKDF2
    # because scrypt provides the computational cost itself via ROMix.
    block_len = 128 * r
    b_bytes = pbkdf2_sha256_raw(password, salt, 1, p * block_len)

    # ── Step 2: Run ROMix on each of the p blocks independently ───────────────
    # Each 128*r-byte chunk gets its own independent ROMix pass.
    # In production, these could run in parallel (the `p` parameter controls
    # parallelism). We run them sequentially here for simplicity.
    mixed =
      for i <- 0..(p - 1)//1 do
        chunk = binary_part(b_bytes, i * block_len, block_len)
        ro_mix(chunk, n)
      end

    b_mixed = IO.iodata_to_binary(mixed)

    # ── Step 3: Finalize using PBKDF2-HMAC-SHA256 ─────────────────────────────
    # The mixed blocks become the new "salt" in this second PBKDF2 call.
    # The password is used again to prevent the output from depending only on
    # the intermediate state — an attacker who learns b_mixed still cannot
    # derive the key without knowing the password.
    pbkdf2_sha256_raw(password, b_mixed, 1, dk_len)
  end

  @doc """
  Like `scrypt/6` but returns a lowercase hex string instead of raw bytes.

  ## Examples

      iex> CodingAdventures.Scrypt.scrypt_hex("", "", 16, 1, 1, 64)
      "77d6576238657b203b19ca42c18a0497f16b4844e3074ae8dfdffa3fede21442fcd0069ded0948f8326a753a0fc81f17e8d3e0fb2e0d3628cf35e20c38d18906"

  """
  @spec scrypt_hex(binary, binary, pos_integer, pos_integer, pos_integer, pos_integer) ::
          String.t()
  def scrypt_hex(password, salt, n, r, p, dk_len) do
    scrypt(password, salt, n, r, p, dk_len)
    |> Base.encode16(case: :lower)
  end
end
