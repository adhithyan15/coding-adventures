// CMP09 — ZIP archive format (PKZIP, 1989).
//
// ZIP bundles one or more files into a single `.zip` archive, compressing each
// entry independently with DEFLATE (method 8) or storing it verbatim (method 0).
// The same container format underlies Java JARs, Office Open XML (.docx), Android
// APKs, Python wheels, and many other formats.
//
// Architecture
// ────────────
//
//   ┌─────────────────────────────────────────────────────┐
//   │  [Local File Header + File Data]  ← entry 1         │
//   │  [Local File Header + File Data]  ← entry 2         │
//   │  ...                                                │
//   │  ══════════ Central Directory ══════════            │
//   │  [Central Dir Header]  ← entry 1 (has local offset)│
//   │  [Central Dir Header]  ← entry 2                   │
//   │  [End of Central Directory Record]                  │
//   └─────────────────────────────────────────────────────┘
//
// The dual-header design supports two workflows:
//   - Sequential write: append Local Headers + data, write Central Directory at end.
//   - Random-access read: seek to EOCD, read Central Directory, jump to any entry.
//
// Series
// ──────
//   CMP00 (LZ77,    1977) — Sliding-window backreferences.
//   CMP01 (LZ78,    1978) — Explicit dictionary (trie).
//   CMP02 (LZSS,    1982) — LZ77 + flag bits.
//   CMP03 (LZW,     1984) — LZ78 + pre-initialized alphabet; GIF.
//   CMP04 (Huffman, 1952) — Entropy coding.
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
//   CMP09 (ZIP,     1989) — DEFLATE container; universal archive.  ← THIS FILE

using System.Buffers.Binary;
using System.Text;
using CodingAdventures.Lzss;

namespace CodingAdventures.Zip;

// =============================================================================
// Wire Format Constants
// =============================================================================
//
// ZIP uses four-byte "magic number" signatures to identify each structural
// region. All integers in the wire format are little-endian.

internal static class ZipConstants
{
    // Local File Header signature: "PK\x03\x04"
    public const uint LocalSig = 0x04034B50;

    // Central Directory Header signature: "PK\x01\x02"
    public const uint CdSig = 0x02014B50;

    // End of Central Directory Record signature: "PK\x05\x06"
    public const uint EocdSig = 0x06054B50;

    // Fixed timestamp: 1980-01-01 00:00:00
    // DOS date = (0<<9)|(1<<5)|1 = 0x0021; time = 0 → combined 0x00210000
    public const uint DosEpoch = 0x00210000;

    // General Purpose Bit Flag: bit 11 = UTF-8 filename encoding
    public const ushort FlagUtf8 = 0x0800;

    // Compression methods
    public const ushort MethodStored = 0;
    public const ushort MethodDeflate = 8;

    // Version needed: 2.0 for DEFLATE, 1.0 for Stored
    public const ushort VersionDeflate = 20;
    public const ushort VersionStored = 10;

    // Version made by: 0x031E = Unix OS (high byte 3), specification version 30 (low byte 0x1E)
    public const ushort VersionMadeBy = 0x031E;

    // Unix file modes embedded in Central Directory external_attrs (shifted left 16 bits).
    // 0o100644 = regular file, rw-r--r-- (octal); 0o040755 = directory, rwxr-xr-x.
    // C# has no octal literals; these are the decimal equivalents.
    public const uint UnixModeFile = 33188u; // 0o100644 decimal
    public const uint UnixModeDir  = 16877u; // 0o040755 decimal
}

// =============================================================================
// CRC-32
// =============================================================================
//
// CRC-32 uses polynomial 0xEDB88320 (reflected form of 0x04C11DB7).
// It is computed over the *uncompressed* bytes and stored in the headers so
// extractors can verify integrity after decompression.
//
// CRC-32 is NOT a cryptographic hash — it detects accidental corruption only.
// For tamper-detection use AES-GCM or a signed manifest.

internal static class Crc32Helper
{
    // Precomputed 256-entry lookup table. Each entry is the CRC-32 of a single
    // byte value, using the reflected polynomial 0xEDB88320. Building it once
    // at class-load time amortises the cost across all archives.
    private static readonly uint[] Table = BuildTable();

    private static uint[] BuildTable()
    {
        var table = new uint[256];
        for (var i = 0u; i < 256u; i++)
        {
            var c = i;
            for (var k = 0; k < 8; k++)
            {
                // If the LSB is set, XOR with the polynomial (reflected form).
                // This is the "table-driven CRC" algorithm from RFC 1952 §8.
                c = (c & 1) != 0 ? (0xEDB88320u ^ (c >> 1)) : (c >> 1);
            }
            table[i] = c;
        }
        return table;
    }

    /// <summary>
    /// Compute CRC-32 over <paramref name="data"/>.
    /// Pass <paramref name="initial"/> = 0 for a fresh hash, or the previous
    /// result to continue an incremental computation.
    /// </summary>
    public static uint Compute(ReadOnlySpan<byte> data, uint initial = 0)
    {
        // XOR the initial value in (for the first call initial=0 → crc starts at 0xFFFFFFFF).
        var crc = initial ^ 0xFFFF_FFFFu;
        foreach (var b in data)
        {
            crc = Table[(crc ^ b) & 0xFF] ^ (crc >> 8);
        }
        // XOR out to produce the final CRC (two's-complement the initial XOR).
        return crc ^ 0xFFFF_FFFFu;
    }
}

