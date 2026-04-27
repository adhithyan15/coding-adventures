defmodule CodingAdventures.Argon2d do
  @moduledoc """
  Argon2d (RFC 9106) — data-dependent memory-hard password hashing.

  Argon2d picks the reference-block index for every new block from the
  first 64 bits of the previously computed block. The memory access
  pattern therefore depends on the password, which maximises GPU/ASIC
  resistance at the cost of leaking a noisy channel through
  memory-access timing. Use Argon2d only when side-channel attacks are
  *not* in the threat model (e.g. proof-of-work). For password hashing
  prefer `CodingAdventures.Argon2id`.

  Elixir integers are arbitrary precision, so every 64-bit add and XOR
  masks with `0xFFFFFFFFFFFFFFFF` to stay inside a single machine word.

  Reference: https://datatracker.ietf.org/doc/html/rfc9106
  See also: code/specs/KD03-argon2.md
  """

  import Bitwise

  @mask32 0xFFFFFFFF
  @mask64 0xFFFFFFFFFFFFFFFF
  @block_size 1024
  @block_words div(@block_size, 8)
  @sync_points 4
  @argon2_version 0x13
  @type_d 0

  @doc """
  Compute the Argon2d tag.

  ## Parameters

    * `password` — secret input (binary).
    * `salt` — at least 8 bytes, 16+ recommended.
    * `time_cost` — number of passes (t), >= 1.
    * `memory_cost` — memory in KiB (m), >= 8 * parallelism.
    * `parallelism` — lane count (p), in [1, 2^24-1].
    * `tag_length` — output bytes (T), >= 4.
    * `opts` — keyword list: `:key`, `:associated_data`, `:version`.

  Returns exactly `tag_length` bytes.
  """
  @spec argon2d(binary(), binary(), pos_integer(), pos_integer(),
                pos_integer(), pos_integer(), keyword()) :: binary()
  def argon2d(password, salt, time_cost, memory_cost, parallelism, tag_length,
              opts \\ []) do
    key = Keyword.get(opts, :key, <<>>)
    ad = Keyword.get(opts, :associated_data, <<>>)
    version = Keyword.get(opts, :version, @argon2_version)

    :ok = validate!(password, salt, time_cost, memory_cost, parallelism,
                    tag_length, key, ad, version)

    segment_length = div(memory_cost, @sync_points * parallelism)
    m_prime = segment_length * @sync_points * parallelism
    q = div(m_prime, parallelism)

    h0 = compute_h0(password, salt, time_cost, memory_cost, parallelism,
                    tag_length, key, ad, version, @type_d)

    memory = initial_memory(h0, parallelism, q)

    memory =
      Enum.reduce(0..(time_cost - 1), memory, fn r, mem ->
        Enum.reduce(0..(@sync_points - 1), mem, fn sl, mem2 ->
          Enum.reduce(0..(parallelism - 1), mem2, fn lane, mem3 ->
            fill_segment(mem3, r, lane, sl, q, segment_length, parallelism)
          end)
        end)
      end)

    final = final_block(memory, parallelism, q)
    blake2b_long(tag_length, block_to_bytes(final))
  end

  @doc "Like `argon2d/7` but returns lowercase hex."
  @spec argon2d_hex(binary(), binary(), pos_integer(), pos_integer(),
                    pos_integer(), pos_integer(), keyword()) :: String.t()
  def argon2d_hex(password, salt, time_cost, memory_cost, parallelism, tag_length,
                  opts \\ []) do
    argon2d(password, salt, time_cost, memory_cost, parallelism, tag_length, opts)
    |> Base.encode16(case: :lower)
  end

  # ──────────────────────────────────────────────────────────────────────
  # Validation
  # ──────────────────────────────────────────────────────────────────────

  defp validate!(password, salt, t, m, p, tag_length, key, ad, version) do
    cond do
      byte_size(password) > @mask32 ->
        raise ArgumentError, "password length must fit in 32 bits"
      byte_size(salt) < 8 ->
        raise ArgumentError, "salt must be at least 8 bytes"
      byte_size(salt) > @mask32 ->
        raise ArgumentError, "salt length must fit in 32 bits"
      byte_size(key) > @mask32 ->
        raise ArgumentError, "key length must fit in 32 bits"
      byte_size(ad) > @mask32 ->
        raise ArgumentError, "associated_data length must fit in 32 bits"
      tag_length < 4 ->
        raise ArgumentError, "tag_length must be >= 4"
      tag_length > @mask32 ->
        raise ArgumentError, "tag_length must fit in 32 bits"
      not is_integer(p) or p < 1 or p > 0xFFFFFF ->
        raise ArgumentError, "parallelism must be in [1, 2^24-1]"
      m < 8 * p ->
        raise ArgumentError, "memory_cost must be >= 8*parallelism"
      m > @mask32 ->
        raise ArgumentError, "memory_cost must fit in 32 bits"
      t < 1 ->
        raise ArgumentError, "time_cost must be >= 1"
      version != @argon2_version ->
        raise ArgumentError, "only Argon2 v1.3 (0x13) is supported"
      true ->
        :ok
    end
  end

  # ──────────────────────────────────────────────────────────────────────
  # H0 (parameter hash) and H' (variable-length extender, RFC 9106 §3.3)
  # ──────────────────────────────────────────────────────────────────────

  defp compute_h0(password, salt, t, m, p, tag_length, key, ad, version, type) do
    data =
      le32(p) <>
      le32(tag_length) <>
      le32(m) <>
      le32(t) <>
      le32(version) <>
      le32(type) <>
      le32(byte_size(password)) <> password <>
      le32(byte_size(salt)) <> salt <>
      le32(byte_size(key)) <> key <>
      le32(byte_size(ad)) <> ad

    CodingAdventures.Blake2b.blake2b(data, digest_size: 64)
  end

  defp blake2b_long(t, _x) when t <= 0,
    do: raise(ArgumentError, "H' output length must be positive")

  defp blake2b_long(t, x) when t <= 64,
    do: CodingAdventures.Blake2b.blake2b(le32(t) <> x, digest_size: t)

  defp blake2b_long(t, x) do
    input = le32(t) <> x
    r = div(t + 31, 32) - 2
    v0 = CodingAdventures.Blake2b.blake2b(input, digest_size: 64)
    {v, out} =
      Enum.reduce(1..(r - 1)//1, {v0, binary_part(v0, 0, 32)}, fn _i, {vprev, acc} ->
        vnext = CodingAdventures.Blake2b.blake2b(vprev, digest_size: 64)
        {vnext, acc <> binary_part(vnext, 0, 32)}
      end)
    final_size = t - 32 * r
    tail = CodingAdventures.Blake2b.blake2b(v, digest_size: final_size)
    out <> tail
  end

  # ──────────────────────────────────────────────────────────────────────
  # Compression function G(X, Y): row pass then column pass.
  # ──────────────────────────────────────────────────────────────────────

  defp rotr64(x, n), do: band(bor(bsr(x, n), bsl(x, 64 - n)), @mask64)

  defp g_mix(v, a, b, c, d) do
    va = elem(v, a); vb = elem(v, b); vc = elem(v, c); vd = elem(v, d)
    va = band(va + vb + 2 * band(va, @mask32) * band(vb, @mask32), @mask64)
    vd = rotr64(bxor(vd, va), 32)
    vc = band(vc + vd + 2 * band(vc, @mask32) * band(vd, @mask32), @mask64)
    vb = rotr64(bxor(vb, vc), 24)
    va = band(va + vb + 2 * band(va, @mask32) * band(vb, @mask32), @mask64)
    vd = rotr64(bxor(vd, va), 16)
    vc = band(vc + vd + 2 * band(vc, @mask32) * band(vd, @mask32), @mask64)
    vb = rotr64(bxor(vb, vc), 63)
    v
    |> put_elem(a, va)
    |> put_elem(b, vb)
    |> put_elem(c, vc)
    |> put_elem(d, vd)
  end

  # Permutation P on a 16-element tuple of u64s.
  defp permutation_p(v) do
    v
    |> g_mix(0, 4, 8, 12)
    |> g_mix(1, 5, 9, 13)
    |> g_mix(2, 6, 10, 14)
    |> g_mix(3, 7, 11, 15)
    |> g_mix(0, 5, 10, 15)
    |> g_mix(1, 6, 11, 12)
    |> g_mix(2, 7, 8, 13)
    |> g_mix(3, 4, 9, 14)
  end

  # Compression G(X, Y). Inputs and output are lists of 128 u64s.
  defp compress(x, y) do
    r = for i <- 0..(@block_words - 1), do: bxor(Enum.at(x, i), Enum.at(y, i))
    r_tuple = List.to_tuple(r)
    q_tuple =
      Enum.reduce(0..7, r_tuple, fn row, acc ->
        row_tuple =
          for i <- 0..15 do
            elem(acc, row * 16 + i)
          end
          |> List.to_tuple()

        permuted = permutation_p(row_tuple)

        Enum.reduce(0..15, acc, fn i, inner ->
          put_elem(inner, row * 16 + i, elem(permuted, i))
        end)
      end)

    q_tuple =
      Enum.reduce(0..7, q_tuple, fn c, acc ->
        col_tuple =
          for rr <- 0..7, k <- 0..1 do
            elem(acc, rr * 16 + 2 * c + k)
          end
          |> List.to_tuple()

        permuted = permutation_p(col_tuple)

        Enum.reduce(0..7, acc, fn rr, inner ->
          inner
          |> put_elem(rr * 16 + 2 * c, elem(permuted, 2 * rr))
          |> put_elem(rr * 16 + 2 * c + 1, elem(permuted, 2 * rr + 1))
        end)
      end)

    for i <- 0..(@block_words - 1) do
      bxor(elem(r_tuple, i), elem(q_tuple, i))
    end
  end

  # ──────────────────────────────────────────────────────────────────────
  # Initial memory and final block
  # ──────────────────────────────────────────────────────────────────────

  defp initial_memory(h0, p, q) do
    Enum.reduce(0..(p - 1), %{memory: %{}, q: q}, fn i, acc ->
      b0 = blake2b_long(@block_size, h0 <> le32(0) <> le32(i)) |> bytes_to_block()
      b1 = blake2b_long(@block_size, h0 <> le32(1) <> le32(i)) |> bytes_to_block()

      acc
      |> update_in([:memory], &Map.put(&1, {i, 0}, b0))
      |> update_in([:memory], &Map.put(&1, {i, 1}, b1))
    end)
    |> Map.get(:memory)
  end

  defp final_block(memory, p, q) do
    Enum.reduce(1..(p - 1)//1, Map.get(memory, {0, q - 1}), fn lane, acc ->
      other = Map.get(memory, {lane, q - 1})
      for i <- 0..(@block_words - 1), do: bxor(Enum.at(acc, i), Enum.at(other, i))
    end)
  end

  # ──────────────────────────────────────────────────────────────────────
  # Indexing (RFC 9106 §3.4.1.1)
  # ──────────────────────────────────────────────────────────────────────

  defp index_alpha(j1, r, sl, c, same_lane, q, sl_len) do
    {w, start} =
      cond do
        r == 0 and sl == 0 ->
          {c - 1, 0}
        r == 0 ->
          w =
            cond do
              same_lane -> sl * sl_len + c - 1
              c == 0 -> sl * sl_len - 1
              true -> sl * sl_len
            end
          {w, 0}
        true ->
          w =
            cond do
              same_lane -> q - sl_len + c - 1
              c == 0 -> q - sl_len - 1
              true -> q - sl_len
            end
          {w, rem((sl + 1) * sl_len, q)}
      end

    x = bsr(j1 * j1, 32)
    y = bsr(w * x, 32)
    rel = w - 1 - y
    rem(start + rel, q)
  end

  # ──────────────────────────────────────────────────────────────────────
  # fill_segment — Argon2d uses data-dependent addressing: J1/J2 always
  # come from the previous block's first word.
  # ──────────────────────────────────────────────────────────────────────

  defp fill_segment(memory, r, lane, sl, q, sl_len, p) do
    starting_c = if r == 0 and sl == 0, do: 2, else: 0

    Enum.reduce(starting_c..(sl_len - 1)//1, memory, fn i, mem ->
      col = sl * sl_len + i
      prev_col = if col == 0, do: q - 1, else: col - 1
      prev_block = Map.fetch!(mem, {lane, prev_col})

      pseudo_rand = List.first(prev_block)
      j1 = band(pseudo_rand, @mask32)
      j2 = band(bsr(pseudo_rand, 32), @mask32)

      l_prime =
        if r == 0 and sl == 0, do: lane, else: rem(j2, p)

      z_prime = index_alpha(j1, r, sl, i, l_prime == lane, q, sl_len)
      ref_block = Map.fetch!(mem, {l_prime, z_prime})

      new_block = compress(prev_block, ref_block)

      final =
        if r == 0 do
          new_block
        else
          existing = Map.fetch!(mem, {lane, col})
          for k <- 0..(@block_words - 1),
              do: bxor(Enum.at(existing, k), Enum.at(new_block, k))
        end

      Map.put(mem, {lane, col}, final)
    end)
  end

  # ──────────────────────────────────────────────────────────────────────
  # Binary <-> word-list helpers
  # ──────────────────────────────────────────────────────────────────────

  defp le32(n), do: <<n::little-unsigned-32>>

  defp bytes_to_block(data) do
    for <<w::little-unsigned-64 <- data>>, do: w
  end

  defp block_to_bytes(block) do
    for(w <- block, into: <<>>, do: <<w::little-unsigned-64>>)
  end
end
