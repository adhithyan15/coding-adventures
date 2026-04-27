# frozen_string_literal: true

# =============================================================================
# coding_adventures_aes — AES block cipher (FIPS 197)
# =============================================================================
#
# AES (Advanced Encryption Standard) is the most widely deployed symmetric
# encryption algorithm in the world. Published by NIST in 2001 as FIPS 197,
# it replaced DES and is used in TLS/HTTPS, WPA2/WPA3 WiFi, disk encryption
# (BitLocker, LUKS, FileVault), VPNs, and virtually every secure protocol.
#
# Designed by Joan Daemen and Vincent Rijmen (Rijndael), AES is a
# Substitution-Permutation Network (SPN) — a fundamentally different structure
# from DES's Feistel network. All bytes of the state are transformed every round.
#
# Architecture
# ─────────────
#
#   plaintext (16 bytes)
#        │
#   AddRoundKey(state, round_key[0])       ← XOR with first key material
#        │
#   ┌── Nr-1 full rounds ──────────────────────────────────────────────┐
#   │   SubBytes   — non-linear S-box substitution (GF(2^8) inverse)  │
#   │   ShiftRows  — cyclic row shifts (diffusion across columns)      │
#   │   MixColumns — GF(2^8) matrix multiply (diffusion across rows)  │
#   │   AddRoundKey — XOR with round key                              │
#   └───────────────────────────────────────────────────────────────────┘
#        │
#   SubBytes + ShiftRows + AddRoundKey    ← final round (no MixColumns)
#        │
#   ciphertext (16 bytes)
#
# The state is a 4×4 matrix of bytes, indexed state[row][col].
# Bytes are loaded column-major: state[row][col] = block[row + 4*col].
#
# GF(2^8) Connection
# ──────────────────
# AES uses GF(2^8) with polynomial p(x) = x^8 + x^4 + x^3 + x + 1 = 0x11B.
# This package includes an inline GF(2^8) field implementation using Russian
# peasant multiplication, which works for any irreducible polynomial without
# requiring a specific primitive generator.
#
# Key Sizes and Round Counts
# ──────────────────────────
#   Key size   Nk (words)   Nr (rounds)   Round keys
#   128 bits      4             10          11 × 16 bytes
#   192 bits      6             12          13 × 16 bytes
#   256 bits      8             14          15 × 16 bytes
#
# Public API
# ──────────
#   CodingAdventures::Aes.aes_encrypt_block(block, key) → 16-byte binary String
#   CodingAdventures::Aes.aes_decrypt_block(block, key) → 16-byte binary String
#   CodingAdventures::Aes.expand_key(key)               → Array of round-key matrices
#   CodingAdventures::Aes::SBOX     — 256-entry Array
#   CodingAdventures::Aes::INV_SBOX — 256-entry Array

require_relative "coding_adventures/aes/version"