// =============================================================================
// RFC 1951 DEFLATE — Bit I/O (BitWriter)
// =============================================================================
//
// RFC 1951 packs bits LSB-first within bytes. Huffman codes are logically
// MSB-first, so before writing one we bit-reverse it and write the reversed
// value LSB-first into the stream. Extra bits (length/distance extras, stored
// block headers) are written directly in LSB-first order without reversal.

internal sealed class BitWriter
{
    private readonly List<byte> _buf = [];
    private ulong _reg;   // accumulator holding up to 63 unflushable bits
    private int _bits;    // how many bits are currently valid in _reg

    /// <summary>
    /// Write the <paramref name="n"/> low-order bits of <paramref name="val"/>
    /// into the stream, LSB-first. Used for extra bits and block headers.
    /// </summary>
    public void AddBits(ulong val, int n)
    {
        // OR the new bits into the accumulator at the current fill position.
        _reg |= (val & ((1UL << n) - 1)) << _bits;
        _bits += n;
        // Drain complete bytes from the accumulator.
        while (_bits >= 8)
        {
            _buf.Add((byte)(_reg & 0xFF));
            _reg >>= 8;
            _bits -= 8;
        }
    }

    /// <summary>
    /// Write a Huffman code of <paramref name="nbits"/> bits.
    /// Huffman codes are MSB-first logically, so we bit-reverse before storing.
    /// </summary>
    public void WriteHuffman(uint code, int nbits)
    {
        // Reverse the top `nbits` bits of `code` so the MSB becomes the LSB.
        uint reversed = ReverseBits(code, nbits);
        AddBits(reversed, nbits);
    }

    /// <summary>
    /// Reverse the bottom <paramref name="nbits"/> bits of <paramref name="code"/>.
    /// Example: ReverseBits(0b110, 3) == 0b011.
    /// </summary>
    private static uint ReverseBits(uint code, int nbits)
    {
        // Reverse all 32 bits then shift right to keep only the bottom nbits.
        uint rev = 0;
        for (var i = 0; i < nbits; i++)
        {
            rev = (rev << 1) | (code & 1);
            code >>= 1;
        }
        return rev;
    }

    /// <summary>
    /// Flush any partial byte to the buffer (zero-pad the remaining bits).
    /// Required before writing stored-block headers, which must be byte-aligned.
    /// </summary>
    public void Flush()
    {
        if (_bits > 0)
        {
            _buf.Add((byte)(_reg & 0xFF));
            _reg = 0;
            _bits = 0;
        }
    }

    /// <summary>Return the completed byte array. Flushes any partial byte first.</summary>
    public byte[] ToArray()
    {
        Flush();
        return [.. _buf];
    }
}

// =============================================================================
// RFC 1951 DEFLATE — Bit I/O (BitReader)
// =============================================================================
//
// Mirrors BitWriter: fill an accumulator from bytes in the source array,
// reading LSB-first. Huffman code decoding reads MSB-first by bit-reversing
// the extracted value.

internal sealed class BitReader
{
    private readonly byte[] _data;
    private int _pos;   // next byte to consume from _data
    private ulong _buf; // bit accumulator
    private int _bits;  // valid bits in _buf

    public BitReader(byte[] data) { _data = data; }

    /// <summary>
    /// Ensure the accumulator holds at least <paramref name="need"/> bits.
    /// Returns false if the source is exhausted before that many bits are available.
    /// </summary>
    private bool Fill(int need)
    {
        while (_bits < need)
        {
            if (_pos >= _data.Length) return false;
            _buf |= (ulong)_data[_pos++] << _bits;
            _bits += 8;
        }
        return true;
    }

    /// <summary>
    /// Read <paramref name="nbits"/> bits from the stream, LSB-first.
    /// Returns null on end-of-input.
    /// </summary>
    public int? ReadLsb(int nbits)
    {
        if (nbits == 0) return 0;
        if (!Fill(nbits)) return null;
        var mask = (1UL << nbits) - 1;
        var val = (int)(_buf & mask);
        _buf >>= nbits;
        _bits -= nbits;
        return val;
    }

    /// <summary>
    /// Read <paramref name="nbits"/> bits and bit-reverse the result.
    /// Used when decoding Huffman codes (logically MSB-first).
    /// </summary>
    public int? ReadMsb(int nbits)
    {
        var v = ReadLsb(nbits);
        if (v is null) return null;
        // Reverse the bottom nbits bits.
        uint rev = 0;
        var u = (uint)v.Value;
        for (var i = 0; i < nbits; i++) { rev = (rev << 1) | (u & 1); u >>= 1; }
        return (int)rev;
    }

    /// <summary>
    /// Discard any partial-byte bits, aligning to the next byte boundary.
    /// Required before reading stored-block length fields.
    /// </summary>
    public void Align()
    {
        var discard = _bits % 8;
        if (discard > 0) { _buf >>= discard; _bits -= discard; }
    }
}

