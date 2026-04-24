// Zstandard (ZStd) lossless compression algorithm — CMP07.
//
// Zstandard (RFC 8878) is a high-ratio, fast compression format created by
// Yann Collet at Facebook (2015). It combines:
//
//   - LZ77 back-references (via LZSS token generation) to exploit repetition
//     in the data — the same "copy from earlier in the output" trick as
//     DEFLATE, but with a larger window.
//
//   - FSE (Finite State Entropy) coding instead of Huffman for the sequence
//     descriptor symbols. FSE is an asymmetric numeral system that approaches
//     the Shannon entropy limit in a single pass.
//
//   - Predefined decode tables (RFC 8878 Appendix B) so short frames need no
//     table description overhead.
//
// Frame layout (RFC 8878 §3):
//
//   ┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
//   │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
//   │ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
//   └────────┴─────┴──────────────────────┴────────┴──────────────────┘
//
// Each block has a 3-byte header:
//   bit 0        = Last_Block flag
//   bits [2:1]   = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
//   bits [23:3]  = Block_Size
//
// Compression strategy (this implementation):
//   1. Split data into 128 KB blocks (MaxBlockSize).
//   2. For each block, try:
//      a. RLE — all bytes identical → 4 bytes total.
//      b. Compressed (LZ77 + FSE) — if output < input length.
//      c. Raw — verbatim copy as fallback.
//
// Series:
//   CMP00 (LZ77)    — Sliding-window back-references
//   CMP01 (LZ78)    — Explicit dictionary (trie)
//   CMP02 (LZSS)    — LZ77 + flag bits
//   CMP03 (LZW)     — LZ78 + pre-initialised alphabet; GIF
//   CMP04 (Huffman) — Entropy coding
//   CMP05 (DEFLATE) — LZ77 + Huffman; ZIP/gzip/PNG/zlib
//   CMP06 (Brotli)  — DEFLATE + context modelling + static dict
//   CMP07 (ZStd)    — LZ77 + FSE; high ratio + speed  ← this package

using System.Buffers.Binary;
using System.Runtime.CompilerServices;
using CodingAdventures.Lzss;

[assembly: InternalsVisibleTo("CodingAdventures.Zstd.Tests")]

namespace CodingAdventures.Zstd;

// ─── Constants ─────────────────────────────────────────────────────────────────

/// <summary>
/// Pure-C# ZStd compression and decompression (RFC 8878 / CMP07).
/// </summary>
public static class Zstd
{
    // ZStd magic number: 0xFD2FB528 (little-endian bytes: 28 B5 2F FD).
    // Every valid ZStd frame starts with these 4 bytes.
    private const uint Magic = 0xFD2FB528u;

    // Maximum block size: 128 KB.
    // ZStd allows blocks up to 128 KB. Larger inputs are split across multiple blocks.
    private const int MaxBlockSize = 128 * 1024;

    // ─── LL / ML / OF code tables (RFC 8878 §3.1.1.3) ─────────────────────────
    //
    // These tables map a *code number* to a (baseline, extra_bits) pair.
    //
    // For example, LL code 17 means literal_length = 18 + read(1 extra bit),
    // so it covers literal lengths 18 and 19.
    //
    // The FSE state machine tracks one code number per field; extra bits are
    // read directly from the bitstream after state transitions.

    // Literal Length code table: (baseline, extra_bits) for codes 0..=35.
    // Literal length 0..15 each have their own code (0 extra bits).
    // Larger lengths are grouped with increasing ranges.
    private static readonly (uint Base, byte Bits)[] LlCodes =
    [
        // code: value = baseline + read(extra_bits)
        (0, 0),  (1, 0),  (2, 0),  (3, 0),  (4, 0),  (5, 0),
        (6, 0),  (7, 0),  (8, 0),  (9, 0),  (10, 0), (11, 0),
        (12, 0), (13, 0), (14, 0), (15, 0),
        // Grouped ranges start at code 16
        (16, 1), (18, 1), (20, 1), (22, 1),
        (24, 2), (28, 2),
        (32, 3), (40, 3),
        (48, 4), (64, 6),
        (128, 7), (256, 8), (512, 9), (1024, 10), (2048, 11), (4096, 12),
        (8192, 13), (16384, 14), (32768, 15), (65536, 16),
    ];

    // Match Length code table: (baseline, extra_bits) for codes 0..=52.
    // Minimum match length in ZStd is 3 (not 0). Code 0 = match length 3.
    private static readonly (uint Base, byte Bits)[] MlCodes =
    [
        // codes 0..31: individual values 3..34
        (3, 0),  (4, 0),  (5, 0),  (6, 0),  (7, 0),  (8, 0),
        (9, 0),  (10, 0), (11, 0), (12, 0), (13, 0), (14, 0),
        (15, 0), (16, 0), (17, 0), (18, 0), (19, 0), (20, 0),
        (21, 0), (22, 0), (23, 0), (24, 0), (25, 0), (26, 0),
        (27, 0), (28, 0), (29, 0), (30, 0), (31, 0), (32, 0),
        (33, 0), (34, 0),
        // codes 32+: grouped ranges
        (35, 1), (37, 1),  (39, 1),  (41, 1),
        (43, 2), (47, 2),
        (51, 3), (59, 3),
        (67, 4), (83, 4),
        (99, 5), (131, 7),
        (259, 8), (515, 9), (1027, 10), (2051, 11),
        (4099, 12), (8195, 13), (16387, 14), (32771, 15), (65539, 16),
    ];

    // ─── FSE predefined distributions (RFC 8878 Appendix B) ───────────────────
    //
    // "Predefined_Mode" means no per-frame table description is transmitted.
    // The decoder builds the same table from these fixed distributions.
    //
    // Entries of -1 mean "probability 1/table_size" — these symbols get one slot
    // in the decode table and their encoder state never needs extra bits.

    // Predefined normalised distribution for Literal Length FSE.
    // Table accuracy log = 6 → 64 slots.
    private static readonly short[] LlNorm =
    [
         4,  3,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  1,  1,  1,
         2,  2,  2,  2,  2,  2,  2,  2,  2,  3,  2,  1,  1,  1,  1,  1,
        -1, -1, -1, -1,
    ];
    private const int LlAccLog = 6; // table_size = 64

    // Predefined normalised distribution for Match Length FSE.
    // Table accuracy log = 6 → 64 slots.
    private static readonly short[] MlNorm =
    [
         1,  4,  3,  2,  2,  2,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
         1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,
         1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1, -1, -1,
        -1, -1, -1, -1, -1,
    ];
    private const int MlAccLog = 6;