module CodingAdventures
  module Aes
    # ─────────────────────────────────────────────────────────────────────────
    # GF(2^8) field with polynomial 0x11B for AES arithmetic.
    #
    # Uses Russian peasant (shift-and-XOR) multiplication, which works for any
    # irreducible polynomial without needing a specific primitive generator.
    # (Log/antilog tables require g=2 to be primitive; g=2 is NOT primitive for
    # 0x11B — AES uses g=0x03 per FIPS 197 §4.1.)
    # ─────────────────────────────────────────────────────────────────────────

    # AES irreducible polynomial: x^8 + x^4 + x^3 + x + 1 = 0x11B.
    AES_POLYNOMIAL = 0x11B
    # Low byte of polynomial used in reduction step.
    AES_REDUCE = AES_POLYNOMIAL & 0xFF

    # Multiply two bytes in GF(2^8) with AES polynomial 0x11B.
    #
    # Russian peasant algorithm:
    #   For each bit of b (LSB first):
    #     if bit set: result ^= a
    #     carry = a & 0x80; a = (a << 1) & 0xFF
    #     if carry: a ^= AES_REDUCE
    #
    # @param a [Integer] field element (0..255)
    # @param b [Integer] field element (0..255)
    # @return [Integer] a * b in GF(2^8) with polynomial 0x11B
    def self.gf_mul(a, b)
      result = 0
      aa = a
      8.times do
        result ^= aa if (b & 1) != 0
        hi = aa & 0x80
        aa = (aa << 1) & 0xFF
        aa ^= AES_REDUCE if hi != 0
        b >>= 1
      end
      result
    end

    # Raise base to exp via repeated squaring in GF(2^8).
    #
    # @param base [Integer] field element
    # @param exp [Integer] non-negative integer exponent
    # @return [Integer] base^exp in GF(2^8)
    def self.gf_pow(base, exp)
      return exp == 0 ? 1 : 0 if base == 0
      return 1 if exp == 0

      result = 1
      b = base
      e = exp
      while e > 0
        result = gf_mul(result, b) if (e & 1) != 0
        b = gf_mul(b, b)
        e >>= 1
      end
      result
    end

    # Multiplicative inverse in GF(2^8): a^(-1) = a^254 (since a^255 = 1).
    #
    # @param a [Integer] field element (must not be 0)
    # @return [Integer] multiplicative inverse
    def self.gf_inverse(a)
      raise ArgumentError, "GF: zero has no multiplicative inverse" if a == 0

      gf_pow(a, 254)
    end

    # ─────────────────────────────────────────────────────────────────────────
    # S-box and inverse S-box generation
    #
    # SubBytes maps each byte b to:
    #   1. inv = b^{-1} in GF(2^8)   (0 maps to 0)
    #   2. Affine transform: s_i = b_i ^ b_{(i+4)%8} ^ b_{(i+5)%8} ^ b_{(i+6)%8} ^ b_{(i+7)%8} ^ c_i
    #      where c = 0x63
    #
    # The affine constant 0x63 ensures no fixed points: SBOX[b] != b for any b.
    # ─────────────────────────────────────────────────────────────────────────

    # AES affine transformation over GF(2).
    # Applied bit-by-bit:
    #   s_i = b_i XOR b_{(i+4)%8} XOR b_{(i+5)%8} XOR b_{(i+6)%8} XOR b_{(i+7)%8} XOR c_i
    # where c = 0x63 = 01100011.
    #
    # @param b [Integer] input byte (0..255)
    # @return [Integer] transformed byte
    def self.affine_transform(b)
      result = 0
      8.times do |i|
        bit = ((b >> i) & 1) ^
              ((b >> ((i + 4) % 8)) & 1) ^
              ((b >> ((i + 5) % 8)) & 1) ^
              ((b >> ((i + 6) % 8)) & 1) ^
              ((b >> ((i + 7) % 8)) & 1) ^
              ((0x63 >> i) & 1)
        result |= (bit << i)
      end
      result
    end

    # Build the AES S-box and inverse S-box at module load time.
    _sbox = Array.new(256, 0)
    256.times do |b|
      inv = b == 0 ? 0 : gf_pow(b, 254)
      _sbox[b] = affine_transform(inv)
    end
    _inv_sbox = Array.new(256, 0)
    256.times { |b| _inv_sbox[_sbox[b]] = b }

    # AES S-box: 256-entry substitution table.
    # Spot-checks (FIPS 197 Figure 7): SBOX[0x00]=0x63, SBOX[0x01]=0x7c, SBOX[0xff]=0x16.
    SBOX = _sbox.freeze

    # Inverse S-box: INV_SBOX[SBOX[b]] = b for all b.
    INV_SBOX = _inv_sbox.freeze

    # ─────────────────────────────────────────────────────────────────────────
    # Round constants (Rcon) for the key schedule
    #
    # Rcon[i] = 2^{i-1} in GF(2^8) for i = 1..14.
    # Used to break symmetry so no two round keys are equal.
    # ─────────────────────────────────────────────────────────────────────────

    _rcon = [0x00]  # index 0 unused; NIST is 1-indexed
    _val = 1
    14.times do
      _rcon << _val
      _val = gf_mul(_val, 0x02)
    end
    RCON = _rcon.freeze

    # ─────────────────────────────────────────────────────────────────────────
    # MixColumns helpers
    #
    # Each column of the 4×4 state is treated as a polynomial in GF(2^8) and
    # multiplied by the AES MixColumns matrix:
    #
    #   [2 3 1 1]   [s0]
    #   [1 2 3 1] × [s1]
    #   [1 1 2 3]   [s2]
    #   [3 1 1 2]   [s3]
    #
    # InvMixColumns uses the inverse matrix:
    #   [14 11 13  9]
    #   [ 9 14 11 13]
    #   [13  9 14 11]
    #   [11 13  9 14]
    # ─────────────────────────────────────────────────────────────────────────

    # Apply MixColumns to one 4-byte column.
    # 2·x = gf_mul(x, 2), 3·x = gf_mul(x, 2) ^ x
    def self.mix_col(col)
      s0, s1, s2, s3 = col
      t0 = gf_mul(s0, 2) ^ (gf_mul(s1, 2) ^ s1) ^ s2 ^ s3
      t1 = s0 ^ gf_mul(s1, 2) ^ (gf_mul(s2, 2) ^ s2) ^ s3
      t2 = s0 ^ s1 ^ gf_mul(s2, 2) ^ (gf_mul(s3, 2) ^ s3)
      t3 = (gf_mul(s0, 2) ^ s0) ^ s1 ^ s2 ^ gf_mul(s3, 2)
      [t0, t1, t2, t3]
    end

    # Apply InvMixColumns to one 4-byte column.
    # Coefficients: 14=0x0e, 11=0x0b, 13=0x0d, 9=0x09
    def self.inv_mix_col(col)
      s0, s1, s2, s3 = col
      t0 = gf_mul(0x0e, s0) ^ gf_mul(0x0b, s1) ^ gf_mul(0x0d, s2) ^ gf_mul(0x09, s3)
      t1 = gf_mul(0x09, s0) ^ gf_mul(0x0e, s1) ^ gf_mul(0x0b, s2) ^ gf_mul(0x0d, s3)
      t2 = gf_mul(0x0d, s0) ^ gf_mul(0x09, s1) ^ gf_mul(0x0e, s2) ^ gf_mul(0x0b, s3)
      t3 = gf_mul(0x0b, s0) ^ gf_mul(0x0d, s1) ^ gf_mul(0x09, s2) ^ gf_mul(0x0e, s3)
      [t0, t1, t2, t3]
    end

    # ─────────────────────────────────────────────────────────────────────────
    # State helpers: bytes ↔ 4×4 matrix
    # ─────────────────────────────────────────────────────────────────────────

    # Convert a 16-byte binary String to a 4×4 AES state (column-major).
    #
    # state[row][col] = block[row + 4*col]
    #
    # AES loads bytes column by column:
    #   block[0]  block[4]  block[8]  block[12]
    #   block[1]  block[5]  block[9]  block[13]
    #   block[2]  block[6]  block[10] block[14]
    #   block[3]  block[7]  block[11] block[15]
    def self.bytes_to_state(block)
      bytes = block.bytes
      [
        [bytes[0],  bytes[4],  bytes[8],  bytes[12]],
        [bytes[1],  bytes[5],  bytes[9],  bytes[13]],
        [bytes[2],  bytes[6],  bytes[10], bytes[14]],
        [bytes[3],  bytes[7],  bytes[11], bytes[15]],
      ]
    end

    # Convert a 4×4 AES state back to a 16-byte binary String (column-major).
    def self.state_to_bytes(state)
      result = Array.new(16)
      4.times do |col|
        4.times do |row|
          result[row + 4 * col] = state[row][col]
        end
      end
      result.pack("C*")
    end

    def self.add_round_key(state, round_key)
      state.each_with_index.map do |row, r|
        row.each_with_index.map { |b, c| b ^ round_key[r][c] }
      end
    end

    def self.sub_bytes(state)
      state.map { |row| row.map { |b| SBOX[b] } }
    end

    def self.inv_sub_bytes(state)
      state.map { |row| row.map { |b| INV_SBOX[b] } }
    end

    # ShiftRows: cyclically shift row i left by i positions.
    # Row 0: no shift; Row 1: left 1; Row 2: left 2; Row 3: left 3.
    def self.shift_rows(state)
      [
        state[0].dup,
        [state[1][1], state[1][2], state[1][3], state[1][0]],
        [state[2][2], state[2][3], state[2][0], state[2][1]],
        [state[3][3], state[3][0], state[3][1], state[3][2]],
      ]
    end

    # InvShiftRows: shift row i right by i positions.
    def self.inv_shift_rows(state)
      [
        state[0].dup,
        [state[1][3], state[1][0], state[1][1], state[1][2]],
        [state[2][2], state[2][3], state[2][0], state[2][1]],
        [state[3][1], state[3][2], state[3][3], state[3][0]],
      ]
    end

    # Apply MixColumns to each of the 4 columns.
    def self.mix_columns(state)
      result = Array.new(4) { Array.new(4, 0) }
      4.times do |col|
        column = [state[0][col], state[1][col], state[2][col], state[3][col]]
        mixed = mix_col(column)
        4.times { |row| result[row][col] = mixed[row] }
      end
      result
    end

    # Apply InvMixColumns to each of the 4 columns.
    def self.inv_mix_columns(state)
      result = Array.new(4) { Array.new(4, 0) }
      4.times do |col|
        column = [state[0][col], state[1][col], state[2][col], state[3][col]]
        mixed = inv_mix_col(column)
        4.times { |row| result[row][col] = mixed[row] }
      end
      result
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Key schedule: expand_key
    # ─────────────────────────────────────────────────────────────────────────

    # Expand a 16-, 24-, or 32-byte AES key into round keys.
    #
    # Returns an Array of (Nr+1) round keys, each a 4×4 Array of Integers.
    #
    # Key schedule algorithm (FIPS 197 Section 5.2):
    #   Nk = key length in 32-bit words (4, 6, or 8)
    #   Nr = number of rounds (10, 12, or 14)
    #   W[i] = W[i-1] XOR W[i-Nk]  (i not multiple of Nk)
    #   W[i] = SubWord(RotWord(W[i-1])) XOR Rcon[i/Nk] XOR W[i-Nk]  (i mod Nk == 0)
    #   W[i] = SubWord(W[i-1]) XOR W[i-Nk]  (AES-256: Nk==8 and i mod Nk == 4)
    #
    # @param key [String] binary string of 16, 24, or 32 bytes
    # @return [Array<Array<Array<Integer>>>] (Nr+1) round keys, each a 4×4 matrix
    # @raise [ArgumentError] if key length is not 16, 24, or 32
    def self.expand_key(key)
      key_len = key.bytesize
      raise ArgumentError, "AES key must be 16, 24, or 32 bytes; got #{key_len}" unless [16, 24, 32].include?(key_len)

      nk = key_len / 4
      nr_map = { 4 => 10, 6 => 12, 8 => 14 }
      nr = nr_map[nk]
      total_words = 4 * (nr + 1)

      key_bytes = key.bytes

      # W is a flat list of 4-byte words
      w = Array.new(nk) { |i| key_bytes[4 * i, 4] }

      nk.upto(total_words - 1) do |i|
        temp = w[i - 1].dup
        if (i % nk) == 0
          # RotWord: left-rotate
          temp = [temp[1], temp[2], temp[3], temp[0]]
          # SubWord: apply S-box
          temp = temp.map { |b| SBOX[b] }
          # XOR with round constant
          temp[0] ^= RCON[i / nk]
        elsif nk == 8 && (i % nk) == 4
          # Extra SubWord for AES-256
          temp = temp.map { |b| SBOX[b] }
        end
        w << w[i - nk].zip(temp).map { |a, b| a ^ b }
      end

      # Pack into (Nr+1) round keys, each a 4×4 state (column-major)
      round_keys = []
      (nr + 1).times do |rk|
        rk_words = w[4 * rk, 4]
        # state[row][col] = rk_words[col][row]
        state = [
          [rk_words[0][0], rk_words[1][0], rk_words[2][0], rk_words[3][0]],
          [rk_words[0][1], rk_words[1][1], rk_words[2][1], rk_words[3][1]],
          [rk_words[0][2], rk_words[1][2], rk_words[2][2], rk_words[3][2]],
          [rk_words[0][3], rk_words[1][3], rk_words[2][3], rk_words[3][3]],
        ]
        round_keys << state
      end
      round_keys
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Core block cipher
    # ─────────────────────────────────────────────────────────────────────────

    # Encrypt a single 128-bit (16-byte) block with AES.
    #
    # Supports all three key sizes:
    #   - 16 bytes (AES-128): 10 rounds
    #   - 24 bytes (AES-192): 12 rounds
    #   - 32 bytes (AES-256): 14 rounds
    #
    # @param block [String] 16 bytes of plaintext (binary string)
    # @param key [String] 16, 24, or 32 bytes (binary string)
    # @return [String] 16 bytes of ciphertext (binary string)
    # @raise [ArgumentError] if block is not 16 bytes or key is not 16/24/32 bytes
    def self.aes_encrypt_block(block, key)
      raise ArgumentError, "AES block must be 16 bytes, got #{block.bytesize}" unless block.bytesize == 16

      round_keys = expand_key(key)
      nr = round_keys.length - 1

      state = bytes_to_state(block)
      state = add_round_key(state, round_keys[0])

      1.upto(nr - 1) do |rnd|
        state = sub_bytes(state)
        state = shift_rows(state)
        state = mix_columns(state)
        state = add_round_key(state, round_keys[rnd])
      end

      # Final round: no MixColumns
      state = sub_bytes(state)
      state = shift_rows(state)
      state = add_round_key(state, round_keys[nr])

      state_to_bytes(state)
    end

    # Decrypt a single 128-bit (16-byte) block with AES.
    #
    # Unlike DES (Feistel), AES decryption uses distinct inverse operations:
    # InvShiftRows → InvSubBytes → AddRoundKey → InvMixColumns.
    # AddRoundKey is its own inverse (XOR is self-inverse).
    #
    # @param block [String] 16 bytes of ciphertext (binary string)
    # @param key [String] 16, 24, or 32 bytes (same key used for encryption)
    # @return [String] 16 bytes of plaintext (binary string)
    # @raise [ArgumentError] if block is not 16 bytes or key is not 16/24/32 bytes
    def self.aes_decrypt_block(block, key)
      raise ArgumentError, "AES block must be 16 bytes, got #{block.bytesize}" unless block.bytesize == 16

      round_keys = expand_key(key)
      nr = round_keys.length - 1

      state = bytes_to_state(block)
      state = add_round_key(state, round_keys[nr])

      (nr - 1).downto(1) do |rnd|
        state = inv_shift_rows(state)
        state = inv_sub_bytes(state)
        state = add_round_key(state, round_keys[rnd])
        state = inv_mix_columns(state)
      end

      # Final round
      state = inv_shift_rows(state)
      state = inv_sub_bytes(state)
      state = add_round_key(state, round_keys[0])

      state_to_bytes(state)
    end
  end
end
