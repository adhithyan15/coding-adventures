# CodingAdventures.AesModes — AES Modes of Operation
#
# AES operates on fixed 128-bit (16-byte) blocks. To encrypt messages longer
# than one block, you need a "mode of operation" that defines how multiple
# block cipher calls chain together. The choice of mode critically affects
# security:
#
#   ECB — Electronic Codebook (INSECURE, educational only)
#   CBC — Cipher Block Chaining (legacy, vulnerable to padding oracles)
#   CTR — Counter mode (modern, stream cipher from block cipher)
#   GCM — Galois/Counter Mode (authenticated encryption, gold standard)
#
# Why do modes matter?
# ────────────────────
# A raw block cipher is a fixed-width permutation: 16 bytes in, 16 bytes out.
# Real messages are longer. If you encrypt each block independently (ECB),
# identical plaintext blocks produce identical ciphertext blocks — the famous
# "ECB penguin" shows image structure leaking through encryption.
#
# Dependencies: CodingAdventures.Aes (provides aes_encrypt_block, aes_decrypt_block)

defmodule CodingAdventures.AesModes do
  import Bitwise
  alias CodingAdventures.Aes

  @moduledoc """
  AES modes of operation — ECB, CBC, CTR, and GCM.

  ## Public API

    - `ecb_encrypt/2`, `ecb_decrypt/2` — Electronic Codebook (INSECURE)
    - `cbc_encrypt/3`, `cbc_decrypt/3` — Cipher Block Chaining
    - `ctr_encrypt/3`, `ctr_decrypt/3` — Counter mode
    - `gcm_encrypt/4`, `gcm_decrypt/5` — Galois/Counter Mode (authenticated)
    - `pkcs7_pad/1`, `pkcs7_unpad/1`   — PKCS#7 padding utilities
  """

  # ─────────────────────────────────────────────────────────────────────────────
  # Utility: XOR two equal-length binaries
  #
  # XOR is the fundamental building block of symmetric cryptography. When you
  # XOR plaintext with a random key of the same length, you get a one-time pad.
  # Modes like CTR generate pseudorandom keystream via AES and XOR it with
  # plaintext, approximating a one-time pad.
  # ─────────────────────────────────────────────────────────────────────────────

  defp xor_bytes(a, b) when byte_size(a) == byte_size(b) do
    :crypto.exor(a, b)
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # PKCS#7 Padding
  #
  # Block ciphers need input that is an exact multiple of the block size (16
  # bytes for AES). PKCS#7 padding appends N bytes, each with value N, where
  # N = 16 - (length mod 16). If the input is already aligned, a full 16-byte
  # padding block is added (so the unpadder always has something to remove).
  #
  # Example: "HELLO" (5 bytes) -> "HELLO" + 11 bytes of 0x0B
  # Example: 16 bytes          -> 16 bytes + 16 bytes of 0x10
  # ─────────────────────────────────────────────────────────────────────────────

  @doc "Pad data to a multiple of 16 bytes using PKCS#7."
  def pkcs7_pad(data) when is_binary(data) do
    pad_len = 16 - rem(byte_size(data), 16)
    data <> :binary.copy(<<pad_len>>, pad_len)
  end

  @doc "Remove PKCS#7 padding. Raises on invalid padding."
  def pkcs7_unpad(data) when is_binary(data) do
    len = byte_size(data)
    if len == 0 or rem(len, 16) != 0 do
      raise ArgumentError, "pkcs7_unpad: data must be non-empty and multiple of 16"
    end
    pad_val = :binary.at(data, len - 1)
    if pad_val < 1 or pad_val > 16 do
      raise ArgumentError, "Invalid PKCS#7 padding"
    end
    # Constant-time padding validation: accumulate differences with OR
    # instead of short-circuiting on the first mismatch (prevents timing attacks)
    padding_region = binary_part(data, len - pad_val, pad_val)
    diff = padding_region
      |> :binary.bin_to_list()
      |> Enum.reduce(0, fn byte, acc -> Bitwise.bor(acc, Bitwise.bxor(byte, pad_val)) end)
    if diff != 0 do
      raise ArgumentError, "Invalid PKCS#7 padding"
    end
    binary_part(data, 0, len - pad_val)
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # ECB — Electronic Codebook Mode (INSECURE)
  #
  # The simplest mode: encrypt each 16-byte block independently.
  #
  #   C[i] = AES_encrypt(P[i], key)
  #
  # ECB is deterministic: the same plaintext block always produces the same
  # ciphertext block. This leaks patterns — NEVER use for real data.
  # ─────────────────────────────────────────────────────────────────────────────

  @doc "Encrypt with ECB mode (INSECURE — educational only). PKCS#7 padded."
  def ecb_encrypt(plaintext, key) when is_binary(plaintext) and is_binary(key) do
    padded = pkcs7_pad(plaintext)
    for <<block::binary-size(16) <- padded>>, into: <<>> do
      Aes.aes_encrypt_block(block, key)
    end
  end

  @doc "Decrypt with ECB mode. Removes PKCS#7 padding."
  def ecb_decrypt(ciphertext, key)
      when is_binary(ciphertext) and is_binary(key) do
    len = byte_size(ciphertext)
    if len == 0 or rem(len, 16) != 0 do
      raise ArgumentError, "ecb_decrypt: ciphertext must be non-empty multiple of 16"
    end
    plaintext =
      for <<block::binary-size(16) <- ciphertext>>, into: <<>> do
        Aes.aes_decrypt_block(block, key)
      end
    pkcs7_unpad(plaintext)
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # CBC — Cipher Block Chaining Mode
  #
  #   C[0] = IV
  #   C[i] = AES_encrypt(P[i] XOR C[i-1], key)
  #
  # Decryption:
  #   P[i] = AES_decrypt(C[i], key) XOR C[i-1]
  #
  # Requires unpredictable IV. Vulnerable to padding oracle attacks.
  # ─────────────────────────────────────────────────────────────────────────────

  @doc "Encrypt with CBC mode. IV must be 16 bytes. PKCS#7 padded."
  def cbc_encrypt(plaintext, key, iv)
      when is_binary(plaintext) and is_binary(key) and byte_size(iv) == 16 do
    padded = pkcs7_pad(plaintext)
    {ciphertext, _prev} =
      padded
      |> chunk_blocks()
      |> Enum.reduce({<<>>, iv}, fn block, {acc, prev_block} ->
        xored = xor_bytes(block, prev_block)
        encrypted = Aes.aes_encrypt_block(xored, key)
        {acc <> encrypted, encrypted}
      end)
    ciphertext
  end

  @doc "Decrypt with CBC mode. Removes PKCS#7 padding."
  def cbc_decrypt(ciphertext, key, iv)
      when is_binary(ciphertext) and is_binary(key) and byte_size(iv) == 16 do
    len = byte_size(ciphertext)
    if len == 0 or rem(len, 16) != 0 do
      raise ArgumentError, "cbc_decrypt: ciphertext must be non-empty multiple of 16"
    end
    {plaintext, _prev} =
      ciphertext
      |> chunk_blocks()
      |> Enum.reduce({<<>>, iv}, fn block, {acc, prev_block} ->
        decrypted = Aes.aes_decrypt_block(block, key)
        plain_block = xor_bytes(decrypted, prev_block)
        {acc <> plain_block, block}
      end)
    pkcs7_unpad(plaintext)
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # CTR — Counter Mode
  #
  # Turns block cipher into stream cipher:
  #   keystream[i] = AES_encrypt(nonce_12 || counter_4_be, key)
  #   C[i] = P[i] XOR keystream[i]
  #
  # No padding needed. Counter starts at 1 (GCM reserves 0 for tag).
  # NEVER reuse (key, nonce) pair.
  # ─────────────────────────────────────────────────────────────────────────────

  defp build_counter_block(nonce, ctr_val) do
    nonce <> <<ctr_val::big-unsigned-32>>
  end

  @doc "Encrypt with CTR mode. Nonce must be 12 bytes. No padding."
  def ctr_encrypt(plaintext, key, nonce)
      when is_binary(plaintext) and is_binary(key) and byte_size(nonce) == 12 do
    ctr_process(plaintext, key, nonce, 1, <<>>)
  end

  @doc "Decrypt with CTR mode (same operation as encrypt)."
  def ctr_decrypt(ciphertext, key, nonce)
      when is_binary(ciphertext) and is_binary(key) and byte_size(nonce) == 12 do
    ctr_encrypt(ciphertext, key, nonce)
  end

  defp ctr_process(<<>>, _key, _nonce, _ctr, acc), do: acc
  defp ctr_process(data, key, nonce, ctr_val, acc) do
    block_size = min(byte_size(data), 16)
    <<block::binary-size(block_size), rest::binary>> = data
    counter_block = build_counter_block(nonce, ctr_val)
    keystream = Aes.aes_encrypt_block(counter_block, key)
    # XOR only the bytes we have (handles partial last block)
    ks_slice = binary_part(keystream, 0, block_size)
    encrypted = xor_bytes(block, ks_slice)
    ctr_process(rest, key, nonce, ctr_val + 1, acc <> encrypted)
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # GCM — Galois/Counter Mode (Authenticated Encryption)
  #
  # GCM = CTR encryption + GHASH polynomial MAC over GF(2^128).
  #
  # Algorithm:
  #   1. H = AES_encrypt(0^128, key)       — hash subkey
  #   2. J0 = IV || 0x00000001             — initial counter
  #   3. CTR encrypt starting at J0+1
  #   4. GHASH over padded AAD + padded CT + lengths
  #   5. Tag = GHASH_result XOR AES_encrypt(J0, key)
  #
  # GF(2^128) uses polynomial x^128 + x^7 + x^2 + x + 1.
  # Reduction constant R = 0xE1 << 120.
  #
  # We represent 128-bit values as {hi, lo} tuples of 64-bit integers.
  # ─────────────────────────────────────────────────────────────────────────────

  # The reduction polynomial R = 0xE1 << 120, hi part only (lo = 0)
  @r_hi 0xE100000000000000

  defp bytes_to_u128(<<hi::big-unsigned-64, lo::big-unsigned-64>>) do
    {hi, lo}
  end

  defp u128_to_bytes({hi, lo}) do
    <<hi::big-unsigned-64, lo::big-unsigned-64>>
  end

  # GF(2^128) multiplication: bit-by-bit algorithm.
  # Both inputs and output are {hi, lo} tuples.
  defp gf128_mul({x_hi, x_lo}, {y_hi, y_lo}) do
    gf128_mul_loop(x_hi, x_lo, y_hi, y_lo, 0, 0, 0)
  end

  defp gf128_mul_loop(_x_hi, _x_lo, _v_hi, _v_lo, z_hi, z_lo, 128) do
    {z_hi, z_lo}
  end

  defp gf128_mul_loop(x_hi, x_lo, v_hi, v_lo, z_hi, z_lo, i) do
    # Extract bit i of X (MSB-first)
    {word, bit_pos} =
      if i < 64, do: {x_hi, 63 - i}, else: {x_lo, 127 - i}

    {z_hi2, z_lo2} =
      if band(word >>> bit_pos, 1) == 1 do
        {bxor(z_hi, v_hi), bxor(z_lo, v_lo)}
      else
        {z_hi, z_lo}
      end

    # Right-shift V by 1, conditionally XOR R
    low_bit = band(v_lo, 1)
    new_v_lo = band(v_lo >>> 1, 0x7FFFFFFFFFFFFFFF) ||| band(v_hi, 1) <<< 63
    new_v_hi = band(v_hi >>> 1, 0x7FFFFFFFFFFFFFFF)

    {new_v_hi2, new_v_lo2} =
      if low_bit == 1 do
        {bxor(new_v_hi, @r_hi), new_v_lo}
      else
        {new_v_hi, new_v_lo}
      end

    gf128_mul_loop(x_hi, x_lo, new_v_hi2, new_v_lo2, z_hi2, z_lo2, i + 1)
  end

  # GHASH: polynomial hash over GF(2^128).
  # X[0] = 0; X[i] = (X[i-1] XOR block[i]) * H
  defp ghash_compute(h_pair, data) do
    data
    |> pad_to_blocks()
    |> Enum.reduce({0, 0}, fn block, {x_hi, x_lo} ->
      {b_hi, b_lo} = bytes_to_u128(block)
      xored = {bxor(x_hi, b_hi), bxor(x_lo, b_lo)}
      gf128_mul(xored, h_pair)
    end)
  end

  # Split data into 16-byte blocks, zero-padding the last one if needed.
  defp pad_to_blocks(<<>>), do: []
  defp pad_to_blocks(data) when byte_size(data) <= 16 do
    pad_len = 16 - byte_size(data)
    [data <> :binary.copy(<<0>>, pad_len)]
  end
  defp pad_to_blocks(<<block::binary-size(16), rest::binary>>) do
    [block | pad_to_blocks(rest)]
  end

  # Pad data to multiple of 16 bytes with zeros
  defp gcm_pad(data) do
    leftover = rem(byte_size(data), 16)
    if leftover == 0, do: data, else: data <> :binary.copy(<<0>>, 16 - leftover)
  end

  @doc """
  GCM encrypt: returns {ciphertext, 16-byte tag}.

  ## Parameters
    - `plaintext` — arbitrary-length binary
    - `key` — 16, 24, or 32-byte AES key
    - `iv` — 12-byte initialization vector
    - `aad` — additional authenticated data (authenticated but not encrypted)
  """
  def gcm_encrypt(plaintext, key, iv, aad \\ <<>>)
      when is_binary(plaintext) and is_binary(key) and byte_size(iv) == 12 and is_binary(aad) do
    # Step 1: Hash subkey H = AES_encrypt(0^128, key)
    h_bytes = Aes.aes_encrypt_block(<<0::128>>, key)
    h_pair = bytes_to_u128(h_bytes)

    # Step 2: J0 = IV || 0x00000001
    j0 = iv <> <<0, 0, 0, 1>>

    # Step 3: CTR encrypt starting at counter = 2
    ciphertext = ctr_process(plaintext, key, iv, 2, <<>>)

    # Step 4: GHASH over AAD||pad||CT||pad||len_AAD||len_CT
    ghash_input = gcm_pad(aad) <> gcm_pad(ciphertext)
      <> <<byte_size(aad) * 8::big-unsigned-64>>
      <> <<byte_size(ciphertext) * 8::big-unsigned-64>>
    {tag_hi, tag_lo} = ghash_compute(h_pair, ghash_input)

    # Step 5: Tag = GHASH XOR AES_encrypt(J0, key)
    j0_enc = Aes.aes_encrypt_block(j0, key)
    {j0_hi, j0_lo} = bytes_to_u128(j0_enc)
    final_tag = u128_to_bytes({bxor(tag_hi, j0_hi), bxor(tag_lo, j0_lo)})

    {ciphertext, final_tag}
  end

  @doc """
  GCM decrypt: verifies tag, then decrypts.

  Returns `{:ok, plaintext}` or `{:error, reason}` if tag is invalid.
  """
  def gcm_decrypt(ciphertext, key, iv, aad, tag)
      when is_binary(ciphertext) and is_binary(key) and byte_size(iv) == 12
           and is_binary(aad) and byte_size(tag) == 16 do
    # Compute hash subkey
    h_bytes = Aes.aes_encrypt_block(<<0::128>>, key)
    h_pair = bytes_to_u128(h_bytes)

    # Compute expected tag
    j0 = iv <> <<0, 0, 0, 1>>
    ghash_input = gcm_pad(aad) <> gcm_pad(ciphertext)
      <> <<byte_size(aad) * 8::big-unsigned-64>>
      <> <<byte_size(ciphertext) * 8::big-unsigned-64>>
    {tag_hi, tag_lo} = ghash_compute(h_pair, ghash_input)

    j0_enc = Aes.aes_encrypt_block(j0, key)
    {j0_hi, j0_lo} = bytes_to_u128(j0_enc)
    expected_tag = u128_to_bytes({bxor(tag_hi, j0_hi), bxor(tag_lo, j0_lo)})

    # Constant-time tag comparison: accumulate byte differences with OR
    # instead of short-circuiting on the first mismatch (prevents timing attacks)
    diff = :binary.bin_to_list(expected_tag)
      |> Enum.zip(:binary.bin_to_list(tag))
      |> Enum.reduce(0, fn {a, b}, acc -> Bitwise.bor(acc, Bitwise.bxor(a, b)) end)

    if diff == 0 do
      plaintext = ctr_process(ciphertext, key, iv, 2, <<>>)
      {:ok, plaintext}
    else
      {:error, "gcm_decrypt: authentication tag mismatch"}
    end
  end

  # ─────────────────────────────────────────────────────────────────────────────
  # Helper: split a binary into a list of 16-byte blocks
  # ─────────────────────────────────────────────────────────────────────────────

  defp chunk_blocks(data), do: chunk_blocks(data, [])
  defp chunk_blocks(<<>>, acc), do: Enum.reverse(acc)
  defp chunk_blocks(<<block::binary-size(16), rest::binary>>, acc) do
    chunk_blocks(rest, [block | acc])
  end
end