    // Predefined normalised distribution for Offset FSE.
    // Table accuracy log = 5 → 32 slots.
    private static readonly short[] OfNorm =
    [
         1,  1,  1,  1,  1,  1,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
         1,  1,  1,  1,  1,  1,  1,  1, -1, -1, -1, -1, -1,
    ];
    private const int OfAccLog = 5; // table_size = 32

    // ─── FSE decode/encode table entries ───────────────────────────────────────

    // One cell in the FSE decode table.
    //
    // To decode a symbol from state S:
    //   1. sym is the output symbol.
    //   2. Read nb bits from the bitstream as bits.
    //   3. New state = base + bits.
    internal struct FseDe
    {
        public byte Sym;   // decoded symbol
        public byte Nb;    // number of extra bits to read for next state
        public ushort Base; // base value for next state computation
    }

    // Encode transform for one symbol.
    //
    // Given encoder state S for symbol s:
    //   nb_out = (S + delta_nb) >> 16   (number of bits to emit)
    //   emit low nb_out bits of S
    //   new_S  = state_tbl[(S >> nb_out) + delta_fs]
    internal struct FseEe
    {
        // (max_bits_out << 16) - (count << max_bits_out)
        // Used to derive nb_out: nb_out = (state + delta_nb) >> 16
        public uint DeltaNb;
        // cumulative_count_before_sym - count (may be negative, hence int)
        // Used to index state_tbl: new_S = state_tbl[(S >> nb_out) + delta_fs]
        public int DeltaFs;
    }

    // ─── FSE table construction ─────────────────────────────────────────────────

    // Build an FSE decode table from a normalised probability distribution.
    //
    // The algorithm:
    //   1. Place symbols with probability -1 (very rare) at the top of the table.
    //   2. Spread remaining symbols using a deterministic step function derived
    //      from the table size. This ensures each symbol occupies the correct
    //      fraction of slots.
    //   3. Assign nb (number of state bits to read) and base to each slot so
    //      that the decoder can reconstruct the next state.
    //
    // The step function step = (sz >> 1) + (sz >> 3) + 3 is co-prime to sz when
    // sz is a power of two (which it always is in ZStd), ensuring that the walk
    // visits every slot exactly once.
    internal static FseDe[] BuildDecodeTable(short[] norm, int accLog)
    {
        int sz = 1 << accLog;
        int step = (sz >> 1) + (sz >> 3) + 3;
        var tbl = new FseDe[sz];
        var symNext = new ushort[norm.Length];

        // Phase 1: symbols with probability -1 go at the top (high indices).
        // These symbols each get exactly 1 slot, and their state transition uses
        // the full acc_log bits (they can go to any state).
        int high = sz - 1;
        for (int s = 0; s < norm.Length; s++)
        {
            if (norm[s] == -1)
            {
                tbl[high].Sym = (byte)s;
                if (high > 0) high--;
                symNext[s] = 1;
            }
        }

        // Phase 2: spread remaining symbols into the lower portion of the table.
        // Two-pass approach: first symbols with count > 1, then count == 1.
        // This matches the reference implementation's deterministic ordering.
        int pos = 0;
        for (int pass = 0; pass < 2; pass++)
        {
            for (int s = 0; s < norm.Length; s++)
            {
                if (norm[s] <= 0) continue;
                int cnt = norm[s];
                if ((pass == 0) != (cnt > 1)) continue;
                symNext[s] = (ushort)cnt;
                for (int k = 0; k < cnt; k++)
                {
                    tbl[pos].Sym = (byte)s;
                    pos = (pos + step) & (sz - 1);
                    while (pos > high)
                        pos = (pos + step) & (sz - 1);
                }
            }
        }

        // Phase 3: assign nb (number of state bits to read) and base.
        //
        // For a symbol with count cnt occupying slots i₀, i₁, ...:
        //   The next_state counter starts at cnt and increments.
        //   nb = acc_log - floor(log2(next_state))
        //   base = next_state * (1 << nb) - sz
        //
        // This ensures that when we reconstruct state = base + read(nb bits),
        // we land in the range [sz, 2*sz), which is the valid encoder state range.
        var sn = (ushort[])symNext.Clone();
        for (int i = 0; i < sz; i++)
        {
            int s = tbl[i].Sym;
            uint ns = sn[s];
            sn[s]++;
            // floor(log2(ns)) = 31 - LeadingZeros(ns)
            int nb = accLog - (31 - LeadingZeros(ns));
            // base = ns * (1 << nb) - sz
            ushort baseVal = (ushort)(unchecked((int)(ns << nb) - sz));
            tbl[i].Nb = (byte)nb;
            tbl[i].Base = baseVal;
        }

        return tbl;
    }

    // Count leading zeros of a uint (31 - floor(log2(v)) for v > 0).
    private static int LeadingZeros(uint v)
    {
        if (v == 0) return 32;
        int n = 0;
        if ((v & 0xFFFF0000u) == 0) { n += 16; v <<= 16; }
        if ((v & 0xFF000000u) == 0) { n += 8;  v <<= 8;  }
        if ((v & 0xF0000000u) == 0) { n += 4;  v <<= 4;  }
        if ((v & 0xC0000000u) == 0) { n += 2;  v <<= 2;  }
        if ((v & 0x80000000u) == 0) { n += 1; }
        return n;
    }

