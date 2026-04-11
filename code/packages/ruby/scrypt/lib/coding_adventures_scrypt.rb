# frozen_string_literal: true

# coding_adventures_scrypt — scrypt Password-Based Key Derivation Function
# RFC 7914 — implemented from scratch in Ruby.
#
# What Is scrypt?
# ================
# scrypt is a password-based key derivation function (PBKDF) designed by
# Colin Percival in 2009. It is intentionally expensive in both CPU time and
# memory, making brute-force and dictionary attacks impractical even with
# specialised hardware (GPUs, FPGAs, ASICs).
#
# The intuition: if deriving a single candidate password costs 1 GiB of RAM
# and 1 second of CPU time, an attacker can test at most a few candidates per
# second per gigabyte — far slower than simpler KDFs like bcrypt or PBKDF2.
#
# Parameters
# ===========
#   N  — CPU/memory cost factor. Must be a power of 2, e.g. 2^14 = 16384.
#        Higher N → more memory and time.
#   r  — Block size multiplier. Each "block" in RoMix is 2r × 64 bytes.
#        Higher r → wider memory access patterns, harder to pipeline.
#   p  — Parallelisation factor. Each of the p "lanes" is processed
#        independently by RoMix before the final PBKDF2 step.
#        Higher p → more work, but can be split across cores.
#   dk_len — Desired output key length in bytes.
#
# The recommended interactive defaults (Colin Percival's 2009 paper):
#   N=16384, r=8, p=1  → ~16 MiB RAM, ~0.5 s on 2009 hardware.
# The Argon2 family has since superseded scrypt in new designs, but scrypt
# is still widely used (e.g., Litecoin, Tarsnap, macOS FileVault 2).
#
# How scrypt Works (Big Picture)
# ================================
#   1. PBKDF2-HMAC-SHA256 expands (password, salt) into p×128r bytes — one
#      128r-byte "block" per parallel lane.
#   2. For each lane, RoMix fills a table of N block snapshots, then makes N
#      pseudo-random lookups into that table, mixing the lane block each time.
#      This is the memory-hard step: you either keep the table in RAM (fast)
#      or recompute each entry when needed (slow). Neither is cheap.
#   3. PBKDF2-HMAC-SHA256 again, this time using the scrambled lanes as the
#      salt, to produce the final output key.
#
# Diagram
# ========
#
#   password, salt
#        │
#        ▼ PBKDF2-HMAC-SHA256 (1 iteration)
#   ┌────────────────────────────────┐
#   │  B[0] │ B[1] │ … │ B[p-1]    │  (p × 128r bytes)
#   └────────────────────────────────┘
#        │
#        ▼ RoMix for each B[i]     (memory-hard)
#   ┌────────────────────────────────┐
#   │  B'[0]│ B'[1]│ … │ B'[p-1]  │
#   └────────────────────────────────┘
#        │
#        ▼ PBKDF2-HMAC-SHA256 (1 iteration)
#   derived key (dk_len bytes)
#
# RFC 7914 § Test Vectors
# ========================
# Vector 1:  scrypt("", "", N=16, r=1, p=1, dk_len=64)
#   → 77d6576238657b203b19ca42c18a0497
#     f16b4844e3074ae8dfdffa3fede21442
#     fcd0069ded0948f8326a753a0fc81f17
#     e8d3e0fb2e0d3628cf35e20c38d18906
#   (verified against OpenSSL::KDF.scrypt and Python hashlib.scrypt)
#
# Vector 2:  scrypt("password", "NaCl", N=1024, r=8, p=16, dk_len=64)
#   → fdbabe1c9d3472007856e7190d01e9fe
#     7c6ad7cbc8237830e77376634b373162
#     2eaf30d92e22a3886ff109279d9830da
#     c727afb94a83ee6d8360cbdfa2cc0640
#   (verified against OpenSSL::KDF.scrypt and Python hashlib.scrypt)
#
# IMPORTANT — Empty Password
# ===========================
# RFC vector 1 uses an empty-string password (""). Our HMAC gem rejects empty
# keys as a security measure. To support the RFC vectors correctly, this gem
# implements PBKDF2-HMAC-SHA256 inline, calling the lower-level
# CodingAdventures::Hmac.hmac directly (which accepts any key length) rather
# than the public CodingAdventures::Hmac.hmac_sha256 wrapper.
#
# IMPORTANT — Binary String Encoding
# =====================================
# Ruby distinguishes UTF-8 and ASCII-8BIT (binary) encodings and raises
# Encoding::CompatibilityError when mixing them via concatenation. All strings
# in this implementation are forced to binary encoding with `.b` to avoid
# surprises. Input strings are coerced at function boundaries.

