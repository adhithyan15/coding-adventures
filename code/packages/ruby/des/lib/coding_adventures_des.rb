# frozen_string_literal: true

# =============================================================================
# coding_adventures_des — DES and 3DES block cipher (FIPS 46-3 / SP 800-67)
# =============================================================================
#
# DES (Data Encryption Standard) was published by NIST in 1977 and was the
# world's first openly standardized encryption algorithm. It is now completely
# broken — a 56-bit key can be exhausted in under 24 hours on consumer hardware
# — but it remains a vital historical and educational subject.
#
# This package implements DES for educational purposes only. It should never
# be used to protect real data. Its value is in understanding:
#
#   1. Feistel networks — the structural innovation that lets encryption and
#      decryption share the same circuit (just reverse the subkey order).
#   2. S-boxes — the non-linear heart of DES; hardened by the NSA against
#      differential cryptanalysis a decade before that attack was published.
#   3. Key schedules — how a single 56-bit key expands into 16 round keys.
#   4. Why 56 bits is not enough — the brute-force math that doomed DES.
#
# Architecture
# ─────────────
#
#   plaintext (8 bytes)
#        │
#   IP (initial permutation)       ← scatters bits for 1970s bus alignment
#        │
#   ┌── 16 Feistel rounds ─────────────────────────────────────────────┐
#   │   L_i = R_{i-1}                                                  │
#   │   R_i = L_{i-1} XOR f(R_{i-1}, K_i)                            │
#   │                                                                  │
#   │   f(R, K):                                                       │
#   │     E(R)          32→48 bits (expansion, border bits shared)     │
#   │     XOR K_i       48-bit subkey                                  │
#   │     S-boxes       8 × (6 bits → 4 bits) = 32 bits out           │
#   │     P             32→32 bit permutation                          │
#   └───────────────────────────────────────────────────────────────────┘
#        │
#   FP (final permutation = IP⁻¹)
#        │
#   ciphertext (8 bytes)
#
#   Decryption: identical — apply the 16 subkeys in reverse order.
#
# Ruby Integer Note
# ─────────────────
# Ruby integers have arbitrary precision, so they never overflow.
# We mask to the appropriate bit width where needed (e.g., & 0xFFFFFFFF).
#
# Public API
# ──────────
#   CodingAdventures::Des.expand_key(key)               → Array of 16 subkeys (6-byte Strings)
#   CodingAdventures::Des.des_encrypt_block(block, key) → 8-byte String
#   CodingAdventures::Des.des_decrypt_block(block, key) → 8-byte String
#   CodingAdventures::Des.des_ecb_encrypt(plain, key)   → String (PKCS#7 padded)
#   CodingAdventures::Des.des_ecb_decrypt(cipher, key)  → String
#   CodingAdventures::Des.tdea_encrypt_block(block, k1, k2, k3) → 8-byte String
#   CodingAdventures::Des.tdea_decrypt_block(block, k1, k2, k3) → 8-byte String

require_relative "coding_adventures/des/version"