    // Build FSE encode tables from a normalised distribution.
    //
    // Returns:
    //   ee[sym]: the FseEe transform for each symbol.
    //   st[slot]: the encoder state table (slot → output state in [sz, 2*sz)).
    //
    // The encode/decode symmetry:
    //   The FSE decoder assigns (sym, nb, base) to each table cell in INDEX ORDER.
    //   For symbol s, the j-th cell (in ascending index order) has:
    //     ns = count[s] + j
    //     nb = acc_log - floor(log2(ns))
    //     base = ns * (1<<nb) - sz
    //
    //   The FSE encoder must use the SAME indexing: slot cumul[s]+j maps to the
    //   j-th table cell for symbol s (in ascending index order).
    internal static (FseEe[] Ee, ushort[] St) BuildEncodeTable(short[] norm, int accLog)
    {
        int sz = 1 << accLog;

        // Step 1: compute cumulative sums.
        var cumul = new uint[norm.Length];
        uint total = 0;
        for (int s = 0; s < norm.Length; s++)
        {
            cumul[s] = total;
            uint cnt = norm[s] == -1 ? 1u : (norm[s] <= 0 ? 0u : (uint)norm[s]);
            total += cnt;
        }

        // Step 2: build the spread table (which symbol occupies each table slot).
        //
        // This uses the same spreading algorithm as BuildDecodeTable, producing
        // a mapping from table index to symbol.
        int step = (sz >> 1) + (sz >> 3) + 3;
        var spread = new byte[sz];
        int idxHigh = sz - 1;

        // Phase 1: probability -1 symbols at the high end
        for (int s = 0; s < norm.Length; s++)
        {
            if (norm[s] == -1)
            {
                spread[idxHigh] = (byte)s;
                if (idxHigh > 0) idxHigh--;
            }
        }
        int idxLimit = idxHigh;

        // Phase 2: spread remaining symbols using the step function
        int pos2 = 0;
        for (int pass = 0; pass < 2; pass++)
        {
            for (int s = 0; s < norm.Length; s++)
            {
                if (norm[s] <= 0) continue;
                int cnt = norm[s];
                if ((pass == 0) != (cnt > 1)) continue;
                for (int k = 0; k < cnt; k++)
                {
                    spread[pos2] = (byte)s;
                    pos2 = (pos2 + step) & (sz - 1);
                    while (pos2 > idxLimit)
                        pos2 = (pos2 + step) & (sz - 1);
                }
            }
        }

        // Step 3: build the state table by iterating spread in INDEX ORDER.
        //
        // For each table index i (in ascending order), determine which
        // occurrence of symbol s = spread[i] this is (j = 0, 1, 2, ...).
        // The encode slot is cumul[s] + j, and the encoder output state is
        // i + sz (so the decoder, in state i, will decode symbol s).
        var symOcc = new uint[norm.Length];
        var st = new ushort[sz];

        for (int i = 0; i < sz; i++)
        {
            int s = spread[i];
            int j = (int)symOcc[s];
            symOcc[s]++;
            int slot = (int)cumul[s] + j;
            st[slot] = (ushort)(i + sz);
        }

        // Step 4: build FseEe entries.
        //
        // For symbol s with count c and max_bits_out mbo:
        //   delta_nb = (mbo << 16) - (c << mbo)
        //   delta_fs = cumul[s] - c
        //
        // Encode step: given current encoder state E ∈ [sz, 2*sz):
        //   nb = (E + delta_nb) >> 16     (number of state bits to emit)
        //   emit low nb bits of E
        //   new_E = st[(E >> nb) + delta_fs]
        var ee = new FseEe[norm.Length];
        for (int s = 0; s < norm.Length; s++)
        {
            uint cnt = norm[s] == -1 ? 1u : (norm[s] <= 0 ? 0u : (uint)norm[s]);
            if (cnt == 0) continue;
            uint mbo;
            if (cnt == 1)
            {
                mbo = (uint)accLog;
            }
            else
            {
                // max_bits_out = acc_log - floor(log2(cnt))
                mbo = (uint)accLog - (uint)(31 - LeadingZeros(cnt));
            }
            ee[s].DeltaNb = unchecked((mbo << 16) - (cnt << (int)mbo));
            ee[s].DeltaFs = (int)cumul[s] - (int)cnt;
        }

        return (ee, st);
    }

    // ─── Reverse bit-writer ─────────────────────────────────────────────────────
    //
    // ZStd's sequence bitstream is written *backwards* relative to the data flow:
    // the encoder writes bits that the decoder will read last, first. This allows
    // the decoder to read a forward-only stream while decoding sequences in order.
    //
    // Byte layout: [byte0, byte1, ..., byteN] where byteN is the last byte
    // written, and it contains a sentinel bit (the highest set bit) that marks
    // the end of meaningful data. The decoder initialises by finding this sentinel.
    //
    // Bit layout within each byte: LSB = first bit written.
    //
    // Example: write bits 1, 0, 1, 1 (4 bits) then flush:
    //   reg = 0b1011, bits = 4
    //   flush: sentinel at bit 4 → last byte = 0b0001_1011 = 0x1B
    //   buf = [0x1B]
    //
    // The decoder reads this as: find MSB (bit 4 = sentinel), then read
    // bits 3..0 = 0b1011 = the original 4 bits.

    internal sealed class RevBitWriter
    {
        private readonly List<byte> _buf = new();
        private ulong _reg;  // accumulation register (bits fill from LSB)
        private int _bits;   // number of valid bits in reg

        // Add nb low-order bits of val to the stream.
        public void AddBits(ulong val, int nb)
        {
            if (nb == 0) return;
            ulong mask = nb == 64 ? ulong.MaxValue : (1UL << nb) - 1;
            _reg |= (val & mask) << _bits;
            _bits += nb;
            while (_bits >= 8)
            {
                _buf.Add((byte)(_reg & 0xFF));
                _reg >>= 8;
                _bits -= 8;
            }
        }

        // Flush remaining bits with a sentinel and mark the stream end.
        //
        // The sentinel is a 1 bit placed at position _bits in the last byte.
        // The decoder locates it with leading_zeros arithmetic.
        public void Flush()
        {
            byte sentinel = (byte)(1 << _bits); // bit above all remaining data bits
            byte lastByte = (byte)((_reg & 0xFF) | sentinel);
            _buf.Add(lastByte);
            _reg = 0;
            _bits = 0;
        }

        public byte[] Finish() => [.. _buf];
    }

    // ─── Reverse bit-reader ─────────────────────────────────────────────────────
    //
    // Mirrors RevBitWriter: reads bits from the END of the buffer going backwards.
    // The stream is laid out so that the LAST bits written by the encoder are at
    // the END of the byte buffer (in the sentinel-containing last byte). The
    // reader initialises at the last byte and reads backward toward byte 0.
    //
    // Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side).
    // read_bits(n) extracts the top n bits and shifts the register left by n.
    //
    // Why left-aligned? The writer accumulates bits LSB-first. Within each flushed
    // byte, bit 0 = earliest written, bit N = latest written. To read the LATEST
    // bits first (which were in the highest byte positions and in the high bits of
    // each byte), we need a left-aligned register so that reading from the top
    // gives the highest-position bits first.

    internal sealed class RevBitReader
    {
        private readonly byte[] _data;
        private ulong _reg;   // shift register, valid bits packed at the TOP (MSB side)
        private int _bits;    // how many valid bits are loaded (count from MSB)
        private int _pos;     // index of the next byte to load (decrements toward 0)

