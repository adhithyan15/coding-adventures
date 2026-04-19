defmodule CodingAdventures.Blake2b do
  @moduledoc """
  BLAKE2b cryptographic hash function (RFC 7693), from scratch.

  BLAKE2b is a modern hash that is:

    * Faster than MD5 on 64-bit hardware.
    * As secure as SHA-3 against known attacks.
    * Variable output length (1..64 bytes).
    * Keyed in a single pass (replaces HMAC-SHA-512).
    * Parameterized with salt and personalization (16 bytes each).

  It underlies libsodium, WireGuard, Noise Protocol, IPFS content addressing,
  and -- within this repo -- Argon2.

  Elixir integers are arbitrary precision, so every 64-bit add and XOR masks
  with `0xFFFFFFFFFFFFFFFF` to stay inside a single machine word.  This is
  analogous to the Ruby and Python ports; Go, Rust, and Swift use native
  wrapping `u64` arithmetic instead.

  Sequential mode only.  Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
  BLAKE2Xb, and BLAKE3 are out of scope.

  > Educational implementation. Use a vetted library for real crypto.
  """

  import Bitwise

  @mask64 0xFFFFFFFFFFFFFFFF
  @block_size 128
  @max_digest 64
  @max_key 64

  # Initial Hash Values -- identical to SHA-512 (fractional parts of the
  # square roots of the first eight primes).
  @iv {
    0x6A09E667F3BCC908,
    0xBB67AE8584CAA73B,
    0x3C6EF372FE94F82B,
    0xA54FF53A5F1D36F1,
    0x510E527FADE682D1,
    0x9B05688C2B3E6C1F,
    0x1F83D9ABFB41BD6B,
    0x5BE0CD19137E2179
  }

  # Ten message-schedule permutations.  Round i uses SIGMA[i mod 10];
  # rounds 10 and 11 reuse rows 0 and 1.
  @sigma {
    {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15},
    {14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3},
    {11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4},
    {7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8},
    {9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13},
    {2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9},
    {12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11},
    {13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10},
    {6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5},
    {10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0}
  }

  # ------------------------------------------------------------------
  # Opaque hasher state
  # ------------------------------------------------------------------

  defmodule Hasher do
    @moduledoc false
    defstruct [:state, :buffer, :byte_count, :digest_size]
  end

  # ------------------------------------------------------------------
  # Public API
  # ------------------------------------------------------------------

  @doc """
  Create a new streaming hasher.

  Options:

    * `:digest_size` -- integer in `1..64` (default `64`)
    * `:key` -- binary of length `0..64` (default `<<>>`)
    * `:salt` -- binary of length `0` or `16` (default `<<>>`)
    * `:personal` -- binary of length `0` or `16` (default `<<>>`)
  """
  @spec new(keyword()) :: %Hasher{}
  def new(opts \\ []) do
    digest_size = Keyword.get(opts, :digest_size, 64)
    key = Keyword.get(opts, :key, <<>>)
    salt = Keyword.get(opts, :salt, <<>>)
    personal = Keyword.get(opts, :personal, <<>>)
    validate!(digest_size, key, salt, personal)

    state = initial_state(digest_size, byte_size(key), salt, personal)

    buffer =
      if key == <<>> do
        <<>>
      else
        key <> :binary.copy(<<0>>, @block_size - byte_size(key))
      end

    %Hasher{state: state, buffer: buffer, byte_count: 0, digest_size: digest_size}
  end

  @doc "Feed more bytes into the stream."
  @spec update(%Hasher{}, binary()) :: %Hasher{}
  def update(%Hasher{} = h, data) when is_binary(data) do
    drain(%{h | buffer: h.buffer <> data})
  end

  @doc """
  Finalize and return the digest.  Non-destructive: the hasher keeps its
  state so `digest/1` can be called again or followed by more `update/2`
  calls.
  """
  @spec digest(%Hasher{}) :: binary()
  def digest(%Hasher{state: state, buffer: buf, byte_count: bc, digest_size: ds}) do
    padded = buf <> :binary.copy(<<0>>, @block_size - byte_size(buf))
    total = bc + byte_size(buf)
    new_state = compress(state, padded, total, true)
    <<raw::binary-size(64)>> = state_to_bytes(new_state)
    :binary.part(raw, 0, ds)
  end

  @doc "Finalize and return the digest as lowercase hex."
  @spec hex_digest(%Hasher{}) :: String.t()
  def hex_digest(%Hasher{} = h), do: to_hex(digest(h))

  @doc "Return an independent clone of the current stream state."
  @spec copy(%Hasher{}) :: %Hasher{}
  def copy(%Hasher{} = h), do: %Hasher{h | state: h.state, buffer: h.buffer}

  @doc "One-shot BLAKE2b.  Returns raw bytes of length `digest_size`."
  @spec blake2b(binary(), keyword()) :: binary()
  def blake2b(data, opts \\ []) when is_binary(data) do
    opts |> new() |> update(data) |> digest()
  end

  @doc "One-shot BLAKE2b, lowercase hex."
  @spec blake2b_hex(binary(), keyword()) :: String.t()
  def blake2b_hex(data, opts \\ []), do: to_hex(blake2b(data, opts))

  # ------------------------------------------------------------------
  # Internals
  # ------------------------------------------------------------------

  defp validate!(digest_size, _key, _salt, _personal)
       when not is_integer(digest_size) or digest_size < 1 or digest_size > @max_digest do
    raise ArgumentError, "digest_size must be in [1, 64], got #{inspect(digest_size)}"
  end

  defp validate!(_ds, key, _salt, _personal) when byte_size(key) > @max_key do
    raise ArgumentError, "key length must be in [0, 64], got #{byte_size(key)}"
  end

  defp validate!(_ds, _key, salt, _personal)
       when byte_size(salt) != 0 and byte_size(salt) != 16 do
    raise ArgumentError, "salt must be exactly 16 bytes (or empty), got #{byte_size(salt)}"
  end

  defp validate!(_ds, _key, _salt, personal)
       when byte_size(personal) != 0 and byte_size(personal) != 16 do
    raise ArgumentError,
          "personal must be exactly 16 bytes (or empty), got #{byte_size(personal)}"
  end

  defp validate!(_, _, _, _), do: :ok

  # Flush every full block except the latest -- at least one byte must remain
  # in the buffer so the final compression is the one flagged final.
  defp drain(%Hasher{buffer: buf} = h) when byte_size(buf) > @block_size do
    <<block::binary-size(@block_size), rest::binary>> = buf
    bc = h.byte_count + @block_size
    new_state = compress(h.state, block, bc, false)
    drain(%Hasher{h | state: new_state, buffer: rest, byte_count: bc})
  end

  defp drain(%Hasher{} = h), do: h

  # Build the parameter-block-XOR-ed starting state.  Sequential mode only
  # (fanout=1, depth=1).
  defp initial_state(digest_size, key_len, salt, personal) do
    salt_padded = salt <> :binary.copy(<<0>>, 16 - byte_size(salt))
    personal_padded = personal <> :binary.copy(<<0>>, 16 - byte_size(personal))
    # Parameter block: ds | kl | fanout=1 | depth=1 | leaf(4)=0 | node_offset(8)=0
    #                  | node_depth=0 | inner_length=0 | reserved(14)=0
    #                  | salt(16) | personal(16)
    p =
      <<digest_size::8, key_len::8, 1::8, 1::8, 0::32, 0::64, 0::8, 0::8, 0::unit(8)-size(14),
        salt_padded::binary, personal_padded::binary>>

    words = for <<w::little-64 <- p>>, do: w
    iv_list = Tuple.to_list(@iv)

    iv_list
    |> Enum.zip(words)
    |> Enum.map(fn {a, b} -> bxor(a, b) end)
    |> List.to_tuple()
  end

  # Parse a 128-byte block as a 16-tuple of little-endian 64-bit words.
  defp parse_block(block) do
    for(<<w::little-64 <- block>>, do: w) |> List.to_tuple()
  end

  # Compression function F.  `t` is the 128-bit total byte count so far.
  # `is_final?` triggers the v[14] inversion for the final block.
  defp compress(state_tuple, block, t, is_final?) do
    m = parse_block(block)

    # Build the 16-word working vector v.  state[0..7] || IV[0..7].
    v = state_tuple |> Tuple.to_list() |> Kernel.++(Tuple.to_list(@iv)) |> List.to_tuple()

    v =
      v
      |> tuple_update(12, fn x -> bxor(x, t &&& @mask64) end)
      |> tuple_update(13, fn x -> bxor(x, t >>> 64 &&& @mask64) end)

    v = if is_final?, do: tuple_update(v, 14, fn x -> bxor(x, @mask64) end), else: v

    v = Enum.reduce(0..11, v, fn i, acc -> round_fn(acc, m, elem(@sigma, rem(i, 10))) end)

    # Davies-Meyer feed-forward: state[i] ^= v[i] ^ v[i+8].
    Enum.reduce(0..7, state_tuple, fn i, acc ->
      tuple_update(acc, i, fn h -> h |> bxor(elem(v, i)) |> bxor(elem(v, i + 8)) end)
    end)
  end

  # One of twelve rounds: four column G's then four diagonal G's.
  defp round_fn(v, m, s) do
    v
    |> g(0, 4, 8, 12, elem(m, elem(s, 0)), elem(m, elem(s, 1)))
    |> g(1, 5, 9, 13, elem(m, elem(s, 2)), elem(m, elem(s, 3)))
    |> g(2, 6, 10, 14, elem(m, elem(s, 4)), elem(m, elem(s, 5)))
    |> g(3, 7, 11, 15, elem(m, elem(s, 6)), elem(m, elem(s, 7)))
    |> g(0, 5, 10, 15, elem(m, elem(s, 8)), elem(m, elem(s, 9)))
    |> g(1, 6, 11, 12, elem(m, elem(s, 10)), elem(m, elem(s, 11)))
    |> g(2, 7, 8, 13, elem(m, elem(s, 12)), elem(m, elem(s, 13)))
    |> g(3, 4, 9, 14, elem(m, elem(s, 14)), elem(m, elem(s, 15)))
  end

  # BLAKE2b quarter-round G.  Mutates v[a], v[b], v[c], v[d] with message
  # words x and y.  Rotation constants (R1..R4) = (32, 24, 16, 63).
  defp g(v, a, b, c, d, x, y) do
    va = elem(v, a)
    vb = elem(v, b)
    vc = elem(v, c)
    vd = elem(v, d)

    va = va + vb + x &&& @mask64
    vd = rotr64(bxor(vd, va), 32)
    vc = vc + vd &&& @mask64
    vb = rotr64(bxor(vb, vc), 24)
    va = va + vb + y &&& @mask64
    vd = rotr64(bxor(vd, va), 16)
    vc = vc + vd &&& @mask64
    vb = rotr64(bxor(vb, vc), 63)

    v
    |> put_elem(a, va)
    |> put_elem(b, vb)
    |> put_elem(c, vc)
    |> put_elem(d, vd)
  end

  defp rotr64(x, n) do
    (x >>> n ||| x <<< (64 - n)) &&& @mask64
  end

  defp tuple_update(t, i, fun), do: put_elem(t, i, fun.(elem(t, i)))

  # Pack the 8-word state into a 64-byte binary, little-endian.
  defp state_to_bytes(state) do
    for w <- Tuple.to_list(state), into: <<>>, do: <<w::little-64>>
  end

  defp to_hex(bin) do
    for <<b <- bin>>, into: "", do: <<hex_nibble(b >>> 4), hex_nibble(b &&& 0xF)>>
  end

  defp hex_nibble(n) when n < 10, do: ?0 + n
  defp hex_nibble(n), do: ?a + n - 10
end