module CodingAdventures
  module Des
    # ─────────────────────────────────────────────────────────────────────────
    # Permutation and selection tables
    #
    # All tables are 1-indexed in the DES standard; we store them as-is and
    # subtract 1 when indexing into arrays.
    # ─────────────────────────────────────────────────────────────────────────

    # IP — Initial Permutation
    IP = [
      58, 50, 42, 34, 26, 18, 10,  2,
      60, 52, 44, 36, 28, 20, 12,  4,
      62, 54, 46, 38, 30, 22, 14,  6,
      64, 56, 48, 40, 32, 24, 16,  8,
      57, 49, 41, 33, 25, 17,  9,  1,
      59, 51, 43, 35, 27, 19, 11,  3,
      61, 53, 45, 37, 29, 21, 13,  5,
      63, 55, 47, 39, 31, 23, 15,  7,
    ].freeze

    # FP — Final Permutation (IP⁻¹)
    FP = [
      40,  8, 48, 16, 56, 24, 64, 32,
      39,  7, 47, 15, 55, 23, 63, 31,
      38,  6, 46, 14, 54, 22, 62, 30,
      37,  5, 45, 13, 53, 21, 61, 29,
      36,  4, 44, 12, 52, 20, 60, 28,
      35,  3, 43, 11, 51, 19, 59, 27,
      34,  2, 42, 10, 50, 18, 58, 26,
      33,  1, 41,  9, 49, 17, 57, 25,
    ].freeze

    # PC-1 — Permuted Choice 1 (drops parity bits, 64→56)
    PC1 = [
      57, 49, 41, 33, 25, 17,  9,
       1, 58, 50, 42, 34, 26, 18,
      10,  2, 59, 51, 43, 35, 27,
      19, 11,  3, 60, 52, 44, 36,
      63, 55, 47, 39, 31, 23, 15,
       7, 62, 54, 46, 38, 30, 22,
      14,  6, 61, 53, 45, 37, 29,
      21, 13,  5, 28, 20, 12,  4,
    ].freeze

    # PC-2 — Permuted Choice 2 (56→48 bits per round)
    PC2 = [
      14, 17, 11, 24,  1,  5,
       3, 28, 15,  6, 21, 10,
      23, 19, 12,  4, 26,  8,
      16,  7, 27, 20, 13,  2,
      41, 52, 31, 37, 47, 55,
      30, 40, 51, 45, 33, 48,
      44, 49, 39, 56, 34, 53,
      46, 42, 50, 36, 29, 32,
    ].freeze

    # E — Expansion permutation (32→48 bits)
    E_TABLE = [
      32,  1,  2,  3,  4,  5,
       4,  5,  6,  7,  8,  9,
       8,  9, 10, 11, 12, 13,
      12, 13, 14, 15, 16, 17,
      16, 17, 18, 19, 20, 21,
      20, 21, 22, 23, 24, 25,
      24, 25, 26, 27, 28, 29,
      28, 29, 30, 31, 32,  1,
    ].freeze

    # P — Post-S-box permutation (32→32)
    P_TABLE = [
      16,  7, 20, 21, 29, 12, 28, 17,
       1, 15, 23, 26,  5, 18, 31, 10,
       2,  8, 24, 14, 32, 27,  3,  9,
      19, 13, 30,  6, 22, 11,  4, 25,
    ].freeze

    # Left-rotation amounts for key schedule halves C and D.
    # Rounds 1, 2, 9, 16 rotate by 1; all others by 2.
    SHIFTS = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1].freeze

    # ─────────────────────────────────────────────────────────────────────────
    # S-Boxes: the core non-linearity of DES
    #
    # Eight substitution boxes, each mapping 6 bits → 4 bits.
    # Reading an S-box with 6 input bits b1..b6:
    #   row = 2·b1 + b6         (outer bits, range 0–3)
    #   col = 8·b2 + 4·b3 + 2·b4 + b5  (inner bits, range 0–15)
    # ─────────────────────────────────────────────────────────────────────────

    SBOXES = [
      # S1
      [
        [14,  4, 13,  1,  2, 15, 11,  8,  3, 10,  6, 12,  5,  9,  0,  7],
        [ 0, 15,  7,  4, 14,  2, 13,  1, 10,  6, 12, 11,  9,  5,  3,  8],
        [ 4,  1, 14,  8, 13,  6,  2, 11, 15, 12,  9,  7,  3, 10,  5,  0],
        [15, 12,  8,  2,  4,  9,  1,  7,  5, 11,  3, 14, 10,  0,  6, 13],
      ],
      # S2
      [
        [15,  1,  8, 14,  6, 11,  3,  4,  9,  7,  2, 13, 12,  0,  5, 10],
        [ 3, 13,  4,  7, 15,  2,  8, 14, 12,  0,  1, 10,  6,  9, 11,  5],
        [ 0, 14,  7, 11, 10,  4, 13,  1,  5,  8, 12,  6,  9,  3,  2, 15],
        [13,  8, 10,  1,  3, 15,  4,  2, 11,  6,  7, 12,  0,  5, 14,  9],
      ],
      # S3
      [
        [10,  0,  9, 14,  6,  3, 15,  5,  1, 13, 12,  7, 11,  4,  2,  8],
        [13,  7,  0,  9,  3,  4,  6, 10,  2,  8,  5, 14, 12, 11, 15,  1],
        [13,  6,  4,  9,  8, 15,  3,  0, 11,  1,  2, 12,  5, 10, 14,  7],
        [ 1, 10, 13,  0,  6,  9,  8,  7,  4, 15, 14,  3, 11,  5,  2, 12],
      ],
      # S4
      [
        [ 7, 13, 14,  3,  0,  6,  9, 10,  1,  2,  8,  5, 11, 12,  4, 15],
        [13,  8, 11,  5,  6, 15,  0,  3,  4,  7,  2, 12,  1, 10, 14,  9],
        [10,  6,  9,  0, 12, 11,  7, 13, 15,  1,  3, 14,  5,  2,  8,  4],
        [ 3, 15,  0,  6, 10,  1, 13,  8,  9,  4,  5, 11, 12,  7,  2, 14],
      ],
      # S5
      [
        [ 2, 12,  4,  1,  7, 10, 11,  6,  8,  5,  3, 15, 13,  0, 14,  9],
        [14, 11,  2, 12,  4,  7, 13,  1,  5,  0, 15, 10,  3,  9,  8,  6],
        [ 4,  2,  1, 11, 10, 13,  7,  8, 15,  9, 12,  5,  6,  3,  0, 14],
        [11,  8, 12,  7,  1, 14,  2, 13,  6, 15,  0,  9, 10,  4,  5,  3],
      ],
      # S6
      [
        [12,  1, 10, 15,  9,  2,  6,  8,  0, 13,  3,  4, 14,  7,  5, 11],
        [10, 15,  4,  2,  7, 12,  9,  5,  6,  1, 13, 14,  0, 11,  3,  8],
        [ 9, 14, 15,  5,  2,  8, 12,  3,  7,  0,  4, 10,  1, 13, 11,  6],
        [ 4,  3,  2, 12,  9,  5, 15, 10, 11, 14,  1,  7,  6,  0,  8, 13],
      ],
      # S7
      [
        [ 4, 11,  2, 14, 15,  0,  8, 13,  3, 12,  9,  7,  5, 10,  6,  1],
        [13,  0, 11,  7,  4,  9,  1, 10, 14,  3,  5, 12,  2, 15,  8,  6],
        [ 1,  4, 11, 13, 12,  3,  7, 14, 10, 15,  6,  8,  0,  5,  9,  2],
        [ 6, 11, 13,  8,  1,  4, 10,  7,  9,  5,  0, 15, 14,  2,  3, 12],
      ],
      # S8
      [
        [13,  2,  8,  4,  6, 15, 11,  1, 10,  9,  3, 14,  5,  0, 12,  7],
        [ 1, 15, 13,  8, 10,  3,  7,  4, 12,  5,  6, 11,  0, 14,  9,  2],
        [ 7, 11,  4,  1,  9, 12, 14,  2,  0,  6, 10, 13, 15,  3,  5,  8],
        [ 2,  1, 14,  7,  4, 10,  8, 13, 15, 12,  9,  0,  3,  5,  6, 11],
      ],
    ].freeze

    # ─────────────────────────────────────────────────────────────────────────
    # Bit manipulation helpers
    # ─────────────────────────────────────────────────────────────────────────

    # Convert a binary String (8-bit bytes) to an Array of bits (MSB first).
    #
    # @param data [String] binary string (encoding: BINARY / ASCII-8BIT)
    # @return [Array<Integer>] array of 0s and 1s
    def self.bytes_to_bits(data)
      bits = []
      data.each_byte do |byte|
        7.downto(0) { |i| bits << ((byte >> i) & 1) }
      end
      bits
    end

    # Convert an Array of bits (MSB first) back to a binary String.
    #
    # @param bits [Array<Integer>] array of 0s and 1s (length must be multiple of 8)
    # @return [String] binary string
    def self.bits_to_bytes(bits)
      result = "".b
      bits.each_slice(8) do |chunk|
        byte = chunk.reduce(0) { |acc, b| (acc << 1) | b }
        result << byte.chr
      end
      result
    end

    # Apply a permutation table (1-indexed positions) to a bit array.
    #
    # @param bits [Array<Integer>] input bit array
    # @param table [Array<Integer>] permutation table (1-indexed)
    # @return [Array<Integer>] permuted bit array
    def self.permute(bits, table)
      table.map { |pos| bits[pos - 1] }
    end

    # Left-rotate a 28-bit half of the key register by n positions.
    #
    # @param half [Array<Integer>] 28-bit array
    # @param n [Integer] rotation amount
    # @return [Array<Integer>] rotated array
    def self.left_rotate28(half, n)
      half[n..] + half[0...n]
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Key schedule: expand_key
    # ─────────────────────────────────────────────────────────────────────────

    # Derive the 16 DES round subkeys from an 8-byte key.
    #
    # Algorithm:
    #   1. PC-1: 64 bits → 56 bits (drop parity), split into C₀ (28) and D₀ (28)
    #   2. For each round i = 1..16:
    #        C_i = LeftRotate(C_{i-1}, SHIFTS[i])
    #        D_i = LeftRotate(D_{i-1}, SHIFTS[i])
    #        K_i = PC-2(C_i ∥ D_i)   (56 → 48 bits)
    #
    # @param key [String] binary string of exactly 8 bytes
    # @return [Array<String>] 16 subkeys, each a 6-byte binary String (48 bits)
    # @raise [ArgumentError] if key is not exactly 8 bytes
    def self.expand_key(key)
      raise ArgumentError, "DES key must be exactly 8 bytes, got #{key.bytesize}" unless key.bytesize == 8

      key_bits = bytes_to_bits(key)
      permuted = permute(key_bits, PC1)    # 64 → 56 bits
      c = permuted[0...28]
      d = permuted[28..]

      subkeys = []
      SHIFTS.each do |shift|
        c = left_rotate28(c, shift)
        d = left_rotate28(d, shift)
        subkey_bits = permute(c + d, PC2)  # 56 → 48 bits
        subkeys << bits_to_bytes(subkey_bits)
      end
      subkeys
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Round function f(R, K)
    # ─────────────────────────────────────────────────────────────────────────

    # DES round function f(R, K).
    #
    # Steps:
    #   1. E(R)  — expand 32-bit right half to 48 bits
    #   2. XOR K — mix in the 48-bit round subkey
    #   3. S     — 8 S-boxes (6 bits each → 4 bits)
    #   4. P     — final 32-bit permutation
    #
    # @param right [Array<Integer>] 32-bit right half as a bit array
    # @param subkey [String] 6-byte subkey (48 bits)
    # @return [Array<Integer>] 32-bit output as a bit array
    def self.feistel_f(right, subkey)
      # Step 1: Expand R from 32 → 48 bits
      expanded = permute(right, E_TABLE)

      # Step 2: XOR with subkey
      subkey_bits = bytes_to_bits(subkey)
      xored = expanded.zip(subkey_bits).map { |a, b| a ^ b }

      # Step 3: Apply 8 S-boxes (each 6-bit input → 4-bit output)
      sbox_out = []
      8.times do |box_idx|
        chunk = xored[box_idx * 6, 6]
        row = (chunk[0] << 1) | chunk[5]
        col = (chunk[1] << 3) | (chunk[2] << 2) | (chunk[3] << 1) | chunk[4]
        val = SBOXES[box_idx][row][col]
        3.downto(0) { |bit_pos| sbox_out << ((val >> bit_pos) & 1) }
      end

      # Step 4: P permutation
      permute(sbox_out, P_TABLE)
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Core block cipher
    # ─────────────────────────────────────────────────────────────────────────

    # Encrypt or decrypt a single 8-byte block using the provided subkey list.
    #
    # Encryption: pass subkeys in order (K1..K16)
    # Decryption: pass subkeys in reverse order (K16..K1)
    #
    # @param block [String] binary string of exactly 8 bytes
    # @param subkeys [Array<String>] 16 subkeys
    # @return [String] 8-byte binary string
    def self.des_block(block, subkeys)
      raise ArgumentError, "DES block must be exactly 8 bytes, got #{block.bytesize}" unless block.bytesize == 8

      bits = bytes_to_bits(block)
      bits = permute(bits, IP)

      left = bits[0...32]
      right = bits[32..]

      subkeys.each do |subkey|
        f_out = feistel_f(right, subkey)
        new_right = left.zip(f_out).map { |a, b| a ^ b }
        left = right
        right = new_right
      end

      # Swap and apply final permutation
      bits_to_bytes(permute(right + left, FP))
    end

    # Encrypt a single 64-bit (8-byte) block using DES.
    #
    # @param block [String] 8 bytes of plaintext (binary string)
    # @param key [String] 8 bytes (64 bits, 56 key + 8 parity)
    # @return [String] 8 bytes of ciphertext (binary string)
    def self.des_encrypt_block(block, key)
      des_block(block, expand_key(key))
    end

    # Decrypt a single 64-bit (8-byte) block using DES.
    #
    # @param block [String] 8 bytes of ciphertext (binary string)
    # @param key [String] 8 bytes (same key used for encryption)
    # @return [String] 8 bytes of plaintext (binary string)
    def self.des_decrypt_block(block, key)
      des_block(block, expand_key(key).reverse)
    end

    # ─────────────────────────────────────────────────────────────────────────
    # ECB mode (educational only)
    # ─────────────────────────────────────────────────────────────────────────

    # PKCS#7 padding: append N bytes each with value N.
    # N = block_size - (len % block_size), minimum 1, maximum block_size.
    #
    # @param data [String] binary string
    # @param block_size [Integer] block size in bytes
    # @return [String] padded binary string
    def self.pkcs7_pad(data, block_size)
      pad_len = block_size - (data.bytesize % block_size)
      data + (pad_len.chr * pad_len).b
    end

    # Remove PKCS#7 padding. Raises ArgumentError if padding is invalid.
    #
    # @param data [String] padded binary string (block size 8)
    # @return [String] unpadded binary string
    # @raise [ArgumentError] if padding is invalid
    def self.pkcs7_unpad(data)
      raise ArgumentError, "Cannot unpad empty data" if data.empty?

      pad_len = data.bytes.last
      raise ArgumentError, "Invalid PKCS#7 padding byte: #{pad_len}" if pad_len == 0 || pad_len > 8
      raise ArgumentError, "Padding length exceeds data length" if data.bytesize < pad_len
      raise ArgumentError, "Invalid PKCS#7 padding (bytes do not match)" unless data[-pad_len..].bytes.all? { |b| b == pad_len }

      data[0...-pad_len]
    end

    # Encrypt variable-length plaintext with DES in ECB mode (PKCS#7 padding).
    #
    # WARNING: ECB mode is insecure. Identical 8-byte plaintext blocks always
    # produce identical ciphertext blocks, leaking data patterns.
    #
    # @param plaintext [String] binary string (any length)
    # @param key [String] 8-byte key
    # @return [String] encrypted binary string (multiple of 8 bytes)
    def self.des_ecb_encrypt(plaintext, key)
      subkeys = expand_key(key)
      padded = pkcs7_pad(plaintext.b, 8)
      result = "".b
      (0...padded.bytesize).step(8) do |i|
        result << des_block(padded[i, 8], subkeys)
      end
      result
    end

    # Decrypt variable-length ciphertext with DES in ECB mode.
    #
    # @param ciphertext [String] binary string (must be a multiple of 8 bytes)
    # @param key [String] 8-byte key
    # @return [String] decrypted binary string with PKCS#7 padding removed
    # @raise [ArgumentError] if ciphertext is empty or not a multiple of 8 bytes
    def self.des_ecb_decrypt(ciphertext, key)
      raise ArgumentError, "Ciphertext must not be empty" if ciphertext.empty?
      raise ArgumentError, "DES ECB ciphertext length must be a multiple of 8 bytes" unless ciphertext.bytesize % 8 == 0

      subkeys = expand_key(key).reverse
      result = "".b
      (0...ciphertext.bytesize).step(8) do |i|
        result << des_block(ciphertext[i, 8], subkeys)
      end
      pkcs7_unpad(result)
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Triple DES (3DES / TDEA)
    # ─────────────────────────────────────────────────────────────────────────

    # Encrypt one 8-byte block with Triple DES (3TDEA / EDE mode).
    #
    # Algorithm (NIST SP 800-67): C = E_K1(D_K2(E_K3(P)))
    #
    # The EDE structure gives backward compatibility: K1=K2=K3 → single DES.
    #
    # @param block [String] 8-byte binary string
    # @param k1 [String] 8-byte key 1
    # @param k2 [String] 8-byte key 2
    # @param k3 [String] 8-byte key 3
    # @return [String] 8-byte binary string
    def self.tdea_encrypt_block(block, k1, k2, k3)
      step1 = des_encrypt_block(block, k3)  # E_K3(P)
      step2 = des_decrypt_block(step1, k2)  # D_K2(E_K3(P))
      des_encrypt_block(step2, k1)          # E_K1(D_K2(E_K3(P)))
    end

    # Decrypt one 8-byte block with Triple DES (3TDEA / EDE mode).
    #
    # Algorithm (NIST SP 800-67): P = D_K3(E_K2(D_K1(C)))
    #
    # @param block [String] 8-byte binary string
    # @param k1 [String] 8-byte key 1
    # @param k2 [String] 8-byte key 2
    # @param k3 [String] 8-byte key 3
    # @return [String] 8-byte binary string
    def self.tdea_decrypt_block(block, k1, k2, k3)
      step1 = des_decrypt_block(block, k1)  # D_K1(C)
      step2 = des_encrypt_block(step1, k2)  # E_K2(D_K1(C))
      des_decrypt_block(step2, k3)          # D_K3(E_K2(D_K1(C)))
    end
  end
end