// =============================================================================
// RFC 1951 DEFLATE — Fixed Huffman Tables
// =============================================================================
//
// RFC 1951 §3.2.6 defines a canonical "fixed" Huffman alphabet that both
// encoder and decoder know in advance. Using BTYPE=01 (fixed Huffman) means we
// never need to transmit code-length tables, keeping the implementation simple.
//
// Literal/Length code lengths:
//   Symbols   0–143: 8-bit codes, base 0x30 (0b00110000)
//   Symbols 144–255: 9-bit codes, base 0x190 (0b110010000)
//   Symbols 256–279: 7-bit codes, base 0x00
//   Symbols 280–287: 8-bit codes, base 0xC0 (0b11000000)
//
// Distance codes: 5-bit codes equal to the code number (0–29).

internal static class FixedHuffman
{
    /// <summary>
    /// Return the (code, nbits) pair for encoding literal/length symbol 0–287.
    /// </summary>
    public static (uint Code, int Bits) EncodeLL(int sym)
    {
        return sym switch
        {
            // Literals 0–143: 8-bit codes starting at 0x30 (48 decimal).
            >= 0   and <= 143 => ((uint)(sym + 0x30), 8),
            // Literals 144–255: 9-bit codes starting at 0x190 (400 decimal).
            >= 144 and <= 255 => ((uint)(sym - 144 + 0x190), 9),
            // EOB + length codes 256–279: 7-bit codes starting at 0.
            >= 256 and <= 279 => ((uint)(sym - 256), 7),
            // Length codes 280–287: 8-bit codes starting at 0xC0 (192 decimal).
            >= 280 and <= 287 => ((uint)(sym - 280 + 0xC0), 8),
            _ => throw new InvalidDataException($"FixedHuffman.EncodeLL: invalid symbol {sym}"),
        };
    }

    /// <summary>
    /// Decode one literal/length symbol from <paramref name="br"/> using the
    /// RFC 1951 fixed Huffman table.  Reads incrementally (7 → 8 → 9 bits).
    /// Returns null on end-of-input.
    /// </summary>
    public static int? DecodeLL(BitReader br)
    {
        // Try 7 bits first (covers symbols 256–279, codes 0–23).
        var v7 = br.ReadMsb(7);
        if (v7 is null) return null;

        if (v7.Value <= 23)
        {
            // 7-bit code → symbols 256–279.
            return v7.Value + 256;
        }

        // Need one more bit to reach 8-bit codes.
        var extra1 = br.ReadLsb(1);
        if (extra1 is null) return null;
        var v8 = (v7.Value << 1) | extra1.Value;

        if (v8 is >= 48 and <= 191)
        {
            // 8-bit code → literals 0–143.
            return v8 - 48;
        }
        if (v8 is >= 192 and <= 199)
        {
            // 8-bit code → symbols 280–287 (192 + 88 = 280).
            return v8 + 88;
        }

        // Need one more bit for 9-bit codes (literals 144–255).
        var extra2 = br.ReadLsb(1);
        if (extra2 is null) return null;
        var v9 = (v8 << 1) | extra2.Value;

        if (v9 is >= 400 and <= 511)
        {
            // 9-bit code → literals 144–255 (400 – 256 = 144).
            return v9 - 256;
        }

        return null; // malformed bit-stream
    }
}

// =============================================================================
// RFC 1951 DEFLATE — Length / Distance Tables
// =============================================================================
//
// Match lengths (3–258 bytes) map to LL symbols 257–285 plus extra bits.
// Match distances (1–32768 bytes) map to distance codes 0–29 plus extra bits.
// The tables below come directly from RFC 1951 §3.2.5.

internal static class DeflateTable
{
    // (base_length, extra_bits) indexed by (LL_symbol - 257).
    // Symbol 285 has a special-case base of 258 with 0 extra bits.
    public static readonly (int Base, int Extra)[] Length =
    [
        (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0), (10, 0), // 257–264
        (11, 1), (13, 1), (15, 1), (17, 1),                                 // 265–268
        (19, 2), (23, 2), (27, 2), (31, 2),                                 // 269–272
        (35, 3), (43, 3), (51, 3), (59, 3),                                 // 273–276
        (67, 4), (83, 4), (99, 4), (115, 4),                               // 277–280
        (131, 5), (163, 5), (195, 5), (227, 5), (258, 0),                  // 281–285
    ];

    // (base_distance, extra_bits) indexed by distance code 0–29.
    public static readonly (int Base, int Extra)[] Dist =
    [
        (1, 0), (2, 0), (3, 0), (4, 0),
        (5, 1), (7, 1), (9, 2), (13, 2),
        (17, 3), (25, 3), (33, 4), (49, 4),
        (65, 5), (97, 5), (129, 6), (193, 6),
        (257, 7), (385, 7), (513, 8), (769, 8),
        (1025, 9), (1537, 9), (2049, 10), (3073, 10),
        (4097, 11), (6145, 11), (8193, 12), (12289, 12),
        (16385, 13), (24577, 13),
    ];

    /// <summary>
    /// Encode a match <paramref name="length"/> (3–258) as an RFC 1951 LL symbol
    /// plus extra bits. Returns (ll_symbol, base, extra_bit_count).
    /// </summary>
    public static (int Symbol, int Base, int ExtraBits) EncodeLength(int length)
    {
        // Walk the table from the highest entry downward to find the right slot.
        for (var i = Length.Length - 1; i >= 0; i--)
        {
            if (length >= Length[i].Base)
                return (257 + i, Length[i].Base, Length[i].Extra);
        }
        throw new InvalidDataException($"EncodeLength: unreachable for length={length}");
    }