        public RevBitReader(byte[] data)
        {
            if (data.Length == 0)
                throw new InvalidDataException("empty bitstream");

            // Find the sentinel bit in the last byte.
            // The sentinel is the highest set bit; valid data bits are below it.
            byte last = data[^1];
            if (last == 0)
                throw new InvalidDataException("bitstream last byte is zero (no sentinel)");

            // sentinel_pos = bit index (0 = LSB) of the sentinel in the last byte
            // = 7 - leading_zeros_in_byte(last)
            int sentinelPos = 7 - LeadingZeros((uint)last) + 24; // adjust for 32-bit calc
            // LeadingZeros works on uint; for a byte: leading zeros in [31..8] bits + byte bits
            // More directly:
            sentinelPos = 0;
            for (int b = 7; b >= 0; b--)
            {
                if ((last & (1 << b)) != 0) { sentinelPos = b; break; }
            }
            int validBits = sentinelPos; // number of data bits below the sentinel

            // Place the valid bits of the sentinel byte at the TOP of the register.
            ulong mask = validBits == 0 ? 0UL : (1UL << validBits) - 1;
            ulong reg = validBits == 0 ? 0UL : ((ulong)last & mask) << (64 - validBits);

            _data = data;
            _reg = reg;
            _bits = validBits;
            _pos = data.Length - 1; // sentinel byte already consumed; load from here-1

            // Fill the register from earlier bytes.
            Reload();
        }

        // Load more bytes into the register from the stream going backward.
        //
        // Each new byte is placed just BELOW the currently loaded bits (in the
        // left-aligned register, that means at position 64 - bits - 8).
        private void Reload()
        {
            while (_bits <= 56 && _pos > 0)
            {
                _pos--;
                int shift = 64 - _bits - 8;
                _reg |= (ulong)_data[_pos] << shift;
                _bits += 8;
            }
        }

        // Read nb bits from the top of the register (returns 0 if nb == 0).
        //
        // This returns the most recently written bits first (highest stream
        // positions first), mirroring the encoder's backward order.
        public ulong ReadBits(int nb)
        {
            if (nb == 0) return 0;
            ulong val = _reg >> (64 - nb);
            _reg = nb == 64 ? 0UL : _reg << nb;
            _bits = Math.Max(0, _bits - nb);
            if (_bits < 24)
                Reload();
            return val;
        }
    }

    // ─── FSE encode/decode helpers ──────────────────────────────────────────────

    // Encode one symbol into the backward bitstream, updating the FSE state.
    //
    // The encoder maintains state in [sz, 2*sz). To emit symbol sym:
    //   1. Compute how many bits to flush: nb = (state + delta_nb) >> 16
    //   2. Write the low nb bits of state to the bitstream.
    //   3. New state = st[(state >> nb) + delta_fs]
    //
    // After all symbols are encoded, the final state (minus sz) is written as
    // acc_log bits to allow the decoder to initialise.
    private static void FseEncodeSym(
        ref uint state,
        int sym,
        FseEe[] ee,
        ushort[] st,
        RevBitWriter bw)
    {
        ref FseEe e = ref ee[sym];
        int nb = (int)((state + e.DeltaNb) >> 16);
        bw.AddBits(state, nb);
        int slotI = (int)(state >> nb) + e.DeltaFs;
        int slot = Math.Max(0, slotI);
        state = st[slot];
    }

    // Decode one symbol from the backward bitstream, updating the FSE state.
    //
    //   1. Look up de[state] to get sym, nb, and base.
    //   2. New state = base + read(nb bits).
    private static byte FseDecodeSym(ref ushort state, FseDe[] de, RevBitReader br)
    {
        var e = de[state];
        byte sym = e.Sym;
        state = (ushort)(e.Base + br.ReadBits(e.Nb));
        return sym;
    }

    // ─── LL/ML/OF code number computation ──────────────────────────────────────

    // Map a literal length value to its LL code number (0..35).
    // Codes 0..15 are identity; codes 16+ cover ranges via lookup.
    internal static int LlToCode(uint ll)
    {
        // Simple linear scan over LlCodes table.
        // Codes are in increasing baseline order, so the last code whose
        // baseline ≤ ll is the correct code.
        int code = 0;
        for (int i = 0; i < LlCodes.Length; i++)
        {
            if (LlCodes[i].Base <= ll)
                code = i;
            else
                break;
        }
        return code;
    }

    // Map a match length value to its ML code number (0..52).
    internal static int MlToCode(uint ml)
    {
        int code = 0;
        for (int i = 0; i < MlCodes.Length; i++)
        {
            if (MlCodes[i].Base <= ml)
                code = i;
            else
                break;
        }
        return code;
    }

    // ─── Sequence struct ────────────────────────────────────────────────────────

    // One ZStd sequence: (literal_length, match_length, match_offset).
    //
    // A sequence means: emit ll literal bytes from the literals section,
    // then copy ml bytes starting off positions back in the output buffer.
    // After all sequences, any remaining literals are appended.
    private readonly struct Seq(uint ll, uint ml, uint off)
    {
        public readonly uint Ll = ll;  // literal length
        public readonly uint Ml = ml;  // match length
        public readonly uint Off = off; // match offset (1-indexed)
    }

    // Convert LZSS tokens into ZStd sequences + a flat literals buffer.
    //
    // LZSS produces a stream of Literal(byte) and Match{offset, length}.
    // ZStd groups consecutive literals before each match into a single sequence.
    // Any trailing literals (after the last match) go into the literals buffer
    // without a corresponding sequence entry.
    private static (byte[] Lits, List<Seq> Seqs) TokensToSeqs(List<LzssToken> tokens)
    {
        var lits = new List<byte>();
        var seqs = new List<Seq>();
        uint litRun = 0;

        foreach (var tok in tokens)
        {
            if (tok is LzssLiteral lit)
            {
                lits.Add(lit.Byte);
                litRun++;
            }
            else if (tok is LzssMatch match)
            {
                seqs.Add(new Seq(litRun, (uint)match.Length, (uint)match.Offset));
                litRun = 0;
            }
        }
        // Trailing literals stay in lits; no sequence for them.
        return ([.. lits], seqs);
    }

