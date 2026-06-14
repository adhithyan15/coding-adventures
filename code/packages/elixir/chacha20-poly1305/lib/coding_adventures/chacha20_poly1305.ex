# CodingAdventures.ChaCha20Poly1305 — ChaCha20-Poly1305 AEAD (RFC 8439)
#
# ChaCha20-Poly1305 is a modern authenticated encryption with associated data
# (AEAD) cipher, standardised in RFC 8439 (formerly RFC 7539). It is used in
# TLS 1.3, WireGuard, SSH, and many other protocols because it is:
#
#   - Fast on CPUs without AES hardware acceleration (mobile, embedded)
#   - Immune to timing side-channels (pure arithmetic, no table lookups)
#   - Simpler to implement correctly than AES-GCM
#
# The construction pairs two algorithms:
#
#   ChaCha20  — a stream cipher that generates a pseudorandom keystream
#               from (key, nonce, counter). XOR with plaintext → ciphertext.
#
#   Poly1305  — a one-time MAC (message authentication code) built on
#               arithmetic modulo the prime 2^130 - 5.  It authenticates
#               both the ciphertext and the associated data (AAD) so an
#               attacker cannot tamper with either.
#
# Putting them together:
#
#   Encrypt:
#     1. Derive a 32-byte Poly1305 key: chacha20_block(key, counter=0, nonce)[0..31]
#     2. Encrypt plaintext:             chacha20_encrypt(pt, key, nonce, counter=1)
#     3. Build MAC input:               pad16(aad) || pad16(ct) || len64(aad) || len64(ct)
#     4. Compute tag:                   poly1305_mac(mac_input, poly_key)
#     5. Return (ciphertext, tag)
#
#   Decrypt:
#     1-4. Same as encrypt to recompute expected tag
#     5. Constant-time compare expected_tag == provided_tag
#     6. If equal, decrypt ciphertext (same XOR as encrypt); else return error
#
# Security note:  NEVER reuse a (key, nonce) pair.  Poly1305 uses a one-time
# key derived fresh for each nonce.  Reuse leaks the authentication key and
# breaks confidentiality.
#
# ─── ChaCha20 Block Function ──────────────────────────────────────────────────
#
# The ChaCha20 state is a 4×4 matrix of 32-bit words (16 words = 512 bits).
# It is initialised as:
#
#   constants[0]  constants[1]  constants[2]  constants[3]
#   key[0]        key[1]        key[2]        key[3]
#   key[4]        key[5]        key[6]        key[7]
#   counter       nonce[0]      nonce[1]      nonce[2]
#
# The "constants" are the ASCII bytes of "expand 32-byte k":
#   0x61707865  0x3320646e  0x79622d32  0x6b206574
#
# Twenty rounds (10 double-rounds) of the quarter-round function mix the state.
# Then the original state is added back (mod 2^32) to produce the keystream block.
#
# Quarter Round  QR(a, b, c, d):
#
#   a += b;  d ^= a;  d <<<= 16;
#   c += d;  b ^= c;  b <<<= 12;
#   a += b;  d ^= a;  d <<<<= 8;
#   c += d;  b ^= c;  b <<<<= 7;
#
# A double-round applies QR to columns then to diagonals:
#
#   Column rounds:   QR(0,4,8,12) QR(1,5,9,13) QR(2,6,10,14) QR(3,7,11,15)
#   Diagonal rounds: QR(0,5,10,15) QR(1,6,11,12) QR(2,7,8,13) QR(3,4,9,14)
#
# ─── Poly1305 MAC ─────────────────────────────────────────────────────────────
#
# Poly1305 evaluates a polynomial over a prime field.  Given a 32-byte one-time
# key split as (r || s):
#
#   r = lower 16 bytes, clamped (certain bits zeroed for implementation reasons)
#   s = upper 16 bytes (an opaque additive constant)
#
# For each 16-byte chunk m_i of the message (with a 1-bit appended):
#
#   acc = ((acc + m_i) * r) mod (2^130 - 5)
#
# Final tag = (acc + s) mod 2^128, serialised little-endian.
#
# Elixir's arbitrary-precision integers make this beautifully clean — no 128-bit
# overflow gymnastics required.

