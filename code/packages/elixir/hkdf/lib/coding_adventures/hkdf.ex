defmodule CodingAdventures.Hkdf do
  @moduledoc """
  HKDF (HMAC-based Extract-and-Expand Key Derivation Function) — RFC 5869.

  ## What Is HKDF?

  HKDF is a simple, well-analyzed key derivation function built on top of HMAC.
  It was designed by Hugo Krawczyk and published as RFC 5869 in 2010.

  HKDF is used in:
  - TLS 1.3 (the primary key derivation mechanism)
  - Signal Protocol (Double Ratchet key derivation)
  - WireGuard VPN (handshake key expansion)
  - Noise Protocol Framework
  - IKEv2 (Internet Key Exchange)

  ## Why Do We Need a KDF?

  Raw cryptographic keys often come from sources with uneven entropy:
  - Diffie-Hellman shared secrets have algebraic structure (not uniform)
  - Passwords have low entropy concentrated in certain bits
  - Hardware RNGs may have bias in certain bit positions

  A KDF "extracts" the entropy from such sources into a uniformly random
  pseudorandom key (PRK), then "expands" that PRK into as many output
  bytes as needed — each cryptographically independent.

  ## The Two-Stage Design

  HKDF separates key derivation into two logically distinct stages:

  ### Stage 1 — Extract

  ```
  PRK = HMAC-Hash(salt, IKM)
  ```

  The salt is used as the HMAC key and IKM (Input Keying Material) as the
  message. This is intentional — the salt acts as a randomness extractor.
  If no salt is provided, HashLen zero bytes are used (per RFC 5869).

  ### Stage 2 — Expand

  ```
  T(0) = ""  (empty)
  T(1) = HMAC-Hash(PRK, T(0) || info || 0x01)
  T(2) = HMAC-Hash(PRK, T(1) || info || 0x02)
  ...
  T(N) = HMAC-Hash(PRK, T(N-1) || info || 0x0N)
  OKM = first L bytes of T(1) || T(2) || ... || T(N)
  ```

  The `info` parameter provides domain separation — different `info` values
  produce independent output keys from the same PRK. The counter byte is a
  single octet (1-indexed, max 255), limiting maximum output to 255 * HashLen.

  ## Supported Hash Functions

  | Algorithm | HashLen | Block Size |
  |-----------|---------|------------|
  | SHA-256   | 32      | 64         |
  | SHA-512   | 64      | 128        |

  ## Public API

  ```elixir
  # Combined extract + expand
  hkdf(salt, ikm, info, length, hash \\\\ :sha256)  # -> binary

  # Separate stages
  extract(salt, ikm, hash \\\\ :sha256)               # -> binary (PRK)
  expand(prk, info, length, hash \\\\ :sha256)         # -> binary (OKM)
  ```
  """

  alias CodingAdventures.Hmac

  # ---------------------------------------------------------------------------
  # Hash configuration
  # ---------------------------------------------------------------------------
  #
  # Each hash algorithm needs:
  #   - An HMAC function reference (key, message) -> binary
  #   - The hash output length (HashLen) in bytes
  #
  # We use atoms as keys (:sha256, :sha512) for Elixir-idiomatic pattern matching.

  @hash_configs %{
    sha256: %{hmac_fn: &Hmac.hmac_sha256/2, hash_len: 32},
    sha512: %{hmac_fn: &Hmac.hmac_sha512/2, hash_len: 64}
  }

  # ---------------------------------------------------------------------------
  # HKDF-Extract (RFC 5869 Section 2.2)
  # ---------------------------------------------------------------------------

  @doc """
  Extract a pseudorandom key (PRK) from input keying material.

  HKDF-Extract condenses potentially non-uniform input keying material
  into a fixed-length, uniformly distributed pseudorandom key.

  The extraction uses HMAC with the salt as the key and IKM as the message:

      PRK = HMAC-Hash(salt, IKM)

  If `salt` is empty (`""`), HashLen zero bytes are used as specified by
  RFC 5869 Section 2.2.

  ## Parameters
  - `salt` — binary salt value (HMAC key); empty binary for default
  - `ikm` — input keying material (the raw secret)
  - `hash_name` — `:sha256` or `:sha512` (default: `:sha256`)

  ## Returns
  PRK as a binary of HashLen bytes.

  ## Examples

      iex> ikm = :binary.copy(<<0x0b>>, 22)
      iex> salt = Base.decode16!("000102030405060708090a0b0c", case: :lower)
      iex> CodingAdventures.Hkdf.extract(salt, ikm) |> Base.encode16(case: :lower)
      "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"

  """
  @spec extract(binary, binary, atom) :: binary
  def extract(salt, ikm, hash_name \\ :sha256)
      when is_binary(salt) and is_binary(ikm) and is_atom(hash_name) do
    config = get_config!(hash_name)

    # RFC 5869 Section 2.2: "if not provided, [salt] is set to a string
    # of HashLen zeros." An empty salt is treated as "not provided."
    effective_salt =
      if byte_size(salt) == 0 do
        :binary.copy(<<0x00>>, config.hash_len)
      else
        salt
      end

    # HMAC returns a binary in the Elixir implementation.
    config.hmac_fn.(effective_salt, ikm)
  end

  # ---------------------------------------------------------------------------
  # HKDF-Expand (RFC 5869 Section 2.3)
  # ---------------------------------------------------------------------------

  @doc """
  Expand a pseudorandom key into output keying material of desired length.

  HKDF-Expand generates arbitrary-length output from a fixed-length PRK
  using an iterative HMAC construction:

      T(0) = ""
      T(i) = HMAC-Hash(PRK, T(i-1) || info || byte(i))
      OKM  = first L bytes of T(1) || T(2) || ... || T(N)

  Each T(i) feeds back into the next iteration, creating a chain. The `info`
  parameter provides domain separation. The counter is a single byte (0x01
  through 0xFF), limiting maximum output to 255 * HashLen bytes.

  ## Parameters
  - `prk` — pseudorandom key from extract (binary, >= HashLen bytes)
  - `info` — context/application-specific info (binary, can be empty)
  - `output_length` — desired output length in bytes (1..255*HashLen)
  - `hash_name` — `:sha256` or `:sha512` (default: `:sha256`)

  ## Returns
  OKM as a binary of exactly `output_length` bytes.

  ## Examples

      iex> prk = Base.decode16!("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5", case: :lower)
      iex> info = Base.decode16!("f0f1f2f3f4f5f6f7f8f9", case: :lower)
      iex> CodingAdventures.Hkdf.expand(prk, info, 42) |> Base.encode16(case: :lower)
      "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"

  """
  @spec expand(binary, binary, pos_integer, atom) :: binary
  def expand(prk, info, output_length, hash_name \\ :sha256)
      when is_binary(prk) and is_binary(info) and is_integer(output_length) and
             is_atom(hash_name) do
    config = get_config!(hash_name)

    # Validate output length.
    max_length = 255 * config.hash_len

    if output_length <= 0 do
      raise ArgumentError, "HKDF expand length must be > 0"
    end

    if output_length > max_length do
      raise ArgumentError,
            "HKDF expand length #{output_length} exceeds maximum #{max_length} (255 * #{config.hash_len})"
    end

    # Number of HMAC iterations: ceil(L / HashLen)
    n = div(output_length + config.hash_len - 1, config.hash_len)

    # Iterative expansion using Enum.reduce.
    # We accumulate {previous_t_block, list_of_t_blocks_reversed}.
    # T(0) is empty. Each T(i) = HMAC(PRK, T(i-1) || info || byte(i)).
    {_last_t, okm_parts} =
      Enum.reduce(1..n, {<<>>, []}, fn counter, {t_prev, acc} ->
        # Build message: T(i-1) || info || counter_byte
        msg = t_prev <> info <> <<counter>>
        t_current = config.hmac_fn.(prk, msg)
        {t_current, [t_current | acc]}
      end)

    # Reverse the accumulated blocks (they're in reverse order from reduce),
    # concatenate, and truncate to the exact requested length.
    okm_parts
    |> Enum.reverse()
    |> IO.iodata_to_binary()
    |> binary_part(0, output_length)
  end

  # ---------------------------------------------------------------------------
  # Combined HKDF (RFC 5869 Section 2.1)
  # ---------------------------------------------------------------------------

  @doc """
  Derive output keying material from input keying material in one step.

  Combines HKDF-Extract and HKDF-Expand:

      OKM = HKDF-Expand(HKDF-Extract(salt, IKM), info, L)

  ## Parameters
  - `salt` — binary salt for extraction (empty = HashLen zeros)
  - `ikm` — input keying material
  - `info` — context info for expansion (can be empty)
  - `output_length` — desired output length (1..255*HashLen)
  - `hash_name` — `:sha256` or `:sha512` (default: `:sha256`)

  ## Returns
  OKM as a binary of exactly `output_length` bytes.

  ## Examples

      iex> ikm = :binary.copy(<<0x0b>>, 22)
      iex> salt = Base.decode16!("000102030405060708090a0b0c", case: :lower)
      iex> info = Base.decode16!("f0f1f2f3f4f5f6f7f8f9", case: :lower)
      iex> CodingAdventures.Hkdf.hkdf(salt, ikm, info, 42) |> Base.encode16(case: :lower)
      "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"

  """
  @spec hkdf(binary, binary, binary, pos_integer, atom) :: binary
  def hkdf(salt, ikm, info, output_length, hash_name \\ :sha256)
      when is_binary(salt) and is_binary(ikm) and is_binary(info) and
             is_integer(output_length) and is_atom(hash_name) do
    prk = extract(salt, ikm, hash_name)
    expand(prk, info, output_length, hash_name)
  end

  # ---------------------------------------------------------------------------
  # Hex convenience functions
  # ---------------------------------------------------------------------------

  @doc "HKDF-Extract returning a lowercase hex string."
  @spec extract_hex(binary, binary, atom) :: String.t()
  def extract_hex(salt, ikm, hash_name \\ :sha256) do
    extract(salt, ikm, hash_name) |> Base.encode16(case: :lower)
  end

  @doc "HKDF-Expand returning a lowercase hex string."
  @spec expand_hex(binary, binary, pos_integer, atom) :: String.t()
  def expand_hex(prk, info, output_length, hash_name \\ :sha256) do
    expand(prk, info, output_length, hash_name) |> Base.encode16(case: :lower)
  end

  @doc "Combined HKDF returning a lowercase hex string."
  @spec hkdf_hex(binary, binary, binary, pos_integer, atom) :: String.t()
  def hkdf_hex(salt, ikm, info, output_length, hash_name \\ :sha256) do
    hkdf(salt, ikm, info, output_length, hash_name) |> Base.encode16(case: :lower)
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Look up hash configuration, raising for unsupported algorithms.
  defp get_config!(hash_name) do
    case Map.fetch(@hash_configs, hash_name) do
      {:ok, config} -> config
      :error -> raise ArgumentError, "unsupported hash algorithm: #{inspect(hash_name)}"
    end
  end
end
