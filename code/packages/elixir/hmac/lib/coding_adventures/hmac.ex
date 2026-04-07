defmodule CodingAdventures.Hmac do
  @moduledoc """
  HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1.

  ## What Is HMAC?

  HMAC takes a secret key and a message and produces a fixed-size authentication
  tag. It is used to verify BOTH the integrity (the message hasn't changed) AND
  the authenticity (the sender knows the secret key) of a message.

  HMAC is at the heart of:
  - TLS 1.2 handshakes (HMAC-SHA256 for PRF)
  - JWT (JSON Web Token) signatures (`HS256`, `HS512`)
  - WPA2 Wi-Fi authentication (HMAC-SHA1 in PBKDF2)
  - TOTP / HOTP one-time passwords (RFC 6238, RFC 4226)
  - SSH MAC validation
  - S3 / AWS Signature Version 4 request signing

  ## Why Not hash(key || message)?

  A naive approach would be `hash(key || message)`. This is vulnerable to a
  **length extension attack**: anyone who knows the hash of a message can compute
  the hash of a longer message without knowing the key.

  Example with SHA-256 (a Merkle-Damgård construction):
  ```
  known: SHA-256(key || "GET /admin")
  attacker can compute: SHA-256(key || "GET /admin" || padding || "/delete")
  without knowing `key`
  ```

  This works because Merkle-Damgård hashes resume from the state left by the
  previous block — if you know the state after hashing `key || message`, you can
  just add more blocks.

  ## The HMAC Construction (RFC 2104)

  HMAC defeats length extension by using two nested hash calls with different keys:

  ```
  HMAC(K, M) = H((K' XOR opad) || H((K' XOR ipad) || M))

  where:
    K'    = key padded (or hashed) to the hash's block size
    ipad  = 0x36 repeated to block size  ("inner pad")
    opad  = 0x5C repeated to block size  ("outer pad")
    H     = the underlying hash function (MD5, SHA-1, SHA-256, SHA-512)
    ||    = concatenation
  ```

  **Why two nested calls work:**

  The inner call `H(ipad_key || M)` produces a hash that commits to the message
  under a modified key. The outer call `H(opad_key || inner)` wraps that result
  under a DIFFERENT modified key. An attacker cannot extend the message because
  they would need to break into the outer hash, which they cannot do without
  knowing `opad_key`.

  **The constants 0x36 and 0x5C:**

  These values were chosen by Hugo Krawczyk (the main HMAC designer) because
  they are Hamming-distance-maximized hex values: 0x36 = 0011_0110 and
  0x5C = 0101_1100. They differ in 4 of 8 bits, ensuring the inner and outer
  keys are as different as possible even though both are derived from K'.

  ## Step-by-Step Algorithm

  ```
  Input: key K (any length), message M, hash function H, block size B

  Step 1 — Normalize the key to exactly B bytes:
    If len(K) > B:  K' = H(K)    (hash long keys down to digest size)
    If len(K) < B:  K' = K || 0x00 * (B - len(K))  (zero-pad short keys)
    If len(K) == B: K' = K

  Step 2 — Create padded keys:
    inner_key = K' XOR (0x36 * B)   (every byte of K' XOR'd with 0x36)
    outer_key = K' XOR (0x5C * B)   (every byte of K' XOR'd with 0x5C)

  Step 3 — Compute the nested hash:
    inner = H(inner_key || M)
    result = H(outer_key || inner)

  Output: result (same length as H's digest)
  ```

  ## Block Sizes and Digest Sizes

  | Algorithm | Block Size | Digest Size | RFC Reference |
  |-----------|-----------|-------------|---------------|
  | MD5       | 64 bytes  | 16 bytes    | RFC 1321      |
  | SHA-1     | 64 bytes  | 20 bytes    | FIPS 180-4    |
  | SHA-256   | 64 bytes  | 32 bytes    | FIPS 180-4    |
  | SHA-512   | 128 bytes | 64 bytes    | FIPS 180-4    |

  SHA-512 uses a 128-byte block because it processes 64-bit words (not 32-bit),
  so it needs twice as many bytes per block to hold the same number of words.
  This means the ipad and opad strings are 128 bytes long, not 64.

  ## Public API

  ```elixir
  # Generic HMAC — bring your own hash function
  hmac(hash_fn, block_size, key, message)  # -> binary (digest bytes)

  # Named variants
  hmac_md5(key, message)     # -> 16-byte binary
  hmac_sha1(key, message)    # -> 20-byte binary
  hmac_sha256(key, message)  # -> 32-byte binary
  hmac_sha512(key, message)  # -> 64-byte binary

  # Hex-string variants
  hmac_md5_hex(key, message)    # -> 32-char hex string
  hmac_sha1_hex(key, message)   # -> 40-char hex string
  hmac_sha256_hex(key, message) # -> 64-char hex string
  hmac_sha512_hex(key, message) # -> 128-char hex string
  ```

  ## RFC 4231 Test Vector (TC1, HMAC-SHA256)

  ```
  Key:  <<0x0b, 0x0b, ..., 0x0b>>  # 20 bytes of 0x0b
  Data: "Hi There"
  HMAC-SHA256: b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7
  ```
  """

  alias CodingAdventures.Md5
  alias CodingAdventures.Sha1
  alias CodingAdventures.Sha256
  alias CodingAdventures.Sha512

  # ---------------------------------------------------------------------------
  # Generic HMAC
  # ---------------------------------------------------------------------------
  #
  # This is the heart of the module. All named variants delegate here.
  #
  # Arguments:
  #   hash_fn    — a 1-arity function that takes a binary and returns a binary
  #   block_size — the hash function's block size in bytes (64 or 128)
  #   key        — the secret key (any length)
  #   message    — the data to authenticate (any length)
  #
  # Returns: the HMAC digest as a binary

  @doc """
  Compute HMAC using any hash function.

  ## Parameters
  - `hash_fn` — a function `data :: binary -> binary` (e.g. `&Sha256.sha256/1`)
  - `block_size` — block size of the hash function in bytes (64 for MD5/SHA-1/SHA-256, 128 for SHA-512)
  - `key` — secret key, any length
  - `message` — data to authenticate, any length

  ## Examples

      iex> import CodingAdventures.Hmac
      iex> hmac(&CodingAdventures.Sha256.sha256/1, 64, "key", "message") |> Base.encode16(case: :lower)
      "6e9ef29b75fffc5b7abae527d58fdadb2fe42e7219011976917343065f58ed4a"

  """
  @spec hmac((binary -> binary), pos_integer, binary, binary) :: binary
  def hmac(hash_fn, block_size, key, message)
      when is_function(hash_fn, 1) and is_integer(block_size) and block_size > 0 and
             is_binary(key) and is_binary(message) do
    # Step 1 — normalize key to exactly block_size bytes
    normalized_key = normalize_key(hash_fn, block_size, key)

    # Step 2 — derive inner and outer padded keys
    #   ipad = 0x36 repeated; opad = 0x5C repeated
    #   XOR each byte of the normalized key with ipad/opad byte
    inner_key = xor_bytes(normalized_key, 0x36)
    outer_key = xor_bytes(normalized_key, 0x5C)

    # Step 3 — nested hash calls
    #   inner hash: H(inner_key || message)
    #   outer hash: H(outer_key || inner_hash)
    inner_hash = hash_fn.(inner_key <> message)
    hash_fn.(outer_key <> inner_hash)
  end

  # ---------------------------------------------------------------------------
  # Named HMAC Variants
  # ---------------------------------------------------------------------------
  #
  # Each variant hard-codes the hash function and block size so callers don't
  # have to remember which block size belongs to which hash.

  @doc """
  HMAC-MD5: 16-byte (128-bit) authentication tag.

  MD5 is cryptographically broken for collision resistance but HMAC-MD5 is
  still considered secure as a MAC (because MAC security doesn't require
  collision resistance of the underlying hash). It appears in legacy protocols.

  ## Examples

      iex> CodingAdventures.Hmac.hmac_md5("Jefe", "what do ya want for nothing?") |> Base.encode16(case: :lower)
      "750c783e6ab0b503eaa86e310a5db738"

  """
  @spec hmac_md5(binary, binary) :: binary
  def hmac_md5(key, message) when is_binary(key) and is_binary(message) do
    if byte_size(key) == 0, do: raise(ArgumentError, "HMAC key must not be empty")
    hmac(&Md5.md5/1, 64, key, message)
  end

  @doc """
  HMAC-SHA1: 20-byte (160-bit) authentication tag.

  Used in WPA2 (PBKDF2-HMAC-SHA1), older TLS versions, and SSH.

  ## Examples

      iex> CodingAdventures.Hmac.hmac_sha1("Jefe", "what do ya want for nothing?") |> Base.encode16(case: :lower)
      "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"

  """
  @spec hmac_sha1(binary, binary) :: binary
  def hmac_sha1(key, message) when is_binary(key) and is_binary(message) do
    if byte_size(key) == 0, do: raise(ArgumentError, "HMAC key must not be empty")
    hmac(&Sha1.sha1/1, 64, key, message)
  end

  @doc """
  HMAC-SHA256: 32-byte (256-bit) authentication tag.

  The modern standard. Used in TLS 1.3, JWT HS256, AWS Signature V4,
  PBKDF2-HMAC-SHA256.

  ## Examples

      iex> key = :binary.copy(<<0x0b>>, 20)
      iex> CodingAdventures.Hmac.hmac_sha256(key, "Hi There") |> Base.encode16(case: :lower)
      "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"

  """
  @spec hmac_sha256(binary, binary) :: binary
  def hmac_sha256(key, message) when is_binary(key) and is_binary(message) do
    if byte_size(key) == 0, do: raise(ArgumentError, "HMAC key must not be empty")
    hmac(&Sha256.sha256/1, 64, key, message)
  end

  @doc """
  HMAC-SHA512: 64-byte (512-bit) authentication tag.

  Used in JWT HS512, TLS PRF for high-security configurations, and PBKDF2
  with SHA-512 for password hashing.

  SHA-512's 128-byte block means the key normalization and padding use 128
  bytes of ipad/opad instead of the 64 bytes used by MD5/SHA-1/SHA-256.

  ## Examples

      iex> key = :binary.copy(<<0x0b>>, 20)
      iex> CodingAdventures.Hmac.hmac_sha512(key, "Hi There") |> Base.encode16(case: :lower)
      "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"

  """
  @spec hmac_sha512(binary, binary) :: binary
  def hmac_sha512(key, message) when is_binary(key) and is_binary(message) do
    if byte_size(key) == 0, do: raise(ArgumentError, "HMAC key must not be empty")
    hmac(&Sha512.sha512/1, 128, key, message)
  end

  # ---------------------------------------------------------------------------
  # Hex-string variants
  # ---------------------------------------------------------------------------

  @doc "HMAC-MD5 returning a lowercase hex string."
  @spec hmac_md5_hex(binary, binary) :: String.t()
  def hmac_md5_hex(key, message), do: hmac_md5(key, message) |> Base.encode16(case: :lower)

  @doc "HMAC-SHA1 returning a lowercase hex string."
  @spec hmac_sha1_hex(binary, binary) :: String.t()
  def hmac_sha1_hex(key, message), do: hmac_sha1(key, message) |> Base.encode16(case: :lower)

  @doc "HMAC-SHA256 returning a lowercase hex string."
  @spec hmac_sha256_hex(binary, binary) :: String.t()
  def hmac_sha256_hex(key, message), do: hmac_sha256(key, message) |> Base.encode16(case: :lower)

  @doc "HMAC-SHA512 returning a lowercase hex string."
  @spec hmac_sha512_hex(binary, binary) :: String.t()
  def hmac_sha512_hex(key, message), do: hmac_sha512(key, message) |> Base.encode16(case: :lower)

  # ---------------------------------------------------------------------------
  # Constant-time tag verification
  # ---------------------------------------------------------------------------

  @doc """
  Compare two HMAC tags in constant time.

  Use this instead of `==` when checking whether a received tag matches an
  expected tag. The `==` operator short-circuits on the first differing byte,
  leaking information about *how many bytes* match through timing. Over many
  requests an attacker can use these timing differences to reconstruct the
  expected tag byte by byte — a **timing attack**.

  `secure_compare/2` takes the same time regardless of where (or whether)
  the two binaries differ. It delegates to `:crypto.hash_equals/2`, which is
  an OpenSSL constant-time comparison available in Erlang/OTP 25+.

  Returns `true` iff `a` and `b` are byte-for-byte identical.

  ## Examples

      iex> tag = CodingAdventures.Hmac.hmac_sha256("secret", "message")
      iex> CodingAdventures.Hmac.secure_compare(tag, tag)
      true
      iex> CodingAdventures.Hmac.secure_compare(tag, "wrong")
      false

  """
  @spec secure_compare(binary, binary) :: boolean
  def secure_compare(a, b) when is_binary(a) and is_binary(b) and byte_size(a) == byte_size(b) do
    :crypto.hash_equals(a, b)
  end

  def secure_compare(_a, _b), do: false

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # normalize_key/3 — bring any key to exactly block_size bytes.
  #
  # Three cases from RFC 2104 Section 2:
  #   len(key) > block_size → hash the key, then zero-pad the hash to block_size
  #   len(key) < block_size → zero-pad the key on the right
  #   len(key) == block_size → use the key as-is
  #
  # Note: hashing a long key may produce a digest shorter than block_size
  # (e.g. SHA-256 produces 32 bytes; for block_size=64 we then zero-pad).
  defp normalize_key(hash_fn, block_size, key) do
    key_len = byte_size(key)

    normalized =
      if key_len > block_size do
        hash_fn.(key)
      else
        key
      end

    # Zero-pad to exactly block_size
    pad_size = block_size - byte_size(normalized)

    if pad_size > 0 do
      normalized <> :binary.copy(<<0x00>>, pad_size)
    else
      normalized
    end
  end

  # xor_bytes/2 — XOR every byte in a binary with a constant value.
  #
  # This implements the ipad/opad key modification from RFC 2104.
  # Input: a binary of exactly block_size bytes, a single byte value (0x36 or 0x5C).
  # Output: a binary of the same length with every byte XOR'd by the constant.
  #
  # Example: xor_bytes(<<0xAB, 0xCD>>, 0x36) = <<0xAB ^^^ 0x36, 0xCD ^^^ 0x36>>
  #                                           = <<0x9D, 0xFB>>
  defp xor_bytes(binary, constant) do
    for <<byte <- binary>>, into: <<>> do
      <<Bitwise.bxor(byte, constant)>>
    end
  end
end