defmodule CodingAdventures.ChaCha20Poly1305 do
  import Bitwise

  @moduledoc """
  ChaCha20-Poly1305 AEAD cipher (RFC 8439).

  Provides authenticated encryption with associated data (AEAD) using the
  ChaCha20 stream cipher for encryption and Poly1305 for authentication.

  ## Quick Start

      iex> key   = :crypto.strong_rand_bytes(32)
      iex> nonce = :crypto.strong_rand_bytes(12)
      iex> {ct, tag} = CodingAdventures.ChaCha20Poly1305.aead_encrypt("hello", key, nonce, "")
      iex> {:ok, "hello"} = CodingAdventures.ChaCha20Poly1305.aead_decrypt(ct, key, nonce, "", tag)

  ## Security

  - Key must be 32 bytes (256 bits), generated with a CSPRNG.
  - Nonce must be 12 bytes (96 bits) and MUST NOT be reused for the same key.
    Nonce reuse destroys both confidentiality and authentication.
  - This implementation is for education.  For production Elixir, use
    `:crypto.crypto_one_time_aead/6`.
  """

  # "expand 32-byte k" in little-endian 32-bit words — the magic ChaCha20
  # constants that were chosen to have no hidden trapdoors (nothing-up-my-sleeve
  # numbers: they are just an ASCII string).
  @constants [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574]

  # Poly1305 prime: 2^130 - 5.  Chosen because arithmetic mod this prime is
  # efficient — 2^130 is a round power-of-two, and subtracting 5 keeps it prime.
  @poly_prime (1 <<< 130) - 5

  # Poly1305 r-clamping mask.  RFC 8439 §2.5 requires that certain bits of r
  # are zeroed before use.  This prevents differential attacks on the MAC.
  # In hex: 0x0FFFFFFC_0FFFFFFC_0FFFFFFC_0FFFFFFF
  @poly_clamp 0x0FFFFFFC0FFFFFFC0FFFFFFC0FFFFFFF

  # 32-bit mask used to keep additions within a single 32-bit word.
  @u32 0xFFFFFFFF

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Generate one 64-byte ChaCha20 keystream block.

  The ChaCha20 block function takes a 256-bit key, a 32-bit counter, and a
  96-bit nonce and produces 64 bytes of pseudorandom output.  Blocks for
  successive 64-byte segments of the keystream are generated by incrementing
  the counter.

  ## Parameters

  - `key`     — 32-byte binary key
  - `counter` — 32-bit block counter (non-negative integer)
  - `nonce`   — 12-byte nonce

  ## Returns

  64-byte binary keystream block.

  ## Examples

      # RFC 8439 §2.1.2 test vector
      key   = :binary.list_to_bin(Enum.to_list(0..31))
      nonce = Base.decode16!("000000090000004A00000000")
      block = CodingAdventures.ChaCha20Poly1305.chacha20_block(key, 1, nonce)
  """
  def chacha20_block(key, counter, nonce)
      when byte_size(key) == 32 and is_integer(counter) and counter >= 0 and
             byte_size(nonce) == 12 do
    # Decode key and nonce as sequences of little-endian 32-bit words.
    # The `for <<w::little-32 <- bin>>` comprehension is idiomatic Elixir for
    # iterating over a binary in fixed-size chunks.
    key_words = for(<<w::little-32 <- key>>, do: w)
    nonce_words = for(<<w::little-32 <- nonce>>, do: w)

    # Assemble the 16-word (512-bit) initial state as a flat list, then convert
    # to a tuple for O(1) random access during the inner rounds.
    initial_list = @constants ++ key_words ++ [counter] ++ nonce_words
    state0 = List.to_tuple(initial_list)

    # Apply 10 double-rounds (= 20 rounds total).
    # Each double-round: 4 column quarter-rounds, then 4 diagonal quarter-rounds.
    state =
      Enum.reduce(1..10, state0, fn _, s ->
        s
        # Column rounds — operate on the four columns of the 4×4 matrix:
        |> quarter_round(0, 4, 8, 12)
        |> quarter_round(1, 5, 9, 13)
        |> quarter_round(2, 6, 10, 14)
        |> quarter_round(3, 7, 11, 15)
        # Diagonal rounds — operate on the four diagonals:
        |> quarter_round(0, 5, 10, 15)
        |> quarter_round(1, 6, 11, 12)
        |> quarter_round(2, 7, 8, 13)
        |> quarter_round(3, 4, 9, 14)
      end)

    # Add the initial state to the mixed state (mod 2^32 per word).
    # This step prevents reversing the block function from the output alone,
    # turning the Feistel-like permutation into a proper PRF.
    final_words =
      Enum.map(0..15, fn i ->
        band(elem(state, i) + Enum.at(initial_list, i), @u32)
      end)

    # Serialise each 32-bit word back as a little-endian 32-bit chunk.
    for w <- final_words, into: <<>>, do: <<w::little-32>>
  end

  @doc """
  Encrypt (or decrypt) data using the ChaCha20 stream cipher.

  ChaCha20 is a stream cipher: encryption and decryption are identical
  operations (XOR with the keystream).  The keystream is generated by
  concatenating successive block outputs starting at `counter`.

  For AEAD use, the RFC requires the keystream to start at counter=1
  (counter=0 is reserved for deriving the Poly1305 key).

  ## Parameters

  - `plaintext` — binary to encrypt/decrypt
  - `key`       — 32-byte key
  - `nonce`     — 12-byte nonce
  - `counter`   — starting block counter (default: 1)

  ## Returns

  Binary of the same length as `plaintext`.
  """
  def chacha20_encrypt(plaintext, key, nonce, counter \\ 1)

  def chacha20_encrypt(<<>>, _key, _nonce, _counter), do: <<>>

  def chacha20_encrypt(plaintext, key, nonce, counter) do
    do_chacha20_encrypt(plaintext, key, nonce, counter, <<>>)
  end

  @doc """
  Compute a Poly1305 MAC tag.

  Given a 32-byte one-time key and a message of arbitrary length, returns
  a 16-byte authentication tag.

  The key is used ONCE.  The Poly1305 key for AEAD use is derived fresh for
  each message via `chacha20_block(key, 0, nonce)`.

  ## Parameters

  - `message` — binary to authenticate
  - `key`     — 32-byte one-time Poly1305 key

  ## Returns

  16-byte authentication tag (binary).
  """
  def poly1305_mac(message, key) when byte_size(key) == 32 do
    # Split the 32-byte key into r (lower 16 bytes) and s (upper 16 bytes).
    <<r_bytes::binary-16, s_bytes::binary-16>> = key

    # Clamp r: zero out bits that Poly1305 specifies must be clear.
    # This is required by the spec to make differential analysis harder.
    r = :binary.decode_unsigned(r_bytes, :little) &&& @poly_clamp

    # s is used as a one-time pad added to the final accumulator.
    s = :binary.decode_unsigned(s_bytes, :little)

    # Process the message in 16-byte chunks, accumulating the polynomial.
    acc = poly1305_accumulate(message, r, 0)

    # Final tag: (acc + s) mod 2^128, then serialise little-endian.
    tag_int = rem(acc + s, 1 <<< 128)
    tag_int |> :binary.encode_unsigned(:little) |> pad_to(16)
  end

  @doc """
  AEAD encrypt: authenticate-then-encrypt with ChaCha20-Poly1305.

  Returns `{ciphertext, tag}` where `tag` is a 16-byte Poly1305 MAC over
  the ciphertext and associated data.

  The associated data (`aad`) is authenticated but NOT encrypted — it is
  typically protocol metadata like packet headers.

  ## Parameters

  - `plaintext` — binary to encrypt
  - `key`       — 32-byte key
  - `nonce`     — 12-byte nonce (must be unique per (key, message) pair)
  - `aad`       — additional authenticated data (may be empty binary `""`)

  ## Returns

  `{ciphertext, tag}` tuple where both are binaries.
  """
  def aead_encrypt(plaintext, key, nonce, aad)
      when byte_size(key) == 32 and byte_size(nonce) == 12 do
    # Derive the Poly1305 one-time key from block 0 of the keystream.
    # Blocks 1..N are then used for encrypting the message.
    poly_key = binary_part(chacha20_block(key, 0, nonce), 0, 32)

    # Encrypt the plaintext (counter starts at 1 per RFC 8439 §2.6).
    ciphertext = chacha20_encrypt(plaintext, key, nonce, 1)

    # Build the MAC input per RFC 8439 §2.8:
    #   pad16(aad) || pad16(ciphertext) || len64(aad) || len64(ciphertext)
    # Padding to 16-byte boundaries ensures chunk alignment without ambiguity.
    # The lengths disambiguate message boundaries (preventing length-extension).
    mac_data =
      pad16(aad) <>
        pad16(ciphertext) <>
        <<byte_size(aad)::little-64>> <>
        <<byte_size(ciphertext)::little-64>>

    tag = poly1305_mac(mac_data, poly_key)
    {ciphertext, tag}
  end

  @doc """
  AEAD decrypt: verify tag then decrypt with ChaCha20-Poly1305.

  Returns `{:ok, plaintext}` if the tag is valid, or
  `{:error, :authentication_failed}` if the tag does not match.

  The tag check is performed in constant time to prevent timing oracle attacks
  (an attacker must not learn anything from how long verification takes).

  ## Parameters

  - `ciphertext` — binary to decrypt
  - `key`        — 32-byte key
  - `nonce`      — 12-byte nonce
  - `aad`        — additional authenticated data (must match what was used during encrypt)
  - `tag`        — 16-byte Poly1305 tag to verify

  ## Returns

  `{:ok, plaintext}` or `{:error, :authentication_failed}`.
  """
  def aead_decrypt(ciphertext, key, nonce, aad, tag)
      when byte_size(key) == 32 and byte_size(nonce) == 12 and byte_size(tag) == 16 do
    # Recompute the Poly1305 key using block 0 (same as during encryption).
    poly_key = binary_part(chacha20_block(key, 0, nonce), 0, 32)

    # Build MAC input identically to encryption.
    mac_data =
      pad16(aad) <>
        pad16(ciphertext) <>
        <<byte_size(aad)::little-64>> <>
        <<byte_size(ciphertext)::little-64>>

    expected_tag = poly1305_mac(mac_data, poly_key)

    # Use a constant-time comparison to prevent timing oracle attacks.
    # An attacker must not be able to determine HOW MANY bytes matched.
    if constant_time_eq(expected_tag, tag) do
      {:ok, chacha20_encrypt(ciphertext, key, nonce, 1)}
    else
      {:error, :authentication_failed}
    end
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Quarter Round — the core mixing function of ChaCha20.
  #
  # Takes the state tuple and four indices (a, b, c, d), reads the words at
  # those positions, applies 4 ARX (Add-Rotate-XOR) steps, then writes the
  # updated words back.
  #
  # ARX operations are fast on all CPUs and produce good avalanche behaviour
  # without any table lookups (avoiding cache-timing side-channels).
  #
  # Variable naming: sa/sb/sc/sd are the "slot" values at positions a/b/c/d.
  # We deliberately avoid names like `a`, `b`, `c`, `d` as standalone bindings
  # because they shadow the position indices.  The `s` prefix ("slot") avoids
  # any Elixir reserved-word collision.
  defp quarter_round(state, pa, pb, pc, pd) do
    sa = elem(state, pa)
    sb = elem(state, pb)
    sc = elem(state, pc)
    sd = elem(state, pd)

    # Step 1
    sa = band(sa + sb, @u32)
    sd = bxor(sd, sa)
    sd = rotl32(sd, 16)
    # Step 2
    sc = band(sc + sd, @u32)
    sb = bxor(sb, sc)
    sb = rotl32(sb, 12)
    # Step 3
    sa = band(sa + sb, @u32)
    sd = bxor(sd, sa)
    sd = rotl32(sd, 8)
    # Step 4
    sc = band(sc + sd, @u32)
    sb = bxor(sb, sc)
    sb = rotl32(sb, 7)

    state
    |> put_elem(pa, sa)
    |> put_elem(pb, sb)
    |> put_elem(pc, sc)
    |> put_elem(pd, sd)
  end

  # Left-rotate a 32-bit word by `n` bits.
  #
  # In a stream cipher, rotation provides diffusion across bit positions.
  # The four rotation amounts (16, 12, 8, 7) were chosen to maximise the
  # algebraic independence of the output bits.
  defp rotl32(x, n), do: band(bor(x <<< n, x >>> (32 - n)), @u32)

  # Recursively encrypt chunks of plaintext, one 64-byte keystream block at a time.
  defp do_chacha20_encrypt(<<>>, _key, _nonce, _ctr, acc), do: acc

  defp do_chacha20_encrypt(plaintext, key, nonce, ctr, acc) do
    keystream = chacha20_block(key, ctr, nonce)
    chunk_size = min(byte_size(plaintext), 64)
    <<chunk::binary-size(chunk_size), rest::binary>> = plaintext
    <<ks::binary-size(chunk_size), _::binary>> = keystream
    xored = :crypto.exor(chunk, ks)
    do_chacha20_encrypt(rest, key, nonce, ctr + 1, acc <> xored)
  end

  # Poly1305 accumulation loop.
  #
  # For each 16-byte (or shorter final) chunk, interpret the bytes as a
  # little-endian integer, append a 1-bit at position 8*chunk_size (this
  # distinguishes chunks shorter than 16 bytes from zero-padding), then:
  #
  #   acc = (acc + n) * r  mod  (2^130 - 5)
  #
  # Elixir's native bignum arithmetic handles the 130-bit modulus cleanly.
  defp poly1305_accumulate(<<>>, _r, acc), do: acc

  defp poly1305_accumulate(message, r, acc) do
    chunk_size = min(byte_size(message), 16)
    <<chunk::binary-size(chunk_size), rest::binary>> = message
    # Decode the chunk as a little-endian integer, then set the high bit
    # at position (8 * chunk_size) to mark the end of the block.
    n = :binary.decode_unsigned(chunk, :little) ||| (1 <<< (8 * chunk_size))
    new_acc = rem((acc + n) * r, @poly_prime)
    poly1305_accumulate(rest, r, new_acc)
  end

  # Pad a binary to a multiple of 16 bytes by appending zero bytes.
  # Used when constructing the Poly1305 MAC input per RFC 8439 §2.8.
  defp pad16(data) do
    remainder = rem(byte_size(data), 16)

    case remainder do
      0 -> data
      r -> data <> :binary.copy(<<0>>, 16 - r)
    end
  end

  # Pad (or truncate) a binary to exactly `target` bytes.
  # Used to ensure the Poly1305 tag is always exactly 16 bytes even if
  # :binary.encode_unsigned produces fewer (leading-zero suppression).
  defp pad_to(bin, target) when byte_size(bin) >= target,
    do: binary_part(bin, 0, target)

  defp pad_to(bin, target),
    do: bin <> :binary.copy(<<0>>, target - byte_size(bin))

  # Constant-time binary comparison.
  #
  # A naive `expected_tag == provided_tag` comparison can leak timing
  # information: it returns false as soon as a differing byte is found.
  # An attacker making many queries can use these timing differences to
  # learn how many bytes of a forged tag matched — a "padding oracle"-style
  # attack on the MAC.
  #
  # This implementation:
  #   1. XORs every byte pair (accumulating into a single integer with |||).
  #   2. If all bytes are equal, every XOR is 0 and the accumulator stays 0.
  #   3. The loop always runs all N iterations regardless of the byte values.
  #
  # The `Enum.zip` + `Enum.reduce` pattern ensures the same number of
  # operations regardless of where the first differing byte appears.
  defp constant_time_eq(a, b) when byte_size(a) != byte_size(b), do: false

  defp constant_time_eq(a, b) do
    :binary.bin_to_list(a)
    |> Enum.zip(:binary.bin_to_list(b))
    |> Enum.reduce(0, fn {x, y}, acc -> acc ||| bxor(x, y) end)
    |> Kernel.==(0)
  end
end