    /// <summary>
    /// Encode a match <paramref name="distance"/> (1–32768) as an RFC 1951 distance
    /// code plus extra bits. Returns (dist_code, base, extra_bit_count).
    /// </summary>
    public static (int Code, int Base, int ExtraBits) EncodeDist(int distance)
    {
        for (var i = Dist.Length - 1; i >= 0; i--)
        {
            if (distance >= Dist[i].Base)
                return (i, Dist[i].Base, Dist[i].Extra);
        }
        throw new InvalidDataException($"EncodeDist: unreachable for distance={distance}");
    }
}

// =============================================================================
// RFC 1951 DEFLATE — Compressor
// =============================================================================
//
// Strategy:
//   1. Run LZSS match-finding (window=32768, max match=255, min=3).
//   2. Emit a single BTYPE=01 (fixed Huffman) block over all tokens.
//   3. Literals → fixed LL Huffman code.
//   4. Matches → length LL code + extra bits + distance code + extra bits.
//   5. End-of-block symbol 256.
//
// For empty input we emit a stored block (BTYPE=00) instead — this is the
// canonical representation for zero bytes in raw DEFLATE.

internal static class DeflateCompressor
{
    /// <summary>
    /// Compress <paramref name="data"/> to raw RFC 1951 DEFLATE (no zlib wrapper).
    /// Uses fixed Huffman (BTYPE=01) for non-empty input.
    /// </summary>
    public static byte[] Compress(byte[] data)
    {
        var bw = new BitWriter();

        if (data.Length == 0)
        {
            // Empty stored block: BFINAL=1, BTYPE=00, aligned, LEN=0, NLEN=0xFFFF.
            bw.AddBits(1, 1); // BFINAL = 1 (last block)
            bw.AddBits(0, 2); // BTYPE = 00 (stored)
            bw.Flush();       // align to byte boundary (RFC 1951 §3.2.4)
            bw.AddBits(0x0000, 16); // LEN = 0
            bw.AddBits(0xFFFF, 16); // NLEN = one's complement of LEN
            return bw.ToArray();
        }

        // Run LZSS tokenization. Window = 32768 so every match distance fits in
        // the RFC 1951 distance table. Max match = 255 to fit the length table.
        var tokens = CodingAdventures.Lzss.Lzss.Encode(data, windowSize: 32768, maxMatch: 255, minMatch: 3);

        // Block header: BFINAL=1 (single block), BTYPE=01 (fixed Huffman).
        // Bits are written LSB-first: BFINAL in bit 0, BTYPE in bits 1-2.
        bw.AddBits(1, 1); // BFINAL = 1
        bw.AddBits(1, 1); // BTYPE bit 0 = 1  }
        bw.AddBits(0, 1); // BTYPE bit 1 = 0  } → BTYPE = 01 (fixed Huffman)

        foreach (var token in tokens)
        {
            switch (token)
            {
                case LzssLiteral lit:
                {
                    // Literal byte: emit the fixed Huffman code for symbol `lit.Byte`.
                    var (code, bits) = FixedHuffman.EncodeLL(lit.Byte);
                    bw.WriteHuffman(code, bits);
                    break;
                }
                case LzssMatch match:
                {
                    // Length: find the LL symbol + extra bits, then emit them.
                    var (lenSym, lenBase, lenExtra) = DeflateTable.EncodeLength(match.Length);
                    var (lenCode, lenBits) = FixedHuffman.EncodeLL(lenSym);
                    bw.WriteHuffman(lenCode, lenBits);
                    if (lenExtra > 0)
                        bw.AddBits((ulong)(match.Length - lenBase), lenExtra);

                    // Distance: the 5-bit fixed distance code equals the code number.
                    var (distCode, distBase, distExtra) = DeflateTable.EncodeDist(match.Offset);
                    bw.WriteHuffman((uint)distCode, 5);
                    if (distExtra > 0)
                        bw.AddBits((ulong)(match.Offset - distBase), distExtra);
                    break;
                }
            }
        }

        // End-of-block symbol (256) — signals the decoder to stop.
        var (eobCode, eobBits) = FixedHuffman.EncodeLL(256);
        bw.WriteHuffman(eobCode, eobBits);

        return bw.ToArray();
    }
}

// =============================================================================
// RFC 1951 DEFLATE — Decompressor
// =============================================================================
//
// Handles stored blocks (BTYPE=00) and fixed Huffman blocks (BTYPE=01).
// Dynamic Huffman blocks (BTYPE=10) throw InvalidDataException — we only write
// BTYPE=01, but stored blocks from other tools must be accepted too.
//
// Security limits:
//   - Maximum output: 256 MB (decompression bomb guard)
//   - LEN/NLEN validation on stored blocks

internal static class DeflateDecompressor
{
    private const int MaxOutputBytes = 256 * 1024 * 1024;

