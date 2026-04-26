defmodule CodingAdventures.Zstd do
  import Bitwise

  @moduledoc """
  Zstandard (ZStd) lossless compression algorithm — CMP07.

  Zstandard (RFC 8878) is a high-ratio, fast compression format created by
  Yann Collet at Facebook (2015). It combines:

  - **LZ77 back-references** (via LZSS token generation) to exploit repetition
    in the data — the same "copy from earlier in the output" trick as DEFLATE,
    but with a larger 32 KB window.
  - **FSE (Finite State Entropy)** coding for the sequence descriptor symbols.
    FSE is an asymmetric numeral system that approaches the Shannon entropy limit
    in a single pass. It is strictly more efficient than Huffman coding.
  - **Predefined decode tables** (RFC 8878 Appendix B) so short frames need no
    table description overhead at all.

  ## Frame layout (RFC 8878 §3)

      ┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
      │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
      │ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
      └────────┴─────┴──────────────────────┴────────┴──────────────────┘

  Each **block** has a 3-byte header:

      bit 0       = Last_Block flag
      bits [2:1]  = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
      bits [23:3] = Block_Size

  ## Compression strategy

  1. Split data into 128 KB blocks (MAX_BLOCK_SIZE).
  2. For each block, try in order:
     a. **RLE** — all bytes identical → 4 bytes total (header + 1 byte).
     b. **Compressed** (LZ77 + FSE) — if smaller than raw input.
     c. **Raw** — verbatim copy as fallback.

  ## Series

      CMP00 (LZ77)     — Sliding-window back-references
      CMP01 (LZ78)     — Explicit dictionary (trie)
      CMP02 (LZSS)     — LZ77 + flag bits
      CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
      CMP04 (Huffman)  — Entropy coding
      CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
      CMP06 (Brotli)   — DEFLATE + context modelling + static dict
      CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed  ← this module

  ## Examples

      iex> data = "the quick brown fox jumps over the lazy dog"
      iex> compressed = CodingAdventures.Zstd.compress(data)
      iex> {:ok, decompressed} = CodingAdventures.Zstd.decompress(compressed)
      iex> decompressed == data
      true
  """

  # ─── Constants ─────────────────────────────────────────────────────────────────
  #
  # ZStd magic: every valid ZStd frame starts with these 4 bytes (LE).
  # Chosen to be unlikely in arbitrary plaintext; also identifies the format
  # to tools like `file(1)` and `binwalk`.
  @magic 0xFD2FB528

  # Maximum block size: 128 KB. Larger inputs are split across multiple blocks.
  # The spec maximum is min(WindowSize, 128 KB); we use the fixed limit here.
  @max_block_size 128 * 1024

  # Decompression bomb guard: cap total output at 256 MB. Without this, a
  # carefully crafted RLE block of size 2^30 would allocate gigabytes of RAM.
  @max_output 256 * 1024 * 1024

  # ─── FSE table accuracy logs ────────────────────────────────────────────────────
  #
  # "Accuracy log" = log2(table_size). A higher acc_log gives finer probability
  # resolution at the cost of a larger table.
  @ll_acc_log 6  # LL table: 2^6 = 64 slots
  @ml_acc_log 6  # ML table: 2^6 = 64 slots
  @of_acc_log 5  # OF table: 2^5 = 32 slots

  # ─── FSE predefined distributions (RFC 8878 Appendix B) ─────────────────────────
  #
  # "Predefined_Mode" means no per-frame table description is transmitted;
  # the decoder builds the same table from these fixed distributions.
  #
  # Entries of -1 mean "probability 1/table_size" — these symbols each get
  # exactly one slot in the decode table and their encoder state never needs
  # extra bits.

  # Normalised distribution for Literal Length FSE.  64 slots total.
  @ll_norm [4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1,
            2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
            -1, -1, -1, -1]

  # Normalised distribution for Match Length FSE.  64 slots total.
  @ml_norm [1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1,
            -1, -1, -1, -1, -1]

  # Normalised distribution for Offset FSE.  32 slots total.
  @of_norm [1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1]

  # ─── LL / ML code tables (RFC 8878 §3.1.1.3) ──────────────────────────────────
  #
  # These tables map a *code number* to a {baseline, extra_bits} pair.
  # For example, LL code 17 means: literal_length = 18 + read(1 extra bit),
  # so it covers literal lengths 18 and 19.
  #
  # The FSE state machine tracks one code number per field; extra bits are
  # read directly from the bitstream after state transitions.

  # Literal Length code table: {baseline, extra_bits} for codes 0..35.
  # Codes 0..15 are identity (one code per value); codes 16+ cover ranges
  # with increasing extra bit counts.
  @ll_codes [
    {0,0},{1,0},{2,0},{3,0},{4,0},{5,0},{6,0},{7,0},
    {8,0},{9,0},{10,0},{11,0},{12,0},{13,0},{14,0},{15,0},
    {16,1},{18,1},{20,1},{22,1},
    {24,2},{28,2},
    {32,3},{40,3},
    {48,4},{64,6},
    {128,7},{256,8},{512,9},{1024,10},{2048,11},{4096,12},
    {8192,13},{16384,14},{32768,15},{65536,16}
  ]

  # Match Length code table: {baseline, extra_bits} for codes 0..52.
  # Minimum match length in ZStd is 3 (not 0). Code 0 = match length 3.
  @ml_codes [
    {3,0},{4,0},{5,0},{6,0},{7,0},{8,0},{9,0},{10,0},{11,0},{12,0},
    {13,0},{14,0},{15,0},{16,0},{17,0},{18,0},{19,0},{20,0},{21,0},{22,0},
    {23,0},{24,0},{25,0},{26,0},{27,0},{28,0},{29,0},{30,0},{31,0},{32,0},
    {33,0},{34,0},
    {35,1},{37,1},{39,1},{41,1},
    {43,2},{47,2},
    {51,3},{59,3},
    {67,4},{83,4},
    {99,5},{131,7},
    {259,8},{515,9},{1027,10},{2051,11},
    {4099,12},{8195,13},{16387,14},{32771,15},{65539,16}
  ]

  # ─── floor_log2 helper ─────────────────────────────────────────────────────────
  #
  # Compute floor(log2(n)) for n >= 1 using integer arithmetic.
  # We need this for FSE table construction (computing nb = acc_log - floor_log2(ns)).
  #
  # Example: floor_log2(8) = 3, floor_log2(9) = 3, floor_log2(16) = 4.
  defp floor_log2(n) when n >= 1 do
    do_floor_log2(n >>> 1, 0)
  end

  defp do_floor_log2(0, acc), do: acc
  defp do_floor_log2(n, acc), do: do_floor_log2(n >>> 1, acc + 1)

  # ─── FSE decode table construction ────────────────────────────────────────────
  #
  # An FSE decode table maps a state index (0..sz-1) to a triple
  # {sym, nb, base} where:
  #   sym  = the output symbol at this state
  #   nb   = number of extra bits to read for the next state transition
  #   base = added to those extra bits to form the next state
  #
  # Construction (must mirror the Rust implementation exactly):
  #
  # Phase 1: symbols with norm[s] == -1 (prob = 1/sz) go at the HIGH end of the
  #   table (indices sz-1, sz-2, ...). These get exactly one slot.
  #
  # Phase 2: remaining symbols are spread across the LOW portion using a
  #   deterministic step function. This ensures each symbol occupies the
  #   correct fraction of slots proportional to its normalised count.
  #   Two-pass: first symbols with count > 1, then count == 1.
  #   This matches the reference implementation ordering.
  #
  # Phase 3: assign nb and base to each slot.
  #   For symbol s with sym_next counter ns:
  #     nb   = acc_log - floor_log2(ns)
  #     base = (ns << nb) - sz
  #   The sym_next counter for each symbol starts at its count and increments
  #   as we assign slots in index order.

  defp build_decode_table(norm, acc_log) do
    sz = 1 <<< acc_log
    step = (sz >>> 1) + (sz >>> 3) + 3

    # Phase 1: place -1 probability symbols at the top.
    # We accumulate {high_index, table_map} using a Map for O(1) writes.
    {high, tbl0, sym_next0} =
      norm
      |> Enum.with_index()
      |> Enum.reduce({sz - 1, %{}, %{}}, fn {c, s}, {high, tbl, sn} ->
        if c == -1 do
          new_high = if high > 0, do: high - 1, else: 0
          {new_high, Map.put(tbl, high, {s, 0, 0}), Map.put(sn, s, 1)}
        else
          {high, tbl, sn}
        end
      end)

    # Phase 2: spread remaining symbols.
    # The step function step = (sz>>1) + (sz>>3) + 3 is co-prime to sz
    # (which is always a power of two), so it visits every slot in [0..high] exactly once.
    {tbl1, sym_next1} =
      Enum.reduce(0..1, {tbl0, sym_next0, 0}, fn pass, {tbl, sn, pos} ->
        {new_tbl, new_sn, new_pos} =
          norm
          |> Enum.with_index()
          |> Enum.reduce({tbl, sn, pos}, fn {c, s}, {tbl_acc, sn_acc, pos_acc} ->
            if c <= 0 do
              {tbl_acc, sn_acc, pos_acc}
            else
              cnt = c
              skip = if pass == 0, do: cnt <= 1, else: cnt != 1
              if skip do
                {tbl_acc, sn_acc, pos_acc}
              else
                new_sn = Map.put(sn_acc, s, cnt)
                # Spread cnt occurrences of symbol s across the table
                {final_tbl, final_pos} =
                  Enum.reduce(1..cnt, {tbl_acc, pos_acc}, fn _, {t, p} ->
                    t2 = Map.put(t, p, {s, 0, 0})
                    # Advance pos using the step, skipping slots above `high`
                    next_p = advance_pos(p, step, sz, high)
                    {t2, next_p}
                  end)
                {final_tbl, new_sn, final_pos}
              end
            end
          end)
        {new_tbl, new_sn, new_pos}
      end)
      |> then(fn {tbl, sn, _pos} -> {tbl, sn} end)

    # Phase 3: assign nb and base using sym_next counters.
    # We iterate slots in ascending index order, tracking which occurrence
    # of each symbol we're at. nb = acc_log - floor_log2(ns), base = (ns << nb) - sz.
    {final_tbl, _sn} =
      Enum.reduce(0..(sz - 1), {tbl1, sym_next1}, fn i, {tbl, sn} ->
        {s, _nb, _base} = Map.get(tbl, i, {0, 0, 0})
        ns = Map.get(sn, s, 1)
        nb = acc_log - floor_log2(ns)
        base = (ns <<< nb) - sz
        new_tbl = Map.put(tbl, i, {s, nb, base})
        new_sn = Map.put(sn, s, ns + 1)
        {new_tbl, new_sn}
      end)

    # Convert the map to a flat list indexed 0..sz-1 for O(1) access.
    Enum.map(0..(sz - 1), fn i -> Map.fetch!(final_tbl, i) end)
  end

  # Advance position using the step function, skipping slots above `high`.
  # This wraps around modulo sz, skipping any position > high.
  defp advance_pos(pos, step, sz, high) do
    next = (pos + step) &&& (sz - 1)
    if next > high, do: advance_pos(next, step, sz, high), else: next
  end

  # ─── FSE encode symbol table construction ──────────────────────────────────────
  #
  # The encoder needs two tables:
  #
  # 1. ee (encode_entry): For each symbol s, stores {delta_nb, delta_fs}.
  #    Given encoder state E in [sz, 2*sz):
  #      nb_out = (E + delta_nb) >>> 16      (how many state bits to emit)
  #      emit low nb_out bits of E
  #      new_E  = st[(E >>> nb_out) + delta_fs]
  #
  # 2. st (state_table): Maps slot index to encoder output state.
  #    slot = cumul[s] + j (j = which occurrence of s in the spread table)
  #    output_state = decode_table_index + sz
  #
  # The key insight: the encoder and decoder must be SYMMETRIC. The decoder
  # assigns {sym, nb, base} to each decode table cell in INDEX ORDER. The
  # encoder must use the same indexing to ensure that after encoding symbol s
  # from slot cumul[s]+j, the decoder at that cell index will decode s correctly.

  defp build_encode_sym(norm, acc_log) do
    sz = 1 <<< acc_log

    # Step 1: compute cumulative counts for all symbols.
    {cumul, _total} =
      norm
      |> Enum.with_index()
      |> Enum.reduce({%{}, 0}, fn {c, s}, {cum, total} ->
        cnt = if c == -1, do: 1, else: max(c, 0)
        {Map.put(cum, s, total), total + cnt}
      end)

    # Step 2: rebuild the spread table (same algorithm as build_decode_table phase 1+2).
    # This gives spread[index] = symbol, telling us which symbol occupies each slot.
    step = (sz >>> 1) + (sz >>> 3) + 3

    {_high, spread0} =
      norm
      |> Enum.with_index()
      |> Enum.reduce({sz - 1, %{}}, fn {c, s}, {high, sp} ->
        if c == -1 do
          new_high = if high > 0, do: high - 1, else: 0
          {new_high, Map.put(sp, high, s)}
        else
          {high, sp}
        end
      end)

    high = map_size(spread0)
    # idx_limit = the last index filled by -1 symbols in ascending order
    # = sz - 1 - (count of -1 symbols). We compute it as the minimum free slot.
    idx_limit = find_idx_limit(spread0, sz)

    {spread1, _} =
      Enum.reduce(0..1, {spread0, 0}, fn pass, {sp, pos} ->
        {new_sp, new_pos} =
          norm
          |> Enum.with_index()
          |> Enum.reduce({sp, pos}, fn {c, s}, {sp_acc, pos_acc} ->
            if c <= 0 do
              {sp_acc, pos_acc}
            else
              cnt = c
              skip = if pass == 0, do: cnt <= 1, else: cnt != 1
              if skip do
                {sp_acc, pos_acc}
              else
                {final_sp, final_pos} =
                  Enum.reduce(1..cnt, {sp_acc, pos_acc}, fn _, {sp2, p} ->
                    sp3 = Map.put(sp2, p, s)
                    next_p = advance_pos(p, step, sz, idx_limit)
                    {sp3, next_p}
                  end)
                {final_sp, final_pos}
              end
            end
          end)
        {new_sp, new_pos}
      end)
      |> then(fn {sp, pos} -> {sp, pos} end)

    _ = high  # suppress unused warning

    # Step 3: build the state table.
    # For each decode table index i (ascending order):
    #   s = spread[i]
    #   j = sym_occ[s]   (occurrence count so far, starting at 0)
    #   slot = cumul[s] + j
    #   st[slot] = i + sz  (output state = decode index + sz)
    {st, _sym_occ} =
      Enum.reduce(0..(sz - 1), {%{}, %{}}, fn i, {st_acc, sym_occ} ->
        s = Map.fetch!(spread1, i)
        j = Map.get(sym_occ, s, 0)
        slot = Map.fetch!(cumul, s) + j
        new_st = Map.put(st_acc, slot, i + sz)
        new_occ = Map.put(sym_occ, s, j + 1)
        {new_st, new_occ}
      end)

    # Step 4: build ee (encode entries) for each symbol.
    # For symbol s with count cnt:
    #   max_bits_out (mbo):
    #     cnt == 1 → mbo = acc_log  (symbol fits in exactly one slot per table)
    #     cnt > 1  → mbo = acc_log - floor_log2(cnt)
    #   delta_nb = (mbo << 16) - (cnt << mbo)
    #   delta_fs = cumul[s] - cnt
    ee =
      norm
      |> Enum.with_index()
      |> Enum.map(fn {c, s} ->
        cnt = if c == -1, do: 1, else: max(c, 0)
        if cnt == 0 do
          {s, {0, 0}}
        else
          mbo = if cnt == 1, do: acc_log, else: acc_log - floor_log2(cnt)
          delta_nb = (mbo <<< 16) - (cnt <<< mbo)
          delta_fs = Map.fetch!(cumul, s) - cnt
          {s, {delta_nb, delta_fs}}
        end
      end)
      |> Enum.into(%{}, fn {s, entry} -> {s, entry} end)

    # Convert st map to list indexed 0..sz-1.
    st_list = Enum.map(0..(sz - 1), fn slot -> Map.get(st, slot, 0) end)

    {ee, st_list}
  end

  # Find idx_limit: the highest index available for non-(-1) symbols.
  # The -1 symbols occupy indices from the top downward.
  # idx_limit = (sz - 1) - count_of_neg1_symbols.
  defp find_idx_limit(spread_map, sz) do
    neg1_count = map_size(spread_map)
    sz - 1 - neg1_count
  end

  # ─── Reverse Bit Writer ────────────────────────────────────────────────────────
  #
  # ZStd's sequence bitstream is written *backwards* relative to the data flow:
  # the encoder writes bits that the decoder will read LAST, FIRST. This allows
  # the decoder to operate on a forward-only stream while decoding sequences
  # in logical order.
  #
  # Byte layout: the last byte written contains a **sentinel bit** (the highest
  # set bit) that marks the end of meaningful data. The decoder initialises by
  # finding this sentinel.
  #
  # Bit layout within each byte: LSB = first bit written.
  #
  # Example: write bits 1,0,1,1 (4 bits) then flush:
  #   reg = 0b1011, bits = 4
  #   flush: sentinel at bit 4 → last byte = 0b0001_1011 = 0x1B
  #   output = [0x1B]
  #
  # The buffer is accumulated by PREPENDING bytes (so the list is reversed),
  # then Enum.reverse/1 gives the correct byte order at the end.

  defmodule RevBitWriter do
    @moduledoc false
    defstruct buf: [], reg: 0, bits: 0

    def new(), do: %RevBitWriter{}

    # Adding zero bits is a no-op.
    def add_bits(%RevBitWriter{} = rbw, _val, 0), do: rbw

    def add_bits(%RevBitWriter{buf: buf, reg: reg, bits: bits}, val, nb) do
      # Mask to only the requested number of bits, then OR them into the
      # register starting at the current bit position (LSB side).
      mask = (1 <<< nb) - 1
      reg2 = reg ||| ((val &&& mask) <<< bits)
      bits2 = bits + nb
      {buf2, reg3, bits3} = flush_bytes(buf, reg2, bits2)
      %RevBitWriter{buf: buf2, reg: reg3, bits: bits3}
    end

    # Flush complete bytes (8 bits at a time) from the low end of the register.
    # Bytes are prepended so the list accumulates in reverse order.
    defp flush_bytes(buf, reg, bits) when bits >= 8 do
      flush_bytes([reg &&& 0xFF | buf], reg >>> 8, bits - 8)
    end
    defp flush_bytes(buf, reg, bits), do: {buf, reg, bits}

    # Flush remaining partial byte with sentinel bit.
    # The sentinel is a 1 placed just above the remaining data bits.
    # Example: reg = 0b1011, bits = 4 → sentinel at bit 4 → byte = 0b0001_1011 = 0x1B.
    def flush(%RevBitWriter{buf: buf, reg: reg, bits: bits}) do
      sentinel = 1 <<< bits
      last_byte = (reg &&& 0xFF) ||| sentinel
      Enum.reverse([last_byte | buf])
    end
  end

  # ─── Reverse Bit Reader ────────────────────────────────────────────────────────
  #
  # Mirrors RevBitWriter: reads bits from the END of the buffer going backward.
  # The LAST bits written by the encoder (the initial FSE states) are at the END
  # of the byte buffer (in the sentinel-containing last byte). The reader starts
  # there and reads backward toward byte 0.
  #
  # Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side).
  # read_bits(n) extracts the top n bits and shifts the register left by n.
  #
  # Why left-aligned? The writer accumulates bits LSB-first. Within each flushed
  # byte, bit 0 = earliest written, bit N = latest written. To read the LATEST
  # bits first (they're at the high end of the stream), we need a left-aligned
  # register so that reading from the top gives the highest-position bits first.
  #
  # State is a plain map: %{data: list_of_bytes, reg: integer, bits: int, pos: int}
  # where pos is the index of the next byte to load (decrements toward 0).

  defp rbr_new(bytes) when is_list(bytes) do
    n = length(bytes)
    cond do
      n == 0 ->
        {:error, "empty bitstream"}

      List.last(bytes) == 0 ->
        {:error, "bitstream last byte is zero (no sentinel)"}

      true ->
        last = List.last(bytes)

        # sentinel_pos = index of the highest set bit in `last`.
        # Example: last = 0b00011110 → floor_log2(last) = 4 → sentinel at bit 4.
        # valid_bits = number of data bits below the sentinel = sentinel_pos.
        sentinel_pos = floor_log2(last)
        valid_bits = sentinel_pos

        # Place the data bits of the sentinel byte at the TOP of the 64-bit register.
        # Example: last = 0b00011110, valid_bits = 4, data bits = 0b1110.
        #   After left-aligning to 64 bits: reg = 0b1110 << 60.
        mask = if valid_bits == 0, do: 0, else: (1 <<< valid_bits) - 1
        reg = if valid_bits == 0, do: 0, else: (last &&& mask) <<< (64 - valid_bits)

        state = %{data: bytes, reg: reg, bits: valid_bits, pos: n - 1}
        {:ok, rbr_reload(state)}
    end
  end

  # Load more bytes into the register from the stream going backward.
  # Each new byte is placed just BELOW the currently loaded bits.
  # In the left-aligned register, "just below" means at position 64 - bits - 8.
  defp rbr_reload(%{bits: bits, pos: pos} = state) when bits <= 56 and pos > 0 do
    new_pos = pos - 1
    byte = Enum.at(state.data, new_pos)
    shift = 64 - bits - 8
    new_reg = state.reg ||| (byte <<< shift)
    rbr_reload(%{state | reg: new_reg, bits: bits + 8, pos: new_pos})
  end
  defp rbr_reload(state), do: state

  # Read nb bits from the top of the register (most recently written first).
  defp rbr_read_bits(state, 0), do: {0, state}
  defp rbr_read_bits(%{reg: reg, bits: bits} = state, nb) do
    # Extract the top nb bits from the left-aligned register.
    val = reg >>> (64 - nb)
    # Shift the register left to consume those bits.
    new_reg = if nb == 64, do: 0, else: reg <<< nb
    new_bits = max(bits - nb, 0)
    new_state = %{state | reg: new_reg &&& 0xFFFFFFFFFFFFFFFF, bits: new_bits}
    new_state = if new_bits < 24, do: rbr_reload(new_state), else: new_state
    {val, new_state}
  end

  # ─── FSE encode/decode operations ─────────────────────────────────────────────
  #
  # The encoder and decoder both maintain FSE state in [sz, 2*sz).
  # Encoding symbol `sym` from state E:
  #   1. Compute nb = (E + delta_nb) >>> 16   (bits to emit)
  #   2. Write low nb bits of E to the bitstream
  #   3. new_E = st[(E >>> nb) + delta_fs]
  #
  # Decoding symbol from state S:
  #   1. Look up decode_table[S] → {sym, nb, base}
  #   2. new_S = base + read(nb bits)
  #   3. Return sym

  defp fse_encode_sym(state, sym, ee, st) do
    # ee is a map: symbol -> {delta_nb, delta_fs}
    {delta_nb, delta_fs} = Map.fetch!(ee, sym)
    nb = (state + delta_nb) >>> 16
    slot_i = (state >>> nb) + delta_fs
    slot = max(slot_i, 0)
    new_state = Enum.at(st, slot)
    {nb, state, new_state}
  end

  defp fse_decode_sym(state, dt, br_state) do
    # dt is a list of {sym, nb, base} indexed by state
    {sym, nb, base} = Enum.at(dt, state)
    {bits, new_br} = rbr_read_bits(br_state, nb)
    new_state = base + bits
    {sym, new_state, new_br}
  end

  # ─── LL / ML code number computation ─────────────────────────────────────────
  #
  # Map a literal length or match length to its code number by finding the last
  # entry in the code table whose baseline <= the given value.
  # Since baselines are in strictly increasing order, we scan left-to-right and
  # keep updating `code` as long as the baseline fits.

  defp ll_to_code(ll) do
    @ll_codes
    |> Enum.with_index()
    |> Enum.reduce(0, fn {{base, _bits}, i}, best ->
      if base <= ll, do: i, else: best
    end)
  end

  defp ml_to_code(ml) do
    @ml_codes
    |> Enum.with_index()
    |> Enum.reduce(0, fn {{base, _bits}, i}, best ->
      if base <= ml, do: i, else: best
    end)
  end

  # ─── Token-to-sequence conversion ─────────────────────────────────────────────
  #
  # LZSS produces a flat stream of {:literal, byte} and {:match, offset, length}.
  # ZStd groups consecutive literals before each match into a single Sequence:
  #   {ll, ml, off}  where ll = literal run length, ml = match length, off = offset.
  # Any trailing literals (after the last match) stay in the literals buffer
  # without a corresponding sequence entry.
  #
  # This mirrors the Rust tokens_to_seqs function exactly.

  defp tokens_to_seqs(tokens) do
    {lits, seqs, _lit_run} =
      Enum.reduce(tokens, {[], [], 0}, fn tok, {lits, seqs, lit_run} ->
        case tok do
          %{kind: :literal, byte: b} ->
            {[b | lits], seqs, lit_run + 1}

          %{kind: :match, offset: off, length: len} ->
            seq = {lit_run, len, off}
            {lits, [seq | seqs], 0}
        end
      end)

    # Reverse because we accumulated in reverse order.
    {Enum.reverse(lits), Enum.reverse(seqs)}
  end

  # ─── Literals section encoding ────────────────────────────────────────────────
  #
  # ZStd literals can be Huffman-coded or raw. We use Raw_Literals (type=0),
  # the simplest: no Huffman table, bytes stored verbatim.
  #
  # Header format (RFC 8878 §3.1.1.2.1):
  #   bits [1:0] = Literals_Block_Type = 00 (Raw)
  #   bits [3:2] = Size_Format:
  #     00 or 10 → 1-byte header: size in bits [7:3] (5 bits, values 0..31)
  #     01       → 2-byte header: size in bits [11:4] (12 bits, 0..4095)
  #     11       → 3-byte header: size in bits [19:4] (20 bits, 0..1MB)

  defp encode_literals_section(lits) when is_list(lits) do
    n = length(lits)
    hdr_bytes =
      cond do
        n <= 31 ->
          # 1-byte header: size_format=00, type=00
          [n <<< 3]

        n <= 4095 ->
          # 2-byte header: size_format=01, type=00 → 0b0100
          hdr = (n <<< 4) ||| 0b0100
          [hdr &&& 0xFF, (hdr >>> 8) &&& 0xFF]

        true ->
          # 3-byte header: size_format=11, type=00 → 0b1100
          hdr = (n <<< 4) ||| 0b1100
          [hdr &&& 0xFF, (hdr >>> 8) &&& 0xFF, (hdr >>> 16) &&& 0xFF]
      end
    hdr_bytes ++ lits
  end

  # Decode literals section, returning {literals_list, bytes_consumed}.
  defp decode_literals_section(data) when is_binary(data) do
    if byte_size(data) == 0 do
      {:error, "empty literals section"}
    else
      b0 = :binary.at(data, 0)
      ltype = b0 &&& 0b11

      if ltype != 0 do
        {:error, "unsupported literals type #{ltype} (only Raw=0 supported)"}
      else
        size_format = (b0 >>> 2) &&& 0b11

        case size_format do
          sf when sf in [0, 2] ->
            # 1-byte header
            n = b0 >>> 3
            decode_lits_extract(data, 1, n)

          1 ->
            # 2-byte header
            if byte_size(data) < 2 do
              {:error, "truncated literals header (2-byte)"}
            else
              n = ((b0 >>> 4) ||| (:binary.at(data, 1) <<< 4))
              decode_lits_extract(data, 2, n)
            end

          3 ->
            # 3-byte header
            if byte_size(data) < 3 do
              {:error, "truncated literals header (3-byte)"}
            else
              n = ((b0 >>> 4) ||| (:binary.at(data, 1) <<< 4) ||| (:binary.at(data, 2) <<< 12))
              decode_lits_extract(data, 3, n)
            end
        end
      end
    end
  end

  defp decode_lits_extract(data, header_bytes, n) do
    start = header_bytes
    end_pos = start + n
    if end_pos > byte_size(data) do
      {:error, "literals data truncated: need #{end_pos}, have #{byte_size(data)}"}
    else
      lits = :binary.bin_to_list(binary_part(data, start, n))
      {:ok, lits, end_pos}
    end
  end

  # ─── Sequences count encoding/decoding ────────────────────────────────────────
  #
  # RFC 8878 §3.1.1.1.3 Number_of_Sequences encoding:
  #
  #   0:         byte0 = 0
  #   1..127:    byte0 = count  (byte0 in 1..127)
  #   128..0x7FFE: 2 bytes:
  #                 byte0 = (count >> 8) | 0x80   (byte0 in 128..254)
  #                 byte1 = count & 0xFF
  #                 Decode: count = (byte0 & 0x7F) << 8 | byte1
  #   0x7FFF+:   3 bytes: 0xFF, byte1, byte2
  #                 count = byte1 + byte2 * 256 + 0x7F00
  #
  # The 2-byte encoding places the HIGH byte of the count (with bit 7 set) FIRST.
  # This ensures byte0 >= 128 for the 2-byte case, distinguishing it from 1-byte.

  defp encode_seq_count(cnt) when cnt == 0, do: [0]
  defp encode_seq_count(cnt) when cnt < 128, do: [cnt]
  defp encode_seq_count(cnt) when cnt <= 0x7FFE do
    # 2-byte: byte0 = (count >> 8) | 0x80, byte1 = count & 0xFF
    byte0 = (cnt >>> 8) ||| 0x80
    byte1 = cnt &&& 0xFF
    [byte0, byte1]
  end
  defp encode_seq_count(cnt) do
    # 3-byte: 0xFF, (count - 0x7F00) as LE u16
    r = cnt - 0x7F00
    [0xFF, r &&& 0xFF, (r >>> 8) &&& 0xFF]
  end

  defp decode_seq_count(data, pos) when pos >= byte_size(data) do
    {:error, "empty sequence count"}
  end
  defp decode_seq_count(data, pos) do
    b0 = :binary.at(data, pos)
    cond do
      b0 < 128 ->
        {:ok, b0, pos + 1}

      b0 < 0xFF ->
        # 2-byte: count = (byte0 & 0x7F) << 8 | byte1
        if pos + 1 >= byte_size(data) do
          {:error, "truncated sequence count"}
        else
          b1 = :binary.at(data, pos + 1)
          cnt = ((b0 &&& 0x7F) <<< 8) ||| b1
          {:ok, cnt, pos + 2}
        end

      true ->
        if pos + 2 >= byte_size(data) do
          {:error, "truncated sequence count (3-byte)"}
        else
          b1 = :binary.at(data, pos + 1)
          b2 = :binary.at(data, pos + 2)
          cnt = 0x7F00 + b1 + (b2 <<< 8)
          {:ok, cnt, pos + 3}
        end
    end
  end

  # ─── Sequences section encoding ───────────────────────────────────────────────
  #
  # Layout:
  #   [symbol_compression_modes: 1 byte]  (0x00 = all Predefined)
  #   [FSE bitstream: variable]
  #
  # The FSE bitstream is a backward bit stream (reverse bit writer):
  #   Sequences are encoded in REVERSE ORDER (last first).
  #   For each sequence (in reverse):
  #     OF extra bits, ML extra bits, LL extra bits (in this order)
  #     FSE encode: ML symbol, then OF symbol, then LL symbol
  #   After all sequences, flush final FSE states:
  #     (state_of - sz_of) as OF_ACC_LOG bits
  #     (state_ml - sz_ml) as ML_ACC_LOG bits
  #     (state_ll - sz_ll) as LL_ACC_LOG bits
  #   Add sentinel and flush.
  #
  # The decoder does the mirror:
  #   1. Read LL_ACC_LOG bits → initial state_ll
  #   2. Read ML_ACC_LOG bits → initial state_ml
  #   3. Read OF_ACC_LOG bits → initial state_of
  #   4. For each sequence:
  #       decode LL symbol (state transition)
  #       decode OF symbol
  #       decode ML symbol
  #       read LL extra bits → final ll value
  #       read ML extra bits → final ml value
  #       read OF extra bits → final offset value
  #   5. Apply sequence to output buffer

  defp encode_sequences_section(seqs) do
    {ee_ll, st_ll} = build_encode_sym(@ll_norm, @ll_acc_log)
    {ee_ml, st_ml} = build_encode_sym(@ml_norm, @ml_acc_log)
    {ee_of, st_of} = build_encode_sym(@of_norm, @of_acc_log)

    sz_ll = 1 <<< @ll_acc_log
    sz_ml = 1 <<< @ml_acc_log
    sz_of = 1 <<< @of_acc_log

    # FSE encoder states start at table_size (= sz). The state range [sz, 2*sz)
    # maps to slot range [0, sz) in the state table.
    init_state_ll = sz_ll
    init_state_ml = sz_ml
    init_state_of = sz_of

    # Encode sequences in reverse order into a RevBitWriter.
    {bw, state_ll, state_ml, state_of} =
      Enum.reverse(seqs)
      |> Enum.reduce(
        {RevBitWriter.new(), init_state_ll, init_state_ml, init_state_of},
        fn {ll, ml, off}, {bw, s_ll, s_ml, s_of} ->
          ll_code = ll_to_code(ll)
          ml_code = ml_to_code(ml)

          # Offset encoding: raw = off + 3 (RFC 8878 §3.1.1.3.2.1)
          # code = floor_log2(raw); extra = raw - (1 << code)
          raw_off = off + 3
          of_code =
            if raw_off <= 1, do: 0, else: floor_log2(raw_off)
          of_extra = raw_off - (1 <<< of_code)

          # Write extra bits (OF, ML, LL in this order for the backward stream).
          bw = RevBitWriter.add_bits(bw, of_extra, of_code)
          {_ml_base, ml_extra_bits} = Enum.at(@ml_codes, ml_code)
          ml_extra = ml - elem(Enum.at(@ml_codes, ml_code), 0)
          bw = RevBitWriter.add_bits(bw, ml_extra, ml_extra_bits)
          {_ll_base, ll_extra_bits} = Enum.at(@ll_codes, ll_code)
          ll_extra = ll - elem(Enum.at(@ll_codes, ll_code), 0)
          bw = RevBitWriter.add_bits(bw, ll_extra, ll_extra_bits)

          # FSE encode symbols. Decode order is LL, OF, ML; encode order (reversed
          # for backward stream) is ML, OF, LL.
          {nb_ml, val_ml, new_s_ml} = fse_encode_sym(s_ml, ml_code, ee_ml, st_ml)
          bw = RevBitWriter.add_bits(bw, val_ml, nb_ml)

          {nb_of, val_of, new_s_of} = fse_encode_sym(s_of, of_code, ee_of, st_of)
          bw = RevBitWriter.add_bits(bw, val_of, nb_of)

          {nb_ll, val_ll, new_s_ll} = fse_encode_sym(s_ll, ll_code, ee_ll, st_ll)
          bw = RevBitWriter.add_bits(bw, val_ll, nb_ll)

          {bw, new_s_ll, new_s_ml, new_s_of}
        end
      )

    # Flush final FSE states (low acc_log bits of state - sz).
    bw = RevBitWriter.add_bits(bw, state_of - sz_of, @of_acc_log)
    bw = RevBitWriter.add_bits(bw, state_ml - sz_ml, @ml_acc_log)
    bw = RevBitWriter.add_bits(bw, state_ll - sz_ll, @ll_acc_log)

    # Flush with sentinel bit and return byte list.
    RevBitWriter.flush(bw)
  end

  # ─── Block-level compress ─────────────────────────────────────────────────────
  #
  # Compress one block, returning either {:ok, compressed_bytes} or :fallback.
  # Returns :fallback if the compressed form is >= the raw input (not beneficial).

  defp compress_block(block) when is_binary(block) do
    # Use LZSS to generate LZ77 tokens.
    # Window = 32 KB, max match = 255, min match = 3.
    tokens = CodingAdventures.LZSS.encode(block, 32768, 255, 3)

    {lits, seqs} = tokens_to_seqs(tokens)

    # If no sequences found, LZ77 had nothing to compress; fall back to raw.
    if seqs == [] do
      :fallback
    else
      lit_section = encode_literals_section(lits)
      seq_count_bytes = encode_seq_count(length(seqs))
      modes_byte = [0x00]  # all Predefined
      bitstream = encode_sequences_section(seqs)

      out_bytes = lit_section ++ seq_count_bytes ++ modes_byte ++ bitstream

      if length(out_bytes) >= byte_size(block) do
        :fallback
      else
        {:ok, out_bytes}
      end
    end
  end

  # ─── Block-level decompress ────────────────────────────────────────────────────
  #
  # Decompress one ZStd compressed block, appending output to `acc` (reversed list).
  # Returns {:ok, new_acc} or {:error, reason}.

  defp decompress_block(data, acc) when is_binary(data) do
    # ── Literals section ─────────────────────────────────────────────────────
    case decode_literals_section(data) do
      {:error, reason} ->
        {:error, reason}

      {:ok, lits, lit_consumed} ->
        pos = lit_consumed
        # Convert literal list to binary for efficient access.
        lits_bin = :erlang.list_to_binary(lits)

        # ── Sequences count ─────────────────────────────────────────────────
        if pos >= byte_size(data) do
          # Block has only literals, no sequences — append directly.
          {:ok, acc <> lits_bin}
        else
          case decode_seq_count(data, pos) do
            {:error, reason} -> {:error, reason}

            {:ok, n_seqs, pos2} ->
              if n_seqs == 0 do
                # No sequences — all content is literals.
                {:ok, acc <> lits_bin}
              else
                decompress_block_seqs(data, pos2, lits_bin, n_seqs, acc)
              end
          end
        end
    end
  end

  defp decompress_block_seqs(data, pos, lits_bin, n_seqs, acc) do
    # ── Symbol compression modes ───────────────────────────────────────────
    if pos >= byte_size(data) do
      {:error, "missing symbol compression modes byte"}
    else
      modes_byte = :binary.at(data, pos)
      pos2 = pos + 1

      ll_mode = (modes_byte >>> 6) &&& 3
      of_mode = (modes_byte >>> 4) &&& 3
      ml_mode = (modes_byte >>> 2) &&& 3

      if ll_mode != 0 or of_mode != 0 or ml_mode != 0 do
        {:error, "unsupported FSE modes: LL=#{ll_mode} OF=#{of_mode} ML=#{ml_mode} (only Predefined=0 supported)"}
      else
        # ── FSE bitstream ─────────────────────────────────────────────────
        bitstream_bytes = :binary.bin_to_list(binary_part(data, pos2, byte_size(data) - pos2))

        case rbr_new(bitstream_bytes) do
          {:error, reason} -> {:error, reason}

          {:ok, br} ->
            dt_ll = build_decode_table(@ll_norm, @ll_acc_log)
            dt_ml = build_decode_table(@ml_norm, @ml_acc_log)
            dt_of = build_decode_table(@of_norm, @of_acc_log)

            # Read initial FSE states.
            {s_ll, br} = rbr_read_bits(br, @ll_acc_log)
            {s_ml, br} = rbr_read_bits(br, @ml_acc_log)
            {s_of, br} = rbr_read_bits(br, @of_acc_log)

            # Use binaries for the output buffer so back-references can use
            # binary_part/3 (O(1)) instead of Enum.at/2 (O(n)).
            # `acc` is the output so far from previous blocks.
            apply_sequences(n_seqs, lits_bin, 0, acc, s_ll, s_ml, s_of, br, dt_ll, dt_ml, dt_of)
        end
      end
    end
  end

  defp apply_sequences(0, lits_bin, lit_pos, out, _s_ll, _s_ml, _s_of, _br, _dt_ll, _dt_ml, _dt_of) do
    # Append any remaining trailing literals.
    n_lits = byte_size(lits_bin)
    if lit_pos < n_lits do
      {:ok, out <> binary_part(lits_bin, lit_pos, n_lits - lit_pos)}
    else
      {:ok, out}
    end
  end

  defp apply_sequences(remaining, lits_bin, lit_pos, out, s_ll, s_ml, s_of, br, dt_ll, dt_ml, dt_of) do
    # Decode LL symbol, then OF symbol, then ML symbol.
    {ll_code, new_s_ll, br} = fse_decode_sym(s_ll, dt_ll, br)
    {of_code, new_s_of, br} = fse_decode_sym(s_of, dt_of, br)
    {ml_code, new_s_ml, br} = fse_decode_sym(s_ml, dt_ml, br)

    if ll_code >= length(@ll_codes) do
      {:error, "invalid LL code #{ll_code}"}
    else
      if ml_code >= length(@ml_codes) do
        {:error, "invalid ML code #{ml_code}"}
      else
        {ll_base, ll_extra_bits} = Enum.at(@ll_codes, ll_code)
        {ml_base, ml_extra_bits} = Enum.at(@ml_codes, ml_code)

        {ll_extra, br} = rbr_read_bits(br, ll_extra_bits)
        {ml_extra, br} = rbr_read_bits(br, ml_extra_bits)
        {of_extra, br} = rbr_read_bits(br, of_code)

        ll = ll_base + ll_extra
        ml = ml_base + ml_extra
        # Offset: of_raw = (1 << of_code) | of_extra; offset = of_raw - 3
        of_raw = (1 <<< of_code) ||| of_extra
        offset = of_raw - 3

        # Emit `ll` literal bytes from the literals binary.
        lit_end = lit_pos + ll
        n_lits = byte_size(lits_bin)
        if lit_end > n_lits do
          {:error, "literal run #{ll} overflows literals buffer (pos=#{lit_pos} len=#{n_lits})"}
        else
          lit_chunk = if ll > 0, do: binary_part(lits_bin, lit_pos, ll), else: <<>>
          out2 = out <> lit_chunk

          # Copy `ml` bytes from `offset` back in the output buffer.
          # `offset` is 1-indexed (1 = last byte written), so the copy starts at
          # byte_size(out2) - offset. The copy may overlap (e.g., offset=1, ml=10
          # expands one byte into 10 identical bytes — run-length expansion).
          out_len = byte_size(out2)
          if offset == 0 or offset > out_len do
            {:error, "bad match offset #{offset} (output len #{out_len})"}
          else
            copy_start = out_len - offset
            copy_chunk = copy_with_overlap(out2, copy_start, offset, ml)
            out3 = out2 <> copy_chunk

            apply_sequences(remaining - 1, lits_bin, lit_end, out3, new_s_ll, new_s_ml, new_s_of, br, dt_ll, dt_ml, dt_of)
          end
        end
      end
    end
  end

  # Copy `length` bytes starting at `start` in `buf`, with overlap-safe semantics.
  # When `length > distance` (where distance = byte_size(buf) - start), the copy
  # wraps around and repeats the `distance` bytes. This implements LZ77 run-length
  # expansion: offset=1 length=10 turns one byte into 10 identical bytes.
  defp copy_with_overlap(buf, start, distance, length) do
    # Optimisation: if no overlap possible, use binary_part directly (O(length)).
    if length <= distance do
      binary_part(buf, start, length)
    else
      # Overlap: repeat the `distance`-byte window.
      # Use a list accumulator (prepend + reverse) for O(n) total, not O(n²).
      bytes = do_copy_overlap(buf, start, distance, length, 0, [])
      :erlang.list_to_binary(bytes)
    end
  end

  # Accumulate bytes for the overlap copy case.
  # `copied` tracks how many bytes have been written so far.
  # Each new byte is at index `start + rem(copied, distance)` within `buf`.
  defp do_copy_overlap(_buf, _start, _distance, 0, _copied, acc), do: Enum.reverse(acc)
  defp do_copy_overlap(buf, start, distance, remaining, copied, acc) do
    effective = start + rem(copied, distance)
    byte_val = :binary.at(buf, effective)
    do_copy_overlap(buf, start, distance, remaining - 1, copied + 1, [byte_val | acc])
  end

  # ─── Public API ───────────────────────────────────────────────────────────────

  @doc """
  Compress `data` to ZStd format (RFC 8878).

  The output is a valid ZStd frame that can be decompressed by the `zstd`
  CLI tool or any conforming implementation.

  ## Parameters

  - `data` — the binary or string to compress.

  ## Examples

      iex> data = "the quick brown fox " |> String.duplicate(20)
      iex> compressed = CodingAdventures.Zstd.compress(data)
      iex> byte_size(compressed) < byte_size(data)
      true

      iex> compressed = CodingAdventures.Zstd.compress("hello")
      iex> {:ok, "hello"} = CodingAdventures.Zstd.decompress(compressed)
      true
  """
  @spec compress(binary()) :: binary()
  def compress(data) when is_binary(data) do
    out_bytes = compress_frame(data)
    :erlang.list_to_binary(out_bytes)
  end

  defp compress_frame(data) do
    # ── ZStd frame header ─────────────────────────────────────────────────────
    # Magic number (4 bytes LE): 0xFD2FB528
    magic_bytes = [
      @magic &&& 0xFF,
      (@magic >>> 8) &&& 0xFF,
      (@magic >>> 16) &&& 0xFF,
      (@magic >>> 24) &&& 0xFF
    ]

    # Frame Header Descriptor (FHD):
    #   bit 7-6: FCS_Field_Size flag = 11 → 8-byte FCS
    #   bit 5:   Single_Segment_Flag = 1 (no Window_Descriptor follows)
    #   bit 4:   Content_Checksum_Flag = 0
    #   bit 3-2: reserved = 0
    #   bit 1-0: Dict_ID_Flag = 0
    # = 0b1110_0000 = 0xE0
    fhd = [0xE0]

    # Frame_Content_Size: 8 bytes LE (uncompressed size).
    fcs_val = byte_size(data)
    fcs = [
      fcs_val &&& 0xFF,
      (fcs_val >>> 8) &&& 0xFF,
      (fcs_val >>> 16) &&& 0xFF,
      (fcs_val >>> 24) &&& 0xFF,
      (fcs_val >>> 32) &&& 0xFF,
      (fcs_val >>> 40) &&& 0xFF,
      (fcs_val >>> 48) &&& 0xFF,
      (fcs_val >>> 56) &&& 0xFF
    ]

    header = magic_bytes ++ fhd ++ fcs

    # ── Blocks ────────────────────────────────────────────────────────────────
    # Handle empty input: emit one empty raw block.
    blocks =
      if byte_size(data) == 0 do
        # Last=1, Type=Raw(00), Size=0 → header = 0b0000_0001 = 0x01
        [0x01, 0x00, 0x00]
      else
        encode_blocks(data, 0, [])
      end

    header ++ blocks
  end

  defp encode_blocks(data, offset, acc) when offset >= byte_size(data) do
    Enum.reverse(acc) |> List.flatten()
  end

  defp encode_blocks(data, offset, acc) do
    blk_end = min(offset + @max_block_size, byte_size(data))
    blk_size = blk_end - offset
    block_bin = binary_part(data, offset, blk_size)
    is_last = blk_end == byte_size(data)
    last_bit = if is_last, do: 1, else: 0

    block_bytes = encode_one_block(block_bin, blk_size, last_bit)
    encode_blocks(data, blk_end, [block_bytes | acc])
  end

  defp encode_one_block(block_bin, blk_size, last_bit) do
    # Try RLE first: if all bytes are identical, use a 4-byte RLE block.
    block_list = :binary.bin_to_list(block_bin)
    first_byte = hd(block_list)

    if Enum.all?(block_list, fn b -> b == first_byte end) do
      # RLE block header: Type=01, Size=blk_size
      hdr = (blk_size <<< 3) ||| (0b01 <<< 1) ||| last_bit
      [hdr &&& 0xFF, (hdr >>> 8) &&& 0xFF, (hdr >>> 16) &&& 0xFF, first_byte]
    else
      # Try compressed block.
      case compress_block(block_bin) do
        {:ok, compressed_bytes} ->
          comp_size = length(compressed_bytes)
          hdr = (comp_size <<< 3) ||| (0b10 <<< 1) ||| last_bit
          [hdr &&& 0xFF, (hdr >>> 8) &&& 0xFF, (hdr >>> 16) &&& 0xFF | compressed_bytes]

        :fallback ->
          # Raw block fallback.
          hdr = (blk_size <<< 3) ||| (0b00 <<< 1) ||| last_bit
          [hdr &&& 0xFF, (hdr >>> 8) &&& 0xFF, (hdr >>> 16) &&& 0xFF | block_list]
      end
    end
  end

  @doc """
  Decompress a ZStd frame, returning `{:ok, data}` or `{:error, reason}`.

  Accepts any valid ZStd frame with:
  - Single-segment or multi-segment layout
  - Raw, RLE, or Compressed blocks
  - Predefined FSE modes (no per-frame table description)

  ## Parameters

  - `data` — a ZStd-compressed binary.

  ## Errors

  Returns `{:error, reason}` if the input is truncated, has a bad magic
  number, or contains unsupported features (non-predefined FSE tables,
  Huffman literals, reserved block types).

  ## Examples

      iex> original = "hello, world!"
      iex> {:ok, decoded} = CodingAdventures.Zstd.decompress(CodingAdventures.Zstd.compress(original))
      iex> decoded == original
      true

      iex> CodingAdventures.Zstd.decompress("not a zstd frame")
      {:error, "bad magic: 0x20746F6E (expected 0xFD2FB528)"}
  """
  @spec decompress(binary()) :: {:ok, binary()} | {:error, String.t()}
  def decompress(data) when is_binary(data) do
    if byte_size(data) < 5 do
      {:error, "frame too short"}
    else
      # ── Validate magic ─────────────────────────────────────────────────────
      <<b0, b1, b2, b3, _rest::binary>> = data
      magic_val = b0 ||| (b1 <<< 8) ||| (b2 <<< 16) ||| (b3 <<< 24)

      if magic_val != @magic do
        {:error, "bad magic: #{format_hex(magic_val)} (expected #{format_hex(@magic)})"}
      else
        decompress_frame(data, 4)
      end
    end
  end

  defp format_hex(n), do: "0x" <> String.upcase(Integer.to_string(n, 16) |> String.pad_leading(8, "0"))

  defp decompress_frame(data, pos) do
    # ── Parse Frame Header Descriptor ────────────────────────────────────────
    fhd = :binary.at(data, pos)
    pos = pos + 1

    # FCS_Field_Size: bits [7:6] of FHD.
    #   00 → 0 bytes if Single_Segment=0, else 1 byte
    #   01 → 2 bytes (value + 256)
    #   10 → 4 bytes
    #   11 → 8 bytes
    fcs_flag = (fhd >>> 6) &&& 3

    # Single_Segment_Flag: bit 5.
    single_seg = (fhd >>> 5) &&& 1

    # Dict_ID_Flag: bits [1:0].
    dict_flag = fhd &&& 3

    # ── Window Descriptor ─────────────────────────────────────────────────────
    # Present only if Single_Segment_Flag = 0.
    pos = if single_seg == 0, do: pos + 1, else: pos

    # ── Dict ID ───────────────────────────────────────────────────────────────
    dict_id_bytes = [0, 1, 2, 4] |> Enum.at(dict_flag)
    pos = pos + dict_id_bytes

    # ── Frame Content Size ─────────────────────────────────────────────────────
    fcs_bytes =
      case fcs_flag do
        0 -> if single_seg == 1, do: 1, else: 0
        1 -> 2
        2 -> 4
        3 -> 8
      end
    pos = pos + fcs_bytes

    # ── Blocks ─────────────────────────────────────────────────────────────────
    # Use a binary accumulator for O(1) back-reference access and efficient append.
    decompress_blocks(data, pos, <<>>)
  end

  defp decompress_blocks(data, pos, acc) do
    if pos + 3 > byte_size(data) do
      {:error, "truncated block header"}
    else
      b0 = :binary.at(data, pos)
      b1 = :binary.at(data, pos + 1)
      b2 = :binary.at(data, pos + 2)
      hdr = b0 ||| (b1 <<< 8) ||| (b2 <<< 16)
      pos = pos + 3

      is_last = (hdr &&& 1) != 0
      btype = (hdr >>> 1) &&& 3
      bsize = hdr >>> 3

      case btype do
        0 ->
          # Raw block: verbatim content.
          if pos + bsize > byte_size(data) do
            {:error, "raw block truncated: need #{bsize} bytes at pos #{pos}"}
          else
            chunk = binary_part(data, pos, bsize)
            new_acc = acc <> chunk
            if byte_size(new_acc) > @max_output do
              {:error, "decompressed size exceeds limit of #{@max_output} bytes"}
            else
              if is_last, do: {:ok, new_acc},
                          else: decompress_blocks(data, pos + bsize, new_acc)
            end
          end

        1 ->
          # RLE block: 1 byte repeated bsize times.
          if pos >= byte_size(data) do
            {:error, "RLE block missing byte"}
          else
            byte_val = :binary.at(data, pos)
            rle_chunk = :binary.copy(<<byte_val>>, bsize)
            new_acc = acc <> rle_chunk
            if byte_size(new_acc) > @max_output do
              {:error, "decompressed size exceeds limit of #{@max_output} bytes"}
            else
              if is_last, do: {:ok, new_acc},
                          else: decompress_blocks(data, pos + 1, new_acc)
            end
          end

        2 ->
          # Compressed block.
          if pos + bsize > byte_size(data) do
            {:error, "compressed block truncated: need #{bsize} bytes"}
          else
            block_data = binary_part(data, pos, bsize)
            case decompress_block(block_data, acc) do
              {:error, reason} -> {:error, reason}
              {:ok, new_acc} ->
                if byte_size(new_acc) > @max_output do
                  {:error, "decompressed size exceeds limit of #{@max_output} bytes"}
                else
                  if is_last, do: {:ok, new_acc},
                              else: decompress_blocks(data, pos + bsize, new_acc)
                end
            end
          end

        3 ->
          {:error, "reserved block type 3"}

        _ ->
          {:error, "unknown block type #{btype}"}
      end
    end
  end
end
