defmodule CodingAdventures.ChaCha20Poly1305 do
  @moduledoc """
  ChaCha20-Poly1305 authenticated encryption (RFC 8439).

  This module implements the three components of the ChaCha20-Poly1305 AEAD:

  1. **ChaCha20** — A stream cipher using only ARX operations (Add, Rotate,
     XOR). Designed by Daniel J. Bernstein as an improvement over Salsa20.

  2. **Poly1305** — A one-time message authentication code (MAC) producing
     a 16-byte tag. Also by Bernstein.

  3. **AEAD** — The combined construction from RFC 8439 Section 2.8 providing
     both confidentiality and authenticity.

  ## Why ChaCha20 over AES?

  AES relies on lookup tables (S-boxes) and is only fast with hardware AES-NI
  instructions. On CPUs without AES-NI (common on mobile), AES is slow and
  vulnerable to cache-timing side-channel attacks. ChaCha20 uses only addition,
  rotation, and XOR — operations that run in constant time on every CPU.

  ## Where is it used?

  TLS 1.3, WireGuard, SSH (chacha20-poly1305@openssh.com), Chrome/Android.

  IMPORTANT: Educational implementation. Use a vetted library for real crypto.
  """

  import Bitwise

  # ==========================================================================
  # Constants
  # ==========================================================================
  #
  # The ChaCha20 state starts with four magic constants that spell
  # "expand 32-byte k" in ASCII (little-endian 32-bit words):
  #
  #   0x61707865 = "expa"
  #   0x3320646e = "nd 3"
  #   0x79622d32 = "2-by"
  #   0x6b206574 = "te k"
  # ==========================================================================

  @mask32 0xFFFFFFFF
  @const0 0x61707865
  @const1 0x3320646E
  @const2 0x79622D32
  @const3 0x6B206574

  # ==========================================================================
  # 32-bit Arithmetic Helpers
  # ==========================================================================
  #
  # Elixir integers are arbitrary precision, so we must mask to 32 bits
  # after additions. Left-rotate uses shift + OR + mask.
  # ==========================================================================

  defp add32(a, b), do: band(a + b, @mask32)

  defp rotl32(x, n), do: band(bsl(x, n) ||| bsr(x, 32 - n), @mask32)

  # ==========================================================================
  # ChaCha20 Quarter Round
  # ==========================================================================
  #
  # The core mixing function. Operates on four 32-bit words using ARX:
  #
  #   a += b; d ^= a; d <<<= 16
  #   c += d; b ^= c; b <<<= 12
  #   a += b; d ^= a; d <<<= 8
  #   c += d; b ^= c; b <<<= 7
  #
  # The rotation amounts (16, 12, 8, 7) maximize diffusion — after a few
  # rounds, every input bit affects every output bit.
  # ==========================================================================

  defp quarter_round(state, ai, bi, ci, di) do
    a = elem(state, ai)
    b = elem(state, bi)
    cc = elem(state, ci)
    d = elem(state, di)

    a = add32(a, b); d = rotl32(bxor(d, a), 16)
    cc = add32(cc, d); b = rotl32(bxor(b, cc), 12)
    a = add32(a, b); d = rotl32(bxor(d, a), 8)
    cc = add32(cc, d); b = rotl32(bxor(b, cc), 7)

    state
    |> put_elem(ai, a)
    |> put_elem(bi, b)
    |> put_elem(ci, cc)
    |> put_elem(di, d)
  end

  # ==========================================================================
  # ChaCha20 Block Function
  # ==========================================================================
  #
  # State: 4x4 matrix of 32-bit words:
  #
  #   [CONST0  CONST1  CONST2  CONST3]   <- magic constants
  #   [key[0]  key[1]  key[2]  key[3]]   <- first half of key
  #   [key[4]  key[5]  key[6]  key[7]]   <- second half of key
  #   [counter nonce0  nonce1  nonce2]    <- counter + nonce
  #
  # 20 rounds = 10 x (column round + diagonal round).
  # After rounds, add original state (feed-forward prevents inversion).
  # ==========================================================================

  defp chacha20_block(key, nonce_bin, counter) do
    <<k0::32-little, k1::32-little, k2::32-little, k3::32-little,
      k4::32-little, k5::32-little, k6::32-little, k7::32-little>> = key

    <<n0::32-little, n1::32-little, n2::32-little>> = nonce_bin

    initial = {
      @const0, @const1, @const2, @const3,
      k0, k1, k2, k3,
      k4, k5, k6, k7,
      band(counter, @mask32), n0, n1, n2
    }

    # 10 double-rounds (20 quarter-rounds total).
    #
    # Column rounds mix within the columns:
    #   QR(0,4, 8,12)  QR(1,5, 9,13)  QR(2,6,10,14)  QR(3,7,11,15)
    #
    # Diagonal rounds mix across the diagonals:
    #   QR(0,5,10,15)  QR(1,6,11,12)  QR(2,7, 8,13)  QR(3,4, 9,14)
    mixed =
      Enum.reduce(1..10, initial, fn _round, state ->
        state
        # Column round
        |> quarter_round(0, 4,  8, 12)
        |> quarter_round(1, 5,  9, 13)
        |> quarter_round(2, 6, 10, 14)
        |> quarter_round(3, 7, 11, 15)
        # Diagonal round
        |> quarter_round(0, 5, 10, 15)
        |> quarter_round(1, 6, 11, 12)
        |> quarter_round(2, 7,  8, 13)
        |> quarter_round(3, 4,  9, 14)
      end)

    # Feed-forward: add original state to prevent inversion.
    0..15
    |> Enum.map(fn i ->
      <<add32(elem(mixed, i), elem(initial, i))::32-little>>
    end)
    |> IO.iodata_to_binary()
  end

  # ==========================================================================
  # ChaCha20 Stream Cipher
  # ==========================================================================
  #
  # Encrypts by XOR-ing plaintext with a keystream generated 64 bytes at a
  # time, incrementing the block counter for each block. Since XOR is its
  # own inverse, encryption and decryption are the same operation.
  # ==========================================================================

  @doc """
  Encrypt (or decrypt) data using ChaCha20.

  Parameters:
  - `data` — input bytes (plaintext or ciphertext)
  - `key_bin` — 32-byte key
  - `nonce_bin` — 12-byte nonce
  - `counter` — initial block counter (usually 0 or 1)

  Returns the output bytes (same length as input).
  """
  def chacha20_encrypt(data, key_bin, nonce_bin, counter \\ 0)

  def chacha20_encrypt(<<>>, _key, _nonce, _counter), do: <<>>

  def chacha20_encrypt(data, key_bin, nonce_bin, counter)
      when byte_size(key_bin) == 32 and byte_size(nonce_bin) == 12 do
    do_chacha20(data, key_bin, nonce_bin, counter, <<>>)
  end

  defp do_chacha20(<<>>, _key, _nonce, _counter, acc), do: acc

  defp do_chacha20(data, key_bin, nonce_bin, counter, acc) do
    block = chacha20_block(key_bin, nonce_bin, counter)
    chunk_size = min(64, byte_size(data))
    <<chunk::binary-size(chunk_size), rest::binary>> = data
    <<ks::binary-size(chunk_size), _::binary>> = block

    xored = :crypto.exor(chunk, ks)

    do_chacha20(rest, key_bin, nonce_bin, counter + 1, acc <> xored)
  end

  # ==========================================================================
  # Poly1305 Message Authentication Code
  # ==========================================================================
  #
  # Poly1305 authenticates a message using polynomial evaluation modulo the
  # prime p = 2^130 - 5. It is a "one-time" MAC — the key must never reuse.
  #
  # Algorithm:
  #   1. Split 32-byte key into r (16, clamped) and s (16).
  #   2. For each 16-byte block:
  #      a. Read block as little-endian integer.
  #      b. Append 0x01 sentinel: add 2^(8 * block_length).
  #      c. acc = ((acc + block_with_sentinel) * r) mod (2^130 - 5)
  #   3. tag = (acc + s) mod 2^128
  #
  # Elixir has arbitrary-precision integers natively, so we can do the
  # 130-bit modular arithmetic directly with rem/2. No limb tricks needed!
  # ==========================================================================

  # The prime p = 2^130 - 5
  @poly_prime bsl(1, 130) - 5

  # 2^128 for the final mod
  @mod128 bsl(1, 128)

  @doc """
  Compute the Poly1305 MAC of a message.

  Parameters:
  - `message` — byte string to authenticate
  - `key_bin` — 32-byte one-time key

  Returns a 16-byte authentication tag.
  """
  def poly1305_mac(message, key_bin) when byte_size(key_bin) == 32 do
    <<r_bytes::binary-16, s_bytes::binary-16>> = key_bin

    # Clamp r per RFC 8439 Section 2.5.
    r_val = clamp_and_decode(r_bytes)

    # Decode s as a little-endian 128-bit integer.
    s_val = le_bytes_to_int(s_bytes)

    # Accumulate over 16-byte blocks.
    acc = poly1305_blocks(message, r_val, 0)

    # Final: tag = (acc + s) mod 2^128
    tag_val = rem(acc + s_val, @mod128)

    int_to_le_bytes(tag_val, 16)
  end

  defp poly1305_blocks(<<>>, _r, acc), do: acc

  defp poly1305_blocks(message, r_val, acc) do
    chunk_size = min(16, byte_size(message))
    <<chunk::binary-size(chunk_size), rest::binary>> = message

    # Read the block as a little-endian integer, then add the sentinel.
    # The sentinel is 1 << (8 * block_length), which appends a 0x01 byte
    # after the last byte of the block.
    n = le_bytes_to_int(chunk) + bsl(1, chunk_size * 8)

    # acc = ((acc + n) * r) mod p
    new_acc = rem((acc + n) * r_val, @poly_prime)

    poly1305_blocks(rest, r_val, new_acc)
  end

  # Clamp the r value and decode as a little-endian integer.
  #
  # Clamping (0-indexed bytes):
  #   bytes[3],bytes[7],bytes[11],bytes[15] AND 0x0F
  #   bytes[4],bytes[8],bytes[12]           AND 0xFC
  defp clamp_and_decode(<<b0, b1, b2, b3, b4, b5, b6, b7,
                          b8, b9, b10, b11, b12, b13, b14, b15>>) do
    clamped = <<
      b0, b1, b2, band(b3, 0x0F),
      band(b4, 0xFC), b5, b6, band(b7, 0x0F),
      band(b8, 0xFC), b9, b10, band(b11, 0x0F),
      band(b12, 0xFC), b13, b14, band(b15, 0x0F)
    >>

    le_bytes_to_int(clamped)
  end

  # Convert a little-endian binary to an integer.
  defp le_bytes_to_int(bytes) do
    bytes
    |> :binary.bin_to_list()
    |> Enum.with_index()
    |> Enum.reduce(0, fn {byte, idx}, acc ->
      acc + bsl(byte, idx * 8)
    end)
  end

  # Convert an integer to a little-endian binary of `size` bytes.
  defp int_to_le_bytes(val, num_bytes) do
    for i <- 0..(num_bytes - 1), into: <<>> do
      <<band(bsr(val, i * 8), 0xFF)>>
    end
  end

  # ==========================================================================
  # AEAD: Authenticated Encryption with Associated Data (RFC 8439 Section 2.8)
  # ==========================================================================
  #
  # The AEAD construction combines ChaCha20 and Poly1305:
  #
  # Encryption:
  #   1. poly_key = first 32 bytes of ChaCha20(zeros, key, nonce, counter=0)
  #   2. ciphertext = ChaCha20(plaintext, key, nonce, counter=1)
  #   3. mac_data = AAD || pad16(AAD) || CT || pad16(CT) ||
  #                 le64(len(AAD)) || le64(len(CT))
  #   4. tag = Poly1305(poly_key, mac_data)
  #
  # Decryption:
  #   1. Verify the tag FIRST (don't release unauthenticated plaintext).
  #   2. If valid, decrypt with ChaCha20(ct, key, nonce, counter=1).
  # ==========================================================================

  @doc """
  Encrypt and authenticate data.

  Parameters:
  - `plaintext` — data to encrypt
  - `key_bin` — 32-byte key
  - `nonce_bin` — 12-byte nonce (never reuse with the same key!)
  - `aad` — associated data (authenticated but not encrypted)

  Returns `{ciphertext, tag}` where tag is 16 bytes.
  """
  def aead_encrypt(plaintext, key_bin, nonce_bin, aad \\ <<>>)
      when byte_size(key_bin) == 32 and byte_size(nonce_bin) == 12 do
    # Step 1: Generate the Poly1305 one-time key.
    <<poly_key::binary-32, _rest::binary>> =
      chacha20_encrypt(:binary.copy(<<0>>, 32), key_bin, nonce_bin, 0)

    # Step 2: Encrypt with counter=1.
    ct = chacha20_encrypt(plaintext, key_bin, nonce_bin, 1)

    # Step 3: Build MAC input and compute tag.
    mac_data = build_mac_data(aad, ct)
    tag_val = poly1305_mac(mac_data, poly_key)

    {ct, tag_val}
  end

  @doc """
  Decrypt and verify authenticated data.

  Parameters:
  - `ct` — encrypted data
  - `key_bin` — 32-byte key
  - `nonce_bin` — 12-byte nonce
  - `aad` — associated data (must match encryption)
  - `tag` — 16-byte authentication tag

  Returns `{:ok, plaintext}` on success, `{:error, reason}` on failure.
  """
  def aead_decrypt(ct, key_bin, nonce_bin, aad, tag)
      when byte_size(key_bin) == 32 and byte_size(nonce_bin) == 12 and byte_size(tag) == 16 do
    # Step 1: Generate the Poly1305 one-time key.
    <<poly_key::binary-32, _rest::binary>> =
      chacha20_encrypt(:binary.copy(<<0>>, 32), key_bin, nonce_bin, 0)

    # Step 2: Verify the tag BEFORE decrypting.
    mac_data = build_mac_data(aad, ct)
    expected_tag = poly1305_mac(mac_data, poly_key)

    if constant_time_equal?(tag, expected_tag) do
      # Step 3: Decrypt.
      plaintext = chacha20_encrypt(ct, key_bin, nonce_bin, 1)
      {:ok, plaintext}
    else
      {:error, :authentication_failed}
    end
  end

  # Pad data to a 16-byte boundary with zero bytes.
  defp pad16(data) do
    remainder = rem(byte_size(data), 16)
    if remainder == 0, do: <<>>, else: :binary.copy(<<0>>, 16 - remainder)
  end

  # Build the MAC input per RFC 8439 Section 2.8.
  defp build_mac_data(aad, ct) do
    aad <> pad16(aad) <>
    ct <> pad16(ct) <>
    <<byte_size(aad)::64-little>> <>
    <<byte_size(ct)::64-little>>
  end

  # Constant-time comparison to prevent timing side-channel attacks.
  defp constant_time_equal?(a, b) when byte_size(a) != byte_size(b), do: false

  defp constant_time_equal?(a, b) do
    a_bytes = :binary.bin_to_list(a)
    b_bytes = :binary.bin_to_list(b)

    diff =
      Enum.zip(a_bytes, b_bytes)
      |> Enum.reduce(0, fn {x, y}, acc -> bor(acc, bxor(x, y)) end)

    diff == 0
  end
end
