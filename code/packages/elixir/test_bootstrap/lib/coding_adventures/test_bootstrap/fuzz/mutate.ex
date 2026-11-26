defmodule CodingAdventures.TestBootstrap.Fuzz.Mutate do
  @moduledoc """
  The mutator algebra (VER06 section 3, VER06n section 3).

  Every mutator takes a binary and returns a NEW binary (BEAM binaries are
  immutable, so "never modifies its argument" is free).  Two BEAM facts shape the
  code:

    * integers never overflow, so every modular wrap is an explicit
      `band(x, 2**(8w) - 1)`;
    * `rem/2` is TRUNCATED (`rem(-1, 8) == -1`), which is the wrong thing for a
      wrapping offset, so offsets always use `Integer.mod/2` (floored,
      `Integer.mod(-1, 8) == 7`).
  """
  import Bitwise

  alias CodingAdventures.TestBootstrap.Prng
  alias CodingAdventures.TestBootstrap.Fuzz.Limits

  # Interest values of section 3.4, by width in bytes.
  @interest_8 [0, 1, 127, 128, 255]
  @interest_16 [0, 1, 255, 256, 32_767, 32_768, 65_535]
  @interest_32 [0, 1, 65_535, 65_536, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF]
  @interest_64 [
    0,
    1,
    0xFFFF_FFFF,
    0x1_0000_0000,
    0x7FFF_FFFF_FFFF_FFFF,
    0x8000_0000_0000_0000,
    0xFFFF_FFFF_FFFF_FFFF
  ]
  # Arithmetic deltas of section 3.3.
  @deltas [-35, -8, -1, 1, 8, 35]

  def interest_8, do: @interest_8
  def interest_16, do: @interest_16
  def interest_32, do: @interest_32
  def interest_64, do: @interest_64
  def interest_by_width, do: %{1 => @interest_8, 2 => @interest_16, 4 => @interest_32, 8 => @interest_64}
  def deltas, do: @deltas

  defmodule WorkExhausted do
    @moduledoc "A `mutate_sequence` call would copy more than its work cap."
    defexception [:work, :cap]

    @impl true
    def message(%{cap: cap}), do: "mutate_sequence work cap of #{cap} bytes exceeded"
  end

  defp int!(v, name), do: Limits.check_int!(v, name)

  defp bytes!(data, _name) when is_binary(data), do: data

  defp bytes!(_data, name),
    do: raise(ArgumentError, "#{name} must be a binary")

  defp endian!(e) when e in ["little", :little], do: :little
  defp endian!(e) when e in ["big", :big], do: :big
  defp endian!(_), do: raise(ArgumentError, "endian must be 'little' or 'big'")

  # ----------------------------------------------------------------------------
  # Flips
  # ----------------------------------------------------------------------------

  @doc """
  Inverts `count` consecutive units (bits when `unit == 1`, bytes when `unit == 8`)
  from `offset mod n`, wrapping around the buffer.  Only the PARITY of each unit's
  flips matters (XOR twice is a no-op): every unit is hit `div(count, n)` times and
  the first `rem(count, n)` units from the wrapped start once more, so the work is
  O(len) for ANY count, where a loop of `count` single flips would never end.

  The buffer is read as ONE little-endian integer: bit `i` is bit `i mod 8` of byte
  `i div 8`, LSB first, exactly how section 3.1 numbers bits.

  Worked example, 2 bytes = 16 bits, `bit_flip(offset 0, count 17)`: 17 div 16 = 1
  (every bit once, odd) and 17 rem 16 = 1 (bit 0 once more, so twice, even), so bits
  1..15 end up inverted and bit 0 unchanged.
  """
  def parity_flip(data, unit, offset, count) do
    raw = bytes!(data, "data")
    size = byte_size(raw)

    if size == 0 or count <= 0 do
      raw
    else
      total = size * 8
      units = div(total, unit)
      start = Integer.mod(offset, units) * unit
      full = div(count, units)
      len = rem(count, units) * unit

      raw = if rem(full, 2) == 1, do: invert_all(raw), else: raw

      cond do
        len == 0 -> raw
        # the window fits before the end of the buffer
        start + len <= total -> flip_range(raw, start, start + len)
        # the part past the end re-enters at bit 0
        true -> raw |> flip_range(start, total) |> flip_range(0, start + len - total)
      end
    end
  end

  # XOR every byte with 0xFF, a 64-bit word at a time.  (A 1 MiB buffer read as ONE
  # integer is 8 million bits: the BEAM refuses to build a bignum that large, so the
  # flips never go through one.)
  defp invert_all(bin) do
    words = div(byte_size(bin), 8) * 8
    <<head::binary-size(words), tail::binary>> = bin

    inverted_head = for <<w::64 <- head>>, into: <<>>, do: <<bxor(w, 0xFFFF_FFFF_FFFF_FFFF)::64>>
    inverted_tail = for <<b <- tail>>, into: <<>>, do: <<bxor(b, 0xFF)>>
    inverted_head <> inverted_tail
  end

  # Inverts bits [a, b) (bit i is bit i mod 8 of byte i div 8), a < b.
  defp flip_range(raw, a, b) when b > a do
    first = div(a, 8)
    last = div(b - 1, 8)
    first_mask = band(0xFF <<< rem(a, 8), 0xFF)
    last_mask = (1 <<< (rem(b - 1, 8) + 1)) - 1

    if first == last do
      xor_byte(raw, first, band(first_mask, last_mask))
    else
      middle = binary_part(raw, first + 1, last - first - 1)

      <<pre::binary-size(first), f, _::binary-size(last - first - 1), l, post::binary>> = raw

      <<pre::binary, bxor(f, first_mask), invert_all(middle)::binary, bxor(l, last_mask), post::binary>>
    end
  end

  defp xor_byte(raw, i, mask) do
    <<pre::binary-size(i), b, post::binary>> = raw
    <<pre::binary, bxor(b, mask), post::binary>>
  end

  @doc "Invert `count` consecutive bits from `bit_offset mod (8 * len)`: `bit_flip(<<0, 0>>, 9)` is `<<0, 2>>`."
  def bit_flip(data, bit_offset, count \\ 1) do
    int!(bit_offset, "bit_offset")
    int!(count, "count")
    parity_flip(data, 1, bit_offset, count)
  end

  @doc "XOR `count` consecutive bytes with 0xFF from `byte_offset mod len`."
  def byte_flip(data, byte_offset, count \\ 1) do
    int!(byte_offset, "byte_offset")
    int!(count, "count")
    parity_flip(data, 8, byte_offset, count)
  end

  # ----------------------------------------------------------------------------
  # Arithmetic and boundary values
  # ----------------------------------------------------------------------------

  defp read(chunk, order), do: :binary.decode_unsigned(chunk, order)
  defp write(v, w, :little), do: <<v::little-size(w * 8)>>
  defp write(v, w, :big), do: <<v::big-size(w * 8)>>

  defp widths_ok?(w, size), do: w in [1, 2, 4, 8] and size >= w

  @doc """
  Add signed `delta` modulo `2**(8*width)` to the integer at the offset.  u16
  little-endian: `ff 00` is 0x00ff; +1 gives 0x0100, stored `00 01`.
  """
  def byte_arithmetic(data, byte_offset, delta, width \\ 1, endian \\ "little") do
    int!(byte_offset, "byte_offset")
    int!(delta, "delta")
    int!(width, "width")
    order = endian!(endian)
    raw = bytes!(data, "data")
    size = byte_size(raw)

    if widths_ok?(width, size) do
      at = Integer.mod(byte_offset, size - width + 1)
      <<pre::binary-size(at), chunk::binary-size(width), post::binary>> = raw
      mask = (1 <<< (8 * width)) - 1
      pre <> write(band(read(chunk, order) + delta, mask), width, order) <> post
    else
      raw
    end
  end

  @doc "Overwrite `width` bytes with `val mod 2**(8*width)`: `boundary_insert(<<0,0,0,0>>, 0, 4, 0x80000000, \"little\")` is `<<0,0,0,0x80>>`."
  def boundary_insert(data, byte_offset, width, val, endian \\ "little") do
    int!(byte_offset, "byte_offset")
    int!(width, "width")
    int!(val, "val")
    order = endian!(endian)
    raw = bytes!(data, "data")
    size = byte_size(raw)

    if widths_ok?(width, size) do
      at = Integer.mod(byte_offset, size - width + 1)
      <<pre::binary-size(at), _::binary-size(width), post::binary>> = raw
      pre <> write(band(val, (1 <<< (8 * width)) - 1), width, order) <> post
    else
      raw
    end
  end

  # ----------------------------------------------------------------------------
  # Blocks and splice
  # ----------------------------------------------------------------------------

  @doc "Delete up to `length` bytes at `offset mod len`; clamped at the end."
  def block_delete(data, offset, length) do
    int!(offset, "offset")
    int!(length, "length")
    raw = bytes!(data, "data")
    size = byte_size(raw)

    if size == 0 or length <= 0 do
      raw
    else
      at = Integer.mod(offset, size)
      cut = min(length, size - at)
      binary_part(raw, 0, at) <> binary_part(raw, at + cut, size - at - cut)
    end
  end

  @doc "Insert at `offset mod (len + 1)`; an empty input becomes the insert."
  def block_insert(data, offset, insert_bytes) do
    int!(offset, "offset")
    raw = bytes!(data, "data")
    ins = bytes!(insert_bytes, "insert_bytes")
    size = byte_size(raw)

    if size == 0 do
      ins
    else
      at = Integer.mod(offset, size + 1)
      binary_part(raw, 0, at) <> ins <> binary_part(raw, at, size - at)
    end
  end

  @doc "Copy up to `length` bytes from `src_offset mod len` to `dst_offset mod (len + 1)` (the end is a valid insertion point)."
  def block_duplicate(data, src_offset, length, dst_offset) do
    int!(src_offset, "src_offset")
    int!(length, "length")
    int!(dst_offset, "dst_offset")
    raw = bytes!(data, "data")
    size = byte_size(raw)

    if size == 0 or length <= 0 do
      raw
    else
      src = Integer.mod(src_offset, size)
      chunk = binary_part(raw, src, min(length, size - src))
      dst = Integer.mod(dst_offset, size + 1)
      binary_part(raw, 0, dst) <> chunk <> binary_part(raw, dst, size - dst)
    end
  end

  @doc "`target[0...ts] <> donor[ds..]` with both splits clamped into their buffers."
  def splice(target, donor, target_split, donor_split) do
    int!(target_split, "target_split")
    int!(donor_split, "donor_split")
    tgt = bytes!(target, "target")
    don = bytes!(donor, "donor")
    t = target_split |> max(0) |> min(byte_size(tgt))
    d = donor_split |> max(0) |> min(byte_size(don))
    binary_part(tgt, 0, t) <> binary_part(don, d, byte_size(don) - d)
  end

  # ----------------------------------------------------------------------------
  # mutate_sequence
  # ----------------------------------------------------------------------------

  @doc """
  `steps` seeded mutations, each chosen by one `bounded(7)` draw:

      0 bit_flip, count 1            3 boundary_insert, width 1, interest_8
      1 byte_flip, count 1           4 block_delete (len 1..n/2), skipped if n <= 1
      2 byte_arithmetic, deltas      5 block_insert of one random byte
                                     6 block_duplicate (len 1..n/2), skipped if n <= 1

  An empty buffer is refilled to `<<0>>` before every step.  The draw order is the
  cross-lane contract.  `max_len` is VER06 section 3.5's length budget: an insert or
  duplicate that would pass it is skipped AFTER its draws, so the stream stays
  aligned and memory stays bounded.

  Returns `{:ok, bytes, work}` where `work` is the bytes copied (the initial copy
  plus one buffer per step), or `{:work_exhausted, work}`: the call stops at the
  step that would pass `work_cap` and reports what it had done (a failed call is
  still charged by its caller).  `steps` over `max_iterations` and inputs over
  `max_input_bytes` are an `ArgumentError`, raised before anything is copied.
  """
  def mutate_sequence_counted(data, seed, steps, max_len, work_cap) do
    int!(seed, "seed")
    int!(steps, "steps")
    int!(max_len, "max_len")
    int!(work_cap, "work_cap")

    if steps > Limits.max_iterations(),
      do: raise(ArgumentError, "steps exceeds the maximum (#{Limits.max_iterations()})")

    cur = bytes!(data, "data")

    if byte_size(cur) > Limits.max_input_bytes(),
      do: raise(ArgumentError, "input exceeds #{Limits.max_input_bytes()} bytes")

    # the initial copy is work too
    work = byte_size(cur)

    if work > work_cap do
      {:work_exhausted, 0}
    else
      seq_loop(cur, Prng.new(seed), max(steps, 0), max_len, work_cap, work)
    end
  end

  defp seq_loop(cur, _rng, 0, _max_len, _cap, work), do: {:ok, cur, work}

  defp seq_loop(<<>>, rng, left, max_len, cap, work),
    do: seq_loop(<<0>>, rng, left, max_len, cap, work)

  defp seq_loop(cur, rng, left, max_len, cap, work) do
    n = byte_size(cur)

    if work + n > cap do
      {:work_exhausted, work}
    else
      {kind, rng} = Prng.bounded(rng, 7)
      {cur, rng} = step(kind, cur, n, rng, max_len)
      seq_loop(cur, rng, left - 1, max_len, cap, work + n)
    end
  end

  defp step(0, cur, n, rng, _max) do
    {i, rng} = Prng.bounded(rng, n * 8)
    {bit_flip(cur, i, 1), rng}
  end

  defp step(1, cur, n, rng, _max) do
    {i, rng} = Prng.bounded(rng, n)
    {byte_flip(cur, i, 1), rng}
  end

  defp step(2, cur, n, rng, _max) do
    {di, rng} = Prng.bounded(rng, length(@deltas))
    {off, rng} = Prng.bounded(rng, n)
    {byte_arithmetic(cur, off, Enum.at(@deltas, di), 1), rng}
  end

  defp step(3, cur, n, rng, _max) do
    {vi, rng} = Prng.bounded(rng, length(@interest_8))
    {off, rng} = Prng.bounded(rng, n)
    {boundary_insert(cur, off, 1, Enum.at(@interest_8, vi)), rng}
  end

  defp step(4, cur, n, rng, _max) when n > 1 do
    {l, rng} = Prng.bounded(rng, max(1, div(n, 2)))
    {off, rng} = Prng.bounded(rng, n)
    {block_delete(cur, off, 1 + l), rng}
  end

  defp step(5, cur, n, rng, max_len) do
    {b, rng} = Prng.bounded(rng, 256)
    {at, rng} = Prng.bounded(rng, n + 1)
    if n + 1 <= max_len, do: {block_insert(cur, at, <<b>>), rng}, else: {cur, rng}
  end

  defp step(6, cur, n, rng, max_len) when n > 1 do
    {l, rng} = Prng.bounded(rng, max(1, div(n, 2)))
    {src, rng} = Prng.bounded(rng, n)
    {dst, rng} = Prng.bounded(rng, n + 1)
    length = 1 + l

    if n + min(length, n - src) <= max_len,
      do: {block_duplicate(cur, src, length, dst), rng},
      else: {cur, rng}
  end

  # kinds 4 and 6 on a one-byte buffer: skipped, no draws
  defp step(_kind, cur, _n, rng, _max), do: {cur, rng}

  @doc "`mutate_sequence_counted/5` with the per-call cap, raising `WorkExhausted` on a breach."
  def mutate_sequence(data, seed, steps, max_len \\ nil, work_cap \\ nil) do
    max_len = max_len || Limits.max_input_bytes()
    work_cap = work_cap || Limits.max_sequence_work()

    case mutate_sequence_counted(data, seed, steps, max_len, work_cap) do
      {:ok, bytes, _work} -> bytes
      {:work_exhausted, work} -> raise WorkExhausted, work: work, cap: work_cap
    end
  end
end