require "coding_adventures_hmac"
require_relative "coding_adventures/scrypt/version"

module CodingAdventures
  module Scrypt
    module_function

    # ─── Public API ──────────────────────────────────────────────────────────

    # Derive a key using scrypt (RFC 7914).
    #
    # @param password [String] the secret passphrase (may be empty — RFC 7914 vector 1)
    # @param salt     [String] random salt (binary)
    # @param n        [Integer] CPU/memory cost factor — must be a power of 2, >= 2
    # @param r        [Integer] block size factor — positive integer
    # @param p        [Integer] parallelisation factor — positive integer
    # @param dk_len   [Integer] desired output length in bytes
    # @return [String] derived key as a binary String of dk_len bytes
    #
    # @example Interactive login (light parameters)
    #   key = CodingAdventures::Scrypt.scrypt("hunter2", "random_salt", 16384, 8, 1, 32)
    #
    # @example RFC 7914 vector 1
    #   hex = CodingAdventures::Scrypt.scrypt_hex("", "", 16, 1, 1, 64)
    #   # => "77d6576238657b20..."
    def scrypt(password, salt, n, r, p, dk_len)
      # ── Parameter validation ─────────────────────────────────────────────
      # N must be a power of 2 and at least 2.
      # A power of 2 satisfies: n & (n-1) == 0. We exclude 1 (= 2^0) because
      # RoMix requires at least two entries in the table V.
      raise ArgumentError, "scrypt N must be a power of 2 and >= 2" if n < 2 || (n & (n - 1)) != 0

      # N > 2^20 would require >= 128 MiB RAM even for r=1. We cap at 2^20 to
      # prevent accidental denial-of-service in production usage.
      raise ArgumentError, "scrypt N must not exceed 2^20" if n > 2**20

      raise ArgumentError, "scrypt r must be a positive integer" if r < 1
      raise ArgumentError, "scrypt p must be a positive integer" if p < 1
      raise ArgumentError, "scrypt dk_len must be between 1 and 2^20" if dk_len < 1 || dk_len > 2**20

      # p * r must not overflow the block-buffer calculation. RFC 7914 §2
      # requires p * r < 2^30.
      raise ArgumentError, "scrypt p * r exceeds limit" if p * r > 2**30

      # Coerce inputs to binary encoding once at the boundary
      password = password.b
      salt = salt.b

      # ── Step 1: Expand using PBKDF2-HMAC-SHA256 ──────────────────────────
      # Produce p×128r bytes total (one 128r-byte lane per parallelisation unit).
      # Iteration count = 1 because scrypt provides its own memory-hard mixing.
      b = pbkdf2_sha256_raw(password, salt, 1, p * 128 * r)

      # ── Step 2: RoMix each lane (memory-hard mixing) ──────────────────────
      # Each 128r-byte lane is processed independently by ro_mix.
      # RoMix fills an N-entry lookup table V, then makes N random lookups into
      # V, accumulating a pseudo-random walk through all N entries.
      p.times do |i|
        chunk = b[i * 128 * r, 128 * r]
        b[i * 128 * r, 128 * r] = ro_mix(chunk, n, r)
      end

      # ── Step 3: Compress using PBKDF2-HMAC-SHA256 ────────────────────────
      # Use the scrambled lanes as the "salt" to produce the final key.
      pbkdf2_sha256_raw(password, b, 1, dk_len)
    end

    # Derive a key using scrypt, returned as a lowercase hex string.
    #
    # All parameters are identical to scrypt/6. Useful for logging and
    # comparison with published test vectors.
    #
    # @return [String] dk_len*2 character lowercase hex string
    def scrypt_hex(password, salt, n, r, p, dk_len)
      scrypt(password, salt, n, r, p, dk_len).unpack1("H*")
    end

    # ─── Private Helpers ─────────────────────────────────────────────────────

    # PBKDF2-HMAC-SHA256 (RFC 8018 §5.2), implemented inline.
    #
    # Why inline? Our HMAC gem's public `hmac_sha256` method rejects empty keys
    # as a security guard — but RFC 7914 vector 1 uses an empty password.
    # By calling `CodingAdventures::Hmac.hmac` directly we bypass that guard
    # while still using the vetted HMAC primitive.
    #
    # PBKDF2 structure:
    #   For each 1-indexed block number i:
    #     seed = salt || i  (i encoded as big-endian uint32)
    #     U1   = HMAC(password, seed)
    #     U2   = HMAC(password, U1)
    #     ...
    #     Ui   = HMAC(password, U{i-1})
    #     Ti   = U1 XOR U2 XOR ... XOR Ui
    #   DK = T1 || T2 || ... (truncated to key_length bytes)
    #
    # With iterations=1, Ti = U1 = HMAC(password, seed). This is exactly what
    # scrypt calls internally for both the expand and compress phases.
    #
    # @param password   [String] binary passphrase (may be empty)
    # @param salt       [String] binary salt
    # @param iterations [Integer] number of PRF applications per block (>=1)
    # @param key_length [Integer] desired output length in bytes
    # @return [String] binary String of exactly key_length bytes
    #
    # @note h_len = 32 for SHA-256 (256-bit digest → 32 bytes)
    private def pbkdf2_sha256_raw(password, salt, iterations, key_length)
      # SHA-256 produces 32-byte digests.
      h_len = 32

      # How many 32-byte PBKDF2 blocks do we need to cover key_length bytes?
      num_blocks = (key_length.to_f / h_len).ceil
      dk = "".b

      num_blocks.times do |i|
        # Block numbers are 1-indexed per RFC 8018.
        block_num = i + 1

        # seed = salt || block_num (block_num packed as 4-byte big-endian).
        # "N" pack code = unsigned 32-bit big-endian integer.
        seed = salt.b + [block_num].pack("N")

        # U1 = HMAC-SHA256(password, seed)
        u = hmac_sha256_bytes(password, seed)
        t = u.dup

        # For iterations > 1, XOR successive HMAC outputs together.
        # With iterations=1 this loop body never executes; Ti = U1.
        (iterations - 1).times do
          u = hmac_sha256_bytes(password, u)
          t = t.bytes.zip(u.bytes).map { |a, b| a ^ b }.pack("C*")
        end

        dk << t
      end

      # Truncate to exactly key_length bytes (the last block may be oversized).
      dk[0, key_length]
    end

    # Compute HMAC-SHA256 and return the result as a 32-byte binary String.
    #
    # This calls CodingAdventures::Hmac.hmac directly — the lower-level method
    # that does NOT enforce the non-empty key guard — so empty passwords work.
    # The SHA-256 lambda matches the signature expected by Hmac.hmac: it accepts
    # a binary String and returns a binary String.
    #
    # @param key [String] binary HMAC key (may be empty)
    # @param msg [String] binary message
    # @return [String] 32-byte binary authentication tag
    private def hmac_sha256_bytes(key, msg)
      sha256_fn = ->(d) { CodingAdventures::Sha256.sha256(d) }
      CodingAdventures::Hmac.hmac(sha256_fn, 64, key.b, msg.b)
    end

    # ─── Salsa20/8 Core ──────────────────────────────────────────────────────
    #
    # Salsa20/8 is the hash function at the heart of scrypt's BlockMix step.
    # It is the Salsa20 stream cipher's core function, reduced from 20 rounds
    # to 8 rounds for speed, applied to a fixed 64-byte block.
    #
    # Salsa20 uses only three operations: 32-bit addition, XOR, and left
    # rotation. These are cheap, reversible in principle (but not easily in
    # practice at 8 rounds), and produce excellent avalanche.
    #
    # A 64-byte block is viewed as a 4×4 matrix of little-endian uint32s:
    #
    #    x0   x1   x2   x3
    #    x4   x5   x6   x7
    #    x8   x9  x10  x11
    #   x12  x13  x14  x15
    #
    # "8 rounds" = 4 double-rounds. Each double-round applies quarter-rounds
    # first down the four columns, then across the four rows. After 8 rounds
    # the original values z[] are added back (as in the ChaCha family) to
    # prevent the function from being "inverted" by just running it backwards.
    #
    # Why add back z[]?
    # This additive feedback — called the "add-then-mix" or Feistel-like
    # structure — ensures Salsa20/8 is a pseudo-random function rather than a
    # permutation. Without it, given the output you could recover the input by
    # running 8 inverse rounds.
    #
    # IMPORTANT: All arithmetic is mod 2^32.
    # Ruby integers are arbitrary-precision ("Bignum"). After every add or
    # rotate we apply `& 0xFFFFFFFF` to keep values within uint32 range.

    # Left-rotate a 32-bit value x by n positions.
    #
    # Rotation wraps the high bits back around to the low end:
    #
    #   rotl32(0b11000000_00000000_00000000_00000000, 1)
    #   = 0b10000000_00000000_00000000_00000001
    #
    # The `& 0xFFFFFFFF` masks off any bits that crept above bit 31 due to
    # Ruby's arbitrary-precision shift.
    #
    # @param x [Integer] 32-bit value (no bits above bit 31)
    # @param n [Integer] rotation amount, 0..31
    # @return [Integer] rotated 32-bit value
    private def rotl32(x, n)
      ((x << n) | (x >> (32 - n))) & 0xFFFFFFFF
    end

    # Apply the Salsa20 quarter-round to four elements of array x at indices
    # a, b, c, d.
    #
    # The quarter-round mixes four 32-bit words together:
    #
    #   b ^= rotl32(a + d,  7)
    #   c ^= rotl32(b + a,  9)
    #   d ^= rotl32(c + b, 13)
    #   a ^= rotl32(d + c, 18)
    #
    # Each word feeds into the rotation of the next, creating a cascade of
    # dependencies that thoroughly mixes all four words. The &= 0xFFFFFFFF
    # guards keep all values within uint32 range in Ruby.
    #
    # The indices a, b, c, d refer to positions in the 4×4 matrix, chosen to
    # ensure that each quarter-round touches one element per matrix row and
    # one per column — a Latin-square-like design for maximal diffusion.
    #
    # @param x [Array<Integer>] 16-element array of uint32s (mutated in place)
    # @param a, b, c, d [Integer] indices into x
    private def quarter_round(x, a, b, c, d)
      x[b] ^= rotl32((x[a] + x[d]) & 0xFFFFFFFF, 7)
      x[c] ^= rotl32((x[b] + x[a]) & 0xFFFFFFFF, 9)
      x[d] ^= rotl32((x[c] + x[b]) & 0xFFFFFFFF, 13)
      x[a] ^= rotl32((x[d] + x[c]) & 0xFFFFFFFF, 18)
      x[b] &= 0xFFFFFFFF
      x[c] &= 0xFFFFFFFF
      x[d] &= 0xFFFFFFFF
      x[a] &= 0xFFFFFFFF
    end

    # Salsa20/8 hash: apply 8 rounds of Salsa20 to a 64-byte block.
    #
    # Procedure:
    #   1. Unpack the 64 input bytes as 16 little-endian uint32s → x[] (working copy)
    #      and save a copy → z[] (for the final addition).
    #   2. Run 4 double-rounds:
    #      - Column round: quarter-round on each column of the 4×4 matrix.
    #      - Row round:    quarter-round on each row.
    #   3. Add back z[]: result[i] = (x[i] + z[i]) mod 2^32.
    #   4. Repack as 16 little-endian uint32s → 64-byte String.
    #
    # Column indices (4×4 matrix laid flat, column-major order):
    #   col0: 0,4,8,12   col1: 5,9,13,1   col2: 10,14,2,6   col3: 15,3,7,11
    # Row indices:
    #   row0: 0,1,2,3    row1: 5,6,7,4    row2: 10,11,8,9   row3: 15,12,13,14
    #
    # Note the non-sequential indices in row rounds — this is the original
    # Salsa20 design, not a typo.
    #
    # @param input [String] 64-byte binary String
    # @return [String] 64-byte binary String (Salsa20/8 output)
    private def salsa20_8(input)
      # Unpack 16 little-endian 32-bit unsigned integers.
      # "V16" = 16 × unsigned 32-bit little-endian (Intel byte order).
      x = input.unpack("V16")
      z = x.dup  # save original for final addition

      4.times do
        # ── Column rounds ──────────────────────────────────────────────────
        quarter_round(x, 0, 4, 8, 12)   # column 0
        quarter_round(x, 5, 9, 13, 1)   # column 1
        quarter_round(x, 10, 14, 2, 6)  # column 2
        quarter_round(x, 15, 3, 7, 11)  # column 3

        # ── Row rounds ─────────────────────────────────────────────────────
        quarter_round(x, 0, 1, 2, 3)    # row 0
        quarter_round(x, 5, 6, 7, 4)    # row 1
        quarter_round(x, 10, 11, 8, 9)  # row 2
        quarter_round(x, 15, 12, 13, 14) # row 3
      end

      # Final add-back: prevents Salsa20/8 from being a simple permutation.
      result = x.zip(z).map { |xi, zi| (xi + zi) & 0xFFFFFFFF }

      # Repack as 16 little-endian uint32s → 64 bytes.
      result.pack("V16")
    end

    # XOR two 64-byte binary Strings byte-by-byte.
    #
    # In RoMix, XOR is used to mix the current working block with a snapshot
    # from the lookup table V. This is the "lookup and mix" step that forces an
    # attacker to access pseudo-random memory locations.
    #
    # @param a [String] 64-byte binary String
    # @param b [String] 64-byte binary String
    # @return [String] 64-byte binary String (a XOR b)
    private def xor64(a, b)
      a.bytes.zip(b.bytes).map { |ai, bi| ai ^ bi }.pack("C*")
    end

    # ─── BlockMix ────────────────────────────────────────────────────────────
    #
    # BlockMix takes a sequence of 2r 64-byte sub-blocks, applies Salsa20/8
    # to each (XOR'd with the previous output), then interleaves the results:
    # even-indexed outputs first, odd-indexed outputs second.
    #
    # The interleaving ensures that successive calls to BlockMix create
    # long-range dependencies: a sub-block in the second half of the output
    # depends on a sub-block from the first half of the same input. This
    # breaks simple sequential memory access patterns, increasing cache pressure.
    #
    # Procedure (for 2r sub-blocks):
    #   x = last sub-block (blocks[2r-1])
    #   for i in 0..(2r-1):
    #     x = Salsa20/8(x XOR blocks[i])
    #     y[i] = x
    #   output = [y[0], y[2], y[4], ..., y[2r-2], y[1], y[3], ..., y[2r-1]]
    #
    # The output is a new Array of 2r 64-byte Strings.
    #
    # @param blocks [Array<String>] array of 2r binary Strings, each 64 bytes
    # @param r      [Integer] block size factor
    # @return [Array<String>] mixed array of 2r binary Strings, each 64 bytes
    private def block_mix(blocks, r)
      two_r = 2 * r

      # Start with the last block (feeds into the first XOR).
      x = blocks[two_r - 1].dup
      y = []

      two_r.times do |i|
        x = salsa20_8(xor64(x, blocks[i]))
        y << x.dup
      end

      # Interleave: even indices (first half), then odd indices (second half).
      # r.times.map creates [0, 1, ..., r-1], giving indices [0, 2, 4, ..., 2r-2]
      # and [1, 3, 5, ..., 2r-1].
      r.times.map { |i| y[2 * i] } + r.times.map { |i| y[2 * i + 1] }
    end

    # ─── Integerify ──────────────────────────────────────────────────────────
    #
    # Convert the first 8 bytes of the last block in a 2r-block sequence into
    # an unsigned 64-bit little-endian integer. This integer is used as an
    # index into the lookup table V, creating pseudo-random memory accesses.
    #
    # Why little-endian? The blocks are maintained in Salsa20's native little-
    # endian format throughout RoMix. Integerify reads from the same layout.
    #
    # "Q<" unpack code: Q = unsigned 64-bit, < = little-endian.
    #
    # @param x [Array<String>] array of 2r 64-byte Strings
    # @return [Integer] unsigned 64-bit integer derived from the last block
    private def integerify(x)
      last = x.last
      last[0, 8].unpack1("Q<")
    end

    # ─── RoMix ───────────────────────────────────────────────────────────────
    #
    # RoMix is the memory-hard core of scrypt.
    #
    # Phase 1 — Fill: Generate N snapshots of the block's evolution under
    # repeated BlockMix. Store them in the lookup table V[0..N-1].
    #
    #   V[0]   = X
    #   V[1]   = BlockMix(V[0])
    #   V[2]   = BlockMix(V[1])
    #   ...
    #   V[N-1] = BlockMix(V[N-2])
    #   X      = BlockMix(V[N-1])  (X is now BlockMix^N of the input)
    #
    # Phase 2 — Mix: Make N pseudo-random lookups into V. Each lookup index j
    # is derived from Integerify(X) mod N — a function of the current state,
    # so the access pattern is not known in advance.
    #
    #   for k in 1..N:
    #     j = Integerify(X) mod N
    #     X = BlockMix(X XOR V[j])
    #
    # An attacker who cannot store V must recompute V[j] from scratch for each
    # lookup — requiring O(N^2) work instead of O(N). An attacker who stores V
    # but can only process one lane uses O(N) memory. Neither is cheap at large N.
    #
    # @param b_bytes [String] 128r-byte binary String (one scrypt "block")
    # @param n       [Integer] cost factor (number of table entries)
    # @param r       [Integer] block size factor
    # @return [String] 128r-byte binary String (mixed output)
    private def ro_mix(b_bytes, n, r)
      two_r = 2 * r

      # Split the flat 128r-byte string into 2r individual 64-byte sub-blocks.
      x = two_r.times.map { |i| b_bytes[i * 64, 64].dup }

      # ── Phase 1: Fill lookup table V ──────────────────────────────────────
      v = []
      n.times do
        v << x.map(&:dup)      # snapshot current state
        x = block_mix(x, r)   # advance one step
      end

      # ── Phase 2: Pseudo-random lookups ────────────────────────────────────
      n.times do
        j = integerify(x) % n
        # XOR each sub-block of X with the corresponding sub-block of V[j].
        xored = x.zip(v[j]).map { |xi, vji| xor64(xi, vji) }
        x = block_mix(xored, r)
      end

      # Flatten the 2r sub-blocks back into a single binary String.
      x.join
    end

    # Mark all helpers as private (module_function copies them as public module
    # methods AND private instance methods; `private` in a module_function
    # context applies to instance methods only, so we also use private def above).
    private :pbkdf2_sha256_raw, :hmac_sha256_bytes, :rotl32, :quarter_round,
      :salsa20_8, :xor64, :block_mix, :integerify, :ro_mix
  end
end
