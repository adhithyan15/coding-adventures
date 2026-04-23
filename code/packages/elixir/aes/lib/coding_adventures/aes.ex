# CodingAdventures.Aes — AES block cipher (FIPS 197)
#
# AES (Advanced Encryption Standard) is the most widely deployed symmetric
# encryption algorithm in the world. Published by NIST in 2001 as FIPS 197,
# it replaced DES and is used in TLS/HTTPS, WPA2/WPA3 WiFi, disk encryption
# (BitLocker, LUKS, FileVault), VPNs, and virtually every secure protocol.
#
# Designed by Joan Daemen and Vincent Rijmen (algorithm name: Rijndael), AES
# is a Substitution-Permutation Network (SPN) — structurally different from
# DES's Feistel network. In an SPN, ALL bytes of the state are transformed on
# every round, not just half.
#
# Architecture
# ─────────────
#   plaintext (16 bytes)
#        │
#   AddRoundKey(state, round_key[0])    ← XOR first key material
#        │
#   ┌── Nr-1 full rounds ──────────────────────────────────────────────────┐
#   │   SubBytes    — non-linear S-box (GF inverse + affine transform)     │
#   │   ShiftRows   — cyclic left shifts of each row (column diffusion)    │
#   │   MixColumns  — GF(2^8) matrix multiply (row diffusion)              │
#   │   AddRoundKey — XOR with round key                                   │
#   └──────────────────────────────────────────────────────────────────────┘
#        │
#   SubBytes + ShiftRows + AddRoundKey  ← final round (no MixColumns)
#        │
#   ciphertext (16 bytes)
#
# GF(2^8) polynomial: 0x11B = x^8 + x^4 + x^3 + x + 1
# (Different from Reed-Solomon's 0x11D; we implement arithmetic inline.)
#
# State Layout: column-major 4×4 bytes
#   state[row][col] = block[row + 4*col]
#
# Key Sizes and Round Counts
# ──────────────────────────
#   Key     Nk    Nr    Round keys
#   128-bit  4    10    11 × 16 bytes
#   192-bit  6    12    13 × 16 bytes
#   256-bit  8    14    15 × 16 bytes