    // ─── Literals section encoding ──────────────────────────────────────────────
    //
    // ZStd literals can be Huffman-coded or raw. We use Raw_Literals (type=0),
    // which is the simplest: no Huffman table, bytes are stored verbatim.
    //
    // Header format depends on literal count:
    //   ≤ 31 bytes:   1-byte header  = (lit_len << 3) | 0b000
    //   ≤ 4095 bytes: 2-byte header  = (lit_len << 4) | 0b0100
    //   else:         3-byte header  = (lit_len << 4) | 0b1100
    //
    // The bottom 2 bits = Literals_Block_Type (0 = Raw).
    // The next 2 bits = Size_Format.

    private static byte[] EncodeLiteralsSection(byte[] lits)
    {
        int n = lits.Length;
        var out2 = new List<byte>(n + 3);

        // Raw_Literals header format (RFC 8878 §3.1.1.2.1):
        // bits [1:0] = Literals_Block_Type = 00 (Raw)
        // bits [3:2] = Size_Format: 00 or 10 = 1-byte, 01 = 2-byte, 11 = 3-byte
        //
        // 1-byte:  size in bits [7:3] (5 bits) — header = (size << 3) | 0b000
        // 2-byte:  size in bits [11:4] (12 bits) — header = (size << 4) | 0b0100
        // 3-byte:  size in bits [19:4] (16 bits) — header = (size << 4) | 0b1100
        if (n <= 31)
        {
            // 1-byte header: size_format=00, type=00
            out2.Add((byte)((n << 3) & 0xFF));
        }
        else if (n <= 4095)
        {
            // 2-byte header: size_format=01, type=00 → 0b0100
            uint hdr = ((uint)n << 4) | 0b0100u;
            out2.Add((byte)(hdr & 0xFF));
            out2.Add((byte)((hdr >> 8) & 0xFF));
        }
        else
        {
            // 3-byte header: size_format=11, type=00 → 0b1100
            uint hdr = ((uint)n << 4) | 0b1100u;
            out2.Add((byte)(hdr & 0xFF));
            out2.Add((byte)((hdr >> 8) & 0xFF));
            out2.Add((byte)((hdr >> 16) & 0xFF));
        }

        out2.AddRange(lits);
        return [.. out2];
    }

    // Decode literals section, returning (literals, bytes_consumed).
    private static (byte[] Lits, int Consumed) DecodeLiteralsSection(byte[] data, int start)
    {
        if (start >= data.Length)
            throw new InvalidDataException("empty literals section");

        byte b0 = data[start];
        int ltype = b0 & 0b11; // bottom 2 bits = Literals_Block_Type

        if (ltype != 0)
            throw new InvalidDataException(
                $"unsupported literals type {ltype} (only Raw=0 supported)");

        // Decode size_format from bits [3:2] of b0
        int sizeFormat = (b0 >> 2) & 0b11;

        // Decode the literal length and header byte count from size_format.
        //
        // Raw_Literals size_format encoding (RFC 8878 §3.1.1.2.1):
        //   0b00 or 0b10 → 1-byte header: size = b0[7:3] (5 bits, values 0..31)
        //   0b01          → 2-byte LE header: size in bits [11:4] (12 bits, values 0..4095)
        //   0b11          → 3-byte LE header: size in bits [19:4] (20 bits, values 0..1MB)
        int n, headerBytes;
        switch (sizeFormat)
        {
            case 0:
            case 2:
                // 1-byte header: size in bits [7:3] (5 bits = values 0..31)
                n = b0 >> 3;
                headerBytes = 1;
                break;
            case 1:
                // 2-byte header: 12-bit size
                if (start + 2 > data.Length)
                    throw new InvalidDataException("truncated literals header (2-byte)");
                n = ((b0 >> 4) & 0xF) | (data[start + 1] << 4);
                headerBytes = 2;
                break;
            case 3:
                // 3-byte header: 20-bit size
                if (start + 3 > data.Length)
                    throw new InvalidDataException("truncated literals header (3-byte)");
                n = ((b0 >> 4) & 0xF) | (data[start + 1] << 4) | (data[start + 2] << 12);
                headerBytes = 3;
                break;
            default:
                throw new InvalidDataException("impossible size_format");
        }

        int dataStart = start + headerBytes;
        int dataEnd = dataStart + n;
        if (dataEnd > data.Length)
            throw new InvalidDataException(
                $"literals data truncated: need {dataEnd}, have {data.Length}");

        return (data[dataStart..dataEnd], dataEnd - start);
    }

    // ─── Sequences section encoding ─────────────────────────────────────────────
    //
    // Layout:
    //   [sequence_count: 1-3 bytes]
    //   [symbol_compression_modes: 1 byte]  (0x00 = all Predefined)
    //   [FSE bitstream: variable]
    //
    // Symbol compression modes byte:
    //   bits [7:6] = LL mode
    //   bits [5:4] = OF mode
    //   bits [3:2] = ML mode
    //   bits [1:0] = reserved (0)
    // Mode 0 = Predefined, Mode 1 = RLE, Mode 2 = FSE_Compressed, Mode 3 = Repeat.
    // We always write 0x00 (all Predefined).
    //
    // The FSE bitstream is a backward bit-stream (reverse bit writer):
    //   - Sequences are encoded in REVERSE ORDER (last first).
    //   - For each sequence:
    //       OF extra bits, ML extra bits, LL extra bits  (in this order)
    //       then FSE symbol for ML, OF, LL              (reversed decode order)
    //   - After all sequences, flush the final FSE states:
    //       (state_of - sz_of) as OF_ACC_LOG bits
    //       (state_ml - sz_ml) as ML_ACC_LOG bits
    //       (state_ll - sz_ll) as LL_ACC_LOG bits
    //   - Add sentinel and flush.
    //
    // The decoder does the mirror:
    //   1. Read LL_ACC_LOG bits → initial state_ll
    //   2. Read ML_ACC_LOG bits → initial state_ml
    //   3. Read OF_ACC_LOG bits → initial state_of
    //   4. For each sequence:
    //       decode LL symbol (state transition)
    //       decode OF symbol
    //       decode ML symbol
    //       read LL extra bits
    //       read ML extra bits
    //       read OF extra bits
    //   5. Apply sequence to output buffer.

    // Encode/decode the Number_of_Sequences field per RFC 8878 §3.1.1.1.2:
    //
    //   0            : 1 byte  = 0x00
    //   1..127       : 1 byte  = count
    //   128..32767   : 2 bytes ; byte0 = 0x80 | ((count - 128) >> 8)
    //                           byte1 = (count - 128) & 0xFF
    //   32768..131071: 3 bytes ; byte0 = 0xFF
    //                           byte1 = (count - 0x7F00) & 0xFF
    //                           byte2 = (count - 0x7F00) >> 8
    //
    // Note: the range 128..32767 encodes as byte0 in [0x80..0xFF), which lets
    // the decoder branch on byte0 to distinguish all three forms cleanly.