    /// <summary>
    /// Decompress raw RFC 1951 DEFLATE bytes into the original data.
    /// Throws <see cref="InvalidDataException"/> for corrupt or unsupported input.
    /// </summary>
    public static byte[] Decompress(byte[] data)
    {
        var br = new BitReader(data);
        var output = new List<byte>();

        while (true)
        {
            var bfinal = br.ReadLsb(1) ?? throw new InvalidDataException("deflate: unexpected EOF reading BFINAL");
            var btype  = br.ReadLsb(2) ?? throw new InvalidDataException("deflate: unexpected EOF reading BTYPE");

            switch (btype)
            {
                case 0b00:
                    // ── Stored block ──────────────────────────────────────────
                    // Align to byte boundary before reading the length fields.
                    br.Align();
                    var len  = br.ReadLsb(16) ?? throw new InvalidDataException("deflate: EOF reading stored LEN");
                    var nlen = br.ReadLsb(16) ?? throw new InvalidDataException("deflate: EOF reading stored NLEN");
                    // RFC 1951 §3.2.4: NLEN is the one's complement of LEN.
                    if ((nlen ^ 0xFFFF) != len)
                        throw new InvalidDataException($"deflate: stored LEN/NLEN mismatch ({len} vs {nlen})");
                    if (output.Count + len > MaxOutputBytes)
                        throw new InvalidDataException("deflate: output size limit exceeded");
                    for (var i = 0; i < len; i++)
                    {
                        var b = br.ReadLsb(8) ?? throw new InvalidDataException("deflate: EOF inside stored block");
                        output.Add((byte)b);
                    }
                    break;

                case 0b01:
                    // ── Fixed Huffman block ───────────────────────────────────
                    while (true)
                    {
                        var sym = FixedHuffman.DecodeLL(br)
                            ?? throw new InvalidDataException("deflate: EOF decoding LL symbol");

                        if (sym is >= 0 and <= 255)
                        {
                            if (output.Count >= MaxOutputBytes)
                                throw new InvalidDataException("deflate: output size limit exceeded");
                            output.Add((byte)sym);
                        }
                        else if (sym == 256)
                        {
                            // End-of-block: leave the inner loop.
                            break;
                        }
                        else if (sym is >= 257 and <= 285)
                        {
                            // Back-reference: decode length, then distance.
                            var idx = sym - 257;
                            if (idx >= DeflateTable.Length.Length)
                                throw new InvalidDataException($"deflate: invalid length sym {sym}");

                            var (baseLen, extraLenBits) = DeflateTable.Length[idx];
                            var extraLen = extraLenBits > 0
                                ? (br.ReadLsb(extraLenBits) ?? throw new InvalidDataException("deflate: EOF reading length extra"))
                                : 0;
                            var matchLen = baseLen + extraLen;

                            // Distance code is always 5 bits, read MSB-first.
                            var distCode = br.ReadMsb(5)
                                ?? throw new InvalidDataException("deflate: EOF reading distance code");
                            if (distCode >= DeflateTable.Dist.Length)
                                throw new InvalidDataException($"deflate: invalid dist code {distCode}");

                            var (baseDist, extraDistBits) = DeflateTable.Dist[distCode];
                            var extraDist = extraDistBits > 0
                                ? (br.ReadLsb(extraDistBits) ?? throw new InvalidDataException("deflate: EOF reading distance extra"))
                                : 0;
                            var offset = baseDist + extraDist;

                            if (offset > output.Count)
                                throw new InvalidDataException(
                                    $"deflate: back-ref offset {offset} > output len {output.Count}");
                            if (output.Count + matchLen > MaxOutputBytes)
                                throw new InvalidDataException("deflate: output size limit exceeded");

                            // Copy byte-by-byte to handle overlapping matches.
                            // Example: offset=1, length=4 expands a single byte into a run of 4.
                            for (var i = 0; i < matchLen; i++)
                            {
                                output.Add(output[output.Count - offset]);
                            }
                        }
                        else
                        {
                            throw new InvalidDataException($"deflate: invalid LL symbol {sym}");
                        }
                    }
                    break;

                case 0b10:
                    throw new InvalidDataException("deflate: dynamic Huffman blocks (BTYPE=10) not supported");

                default:
                    throw new InvalidDataException("deflate: reserved BTYPE=11");
            }

            if (bfinal == 1) break;
        }

        return [.. output];
    }
}

// =============================================================================
// Public API — ZipEntry
// =============================================================================

/// <summary>
/// A single file or directory entry in a ZIP archive.
/// </summary>
/// <param name="Name">The entry name (UTF-8). Directory entries end with '/'.</param>
/// <param name="Data">The uncompressed file bytes. Empty for directories.</param>
public record ZipEntry(string Name, byte[] Data);

// =============================================================================
// ZIP Write — ZipWriter
// =============================================================================
//
// ZipWriter accumulates entries in memory: it writes a Local File Header + data
// for each entry immediately, records the metadata needed for the Central
// Directory, and assembles the final archive in Finish().
//
// Auto-compression policy (per-entry):
//   - If compress=true and compressed < original → use method=8 (DEFLATE).
//   - Otherwise → use method=0 (Stored).
//   Common cases that fall back to Stored: empty files, already-compressed
//   formats (JPEG, PNG, nested ZIP), random data.

/// <summary>
/// Builds a ZIP archive incrementally in memory.
/// </summary>
/// <example>
/// <code>
/// var w = new ZipWriter();
/// w.AddFile("hello.txt", Encoding.UTF8.GetBytes("hello, world!"));
/// w.AddDirectory("mydir/");
/// byte[] archive = w.Finish();
/// </code>
/// </example>
public sealed class ZipWriter
{
    // Central Directory records accumulated during AddFile / AddDirectory calls.
    private readonly List<CdRecord> _entries = [];

    // Raw bytes of the archive so far (Local Headers + file data).
    private readonly List<byte> _buf = [];

