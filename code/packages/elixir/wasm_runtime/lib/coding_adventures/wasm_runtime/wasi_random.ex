defmodule CodingAdventures.WasmRuntime.WasiRandom do
  @moduledoc """
  Behaviour for WASI random byte generation.

  WASI provides `random_get(buf_ptr, buf_len)` to fill a buffer with
  cryptographically secure random bytes. This is equivalent to reading
  from `/dev/urandom` on Linux or using `CryptGenRandom` on Windows.

  ## Why a Behaviour?

  Randomness is fundamentally non-deterministic, which makes tests that
  depend on random values fragile or impossible to verify. By abstracting
  randomness behind a behaviour:

  - **Tests** inject a `FakeRandom` that always returns known bytes
    (e.g., all `0xAB`), making tests fully deterministic.
  - **Production** uses `SystemRandom` backed by `:crypto.strong_rand_bytes/1`,
    Erlang's CSPRNG (Cryptographically Secure Pseudo-Random Number Generator).

  ## Security Note

  `:crypto.strong_rand_bytes/1` is backed by OpenSSL's RAND_bytes, which
  seeds from OS entropy sources (`/dev/urandom`, `getrandom(2)`, etc.).
  It is safe for cryptographic use.
  """

  @doc """
  Generate `n` cryptographically random bytes and return them as a binary.

  The caller writes these bytes into WASM linear memory at the address
  the WASM module requested.
  """
  @callback fill_bytes(n :: integer()) :: binary()
end

defmodule CodingAdventures.WasmRuntime.SystemRandom do
  @moduledoc """
  Production random implementation backed by Erlang's `:crypto` module.

  `:crypto.strong_rand_bytes(n)` returns an `n`-byte binary filled with
  cryptographically secure random data. Under the hood it calls OpenSSL's
  RAND_bytes, which is seeded from OS entropy.

  ## Example

      SystemRandom.fill_bytes(4)
      # => <<219, 14, 58, 103>>  (different each call)
  """

  @behaviour CodingAdventures.WasmRuntime.WasiRandom

  @impl true
  def fill_bytes(n), do: :crypto.strong_rand_bytes(n)
end