    private static byte[] EncodeSeqCount(int count)
    {
        if (count < 128)
            return [(byte)count]; // covers count=0 too
        if (count < 0x8000) // 128..32767
        {
            int delta = count - 128;
            byte b0 = (byte)(0x80 | (delta >> 8));
            byte b1 = (byte)(delta & 0xFF);
            return [b0, b1];
        }
        // 32768..131071: 3-byte encoding
        int r = count - 0x7F00;
        return [0xFF, (byte)(r & 0xFF), (byte)((r >> 8) & 0xFF)];
    }

    private static (int Count, int Consumed) DecodeSeqCount(byte[] data, int pos)
    {
        if (pos >= data.Length)
            throw new InvalidDataException("empty sequence count");
        byte b0 = data[pos];
        if (b0 < 128)
            return (b0, 1);
        if (b0 < 0xFF)
        {
            // 2-byte: delta = ((b0 & 0x7F) << 8) | b1; count = delta + 128
            if (pos + 2 > data.Length)
                throw new InvalidDataException("truncated sequence count");
            int delta = ((b0 & 0x7F) << 8) | data[pos + 1];
            return (delta + 128, 2);
        }
        // 3-byte encoding: byte0=0xFF, then (count - 0x7F00) as LE u16
        if (pos + 3 > data.Length)
            throw new InvalidDataException("truncated sequence count (3-byte)");
        int count2 = 0x7F00 + data[pos + 1] + (data[pos + 2] << 8);
        return (count2, 3);
    }

    // Encode the sequences section using predefined FSE tables.
    private static byte[] EncodeSequencesSection(List<Seq> seqs)
    {
        // Build encode tables (precomputed from the predefined distributions).
        var (eeLl, stLl) = BuildEncodeTable(LlNorm, LlAccLog);
        var (eeMl, stMl) = BuildEncodeTable(MlNorm, MlAccLog);
        var (eeOf, stOf) = BuildEncodeTable(OfNorm, OfAccLog);

        uint szLl = 1u << LlAccLog;
        uint szMl = 1u << MlAccLog;
        uint szOf = 1u << OfAccLog;

        // FSE encoder states start at table_size (= sz).
        // The state range [sz, 2*sz) maps to slot range [0, sz).
        uint stateLl = szLl;
        uint stateMl = szMl;
        uint stateOf = szOf;

        var bw = new RevBitWriter();

        // Encode sequences in reverse order.
        for (int si = seqs.Count - 1; si >= 0; si--)
        {
            var seq = seqs[si];
            int llCode = LlToCode(seq.Ll);
            int mlCode = MlToCode(seq.Ml);

            // Offset encoding: raw = offset + 3 (RFC 8878 §3.1.1.3.2.1)
            // code = floor(log2(raw)); extra = raw - (1 << code)
            uint rawOff = seq.Off + 3;
            int ofCode = rawOff <= 1 ? 0 : (int)(31 - LeadingZeros(rawOff));
            uint ofExtra = rawOff - (1u << ofCode);

            // Write extra bits (OF, ML, LL in this order for backward stream).
            bw.AddBits(ofExtra, ofCode);
            uint mlExtra = seq.Ml - MlCodes[mlCode].Base;
            bw.AddBits(mlExtra, MlCodes[mlCode].Bits);
            uint llExtra = seq.Ll - LlCodes[llCode].Base;
            bw.AddBits(llExtra, LlCodes[llCode].Bits);

            // FSE encode symbols in the order that the backward bitstream reverses
            // to match the decoder's read order (LL first, OF second, ML third).
            //
            // Since the backward stream reverses write order, we write the REVERSE
            // of the decode order: ML → OF → LL (LL is written last = at the top
            // of the bitstream = read first by the decoder).
            //
            // Decode order: LL, OF, ML
            // Encode order (reversed): ML, OF, LL
            FseEncodeSym(ref stateMl, mlCode, eeMl, stMl, bw);
            FseEncodeSym(ref stateOf, ofCode, eeOf, stOf, bw);
            FseEncodeSym(ref stateLl, llCode, eeLl, stLl, bw);
        }

        // Flush final states (low acc_log bits of state - sz).
        bw.AddBits(stateOf - szOf, OfAccLog);
        bw.AddBits(stateMl - szMl, MlAccLog);
        bw.AddBits(stateLl - szLl, LlAccLog);
        bw.Flush();

        return bw.Finish();
    }

    // ─── Block-level compress ───────────────────────────────────────────────────

    // Compress one block into ZStd compressed block format.
    //
    // Returns null if the compressed form is larger than the input (in which
    // case the caller should use a Raw block instead).
    private static byte[]? CompressBlock(byte[] data, int offset, int length)
    {
        // Use LZSS to generate LZ77 tokens.
        // Window = 32 KB, max match = 255, min match = 3
        var blockData = data[offset..(offset + length)];
        var tokens = CodingAdventures.Lzss.Lzss.Encode(blockData, 32768, 255, 3);

        // Convert tokens to ZStd sequences.
        var (lits, seqs) = TokensToSeqs(tokens);

        // If no sequences were found, LZ77 had nothing to compress.
        if (seqs.Count == 0)
            return null;

        var result = new List<byte>();

        // Encode literals section (Raw_Literals).
        result.AddRange(EncodeLiteralsSection(lits));

        // Encode sequences section.
        result.AddRange(EncodeSeqCount(seqs.Count));
        result.Add(0x00); // Symbol_Compression_Modes = all Predefined

        byte[] bitstream = EncodeSequencesSection(seqs);
        result.AddRange(bitstream);

        if (result.Count >= length)
            return null; // Not beneficial

        return [.. result];
    }