    // Metadata saved per entry so we can write the Central Directory at the end.
    private sealed class CdRecord
    {
        public byte[] Name = [];
        public ushort Method;
        public uint DosDt;
        public uint Crc;
        public uint CompressedSize;
        public uint UncompressedSize;
        public uint LocalOffset;
        public uint ExternalAttrs;
    }

    /// <summary>
    /// Add a file entry.
    /// If <paramref name="compress"/> is true, DEFLATE is attempted; the
    /// compressed form is used only if it is strictly smaller than the original.
    /// </summary>
    public void AddFile(string name, byte[] data, bool compress = true)
    {
        ArgumentNullException.ThrowIfNull(name);
        ArgumentNullException.ThrowIfNull(data);
        AddEntry(name, data, compress, ZipConstants.UnixModeFile);
    }

    /// <summary>
    /// Add a directory entry. <paramref name="name"/> must end with '/'.
    /// </summary>
    public void AddDirectory(string name)
    {
        ArgumentNullException.ThrowIfNull(name);
        AddEntry(name, [], compress: false, ZipConstants.UnixModeDir);
    }

    // Internal: write one entry (file or directory) with the given Unix mode.
    private void AddEntry(string name, byte[] data, bool compress, uint unixMode)
    {
        var nameBytes = Encoding.UTF8.GetBytes(name);
        var crc = Crc32Helper.Compute(data);
        var uncompressedSize = (uint)data.Length;

        // Decide compression: try DEFLATE and fall back to Stored if it doesn't help.
        ushort method;
        byte[] fileData;

        if (compress && data.Length > 0)
        {
            var compressed = DeflateCompressor.Compress(data);
            if (compressed.Length < data.Length)
            {
                method = ZipConstants.MethodDeflate;
                fileData = compressed;
            }
            else
            {
                // DEFLATE made it larger (random or already-compressed data) — store raw.
                method = ZipConstants.MethodStored;
                fileData = data;
            }
        }
        else
        {
            method = ZipConstants.MethodStored;
            fileData = data;
        }

        var compressedSize = (uint)fileData.Length;
        var localOffset = (uint)_buf.Count;
        var versionNeeded = method == ZipConstants.MethodDeflate
            ? ZipConstants.VersionDeflate : ZipConstants.VersionStored;

        // ── Local File Header (30 bytes fixed + variable name + data) ──────────
        // All integers are little-endian per the ZIP specification.
        WriteU32(ZipConstants.LocalSig);
        WriteU16(versionNeeded);
        WriteU16(ZipConstants.FlagUtf8);              // bit 11 = UTF-8 filename
        WriteU16(method);
        WriteU16((ushort)(ZipConstants.DosEpoch & 0xFFFF));        // mod_time
        WriteU16((ushort)(ZipConstants.DosEpoch >> 16));           // mod_date
        WriteU32(crc);
        WriteU32(compressedSize);
        WriteU32(uncompressedSize);
        WriteU16((ushort)nameBytes.Length);
        WriteU16(0);                                  // extra_field_length = 0
        _buf.AddRange(nameBytes);
        _buf.AddRange(fileData);

        // Save metadata for the Central Directory pass in Finish().
        _entries.Add(new CdRecord
        {
            Name = nameBytes,
            Method = method,
            DosDt = ZipConstants.DosEpoch,
            Crc = crc,
            CompressedSize = compressedSize,
            UncompressedSize = uncompressedSize,
            LocalOffset = localOffset,
            ExternalAttrs = unixMode << 16,
        });
    }

    /// <summary>
    /// Finish writing: append the Central Directory and EOCD record, then
    /// return the complete archive as a byte array.
    /// </summary>
    public byte[] Finish()
    {
        var cdOffset = (uint)_buf.Count;

        // ── Central Directory Headers ──────────────────────────────────────────
        // One 46-byte fixed record per entry, followed by the variable-length name.
        var cdStart = _buf.Count;
        foreach (var e in _entries)
        {
            var versionNeeded = e.Method == ZipConstants.MethodDeflate
                ? ZipConstants.VersionDeflate : ZipConstants.VersionStored;

            WriteU32(ZipConstants.CdSig);
            WriteU16(ZipConstants.VersionMadeBy);
            WriteU16(versionNeeded);
            WriteU16(ZipConstants.FlagUtf8);
            WriteU16(e.Method);
            WriteU16((ushort)(e.DosDt & 0xFFFF));         // mod_time
            WriteU16((ushort)(e.DosDt >> 16));            // mod_date
            WriteU32(e.Crc);
            WriteU32(e.CompressedSize);
            WriteU32(e.UncompressedSize);
            WriteU16((ushort)e.Name.Length);
            WriteU16(0);                                   // extra_len = 0
            WriteU16(0);                                   // comment_len = 0
            WriteU16(0);                                   // disk_start = 0
            WriteU16(0);                                   // internal_attrs = 0
            WriteU32(e.ExternalAttrs);
            WriteU32(e.LocalOffset);
            _buf.AddRange(e.Name);
            // (no extra field, no file comment)
        }
        var cdSize = (uint)(_buf.Count - cdStart);
        var numEntries = (ushort)_entries.Count;

        // ── End of Central Directory Record (22 bytes) ─────────────────────────
        WriteU32(ZipConstants.EocdSig);
        WriteU16(0);             // disk_number = 0
        WriteU16(0);             // disk_with_cd_start = 0
        WriteU16(numEntries);    // entries_on_this_disk
        WriteU16(numEntries);    // entries_total
        WriteU32(cdSize);        // Central Directory byte size
        WriteU32(cdOffset);      // Central Directory byte offset from archive start
        WriteU16(0);             // comment_length = 0

        return [.. _buf];
    }