defmodule CodingAdventures.Aes do
  import Bitwise

  @moduledoc """
  AES block cipher — FIPS 197. Supports AES-128, AES-192, and AES-256.

  ## Public API

    - `aes_encrypt_block/2` — encrypt one 16-byte block
    - `aes_decrypt_block/2` — decrypt one 16-byte block
    - `expand_key/1`        — expand key into round keys
    - `sbox/0`              — 256-element AES S-box (for inspection)
    - `inv_sbox/0`          — 256-element AES inverse S-box
  """

  # ─────────────────────────────────────────────────────────────────────────────
  # GF(2^8) Arithmetic with AES polynomial 0x11B
  #
  # xtime(b) = multiply b by 2 in GF(2^8):
  #   - Left-shift by 1 (multiply by x)
  #   - If bit 7 was set, XOR 0x1B (reduce mod x^8 + x^4 + x^3 + x + 1)
  #
  # gf_mul(a, b) uses the Russian peasant algorithm:
  #   result = 0
  #   repeat 8 times:
  #     if LSB(b) == 1: result ^= a
  #     a = xtime(a)
  #     b >>= 1
  # ─────────────────────────────────────────────────────────────────────────────

  defp xtime(b) do
    shifted = (b <<< 1) &&& 0xFF
    if (b &&& 0x80) != 0, do: bxor(shifted, 0x1B), else: shifted
  end

  defp gf_mul(a, b), do: gf_mul_loop(a, b, 0, 8)
  defp gf_mul_loop(_a, _b, acc, 0), do: acc
  defp gf_mul_loop(a, b, acc, steps) do
    acc2 = if (b &&& 1) != 0, do: bxor(acc, a), else: acc
    gf_mul_loop(xtime(a), b >>> 1, acc2, steps - 1)
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # S-box and Inverse S-box (FIPS 197, Figures 7 and 14)
  #
  # The S-box maps each byte b to:
  #   1. GF(2^8) multiplicative inverse (with poly 0x11B); 0 maps to 0
  #   2. Affine transformation:
  #        s_i = b_i XOR b_{(i+4)%8} XOR b_{(i+5)%8} XOR b_{(i+6)%8} XOR b_{(i+7)%8} XOR c_i
  #      where c = 0x63
  #
  # These are hardcoded from FIPS 197 for clarity and compile-time availability.
  # ─────────────────────────────────────────────────────────────────────────────

  @sbox_tuple {
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16
  }

  @inv_sbox_tuple {
    0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7, 0xfb,
    0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9, 0xcb,
    0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3, 0x4e,
    0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1, 0x25,
    0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6, 0x92,
    0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84,
    0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06,
    0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b,
    0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73,
    0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e,
    0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b,
    0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4,
    0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f,
    0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef,
    0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
    0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c, 0x7d
  }

  # Round constants for the key schedule (Rcon[i] = 2^{i-1} in GF(2^8)).
  # 1-indexed: Rcon[1]=0x01, Rcon[2]=0x02, ..., Rcon[10]=0x36, etc.
  @rcon_tuple {0x00,
    0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80,
    0x1b, 0x36, 0x6c, 0xd8, 0xab, 0x4d}

  # ─────────────────────────────────────────────────────────────────────────────
  # Public S-box Accessors
  # ─────────────────────────────────────────────────────────────────────────────

  @doc "Returns the AES S-box as a 256-element list."
  def sbox, do: Tuple.to_list(@sbox_tuple)

  @doc "Returns the AES inverse S-box as a 256-element list."
  def inv_sbox, do: Tuple.to_list(@inv_sbox_tuple)

  # ─────────────────────────────────────────────────────────────────────────────
  # Key Schedule: expand_key/1
  # ─────────────────────────────────────────────────────────────────────────────

  @doc """
  Expand a 16-, 24-, or 32-byte AES key into (Nr+1) round keys.

  Returns a list of (Nr+1) round keys, each a 4×4 state:
  `state[row][col]` = byte at position `row + 4*col` in the key block.

  ## Algorithm (FIPS 197 Section 5.2)

    - Nk = key length in 32-bit words (4, 6, or 8)
    - Nr = number of rounds (10, 12, or 14)
    - Build `4*(Nr+1)` words W[0..total-1]:
        W[i] for i < Nk: directly from key
        W[i] = SubWord(RotWord(W[i-1])) XOR Rcon[i/Nk] XOR W[i-Nk]  when i mod Nk == 0
        W[i] = SubWord(W[i-1]) XOR W[i-Nk]  when Nk=8 and i mod Nk == 4
        W[i] = W[i-1] XOR W[i-Nk]  otherwise
  """
  def expand_key(key) when byte_size(key) in [16, 24, 32] do
    key_len = byte_size(key)
    nk = div(key_len, 4)
    nr = case nk do
      4 -> 10
      6 -> 12
      8 -> 14
    end
    total_words = 4 * (nr + 1)

    # Initialize W[0..nk-1] from the key
    key_list = :binary.bin_to_list(key)
    initial_words = key_list |> Enum.chunk_every(4)

    # Build remaining words
    words =
      Enum.reduce(nk..(total_words - 1)//1, initial_words, fn i, w ->
        prev = Enum.at(w, i - 1)
        temp =
          cond do
            rem(i, nk) == 0 ->
              # RotWord: left-rotate 4 bytes
              rotated = Enum.drop(prev, 1) ++ [hd(prev)]
              # SubWord: apply S-box to each byte
              subbed = for b <- rotated, do: elem(@sbox_tuple, b)
              # XOR Rcon into first byte
              rcon_val = elem(@rcon_tuple, div(i, nk))
              List.replace_at(subbed, 0, bxor(hd(subbed), rcon_val))

            nk == 8 and rem(i, nk) == 4 ->
              # Extra SubWord for AES-256
              for b <- prev, do: elem(@sbox_tuple, b)

            true ->
              prev
          end
        w_prev_nk = Enum.at(w, i - nk)
        new_word = Enum.zip_with(w_prev_nk, temp, fn a, b -> bxor(a, b) end)
        w ++ [new_word]
      end)

    # Pack into (Nr+1) round keys, each a 4×4 state (column-major)
    for rk <- 0..nr do
      rk_words = Enum.slice(words, 4 * rk, 4)
      # state[row][col] = rk_words[col][row]
      for row <- 0..3 do
        for col <- 0..3 do
          rk_words |> Enum.at(col) |> Enum.at(row)
        end
      end
    end
  end

  def expand_key(key) do
    raise ArgumentError, "AES key must be 16, 24, or 32 bytes; got #{byte_size(key)}"
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # State Manipulation Helpers
  # ─────────────────────────────────────────────────────────────────────────────

  # Convert 16 bytes to AES state (4×4 column-major).
  # state[row][col] = block[row + 4*col]
  defp bytes_to_state(block) do
    bytes = :binary.bin_to_list(block)
    for row <- 0..3 do
      for col <- 0..3, do: Enum.at(bytes, row + 4 * col)
    end
  end

  # Convert AES state back to 16 bytes (column-major).
  defp state_to_bytes(state) do
    :binary.list_to_bin(
      for col <- 0..3, row <- 0..3 do
        state |> Enum.at(row) |> Enum.at(col)
      end
    )
  end

  # AddRoundKey: XOR state with round key.
  defp add_round_key(state, round_key) do
    for r <- 0..3 do
      for c <- 0..3 do
        bxor(
          state |> Enum.at(r) |> Enum.at(c),
          round_key |> Enum.at(r) |> Enum.at(c)
        )
      end
    end
  end

  # SubBytes: substitute each byte via S-box.
  defp sub_bytes(state) do
    for row <- state, do: for b <- row, do: elem(@sbox_tuple, b)
  end

  # InvSubBytes: substitute each byte via inverse S-box.
  defp inv_sub_bytes(state) do
    for row <- state, do: for b <- row, do: elem(@inv_sbox_tuple, b)
  end

  # ShiftRows: cyclically shift row i left by i positions.
  # Row 0: no shift; Row 1: left 1; Row 2: left 2; Row 3: left 3.
  defp shift_rows(state) do
    for {row, r} <- Enum.with_index(state) do
      Enum.drop(row, r) ++ Enum.take(row, r)
    end
  end

  # InvShiftRows: cyclically shift row i right by i positions.
  defp inv_shift_rows(state) do
    for {row, r} <- Enum.with_index(state) do
      if r == 0, do: row, else: Enum.drop(row, 4 - r) ++ Enum.take(row, 4 - r)
    end
  end

  # MixColumns: multiply each column by AES matrix in GF(2^8).
  # Matrix: [[2,3,1,1],[1,2,3,1],[1,1,2,3],[3,1,1,2]]
  defp mix_col([s0, s1, s2, s3]) do
    t0 = bxor(xtime(s0), bxor(bxor(xtime(s1), s1), bxor(s2, s3)))
    t1 = bxor(s0, bxor(xtime(s1), bxor(bxor(xtime(s2), s2), s3)))
    t2 = bxor(s0, bxor(s1, bxor(xtime(s2), bxor(xtime(s3), s3))))
    t3 = bxor(bxor(xtime(s0), s0), bxor(s1, bxor(s2, xtime(s3))))
    [t0, t1, t2, t3]
  end

  defp mix_columns(state) do
    cols = for col <- 0..3 do
      column = for row <- 0..3, do: state |> Enum.at(row) |> Enum.at(col)
      mix_col(column)
    end
    # cols[col][row] → repack to state[row][col]
    for row <- 0..3 do
      for col <- 0..3, do: cols |> Enum.at(col) |> Enum.at(row)
    end
  end

  # InvMixColumns: inverse MixColumns.
  # Matrix: [[14,11,13,9],[9,14,11,13],[13,9,14,11],[11,13,9,14]]
  defp inv_mix_col([s0, s1, s2, s3]) do
    t0 = bxor(gf_mul(0x0e, s0), bxor(gf_mul(0x0b, s1), bxor(gf_mul(0x0d, s2), gf_mul(0x09, s3))))
    t1 = bxor(gf_mul(0x09, s0), bxor(gf_mul(0x0e, s1), bxor(gf_mul(0x0b, s2), gf_mul(0x0d, s3))))
    t2 = bxor(gf_mul(0x0d, s0), bxor(gf_mul(0x09, s1), bxor(gf_mul(0x0e, s2), gf_mul(0x0b, s3))))
    t3 = bxor(gf_mul(0x0b, s0), bxor(gf_mul(0x0d, s1), bxor(gf_mul(0x09, s2), gf_mul(0x0e, s3))))
    [t0, t1, t2, t3]
  end

  defp inv_mix_columns(state) do
    cols = for col <- 0..3 do
      column = for row <- 0..3, do: state |> Enum.at(row) |> Enum.at(col)
      inv_mix_col(column)
    end
    for row <- 0..3 do
      for col <- 0..3, do: cols |> Enum.at(col) |> Enum.at(row)
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # Core Block Cipher
  # ─────────────────────────────────────────────────────────────────────────────

  @doc """
  Encrypt a single 128-bit (16-byte) block with AES.

  ## Parameters
    - `block` — 16 bytes of plaintext
    - `key`   — 16 bytes (AES-128), 24 bytes (AES-192), or 32 bytes (AES-256)

  ## Returns
  16 bytes of ciphertext.

  ## Example

      iex> key   = Base.decode16!("2b7e151628aed2a6abf7158809cf4f3c", case: :lower)
      iex> plain = Base.decode16!("3243f6a8885a308d313198a2e0370734", case: :lower)
      iex> Base.encode16(CodingAdventures.Aes.aes_encrypt_block(plain, key), case: :lower)
      "3925841d02dc09fbdc118597196a0b32"
  """
  def aes_encrypt_block(block, key) when byte_size(block) == 16 do
    round_keys = expand_key(key)
    nr = length(round_keys) - 1

    # Initial AddRoundKey
    state =
      block
      |> bytes_to_state()
      |> add_round_key(Enum.at(round_keys, 0))

    # Nr-1 full rounds: SubBytes → ShiftRows → MixColumns → AddRoundKey
    state =
      Enum.reduce(1..(nr - 1)//1, state, fn rnd, st ->
        st
        |> sub_bytes()
        |> shift_rows()
        |> mix_columns()
        |> add_round_key(Enum.at(round_keys, rnd))
      end)

    # Final round: no MixColumns
    state
    |> sub_bytes()
    |> shift_rows()
    |> add_round_key(Enum.at(round_keys, nr))
    |> state_to_bytes()
  end

  def aes_encrypt_block(block, _key) do
    raise ArgumentError, "AES block must be 16 bytes, got #{byte_size(block)}"
  end

  @doc """
  Decrypt a single 128-bit (16-byte) block with AES.

  AES decryption uses the inverse of each operation in reverse:
  InvShiftRows → InvSubBytes → AddRoundKey → InvMixColumns.

  (AddRoundKey is its own inverse since XOR is self-inverse.)

  ## Parameters
    - `block` — 16 bytes of ciphertext
    - `key`   — 16, 24, or 32 bytes (same key used for encryption)

  ## Returns
  16 bytes of plaintext.
  """
  def aes_decrypt_block(block, key) when byte_size(block) == 16 do
    round_keys = expand_key(key)
    nr = length(round_keys) - 1

    # Start with final round key
    state =
      block
      |> bytes_to_state()
      |> add_round_key(Enum.at(round_keys, nr))

    # Nr-1 inverse rounds: InvShiftRows → InvSubBytes → AddRoundKey → InvMixColumns
    state =
      Enum.reduce((nr - 1)..1//-1, state, fn rnd, st ->
        st
        |> inv_shift_rows()
        |> inv_sub_bytes()
        |> add_round_key(Enum.at(round_keys, rnd))
        |> inv_mix_columns()
      end)

    # Final inverse round
    state
    |> inv_shift_rows()
    |> inv_sub_bytes()
    |> add_round_key(Enum.at(round_keys, 0))
    |> state_to_bytes()
  end

  def aes_decrypt_block(block, _key) do
    raise ArgumentError, "AES block must be 16 bytes, got #{byte_size(block)}"
  end
end
