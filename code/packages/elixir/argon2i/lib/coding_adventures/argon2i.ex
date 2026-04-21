defmodule CodingAdventures.Argon2i do
  @moduledoc """
  Argon2i (RFC 9106) — data-independent memory-hard password hashing.

  Argon2i derives reference-block indices from a deterministic
  pseudo-random stream seeded purely from public parameters
  (pass, lane, slice, counter, total memory, total passes, type).
  Memory access patterns leak nothing about the password, making
  Argon2i resistant to side-channel attacks at the cost of weaker
  resistance to GPU/ASIC time-memory trade-off attacks. For general
  password hashing prefer `CodingAdventures.Argon2id`.

  Reference: https://datatracker.ietf.org/doc/html/rfc9106
  """

  import Bitwise

  @mask32 0xFFFFFFFF
  @mask64 0xFFFFFFFFFFFFFFFF
  @block_size 1024
  @block_words div(@block_size, 8)
  @sync_points 4
  @argon2_version 0x13
  @type_i 1

  @spec argon2i(binary(), binary(), pos_integer(), pos_integer(),
                pos_integer(), pos_integer(), keyword()) :: binary()
  def argon2i(password, salt, time_cost, memory_cost, parallelism, tag_length,
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
                    tag_length, key, ad, version, @type_i)

    memory = initial_memory(h0, parallelism, q)

    memory =
      Enum.reduce(0..(time_cost - 1), memory, fn r, mem ->
        Enum.reduce(0..(@sync_points - 1), mem, fn sl, mem2 ->
          Enum.reduce(0..(parallelism - 1), mem2, fn lane, mem3 ->
            fill_segment(mem3, r, lane, sl, q, segment_length, parallelism,
                         m_prime, time_cost)
          end)
        end)
      end)

    final = final_block(memory, parallelism, q)
    blake2b_long(tag_length, block_to_bytes(final))
  end

  @spec argon2i_hex(binary(), binary(), pos_integer(), pos_integer(),
                    pos_integer(), pos_integer(), keyword()) :: String.t()
  def argon2i_hex(password, salt, time_cost, memory_cost, parallelism, tag_length,
                  opts \\ []) do
    argon2i(password, salt, time_cost, memory_cost, parallelism, tag_length, opts)
    |> Base.encode16(case: :lower)
  end

  defp validate!(password, salt, t, m, p, tag_length, key, ad, version) do
    cond do
      byte_size(password) > @mask32 -> raise ArgumentError, "password length must fit in 32 bits"
      byte_size(salt) < 8 -> raise ArgumentError, "salt must be at least 8 bytes"
      byte_size(salt) > @mask32 -> raise ArgumentError, "salt length must fit in 32 bits"
      byte_size(key) > @mask32 -> raise ArgumentError, "key length must fit in 32 bits"
      byte_size(ad) > @mask32 -> raise ArgumentError, "associated_data length must fit in 32 bits"
      tag_length < 4 -> raise ArgumentError, "tag_length must be >= 4"
      tag_length > @mask32 -> raise ArgumentError, "tag_length must fit in 32 bits"
      not is_integer(p) or p < 1 or p > 0xFFFFFF -> raise ArgumentError, "parallelism must be in [1, 2^24-1]"
      m < 8 * p -> raise ArgumentError, "memory_cost must be >= 8*parallelism"
      m > @mask32 -> raise ArgumentError, "memory_cost must fit in 32 bits"
      t < 1 -> raise ArgumentError, "time_cost must be >= 1"
      version != @argon2_version -> raise ArgumentError, "only Argon2 v1.3 (0x13) is supported"
      true -> :ok
    end
  end

  defp compute_h0(password, salt, t, m, p, tag_length, key, ad, version, type) do
    data =
      le32(p) <> le32(tag_length) <> le32(m) <> le32(t) <>
      le32(version) <> le32(type) <>
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
    v |> put_elem(a, va) |> put_elem(b, vb) |> put_elem(c, vc) |> put_elem(d, vd)
  end

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

  defp compress(x, y) do
    r = for i <- 0..(@block_words - 1), do: bxor(Enum.at(x, i), Enum.at(y, i))
    r_tuple = List.to_tuple(r)

    q_tuple =
      Enum.reduce(0..7, r_tuple, fn row, acc ->
        row_tuple =
          for(i <- 0..15, do: elem(acc, row * 16 + i)) |> List.to_tuple()
        permuted = permutation_p(row_tuple)
        Enum.reduce(0..15, acc, fn i, inner ->
          put_elem(inner, row * 16 + i, elem(permuted, i))
        end)
      end)

    q_tuple =
      Enum.reduce(0..7, q_tuple, fn c, acc ->
        col_tuple =
          for(rr <- 0..7, k <- 0..1, do: elem(acc, rr * 16 + 2 * c + k))
          |> List.to_tuple()
        permuted = permutation_p(col_tuple)
        Enum.reduce(0..7, acc, fn rr, inner ->
          inner
          |> put_elem(rr * 16 + 2 * c, elem(permuted, 2 * rr))
          |> put_elem(rr * 16 + 2 * c + 1, elem(permuted, 2 * rr + 1))
        end)
      end)

    for i <- 0..(@block_words - 1),
        do: bxor(elem(r_tuple, i), elem(q_tuple, i))
  end

  defp initial_memory(h0, p, _q) do
    Enum.reduce(0..(p - 1), %{}, fn i, mem ->
      b0 = blake2b_long(@block_size, h0 <> le32(0) <> le32(i)) |> bytes_to_block()
      b1 = blake2b_long(@block_size, h0 <> le32(1) <> le32(i)) |> bytes_to_block()

      mem |> Map.put({i, 0}, b0) |> Map.put({i, 1}, b1)
    end)
  end

  defp final_block(memory, p, q) do
    Enum.reduce(1..(p - 1)//1, Map.get(memory, {0, q - 1}), fn lane, acc ->
      other = Map.get(memory, {lane, q - 1})
      for i <- 0..(@block_words - 1), do: bxor(Enum.at(acc, i), Enum.at(other, i))
    end)
  end

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

  # Argon2i: J1/J2 come from a double-compress address stream.
  defp fill_segment(memory, r, lane, sl, q, sl_len, p, m_prime, t_total) do
    zero_block = List.duplicate(0, @block_words)

    initial_input =
      zero_block
      |> List.replace_at(0, r)
      |> List.replace_at(1, lane)
      |> List.replace_at(2, sl)
      |> List.replace_at(3, m_prime)
      |> List.replace_at(4, t_total)
      |> List.replace_at(5, @type_i)

    starting_c = if r == 0 and sl == 0, do: 2, else: 0

    {initial_input, initial_block} =
      if starting_c != 0 do
        regenerate_addresses(zero_block, initial_input)
      else
        {initial_input, zero_block}
      end

    state = %{memory: memory, input: initial_input, address: initial_block}

    final =
      Enum.reduce(starting_c..(sl_len - 1)//1, state, fn i, st ->
        {input_now, address_now} =
          if rem(i, @block_words) == 0 and not (r == 0 and sl == 0 and i == 2) do
            regenerate_addresses(zero_block, st.input)
          else
            {st.input, st.address}
          end

        col = sl * sl_len + i
        prev_col = if col == 0, do: q - 1, else: col - 1
        prev_block = Map.fetch!(st.memory, {lane, prev_col})

        pseudo_rand = Enum.at(address_now, rem(i, @block_words))
        j1 = band(pseudo_rand, @mask32)
        j2 = band(bsr(pseudo_rand, 32), @mask32)

        l_prime = if r == 0 and sl == 0, do: lane, else: rem(j2, p)
        z_prime = index_alpha(j1, r, sl, i, l_prime == lane, q, sl_len)
        ref_block = Map.fetch!(st.memory, {l_prime, z_prime})

        new_block = compress(prev_block, ref_block)

        final =
          if r == 0 do
            new_block
          else
            existing = Map.fetch!(st.memory, {lane, col})
            for k <- 0..(@block_words - 1),
                do: bxor(Enum.at(existing, k), Enum.at(new_block, k))
          end

        %{st | memory: Map.put(st.memory, {lane, col}, final),
              input: input_now, address: address_now}
      end)

    final.memory
  end

  defp regenerate_addresses(zero_block, input) do
    counter = Enum.at(input, 6)
    updated_input = List.replace_at(input, 6, counter + 1)
    z = compress(zero_block, updated_input)
    addresses = compress(zero_block, z)
    {updated_input, addresses}
  end

  defp le32(n), do: <<n::little-unsigned-32>>

  defp bytes_to_block(data) do
    for <<w::little-unsigned-64 <- data>>, do: w
  end

  defp block_to_bytes(block) do
    for(w <- block, into: <<>>, do: <<w::little-unsigned-64>>)
  end
end