    // ── Helper write methods (little-endian) ───────────────────────────────────

    private void WriteU16(ushort v)
    {
        Span<byte> tmp = stackalloc byte[2];
        BinaryPrimitives.WriteUInt16LittleEndian(tmp, v);
        _buf.Add(tmp[0]); _buf.Add(tmp[1]);
    }

    private void WriteU32(uint v)
    {
        Span<byte> tmp = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32LittleEndian(tmp, v);
        _buf.Add(tmp[0]); _buf.Add(tmp[1]); _buf.Add(tmp[2]); _buf.Add(tmp[3]);
    }
}

// =============================================================================
// ZIP Read — ZipReader
// =============================================================================
//
// Strategy (EOCD-first):
//   1. Scan backwards from end of archive for the EOCD signature 0x06054B50.
//      Limit scan to the last 65557 bytes (EOCD 22 + max ZIP comment 65535).
//   2. Read cd_offset + cd_size from EOCD.
//   3. Parse all Central Directory headers into internal metadata.
//   4. Expose an IReadOnlyList<ZipEntry> of names (no data loaded yet).
//   5. Read(name): seek to Local Header via local_offset, skip name+extra,
//      read compressed_size bytes, decompress, verify CRC-32.
//
// Security: use Central Directory as the authoritative source for sizes and
// method. Local Header is consulted only for the variable-length name_len +
// extra_len skip. This prevents malformed Local Headers from causing over-reads.

/// <summary>
/// Reads entries from an in-memory ZIP archive.
/// </summary>
public sealed class ZipReader
{
    // Raw archive bytes (kept alive for deferred reads).
    private readonly byte[] _data;

    // Parsed entry metadata from the Central Directory.
    private readonly List<ZipEntryMeta> _meta = [];

    // Cached decoded ZipEntry list (names + empty data) exposed to callers.
    private readonly List<ZipEntry> _entries;

    // Internal: full metadata per entry needed for lazy reads.
    private sealed class ZipEntryMeta
    {
        public string Name = "";
        public uint LocalOffset;
        public ushort Method;
        public uint Crc;
        public uint CompressedSize;
        public uint UncompressedSize;
        public bool IsDirectory;
    }

    /// <summary>
    /// Parse an in-memory ZIP archive.
    /// Throws <see cref="InvalidDataException"/> if no valid EOCD is found.
    /// </summary>
    public ZipReader(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        _data = data;

        var eocdOffset = FindEocd(data)
            ?? throw new InvalidDataException("zip: no End of Central Directory record found");

        // Read EOCD fields: cd_size at +12, cd_offset at +16.
        var cdOffset = (int)ReadU32(data, eocdOffset + 16);
        var cdSize   = (int)ReadU32(data, eocdOffset + 12);

        if (cdOffset + cdSize > data.Length)
            throw new InvalidDataException(
                $"zip: Central Directory [{cdOffset}, {cdOffset + cdSize}) out of bounds (file size {data.Length})");

        // Parse Central Directory headers.
        var pos = cdOffset;
        while (pos + 4 <= cdOffset + cdSize)
        {
            var sig = ReadU32(data, pos);
            if (sig != ZipConstants.CdSig) break; // end of CD or padding

            var method          = ReadU16(data, pos + 10);
            var crc             = ReadU32(data, pos + 16);
            var compressedSize  = ReadU32(data, pos + 20);
            var uncompressedSize= ReadU32(data, pos + 24);
            var nameLen         = ReadU16(data, pos + 28);
            var extraLen        = ReadU16(data, pos + 30);
            var commentLen      = ReadU16(data, pos + 32);
            var localOffset     = ReadU32(data, pos + 42);

            var nameStart = pos + 46;
            var nameEnd   = nameStart + nameLen;
            if (nameEnd > data.Length)
                throw new InvalidDataException("zip: CD entry name out of bounds");

            var name = Encoding.UTF8.GetString(data, nameStart, nameLen);
            _meta.Add(new ZipEntryMeta
            {
                Name = name,
                LocalOffset = localOffset,
                Method = method,
                Crc = crc,
                CompressedSize = compressedSize,
                UncompressedSize = uncompressedSize,
                IsDirectory = name.EndsWith('/'),
            });

            pos = nameEnd + extraLen + commentLen;
        }

        // Build the public ZipEntry list (name only; data read on demand).
        _entries = _meta.Select(m => new ZipEntry(m.Name, [])).ToList();
    }

    /// <summary>
    /// All entries in the archive (files and directories) in Central Directory order.
    /// The <see cref="ZipEntry.Data"/> field is empty until you call <see cref="Read"/>.
    /// </summary>
    public IReadOnlyList<ZipEntry> Entries => _entries;

    /// <summary>
    /// Decompress and return the data for the named entry.
    /// Throws <see cref="InvalidDataException"/> on CRC mismatch or corrupt data.
    /// </summary>
    public byte[] Read(string name)
    {
        var meta = _meta.FirstOrDefault(m => m.Name == name)
            ?? throw new InvalidDataException($"zip: entry '{name}' not found");
        return ReadEntry(meta);
    }

