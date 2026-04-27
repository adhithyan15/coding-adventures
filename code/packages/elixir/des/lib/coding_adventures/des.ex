# CodingAdventures.Des — DES and 3DES block cipher (FIPS 46-3 / SP 800-67)
#
# DES (Data Encryption Standard) was standardized by NIST in 1977 and became
# the world's first openly standardized encryption algorithm. It is now
# completely broken — a 56-bit key can be exhausted in under 24 hours on
# consumer hardware — but it remains a vital historical and educational subject.
#
# This implementation exists to teach:
#
#   1. Feistel networks — the structural innovation that lets encryption and
#      decryption share the same circuit (just reverse the subkey order).
#      Only the right half passes through the round function f; the left half
#      is simply swapped. Because f never needs inversion, decryption = same
#      hardware, reversed keys.
#
#   2. S-boxes — the non-linear heart of DES. Each S-box maps 6 input bits to
#      4 output bits through a hand-crafted lookup table. Without S-boxes, DES
#      would be entirely linear and solvable with Gaussian elimination.
#
#   3. Key schedule — how a single 56-bit key expands into 16 round keys of
#      48 bits each using PC-1, left rotations, and PC-2.
#
#   4. Why 56 bits is not enough — simple birthday math shows that 2^56 ≈ 72
#      quadrillion keys can be exhausted in hours on GPU clusters.
#
# Algorithm Overview (Feistel Network)
# ─────────────────────────────────────
#
#   plaintext (8 bytes / 64 bits)
#        │
#   IP (initial permutation)   ← scatters bits for 1970s bus alignment
#        │
#   ┌── 16 Feistel rounds ───────────────────────────────────────────────┐
#   │   L_i = R_{i-1}                                                     │
#   │   R_i = L_{i-1} XOR f(R_{i-1}, K_i)                               │
#   │                                                                     │
#   │   f(R, K):                                                          │
#   │     E(R)        32 → 48 bits (expansion, border bits shared)       │
#   │     XOR K_i     48-bit subkey mixed in                             │
#   │     S-boxes     8 × (6 bits → 4 bits) = 32 bits out                │
#   │     P           32 → 32 bit permutation                             │
#   └─────────────────────────────────────────────────────────────────────┘
#        │
#   FP (final permutation = IP⁻¹)
#        │
#   ciphertext (8 bytes)
#
# Decryption is encryption with the 16 subkeys applied in reverse order.