    // Decompress one ZStd compressed block.
    //
    // Reads the literals section, sequences section, and applies the sequences
    // to the output buffer to reconstruct the original data.
    private static void DecompressBlock(byte[] data, int blockStart, int blockLen, List<byte> output)
    {
        // ── Literals section ────────────────────────────────────────────────
        var (lits, litConsumed) = DecodeLiteralsSection(data, blockStart);
        int pos = blockStart + litConsumed;
        int blockEnd = blockStart + blockLen;

        // ── Sequences count ─────────────────────────────────────────────────
        if (pos >= blockEnd)
        {
            // Block has only literals, no sequences.
            output.AddRange(lits);
            return;
        }

        var (nSeqs, scBytes) = DecodeSeqCount(data, pos);
        pos += scBytes;

        if (nSeqs == 0)
        {
            // No sequences — all content is in literals.
            output.AddRange(lits);
            return;
        }

        // ── Symbol compression modes ────────────────────────────────────────
        if (pos >= blockEnd)
            throw new InvalidDataException("missing symbol compression modes byte");
        byte modesByte = data[pos];
        pos++;

        // Check that all modes are Predefined (0).
        int llMode = (modesByte >> 6) & 3;
        int ofMode = (modesByte >> 4) & 3;
        int mlMode = (modesByte >> 2) & 3;
        if (llMode != 0 || ofMode != 0 || mlMode != 0)
            throw new InvalidDataException(
                $"unsupported FSE modes: LL={llMode} OF={ofMode} ML={mlMode} (only Predefined=0 supported)");

        // ── FSE bitstream ────────────────────────────────────────────────────
        int bsLen = blockEnd - pos;
        if (bsLen <= 0)
            throw new InvalidDataException("missing FSE bitstream");

        var bitstreamSlice = data[pos..blockEnd];
        var br = new RevBitReader(bitstreamSlice);

        // Build decode tables from predefined distributions.
        FseDe[] dtLl = BuildDecodeTable(LlNorm, LlAccLog);
        FseDe[] dtMl = BuildDecodeTable(MlNorm, MlAccLog);
        FseDe[] dtOf = BuildDecodeTable(OfNorm, OfAccLog);

        // Initialise FSE states from the bitstream.
        // The encoder wrote: state_ll, state_ml, state_of (each as acc_log bits).
        // The decoder reads them in the same order.
        ushort stateLl = (ushort)br.ReadBits(LlAccLog);
        ushort stateMl = (ushort)br.ReadBits(MlAccLog);
        ushort stateOf = (ushort)br.ReadBits(OfAccLog);

        // Track position in the literals buffer.
        int litPos = 0;

        // Apply each sequence.
        for (int i = 0; i < nSeqs; i++)
        {
            // Decode symbols (state transitions) — order: LL, OF, ML.
            byte llCode = FseDecodeSym(ref stateLl, dtLl, br);
            byte ofCode = FseDecodeSym(ref stateOf, dtOf, br);
            byte mlCode = FseDecodeSym(ref stateMl, dtMl, br);

            // Validate codes.
            if (llCode >= LlCodes.Length)
                throw new InvalidDataException($"invalid LL code {llCode}");
            if (mlCode >= MlCodes.Length)
                throw new InvalidDataException($"invalid ML code {mlCode}");

            var llInfo = LlCodes[llCode];
            var mlInfo = MlCodes[mlCode];

            uint ll = llInfo.Base + (uint)br.ReadBits(llInfo.Bits);
            uint ml = mlInfo.Base + (uint)br.ReadBits(mlInfo.Bits);
            // Offset: raw = (1 << of_code) | extra_bits; offset = raw - 3
            uint ofRaw = (1u << ofCode) | (uint)br.ReadBits(ofCode);
            if (ofRaw < 3)
                throw new InvalidDataException($"decoded offset underflow: of_raw={ofRaw}");
            uint offset = ofRaw - 3;

            // Emit ll literal bytes from the literals buffer.
            int litEnd = litPos + (int)ll;
            if (litEnd > lits.Length)
                throw new InvalidDataException(
                    $"literal run {ll} overflows literals buffer (pos={litPos} len={lits.Length})");
            output.AddRange(lits[litPos..litEnd]);
            litPos = litEnd;

            // Copy ml bytes from offset back in the output buffer.
            if (offset == 0 || (int)offset > output.Count)
                throw new InvalidDataException(
                    $"bad match offset {offset} (output len {output.Count})");
            int copyStart = output.Count - (int)offset;
            for (int j = 0; j < (int)ml; j++)
                output.Add(output[copyStart + j]);
        }

        // Any remaining literals after the last sequence.
        if (litPos < lits.Length)
            output.AddRange(lits[litPos..]);
    }

    // ─── Public API ─────────────────────────────────────────────────────────────

    /// <summary>
    /// Compress <paramref name="data"/> to ZStd format (RFC 8878).
    ///
    /// The output is a valid ZStd frame that can be decompressed by the
    /// <c>zstd</c> CLI tool or any conforming implementation.
    /// </summary>
    /// <param name="data">The uncompressed input bytes.</param>
    /// <returns>ZStd-compressed bytes.</returns>
    /// <example>
    /// <code>
    /// byte[] compressed = Zstd.Compress(Encoding.UTF8.GetBytes("hello!"));
    /// byte[] original = Zstd.Decompress(compressed);
    /// </code>
    /// </example>
    public static byte[] Compress(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var out2 = new List<byte>();

        // ── ZStd frame header ─────────────────────────────────────────────────
        // Magic number (4 bytes LE).
        var magic = new byte[4];
        BinaryPrimitives.WriteUInt32LittleEndian(magic, Magic);
        out2.AddRange(magic);

        // Frame Header Descriptor (FHD):
        //   bit 7-6: FCS_Field_Size flag = 11 → 8-byte FCS
        //   bit 5:   Single_Segment_Flag = 1 (no Window_Descriptor follows)
        //   bit 4:   Content_Checksum_Flag = 0
        //   bit 3-2: reserved = 0
        //   bit 1-0: Dict_ID_Flag = 0
        // = 0b1110_0000 = 0xE0
        out2.Add(0xE0);

        // Frame_Content_Size (8 bytes LE) — the uncompressed size.
        var fcs = new byte[8];
        BinaryPrimitives.WriteUInt64LittleEndian(fcs, (ulong)data.Length);
        out2.AddRange(fcs);

        // ── Blocks ────────────────────────────────────────────────────────────
        // Handle the special case of completely empty input: emit one empty raw block.
        if (data.Length == 0)
        {
            // Last=1, Type=Raw(00), Size=0 → header = 0b0000_0001 = 0x01
            out2.Add(0x01);
            out2.Add(0x00);
            out2.Add(0x00);
            return [.. out2];
        }

        int offset = 0;
        while (offset < data.Length)
        {
            int end = Math.Min(offset + MaxBlockSize, data.Length);
            int blockLen = end - offset;
            bool last = end == data.Length;

            // ── Try RLE block ───────────────────────────────────────────────
            // If all bytes in the block are identical, a single-byte RLE block
            // encodes it in just 1 byte (plus 3-byte header = 4 bytes total).
            bool allSame = true;
            byte firstByte = data[offset];
            for (int i = offset + 1; i < end; i++)
            {
                if (data[i] != firstByte) { allSame = false; break; }
            }

            if (allSame)
            {
                // RLE block header: type=01, size=blockLen, last=1/0
                uint hdr = (((uint)blockLen) << 3) | (0b01u << 1) | (last ? 1u : 0u);
                out2.Add((byte)(hdr & 0xFF));
                out2.Add((byte)((hdr >> 8) & 0xFF));
                out2.Add((byte)((hdr >> 16) & 0xFF));
                out2.Add(firstByte);
            }
            else
            {
                // ── Try compressed block ──────────────────────────────────────
                byte[]? compressed = CompressBlock(data, offset, blockLen);
                if (compressed != null)
                {
                    uint hdr = (((uint)compressed.Length) << 3) | (0b10u << 1) | (last ? 1u : 0u);
                    out2.Add((byte)(hdr & 0xFF));
                    out2.Add((byte)((hdr >> 8) & 0xFF));
                    out2.Add((byte)((hdr >> 16) & 0xFF));
                    out2.AddRange(compressed);
                }
                else
                {
                    // ── Raw block (fallback) ──────────────────────────────────
                    uint hdr = (((uint)blockLen) << 3) | (0b00u << 1) | (last ? 1u : 0u);
                    out2.Add((byte)(hdr & 0xFF));
                    out2.Add((byte)((hdr >> 8) & 0xFF));
                    out2.Add((byte)((hdr >> 16) & 0xFF));
                    out2.AddRange(data[offset..end]);
                }
            }

            offset = end;
        }

        return [.. out2];
    }