    /// <summary>
    /// Alias for <see cref="Read(string)"/> — decompress entry by name.
    /// </summary>
    public byte[] ReadByName(string name) => Read(name);

    // Internal: read and decompress one entry using its local_offset.
    private byte[] ReadEntry(ZipEntryMeta meta)
    {
        if (meta.IsDirectory) return [];

        var lhOff = (int)meta.LocalOffset;

        // Reject encrypted entries (GP flag bit 0 = 1).
        var localFlags = ReadU16(_data, lhOff + 6);
        if ((localFlags & 1) != 0)
            throw new InvalidDataException($"zip: entry '{meta.Name}' is encrypted; not supported");

        // The Local Header name_len and extra_len can differ from the CD header,
        // so we must re-read them to find the actual start of the file data.
        var lhNameLen  = ReadU16(_data, lhOff + 26);
        var lhExtraLen = ReadU16(_data, lhOff + 28);
        var dataStart  = lhOff + 30 + lhNameLen + lhExtraLen;
        var dataEnd    = dataStart + (int)meta.CompressedSize;

        if (dataEnd > _data.Length)
            throw new InvalidDataException(
                $"zip: entry '{meta.Name}' data [{dataStart}, {dataEnd}) out of bounds");

        var compressed = _data[dataStart..dataEnd];

        // Decompress according to method.
        byte[] decompressed = meta.Method switch
        {
            0 => compressed,                              // Stored — verbatim
            8 => DeflateDecompressor.Decompress(compressed),
            var m => throw new InvalidDataException(
                $"zip: unsupported compression method {m} for '{meta.Name}'"),
        };

        // Trim to declared uncompressed size to guard against decompressor over-read.
        if (decompressed.Length > (int)meta.UncompressedSize)
            decompressed = decompressed[..(int)meta.UncompressedSize];

        // Verify CRC-32 — this detects corruption of the decompressed content.
        var actualCrc = Crc32Helper.Compute(decompressed);
        if (actualCrc != meta.Crc)
            throw new InvalidDataException(
                $"zip: CRC-32 mismatch for '{meta.Name}': expected {meta.Crc:X8}, got {actualCrc:X8}");

        return decompressed;
    }

    // ── EOCD search ────────────────────────────────────────────────────────────
    //
    // Scan backwards from the end of the file for the EOCD signature 0x06054B50.
    // Limit the scan to the last 65557 bytes (22-byte minimum EOCD + 65535-byte
    // maximum ZIP comment) to prevent unbounded searches on malformed archives.

    private static int? FindEocd(byte[] data)
    {
        const int EocdMinSize  = 22;
        const int MaxComment   = 65535;

        if (data.Length < EocdMinSize) return null;

        var scanStart = Math.Max(0, data.Length - EocdMinSize - MaxComment);

        for (var i = data.Length - EocdMinSize; i >= scanStart; i--)
        {
            if (ReadU32(data, i) != ZipConstants.EocdSig) continue;

            // Validate: comment_len at offset +20 must account for all remaining bytes.
            var commentLen = ReadU16(data, i + 20);
            if (i + EocdMinSize + commentLen == data.Length)
                return i;
        }
        return null;
    }

    // ── Little-endian readers ──────────────────────────────────────────────────

    private static ushort ReadU16(byte[] data, int offset)
    {
        if (offset + 2 > data.Length)
            throw new InvalidDataException($"zip: read U16 at {offset} out of bounds");
        return BinaryPrimitives.ReadUInt16LittleEndian(data.AsSpan(offset, 2));
    }

    private static uint ReadU32(byte[] data, int offset)
    {
        if (offset + 4 > data.Length)
            throw new InvalidDataException($"zip: read U32 at {offset} out of bounds");
        return BinaryPrimitives.ReadUInt32LittleEndian(data.AsSpan(offset, 4));
    }
}

// =============================================================================
// Convenience API — ZipArchive
// =============================================================================

/// <summary>
/// Convenience functions for one-shot ZIP archive creation and extraction.
/// </summary>
public static class ZipArchive
{
    /// <summary>
    /// Compress a collection of <see cref="ZipEntry"/> objects into a ZIP archive.
    /// Each entry is compressed with DEFLATE if that reduces size, otherwise stored.
    /// </summary>
    public static byte[] Zip(IEnumerable<ZipEntry> entries)
    {
        ArgumentNullException.ThrowIfNull(entries);
        var writer = new ZipWriter();
        foreach (var entry in entries)
        {
            if (entry.Name.EndsWith('/'))
                writer.AddDirectory(entry.Name);
            else
                writer.AddFile(entry.Name, entry.Data);
        }
        return writer.Finish();
    }

    /// <summary>
    /// Extract all file entries from a ZIP archive.
    /// Directory entries are included with empty <see cref="ZipEntry.Data"/>.
    /// Throws <see cref="InvalidDataException"/> on corrupt archives.
    /// </summary>
    public static IReadOnlyList<ZipEntry> Unzip(byte[] data)
    {
        ArgumentNullException.ThrowIfNull(data);
        var reader = new ZipReader(data);
        var result = new List<ZipEntry>();
        foreach (var entry in reader.Entries)
        {
            var bytes = entry.Name.EndsWith('/') ? [] : reader.Read(entry.Name);
            result.Add(new ZipEntry(entry.Name, bytes));
        }
        return result;
    }
}