defmodule CodingAdventures.Des do
  import Bitwise

  @moduledoc """
  DES and 3DES (TDEA) block cipher — FIPS 46-3 / SP 800-67.

  **Warning:** DES is cryptographically broken. The 56-bit key space is
  exhausted in under 24 hours on modern hardware. Use this package for
  education only.

  ## Public API

    - `expand_key/1` — derive 16 subkeys from an 8-byte key
    - `des_encrypt_block/2` — encrypt one 8-byte block
    - `des_decrypt_block/2` — decrypt one 8-byte block
    - `des_ecb_encrypt/2` — ECB-mode encrypt with PKCS#7 padding
    - `des_ecb_decrypt/2` — ECB-mode decrypt
    - `tdea_encrypt_block/4` — 3DES EDE encrypt (K1, K2, K3)
    - `tdea_decrypt_block/4` — 3DES EDE decrypt
  """

  # ─────────────────────────────────────────────────────────────────────────────
  # Permutation and Selection Tables
  #
  # All tables in the DES standard are 1-indexed. We subtract 1 here so they
  # can be used as 0-based indices into a bit list.
  # ─────────────────────────────────────────────────────────────────────────────

  # IP — Initial Permutation (64 positions → 64 positions)
  # Input bit 58 becomes output bit 1, bit 50 becomes bit 2, etc.
  # Designed for efficient parallel loading on 1970s 8-bit buses.
  @ip [
    58, 50, 42, 34, 26, 18, 10, 2,
    60, 52, 44, 36, 28, 20, 12, 4,
    62, 54, 46, 38, 30, 22, 14, 6,
    64, 56, 48, 40, 32, 24, 16, 8,
    57, 49, 41, 33, 25, 17,  9, 1,
    59, 51, 43, 35, 27, 19, 11, 3,
    61, 53, 45, 37, 29, 21, 13, 5,
    63, 55, 47, 39, 31, 23, 15, 7
  ]

  # FP — Final Permutation (IP⁻¹). FP undoes IP: applying both is identity.
  @fp [
    40,  8, 48, 16, 56, 24, 64, 32,
    39,  7, 47, 15, 55, 23, 63, 31,
    38,  6, 46, 14, 54, 22, 62, 30,
    37,  5, 45, 13, 53, 21, 61, 29,
    36,  4, 44, 12, 52, 20, 60, 28,
    35,  3, 43, 11, 51, 19, 59, 27,
    34,  2, 42, 10, 50, 18, 58, 26,
    33,  1, 41,  9, 49, 17, 57, 25
  ]

  # PC-1 — Permuted Choice 1 (64 → 56 bits)
  # Drops the 8 parity bits (positions 8,16,24,32,40,48,56,64) and reorders
  # the remaining 56 bits into two 28-bit halves C and D.
  @pc1 [
    57, 49, 41, 33, 25, 17,  9,
     1, 58, 50, 42, 34, 26, 18,
    10,  2, 59, 51, 43, 35, 27,
    19, 11,  3, 60, 52, 44, 36,
    63, 55, 47, 39, 31, 23, 15,
     7, 62, 54, 46, 38, 30, 22,
    14,  6, 61, 53, 45, 37, 29,
    21, 13,  5, 28, 20, 12,  4
  ]

  # PC-2 — Permuted Choice 2 (56 → 48 bits)
  # Selects 48 of the 56 key bits for each round subkey. The 8 omitted
  # positions (9,18,22,25,35,38,43,54 in the C∥D numbering) act as
  # compression, ensuring no two round keys are identical.
  @pc2 [
    14, 17, 11, 24,  1,  5,
     3, 28, 15,  6, 21, 10,
    23, 19, 12,  4, 26,  8,
    16,  7, 27, 20, 13,  2,
    41, 52, 31, 37, 47, 55,
    30, 40, 51, 45, 33, 48,
    44, 49, 39, 56, 34, 53,
    46, 42, 50, 36, 29, 32
  ]

  # E — Expansion Permutation (32 → 48 bits)
  # Expands the 32-bit right half to 48 bits by sharing border bits of each
  # 4-bit group with adjacent 6-bit groups. This is why bits 1, 4, 5, 8, …
  # appear twice in the expansion — they're the shared border bits.
  @e_perm [
    32,  1,  2,  3,  4,  5,
     4,  5,  6,  7,  8,  9,
     8,  9, 10, 11, 12, 13,
    12, 13, 14, 15, 16, 17,
    16, 17, 18, 19, 20, 21,
    20, 21, 22, 23, 24, 25,
    24, 25, 26, 27, 28, 29,
    28, 29, 30, 31, 32,  1
  ]

  # P — Post-S-box Permutation (32 → 32 bits)
  # Disperses the S-box outputs across all bit positions so that each round
  # affects every bit of the next round. This is the diffusion layer.
  @p_perm [
    16,  7, 20, 21, 29, 12, 28, 17,
     1, 15, 23, 26,  5, 18, 31, 10,
     2,  8, 24, 14, 32, 27,  3,  9,
    19, 13, 30,  6, 22, 11,  4, 25
  ]

  # SHIFTS — Left rotation amounts for the key schedule halves C and D.
  # Rounds 1, 2, 9, 16 rotate by 1; all others rotate by 2.
  # Total rotations across 16 rounds = 1+1+2+2+2+2+2+2+1+2+2+2+2+2+2+1 = 28
  # which is exactly one full rotation of the 28-bit register.
  @shifts [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1]

  # ─────────────────────────────────────────────────────────────────────────────
  # S-Boxes — the Core Non-linearity of DES
  #
  # Eight substitution boxes, each mapping 6 bits → 4 bits.
  # Without S-boxes, DES would be a linear function and solvable in polynomial
  # time via Gaussian elimination over GF(2).
  #
  # Reading an S-box with 6-bit input b₁b₂b₃b₄b₅b₆:
  #   row = 2·b₁ + b₆           (outer bits, range 0–3)
  #   col = 8·b₂ + 4·b₃ + 2·b₄ + b₅  (inner bits, range 0–15)
  #   output = SBOX[box_index][row * 16 + col]
  #
  # We use a flat tuple (1D) for O(1) indexing via elem/2.
  # ─────────────────────────────────────────────────────────────────────────────

  @sboxes {
    # S1
    {14, 4, 13, 1, 2, 15, 11, 8, 3, 10, 6, 12, 5, 9, 0, 7,
      0, 15, 7, 4, 14, 2, 13, 1, 10, 6, 12, 11, 9, 5, 3, 8,
      4, 1, 14, 8, 13, 6, 2, 11, 15, 12, 9, 7, 3, 10, 5, 0,
     15, 12, 8, 2, 4, 9, 1, 7, 5, 11, 3, 14, 10, 0, 6, 13},
    # S2
    {15, 1, 8, 14, 6, 11, 3, 4, 9, 7, 2, 13, 12, 0, 5, 10,
      3, 13, 4, 7, 15, 2, 8, 14, 12, 0, 1, 10, 6, 9, 11, 5,
      0, 14, 7, 11, 10, 4, 13, 1, 5, 8, 12, 6, 9, 3, 2, 15,
     13, 8, 10, 1, 3, 15, 4, 2, 11, 6, 7, 12, 0, 5, 14, 9},
    # S3
    {10, 0, 9, 14, 6, 3, 15, 5, 1, 13, 12, 7, 11, 4, 2, 8,
     13, 7, 0, 9, 3, 4, 6, 10, 2, 8, 5, 14, 12, 11, 15, 1,
     13, 6, 4, 9, 8, 15, 3, 0, 11, 1, 2, 12, 5, 10, 14, 7,
      1, 10, 13, 0, 6, 9, 8, 7, 4, 15, 14, 3, 11, 5, 2, 12},
    # S4
    { 7, 13, 14, 3, 0, 6, 9, 10, 1, 2, 8, 5, 11, 12, 4, 15,
     13, 8, 11, 5, 6, 15, 0, 3, 4, 7, 2, 12, 1, 10, 14, 9,
     10, 6, 9, 0, 12, 11, 7, 13, 15, 1, 3, 14, 5, 2, 8, 4,
      3, 15, 0, 6, 10, 1, 13, 8, 9, 4, 5, 11, 12, 7, 2, 14},
    # S5
    { 2, 12, 4, 1, 7, 10, 11, 6, 8, 5, 3, 15, 13, 0, 14, 9,
     14, 11, 2, 12, 4, 7, 13, 1, 5, 0, 15, 10, 3, 9, 8, 6,
      4, 2, 1, 11, 10, 13, 7, 8, 15, 9, 12, 5, 6, 3, 0, 14,
     11, 8, 12, 7, 1, 14, 2, 13, 6, 15, 0, 9, 10, 4, 5, 3},
    # S6
    {12, 1, 10, 15, 9, 2, 6, 8, 0, 13, 3, 4, 14, 7, 5, 11,
     10, 15, 4, 2, 7, 12, 9, 5, 6, 1, 13, 14, 0, 11, 3, 8,
      9, 14, 15, 5, 2, 8, 12, 3, 7, 0, 4, 10, 1, 13, 11, 6,
      4, 3, 2, 12, 9, 5, 15, 10, 11, 14, 1, 7, 6, 0, 8, 13},
    # S7
    { 4, 11, 2, 14, 15, 0, 8, 13, 3, 12, 9, 7, 5, 10, 6, 1,
     13, 0, 11, 7, 4, 9, 1, 10, 14, 3, 5, 12, 2, 15, 8, 6,
      1, 4, 11, 13, 12, 3, 7, 14, 10, 15, 6, 8, 0, 5, 9, 2,
      6, 11, 13, 8, 1, 4, 10, 7, 9, 5, 0, 15, 14, 2, 3, 12},
    # S8
    {13, 2, 8, 4, 6, 15, 11, 1, 10, 9, 3, 14, 5, 0, 12, 7,
      1, 15, 13, 8, 10, 3, 7, 4, 12, 5, 6, 11, 0, 14, 9, 2,
      7, 11, 4, 1, 9, 12, 14, 2, 0, 6, 10, 13, 15, 3, 5, 8,
      2, 1, 14, 7, 4, 10, 8, 13, 15, 12, 9, 0, 3, 5, 6, 11}
  }

  # ─────────────────────────────────────────────────────────────────────────────
  # Bit Manipulation Helpers
  # ─────────────────────────────────────────────────────────────────────────────

  # Convert a binary (8 bytes = 64 bits) to a list of bits, MSB first.
  # E.g., <<0xFF>> → [1,1,1,1,1,1,1,1]
  defp bytes_to_bits(binary) do
    for <<bit::1 <- binary>>, do: bit
  end

  # Convert a list of bits (MSB first) back to a binary.
  # The bit list length must be a multiple of 8.
  defp bits_to_binary(bits) do
    bits
    |> Enum.chunk_every(8)
    |> Enum.map(fn byte_bits ->
      Enum.reduce(byte_bits, 0, fn b, acc -> (acc <<< 1) ||| b end)
    end)
    |> :erlang.list_to_binary()
  end

  # Apply a permutation table (1-indexed positions) to a bit list.
  # permute([a,b,c,...], [3,1,2]) → [c, a, b]
  defp permute(bits, table) do
    bits_tuple = List.to_tuple(bits)
    for pos <- table, do: elem(bits_tuple, pos - 1)
  end

  # Left-rotate a 28-bit key half by n positions.
  defp left_rotate(half, n) do
    Enum.drop(half, n) ++ Enum.take(half, n)
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # Key Schedule: expand_key/1
  # ─────────────────────────────────────────────────────────────────────────────

  @doc """
  Derive the 16 DES round subkeys from an 8-byte key.

  The DES key is 64 bits wide but only 56 bits are key material — bits at
  positions 8, 16, 24, 32, 40, 48, 56, 64 are parity bits and are dropped by
  PC-1. This function accepts any 8-byte key; parity bits are ignored.

  Returns a list of 16 binaries, each 6 bytes (48 bits).

  ## Key schedule algorithm

    1. PC-1: 64-bit key → 56 bits (drop parity), split into C₀ (28) and D₀ (28)
    2. For each round i = 1..16:
         C_i = LeftRotate(C_{i-1}, SHIFTS[i])
         D_i = LeftRotate(D_{i-1}, SHIFTS[i])
         K_i = PC-2(C_i ∥ D_i)   (56 → 48 bits)

  ## Example

      iex> key = <<0x13, 0x34, 0x57, 0x79, 0x9B, 0xBC, 0xDF, 0xF1>>
      iex> subkeys = CodingAdventures.Des.expand_key(key)
      iex> length(subkeys)
      16
      iex> byte_size(hd(subkeys))
      6
  """
  def expand_key(key) when byte_size(key) == 8 do
    key_bits = bytes_to_bits(key)
    permuted = permute(key_bits, @pc1)   # 64 → 56 bits
    c0 = Enum.take(permuted, 28)
    d0 = Enum.drop(permuted, 28)

    {subkeys, _c, _d} =
      Enum.reduce(@shifts, {[], c0, d0}, fn shift, {acc, c, d} ->
        c_new = left_rotate(c, shift)
        d_new = left_rotate(d, shift)
        subkey_bits = permute(c_new ++ d_new, @pc2)  # 56 → 48 bits
        {acc ++ [bits_to_binary(subkey_bits)], c_new, d_new}
      end)

    subkeys
  end

  def expand_key(key) do
    raise ArgumentError, "DES key must be exactly 8 bytes, got #{byte_size(key)}"
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # Round Function f(R, K)
  # ─────────────────────────────────────────────────────────────────────────────

  # Apply one S-box to 6 input bits. Returns 4 output bits.
  # The S-box index (0–7) determines which of the 8 S-boxes to use.
  # Row = outer bits (b[0] and b[5]); Col = inner bits (b[1]..b[4]).
  defp apply_sbox(box_idx, chunk) do
    sbox = elem(@sboxes, box_idx)
    row = ((hd(chunk) <<< 1) ||| Enum.at(chunk, 5)) |> band(0x3)
    col = ((Enum.at(chunk, 1) <<< 3) ||| (Enum.at(chunk, 2) <<< 2) |||
            (Enum.at(chunk, 3) <<< 1) ||| Enum.at(chunk, 4)) |> band(0xF)
    val = elem(sbox, row * 16 + col)
    # Convert 4-bit value to bits (MSB first)
    for bit_pos <- [3, 2, 1, 0], do: (val >>> bit_pos) &&& 1
  end

  # The DES round function f(R, K):
  #   1. E(R) — expand 32-bit right half to 48 bits
  #   2. XOR  — mix in the 48-bit round subkey
  #   3. S    — 8 S-boxes, 6 bits → 4 bits each, total 48 → 32 bits
  #   4. P    — final 32-bit permutation for diffusion
  defp feistel_f(right, subkey) do
    # Step 1: Expand R from 32 → 48 bits
    expanded = permute(right, @e_perm)

    # Step 2: XOR with subkey
    subkey_bits = bytes_to_bits(subkey)
    xored = Enum.zip_with(expanded, subkey_bits, fn a, b -> bxor(a, b) end)

    # Step 3: Apply 8 S-boxes (6 bits each → 4 bits each)
    sbox_out =
      xored
      |> Enum.chunk_every(6)
      |> Enum.with_index()
      |> Enum.flat_map(fn {chunk, idx} -> apply_sbox(idx, chunk) end)

    # Step 4: P permutation
    permute(sbox_out, @p_perm)
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # Core Block Cipher
  # ─────────────────────────────────────────────────────────────────────────────

  # Encrypt or decrypt a single 8-byte block using the provided subkey list.
  # Encryption: pass subkeys in order (K1..K16)
  # Decryption: pass subkeys in reverse order (K16..K1)
  defp des_block(block, subkeys) when byte_size(block) == 8 do
    bits = bytes_to_bits(block)

    # Initial permutation
    bits = permute(bits, @ip)

    # Split into L₀ and R₀
    left0 = Enum.take(bits, 32)
    right0 = Enum.drop(bits, 32)

    # 16 Feistel rounds
    {left_final, right_final} =
      Enum.reduce(subkeys, {left0, right0}, fn subkey, {left, right} ->
        f_out = feistel_f(right, subkey)
        new_right = Enum.zip_with(left, f_out, fn a, b -> bxor(a, b) end)
        {right, new_right}
      end)

    # Swap halves before final permutation (standard DES step)
    combined = right_final ++ left_final

    # Final permutation (IP⁻¹)
    bits_to_binary(permute(combined, @fp))
  end

  defp des_block(block, _subkeys) do
    raise ArgumentError, "DES block must be exactly 8 bytes, got #{byte_size(block)}"
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # Public Block-Level API
  # ─────────────────────────────────────────────────────────────────────────────

  @doc """
  Encrypt a single 64-bit (8-byte) plaintext block with DES.

  ## Parameters
    - `block` — 8 bytes of plaintext
    - `key`   — 8 bytes (64 bits; bits 8,16,24,32,40,48,56,64 are parity)

  ## Returns
  8 bytes of ciphertext.

  ## Example

      iex> key   = <<0x13, 0x34, 0x57, 0x79, 0x9B, 0xBC, 0xDF, 0xF1>>
      iex> plain = <<0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF>>
      iex> CodingAdventures.Des.des_encrypt_block(plain, key)
      <<0x85, 0xE8, 0x13, 0x54, 0x0F, 0x0A, 0xB4, 0x05>>
  """
  def des_encrypt_block(block, key) do
    subkeys = expand_key(key)
    des_block(block, subkeys)
  end

  @doc """
  Decrypt a single 64-bit (8-byte) ciphertext block with DES.

  Decryption is encryption with the subkeys in reverse order — a direct
  consequence of the Feistel network's self-inverse property.

  ## Parameters
    - `block` — 8 bytes of ciphertext
    - `key`   — 8 bytes (same key used for encryption)

  ## Returns
  8 bytes of plaintext.
  """
  def des_decrypt_block(block, key) do
    subkeys = expand_key(key)
    des_block(block, Enum.reverse(subkeys))
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # ECB Mode (Educational Only)
  # ─────────────────────────────────────────────────────────────────────────────

  # PKCS#7 padding: append N bytes each with value N, where N is the number of
  # bytes needed to reach the next 8-byte block boundary (1 ≤ N ≤ 8).
  # If data is already block-aligned, add a full padding block of 8 × 0x08.
  defp pkcs7_pad(data) do
    pad_len = 8 - rem(byte_size(data), 8)
    data <> :binary.copy(<<pad_len>>, pad_len)
  end

  # Remove PKCS#7 padding. Raises if padding is invalid.
  defp pkcs7_unpad(data) when byte_size(data) > 0 do
    pad_len = :binary.last(data)
    if pad_len == 0 or pad_len > 8 do
      raise ArgumentError, "Invalid PKCS#7 padding byte: #{pad_len}"
    end
    if byte_size(data) < pad_len do
      raise ArgumentError, "Padding length exceeds data length"
    end
    expected = :binary.copy(<<pad_len>>, pad_len)
    actual = binary_part(data, byte_size(data) - pad_len, pad_len)
    if actual != expected do
      raise ArgumentError, "Invalid PKCS#7 padding (bytes do not match)"
    end
    binary_part(data, 0, byte_size(data) - pad_len)
  end

  defp pkcs7_unpad(_) do
    raise ArgumentError, "Cannot unpad empty data"
  end

  @doc """
  Encrypt variable-length plaintext with DES in ECB mode (PKCS#7 padding).

  **Warning:** ECB mode is insecure for most purposes. Identical 8-byte
  plaintext blocks always produce identical ciphertext blocks, leaking
  data patterns. This function exists for educational purposes only.

  ## Parameters
    - `plaintext` — any number of bytes
    - `key`       — 8 bytes

  ## Returns
  Ciphertext (always a multiple of 8 bytes).
  """
  def des_ecb_encrypt(plaintext, key) do
    subkeys = expand_key(key)
    padded = pkcs7_pad(plaintext)
    for <<block::binary-8 <- padded>>, into: <<>> do
      des_block(block, subkeys)
    end
  end

  @doc """
  Decrypt variable-length ciphertext with DES in ECB mode.

  ## Parameters
    - `ciphertext` — bytes (must be a multiple of 8 bytes)
    - `key`        — 8 bytes

  ## Returns
  Plaintext with PKCS#7 padding removed.
  """
  def des_ecb_decrypt(ciphertext, key) when byte_size(ciphertext) > 0 and
      rem(byte_size(ciphertext), 8) == 0 do
    subkeys = expand_key(key)
    rev_subkeys = Enum.reverse(subkeys)
    decrypted =
      for <<block::binary-8 <- ciphertext>>, into: <<>> do
        des_block(block, rev_subkeys)
      end
    pkcs7_unpad(decrypted)
  end

  def des_ecb_decrypt(ciphertext, _key) when byte_size(ciphertext) == 0 do
    raise ArgumentError, "Cannot decrypt empty ciphertext"
  end

  def des_ecb_decrypt(ciphertext, _key) do
    raise ArgumentError,
      "DES ECB ciphertext length must be a multiple of 8 bytes, got #{byte_size(ciphertext)}"
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # 3DES / TDEA (Triple DES)
  # ─────────────────────────────────────────────────────────────────────────────
  #
  # Triple DES (TDEA) applies DES three times using three independent keys.
  # The EDE (Encrypt-Decrypt-Encrypt) ordering is specified in NIST SP 800-67:
  #
  #   Encrypt: C = E_K1( D_K2( E_K3(P) ) )
  #   Decrypt: P = D_K3( E_K2( D_K1(C) ) )
  #
  # When K1 = K2 = K3, TDEA reduces to single DES — backward compatibility.
  # When K1 = K3 ≠ K2, TDEA reduces to 2TDEA (112-bit effective key strength).
  # All three independent is 3TDEA (112-bit effective security, not 168-bit,
  # due to the meet-in-the-middle attack).

  @doc """
  Encrypt a single 8-byte block with Triple DES (TDEA EDE).

  Applies `E_K1(D_K2(E_K3(block)))` — the NIST SP 800-67 ordering.

  When K1 = K2 = K3, this reduces to single DES (backward compatibility):
    E_K(D_K(E_K(P))) = E_K(P)

  ## Parameters
    - `block` — 8 bytes of plaintext
    - `k1`, `k2`, `k3` — three 8-byte DES keys
  """
  def tdea_encrypt_block(block, k1, k2, k3) do
    sk1 = expand_key(k1)
    sk2 = expand_key(k2)
    sk3 = expand_key(k3)
    block
    |> des_block(sk3)                     # E_K3
    |> des_block(Enum.reverse(sk2))       # D_K2
    |> des_block(sk1)                     # E_K1
  end

  @doc """
  Decrypt a single 8-byte block with Triple DES (TDEA EDE).

  Applies `D_K3(E_K2(D_K1(block)))` — the inverse of EDE.

  ## Parameters
    - `block` — 8 bytes of ciphertext
    - `k1`, `k2`, `k3` — three 8-byte DES keys (same as used for encryption)
  """
  def tdea_decrypt_block(block, k1, k2, k3) do
    sk1 = expand_key(k1)
    sk2 = expand_key(k2)
    sk3 = expand_key(k3)
    block
    |> des_block(Enum.reverse(sk1))       # D_K1
    |> des_block(sk2)                     # E_K2
    |> des_block(Enum.reverse(sk3))       # D_K3
  end
end
