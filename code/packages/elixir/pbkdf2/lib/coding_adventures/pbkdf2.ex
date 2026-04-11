defmodule CodingAdventures.Pbkdf2 do
  @moduledoc """
  PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018.

  ## What Is PBKDF2?

  PBKDF2 derives a cryptographic key from a password by applying a pseudorandom
  function (PRF) — typically HMAC — `c` times per output block. The iteration
  count `c` is the tunable cost: every brute-force guess requires the same `c`
  PRF calls as the original derivation.

  Real-world uses:

  * WPA2 Wi-Fi — PBKDF2-HMAC-SHA1, 4096 iterations
  * Django — PBKDF2-HMAC-SHA256, 720,000 iterations (2024)
  * macOS Keychain — PBKDF2-HMAC-SHA256

  ## Algorithm (RFC 8018 § 5.2)

  ```
  DK = T_1 <> T_2 <> ... (first dk_len bytes)

  T_i = U_1 XOR U_2 XOR ... XOR U_c

  U_1 = PRF(password, salt <> INT_32_BE(i))
  U_j = PRF(password, U_{j-1})   for j = 2..c
  ```

  `INT_32_BE(i)` encodes the block counter as a 4-byte big-endian integer.
  This makes each block's first U value unique even when the salt repeats.

  ## Security Notes

  OWASP 2023 minimum iteration counts:
  * HMAC-SHA256: 600,000
  * HMAC-SHA1:   1,300,000

  For new systems prefer Argon2id (memory-hard, resists GPU attacks).

  ## Example

      iex> dk = CodingAdventures.Pbkdf2.pbkdf2_hmac_sha1("password", "salt", 1, 20)
      iex> Base.encode16(dk, case: :lower)
      "0c60c80f961f0e71f3a9b524af6012062fe037a6"
  """

  alias CodingAdventures.Hmac

  # ──────────────────────────────────────────────────────────────────────────────
  # Core loop
  # ──────────────────────────────────────────────────────────────────────────────

  @doc false
  defp pbkdf2_core(prf, h_len, password, salt, iterations, key_length) do
    if byte_size(password) == 0,
      do: raise(ArgumentError, "PBKDF2 password must not be empty")

    if not is_integer(iterations) or iterations <= 0,
      do: raise(ArgumentError, "PBKDF2 iterations must be positive")

    if not is_integer(key_length) or key_length <= 0,
      do: raise(ArgumentError, "PBKDF2 key_length must be positive")

    num_blocks = ceil(key_length / h_len)

    dk =
      for i <- 1..num_blocks do
        # Seed = salt <> INT_32_BE(i)
        # <<i::big-unsigned-integer-size(32)>> is Elixir binary syntax for
        # encoding the integer i as a 4-byte big-endian unsigned integer.
        seed = salt <> <<i::big-unsigned-integer-size(32)>>

        # U_1 = PRF(password, seed)
        u1 = prf.(password, seed)

        # Accumulate XOR of all U values: start with U_1, then fold in U_2..U_c.
        # We handle iterations == 1 explicitly to avoid the empty range 2..1//1.
        {t, _last_u} =
          if iterations == 1 do
            {u1, u1}
          else
            Enum.reduce(2..iterations//1, {u1, u1}, fn _j, {acc, prev_u} ->
              next_u = prf.(password, prev_u)
              new_acc = :crypto.exor(acc, next_u)
              {new_acc, next_u}
            end)
          end

        t
      end
      |> IO.iodata_to_binary()

    binary_part(dk, 0, key_length)
  end

  # ──────────────────────────────────────────────────────────────────────────────
  # Public API — concrete PRF variants
  # ──────────────────────────────────────────────────────────────────────────────

  @doc """
  PBKDF2 with HMAC-SHA1 as the PRF.

  `h_len` = 20 bytes (160-bit SHA-1 output).
  Used in WPA2 (4096 iterations). For new systems prefer `pbkdf2_hmac_sha256/4`.

  ## RFC 6070 test vector

      iex> dk = pbkdf2_hmac_sha1("password", "salt", 1, 20)
      iex> Base.encode16(dk, case: :lower)
      "0c60c80f961f0e71f3a9b524af6012062fe037a6"
  """
  def pbkdf2_hmac_sha1(password, salt, iterations, key_length) do
    prf = fn key, msg -> Hmac.hmac_sha1(key, msg) end
    pbkdf2_core(prf, 20, password, salt, iterations, key_length)
  end

  @doc """
  PBKDF2 with HMAC-SHA256 as the PRF.

  `h_len` = 32 bytes (256-bit SHA-256 output).
  Recommended for new systems. OWASP 2023: ≥ 600,000 iterations.
  """
  def pbkdf2_hmac_sha256(password, salt, iterations, key_length) do
    prf = fn key, msg -> Hmac.hmac_sha256(key, msg) end
    pbkdf2_core(prf, 32, password, salt, iterations, key_length)
  end

  @doc """
  PBKDF2 with HMAC-SHA512 as the PRF.

  `h_len` = 64 bytes (512-bit SHA-512 output).
  Suitable for high-security applications.
  """
  def pbkdf2_hmac_sha512(password, salt, iterations, key_length) do
    prf = fn key, msg -> Hmac.hmac_sha512(key, msg) end
    pbkdf2_core(prf, 64, password, salt, iterations, key_length)
  end

  # ──────────────────────────────────────────────────────────────────────────────
  # Hex variants
  # ──────────────────────────────────────────────────────────────────────────────

  @doc "Like `pbkdf2_hmac_sha1/4` but returns a lowercase hex string."
  def pbkdf2_hmac_sha1_hex(password, salt, iterations, key_length) do
    pbkdf2_hmac_sha1(password, salt, iterations, key_length)
    |> Base.encode16(case: :lower)
  end

  @doc "Like `pbkdf2_hmac_sha256/4` but returns a lowercase hex string."
  def pbkdf2_hmac_sha256_hex(password, salt, iterations, key_length) do
    pbkdf2_hmac_sha256(password, salt, iterations, key_length)
    |> Base.encode16(case: :lower)
  end

  @doc "Like `pbkdf2_hmac_sha512/4` but returns a lowercase hex string."
  def pbkdf2_hmac_sha512_hex(password, salt, iterations, key_length) do
    pbkdf2_hmac_sha512(password, salt, iterations, key_length)
    |> Base.encode16(case: :lower)
  end
end