    /// <summary>
    /// Decompress a ZStd frame, returning the original data.
    ///
    /// Accepts any valid ZStd frame with Raw, RLE, or Compressed blocks using
    /// Predefined FSE modes.
    /// </summary>
    /// <param name="data">ZStd-compressed bytes.</param>
    /// <returns>The original uncompressed bytes.</returns>
    /// <exception cref="InvalidDataException">
    /// Thrown if the input is truncated, has a bad magic number, or contains
    /// unsupported features (non-predefined FSE tables, Huffman literals,
    /// reserved block types).
    /// </exception>
    public static byte[] Decompress(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        if (data.Length < 5)
            throw new InvalidDataException("frame too short");

        // ── Validate magic ────────────────────────────────────────────────────
        uint magic = BinaryPrimitives.ReadUInt32LittleEndian(data.AsSpan(0, 4));
        if (magic != Magic)
            throw new InvalidDataException(
                $"bad magic: 0x{magic:X8} (expected 0x{Magic:X8})");

        int pos = 4;

        // ── Parse Frame Header Descriptor ─────────────────────────────────────
        // FHD encodes several flags that control the header layout.
        byte fhd = data[pos++];

        // FCS_Field_Size: bits [7:6] of FHD.
        //   00 → 0 bytes if Single_Segment=0, else 1 byte
        //   01 → 2 bytes
        //   10 → 4 bytes
        //   11 → 8 bytes
        int fcsFlag = (fhd >> 6) & 3;

        // Single_Segment_Flag: bit 5. When set, the window descriptor is omitted.
        int singleSeg = (fhd >> 5) & 1;

        // Dict_ID_Flag: bits [1:0]. Indicates how many bytes the dict ID occupies.
        int dictFlag = fhd & 3;

        // ── Window Descriptor ─────────────────────────────────────────────────
        // Present only if Single_Segment_Flag = 0. We skip it.
        if (singleSeg == 0) pos++;

        // ── Dict ID ───────────────────────────────────────────────────────────
        int dictIdBytes = dictFlag == 0 ? 0 : (dictFlag == 1 ? 1 : (dictFlag == 2 ? 2 : 4));
        pos += dictIdBytes;

        // ── Frame Content Size ────────────────────────────────────────────────
        // We read but don't validate FCS.
        int fcsBytes = fcsFlag switch
        {
            0 => singleSeg == 1 ? 1 : 0,
            1 => 2,
            2 => 4,
            3 => 8,
            _ => 0
        };
        pos += fcsBytes;

        // ── Blocks ────────────────────────────────────────────────────────────
        // Guard against decompression bombs: cap total output at 256 MB.
        const int MaxOutput = 256 * 1024 * 1024;
        var output = new List<byte>();

        while (true)
        {
            if (pos + 3 > data.Length)
                throw new InvalidDataException("truncated block header");

            // 3-byte little-endian block header.
            uint hdr = (uint)data[pos] | ((uint)data[pos + 1] << 8) | ((uint)data[pos + 2] << 16);
            pos += 3;

            bool last = (hdr & 1) != 0;
            int btype = (int)((hdr >> 1) & 3);
            int bsize = (int)(hdr >> 3);

            switch (btype)
            {
                case 0:
                    // Raw block: bsize bytes of verbatim content.
                    if (pos + bsize > data.Length)
                        throw new InvalidDataException(
                            $"raw block truncated: need {bsize} bytes at pos {pos}");
                    if (output.Count + bsize > MaxOutput)
                        throw new InvalidDataException(
                            $"decompressed size exceeds limit of {MaxOutput} bytes");
                    output.AddRange(data[pos..(pos + bsize)]);
                    pos += bsize;
                    break;

                case 1:
                    // RLE block: 1 byte repeated bsize times.
                    if (pos >= data.Length)
                        throw new InvalidDataException("RLE block missing byte");
                    if (output.Count + bsize > MaxOutput)
                        throw new InvalidDataException(
                            $"decompressed size exceeds limit of {MaxOutput} bytes");
                    byte rleByte = data[pos++];
                    for (int i = 0; i < bsize; i++)
                        output.Add(rleByte);
                    break;

                case 2:
                    // Compressed block.
                    if (pos + bsize > data.Length)
                        throw new InvalidDataException(
                            $"compressed block truncated: need {bsize} bytes");
                    DecompressBlock(data, pos, bsize, output);
                    pos += bsize;
                    break;

                case 3:
                    throw new InvalidDataException("reserved block type 3");

                default:
                    throw new InvalidDataException($"unknown block type {btype}");
            }

            if (last) break;
        }

        return [.. output];
    }
}
