package com.codingadventures.zip;

// CMP09 — ZIP archive format (PKZIP, 1989).
//
// ZIP bundles one or more files into a single `.zip` archive, compressing
// each entry independently with DEFLATE (method 8) or storing it verbatim
// (method 0). The same container format underlies Java JARs, Office Open XML
// (.docx), Android APKs, Python wheels, and many other formats.
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
//   - Sequential write: append Local Headers + data, write CD at end.
//   - Random-access read: seek to EOCD, read CD, jump to any entry.
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

import com.codingadventures.lzss.Lzss;
import com.codingadventures.lzss.LzssToken;

import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

// =============================================================================
// Wire Format Constants
// =============================================================================
//
// ZIP uses four-byte "magic number" signatures to identify each structural
// region. All integers in the wire format are little-endian.

/**
 * ZIP archive format implementation (CMP09).
 *
 * <p>Provides {@link ZipWriter} for creating archives, {@link ZipReader} for
 * reading them, and convenience functions {@link #zip} / {@link #unzip}
 * for one-shot use.</p>
 *
 * <h2>Quick example</h2>
 *
 * <pre>{@code
 * // Write
 * ZipWriter w = new ZipWriter();
 * w.addFile("hello.txt", "hello, world!".getBytes(UTF_8), true);
 * byte[] archive = w.finish();
 *
 * // Read
 * ZipReader r = new ZipReader(archive);
 * byte[] data = r.read("hello.txt");
 * }</pre>
 */
public final class Zip {

    // ── Wire format magic signatures ───────────────────────────────────────────

    /** Local File Header signature: "PK\x03\x04" */
    static final int LOCAL_SIG = 0x04034B50;

    /** Central Directory Header signature: "PK\x01\x02" */
    static final int CD_SIG = 0x02014B50;

    /** End of Central Directory Record signature: "PK\x05\x06" */
    static final int EOCD_SIG = 0x06054B50;

    /**
     * Fixed timestamp: DOS epoch 1980-01-01 00:00:00.
     *
     * <p>DOS dates encode: year-1980 in bits 25-31, month in bits 21-24,
     * day in bits 16-20, hours in bits 11-15, minutes in bits 5-10,
     * seconds/2 in bits 0-4.  1980-01-01 00:00:00 → 0x00210000.</p>
     */
    static final int DOS_EPOCH = 0x00210000;

    /**
     * General Purpose Bit Flag: bit 11 = UTF-8 filename encoding.
     *
     * <p>Setting this flag tells extractors that the filename and comment
     * are encoded as UTF-8 rather than the ZIP spec's default CP437.
     * All modern tools respect it.</p>
     */
    static final short FLAGS = (short) 0x0800;

    /**
     * Version needed to extract: 2.0, required for DEFLATE-compressed entries.
     *
     * <p>Encoded as (major * 10 + minor), so 2.0 → 20.</p>
     */
    static final short VERSION_DEFLATE = 20;

    /** Version needed to extract: 1.0, sufficient for stored (uncompressed) entries. */
    static final short VERSION_STORED = 10;

    /**
     * Version made by: Unix (high byte 3), specification version 3.0 (low byte 30 = 0x1E).
     *
     * <p>The "version made by" field in the Central Directory tells tools what
     * OS created the archive and what spec version was followed.  0x031E = Unix/3.0.</p>
     */
    static final short VERSION_MADE_BY = (short) 0x031E;

    // Unix file attribute mode constants, embedded in Central Directory external_attrs
    // (shifted left 16 bits to place them in the Unix attribute half).
    //
    //   0o100644 = regular file, rw-r--r-- → decimal 33188
    //   0o040755 = directory,   rwxr-xr-x  → decimal 16877

    static final int UNIX_MODE_FILE = 33188; // 0o100644
    static final int UNIX_MODE_DIR  = 16877; // 0o040755

    // Compression method codes used in Local and Central Directory headers.
    static final short METHOD_STORED  = 0;
    static final short METHOD_DEFLATE = 8;

    // Private constructor — all public state is in inner classes.
    private Zip() {}

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

    /**
     * CRC-32 checksum using the standard ZIP polynomial 0xEDB88320.
     *
     * <p>The lookup table is computed once at class load time.  Each entry is
     * the CRC-32 of a single byte value using the reflected polynomial, following
     * the "table-driven CRC" algorithm described in RFC 1952 §8.</p>
     *
     * <p>Reflected polynomial means we process bits LSB-first, which is the
     * standard for Ethernet/ZIP/gzip CRC-32.</p>
     */
    static final class Crc32 {

        // 256-entry lookup table, built once at class-load time.
        private static final long[] TABLE = buildTable();

        private static long[] buildTable() {
            long[] t = new long[256];
            for (int i = 0; i < 256; i++) {
                long c = i & 0xFFFFFFFFL;
                for (int k = 0; k < 8; k++) {
                    // If LSB is set, XOR with the reflected polynomial.
                    c = (c & 1L) != 0 ? (0xEDB88320L ^ (c >>> 1)) : (c >>> 1);
                }
                t[i] = c;
            }
            return t;
        }

        /**
         * Compute CRC-32 over {@code data}.
         *
         * <p>Pass {@code initial = 0} for a fresh hash, or the previous result
         * to continue an incremental computation.</p>
         *
         * @param data    bytes to hash
         * @param initial previous CRC (0 for fresh)
         * @return 32-bit CRC as an unsigned value in a {@code long}
         */
        static long compute(byte[] data, long initial) {
            // XOR-in the initial value (the standard 0xFFFFFFFF initialisation
            // technique for CRC-32).
            long crc = initial ^ 0xFFFFFFFFL;
            for (byte b : data) {
                crc = TABLE[(int)((crc ^ b) & 0xFF)] ^ (crc >>> 8);
            }
            // XOR-out to produce the final CRC.
            return crc ^ 0xFFFFFFFFL;
        }

        /** Compute CRC-32 over {@code data} from scratch. */
        static long compute(byte[] data) {
            return compute(data, 0L);
        }

        private Crc32() {}
    }

    // =============================================================================
    // RFC 1951 DEFLATE — Bit Writer (LSB-first)
    // =============================================================================
    //
    // RFC 1951 packs bits LSB-first within bytes.  Huffman codes are logically
    // MSB-first, so before writing one we bit-reverse it and write the reversed
    // value LSB-first into the stream.  Extra bits (length/distance extras,
    // stored block headers) are written directly in LSB-first order without reversal.

    /**
     * Accumulates bits into a byte stream, LSB-first (RFC 1951 §3.1.1).
     *
     * <p>Uses a {@code long} register that holds up to 63 unflushable bits.
     * Complete bytes are drained into the output buffer as they fill.</p>
     */
    static final class BitWriter {

        private final List<Byte> buf = new ArrayList<>();
        private long reg;   // accumulator — up to 63 unflushable bits
        private int bits;   // how many bits are currently valid in reg

        /**
         * Write the {@code n} low-order bits of {@code val} into the stream,
         * LSB-first.  Used for extra bits and block headers.
         *
         * @param val value whose bottom {@code n} bits are written
         * @param n   number of bits (1–57 is safe given the 63-bit register)
         */
        void addBits(long val, int n) {
            // Mask to the bottom n bits, then OR into the accumulator at the
            // current fill position.
            reg |= (val & ((1L << n) - 1)) << bits;
            bits += n;
            // Drain complete bytes.
            while (bits >= 8) {
                buf.add((byte)(reg & 0xFF));
                reg >>>= 8;
                bits -= 8;
            }
        }

        /**
         * Write a Huffman code of {@code nbits} bits.
         *
         * <p>Huffman codes are logically MSB-first (the shortest codes represent
         * the most common symbols, and "short" means the high bits dominate).
         * Before storing LSB-first we bit-reverse the code.</p>
         *
         * @param code  the Huffman code value
         * @param nbits number of bits in the code
         */
        void writeHuffman(long code, int nbits) {
            long reversed = reverseBits(code, nbits);
            addBits(reversed, nbits);
        }

        /**
         * Reverse the bottom {@code nbits} bits of {@code code}.
         *
         * <p>Example: {@code reverseBits(0b110, 3)} returns {@code 0b011}.</p>
         */
        private static long reverseBits(long code, int nbits) {
            long rev = 0;
            for (int i = 0; i < nbits; i++) {
                rev = (rev << 1) | (code & 1);
                code >>>= 1;
            }
            return rev;
        }

        /**
         * Flush any partial byte to the buffer (zero-padding the remaining bits).
         *
         * <p>Required before writing stored-block headers, which must be
         * byte-aligned per RFC 1951 §3.2.4.</p>
         */
        void flush() {
            if (bits > 0) {
                buf.add((byte)(reg & 0xFF));
                reg = 0;
                bits = 0;
            }
        }

        /**
         * Flush and return the completed byte array.
         *
         * @return all bytes written so far, plus any partial final byte (zero-padded)
         */
        byte[] toArray() {
            flush();
            byte[] out = new byte[buf.size()];
            for (int i = 0; i < out.length; i++) {
                out[i] = buf.get(i);
            }
            return out;
        }
    }

    // =============================================================================
    // RFC 1951 DEFLATE — Bit Reader (LSB-first)
    // =============================================================================
    //
    // Mirrors BitWriter: fill an accumulator from source bytes (LSB-first), then
    // extract requested counts of bits.  Huffman code decoding reads MSB-first
    // by bit-reversing the extracted value.

    /**
     * Reads bits from a byte array, LSB-first (RFC 1951 §3.1.1).
     *
     * <p>An internal accumulator buffer is refilled lazily as bits are consumed.</p>
     */
    static final class BitReader {

        private final byte[] data;
        private int pos;    // next byte to consume from data
        private long buf;   // bit accumulator
        private int bits;   // valid bits in buf

        BitReader(byte[] data) {
            this.data = data;
        }

        /**
         * Ensure the accumulator holds at least {@code need} bits.
         *
         * @return {@code false} if the source is exhausted before that many bits
         */
        private boolean fill(int need) {
            while (bits < need) {
                if (pos >= data.length) return false;
                buf |= (long)(data[pos++] & 0xFF) << bits;
                bits += 8;
            }
            return true;
        }

        /**
         * Read {@code nbits} bits LSB-first.  Returns -1 on end-of-input.
         *
         * <p>Returning -1 (rather than throwing) lets callers distinguish
         * normal EOF from a parse error.</p>
         *
         * @param nbits number of bits to read (0–32 safe)
         * @return extracted value, or -1 on EOF
         */
        int readLsb(int nbits) {
            if (nbits == 0) return 0;
            if (!fill(nbits)) return -1;
            long mask = (1L << nbits) - 1;
            int val = (int)(buf & mask);
            buf >>>= nbits;
            bits -= nbits;
            return val;
        }

        /**
         * Read {@code nbits} bits and bit-reverse the result.
         *
         * <p>Used when decoding Huffman codes, which are logically MSB-first.</p>
         *
         * @param nbits number of bits to read
         * @return bit-reversed extracted value, or -1 on EOF
         */
        int readMsb(int nbits) {
            int v = readLsb(nbits);
            if (v < 0) return -1;
            // Reverse the bottom nbits bits.
            int rev = 0;
            int u = v;
            for (int i = 0; i < nbits; i++) {
                rev = (rev << 1) | (u & 1);
                u >>>= 1;
            }
            return rev;
        }

        /**
         * Discard partial-byte bits, aligning to the next byte boundary.
         *
         * <p>Required before reading stored-block length fields (RFC 1951 §3.2.4).</p>
         */
        void align() {
            int discard = bits % 8;
            if (discard > 0) {
                buf >>>= discard;
                bits -= discard;
            }
        }
    }

    // =============================================================================
    // RFC 1951 DEFLATE — Fixed Huffman Tables
    // =============================================================================
    //
    // RFC 1951 §3.2.6 defines a "fixed" Huffman alphabet that encoder and decoder
    // both know in advance.  Using BTYPE=01 (fixed Huffman) means we never need
    // to transmit code-length tables, keeping this implementation simple.
    //
    // Literal/Length code lengths:
    //   Symbols   0–143: 8-bit codes, base 0x30 (0b00110000)
    //   Symbols 144–255: 9-bit codes, base 0x190 (0b110010000)
    //   Symbols 256–279: 7-bit codes, base 0x00
    //   Symbols 280–287: 8-bit codes, base 0xC0 (0b11000000)
    //
    // Distance codes: 5-bit codes equal to the code number (0–29).

    /** Encode/decode helpers for the RFC 1951 fixed Huffman alphabet. */
    static final class FixedHuffman {

        private FixedHuffman() {}

        /**
         * Return the (code, nbits) pair for encoding literal/length symbol 0–287.
         *
         * <p>The four ranges come directly from RFC 1951 §3.2.6 Table 1.</p>
         *
         * @param sym literal/length symbol (0–287)
         * @return {@code int[]{code, nbits}}
         * @throws IOException if {@code sym} is out of the valid range
         */
        static int[] encodeLL(int sym) throws IOException {
            if (sym >= 0 && sym <= 143) {
                // 8-bit codes: 0x30 … 0xBF
                return new int[]{sym + 0x30, 8};
            } else if (sym >= 144 && sym <= 255) {
                // 9-bit codes: 0x190 … 0x1FF
                return new int[]{sym - 144 + 0x190, 9};
            } else if (sym >= 256 && sym <= 279) {
                // 7-bit codes: 0x00 … 0x17
                return new int[]{sym - 256, 7};
            } else if (sym >= 280 && sym <= 287) {
                // 8-bit codes: 0xC0 … 0xC7
                return new int[]{sym - 280 + 0xC0, 8};
            } else {
                throw new IOException("FixedHuffman.encodeLL: invalid symbol " + sym);
            }
        }

        /**
         * Decode one literal/length symbol from {@code br} using the fixed table.
         *
         * <p>Reads incrementally: 7 bits first (covers 256–279), then 8 bits
         * (covers 0–143, 280–287), then 9 bits (covers 144–255).</p>
         *
         * @param br source bit reader
         * @return decoded symbol (0–287), or -1 on EOF
         */
        static int decodeLL(BitReader br) {
            // Try 7 bits first (covers symbols 256–279, codes 0–23).
            int v7 = br.readMsb(7);
            if (v7 < 0) return -1;

            if (v7 <= 23) {
                // 7-bit code → symbols 256–279 (EOB + length codes).
                return v7 + 256;
            }

            // Need one more bit to reach 8-bit codes.
            int extra1 = br.readLsb(1);
            if (extra1 < 0) return -1;
            int v8 = (v7 << 1) | extra1;

            if (v8 >= 48 && v8 <= 191) {
                // 8-bit codes: literals 0–143  (0x30 … 0xBF).
                return v8 - 48;
            }
            if (v8 >= 192 && v8 <= 199) {
                // 8-bit codes: symbols 280–287 (0xC0 … 0xC7).
                return v8 + 88;
            }

            // Need one more bit for 9-bit codes (literals 144–255).
            int extra2 = br.readLsb(1);
            if (extra2 < 0) return -1;
            int v9 = (v8 << 1) | extra2;

            if (v9 >= 400 && v9 <= 511) {
                // 9-bit codes: literals 144–255 (0x190 … 0x1FF).
                return v9 - 256;
            }

            return -1; // malformed bit-stream
        }
    }

    // =============================================================================
    // RFC 1951 DEFLATE — Length / Distance Tables
    // =============================================================================
    //
    // Match lengths (3–258 bytes) map to LL symbols 257–285 plus extra bits.
    // Match distances (1–32768 bytes) map to distance codes 0–29 plus extra bits.
    // The tables below come directly from RFC 1951 §3.2.5.

    /** Length/distance table lookup for DEFLATE. */
    static final class DeflateTable {

        private DeflateTable() {}

        // (baseLength, extraBits) indexed by (LL_symbol - 257).
        // Symbol 285 has base 258 with 0 extra bits (special case for max match).
        static final int[][] LENGTH = {
            {3,0},{4,0},{5,0},{6,0},{7,0},{8,0},{9,0},{10,0}, // 257–264
            {11,1},{13,1},{15,1},{17,1},                       // 265–268
            {19,2},{23,2},{27,2},{31,2},                       // 269–272
            {35,3},{43,3},{51,3},{59,3},                       // 273–276
            {67,4},{83,4},{99,4},{115,4},                      // 277–280
            {131,5},{163,5},{195,5},{227,5},{258,0},           // 281–285
        };

        // (baseDistance, extraBits) indexed by distance code 0–29.
        static final int[][] DIST = {
            {1,0},{2,0},{3,0},{4,0},
            {5,1},{7,1},{9,2},{13,2},
            {17,3},{25,3},{33,4},{49,4},
            {65,5},{97,5},{129,6},{193,6},
            {257,7},{385,7},{513,8},{769,8},
            {1025,9},{1537,9},{2049,10},{3073,10},
            {4097,11},{6145,11},{8193,12},{12289,12},
            {16385,13},{24577,13},
        };

        /**
         * Encode a match {@code length} (3–258) as an RFC 1951 LL symbol plus
         * extra bits.
         *
         * @param length match length (3–258)
         * @return {@code int[]{llSymbol, base, extraBitCount}}
         * @throws IOException if length is out of range
         */
        static int[] encodeLength(int length) throws IOException {
            // Walk the table from the highest entry downward to find the right slot.
            for (int i = LENGTH.length - 1; i >= 0; i--) {
                if (length >= LENGTH[i][0]) {
                    return new int[]{257 + i, LENGTH[i][0], LENGTH[i][1]};
                }
            }
            throw new IOException("encodeLength: unreachable for length=" + length);
        }

        /**
         * Encode a match {@code distance} (1–32768) as an RFC 1951 distance
         * code plus extra bits.
         *
         * @param distance match distance (1–32768)
         * @return {@code int[]{distCode, base, extraBitCount}}
         * @throws IOException if distance is out of range
         */
        static int[] encodeDist(int distance) throws IOException {
            for (int i = DIST.length - 1; i >= 0; i--) {
                if (distance >= DIST[i][0]) {
                    return new int[]{i, DIST[i][0], DIST[i][1]};
                }
            }
            throw new IOException("encodeDist: unreachable for distance=" + distance);
        }
    }

    // =============================================================================
    // RFC 1951 DEFLATE — Compressor
    // =============================================================================
    //
    // Strategy:
    //   1. Run LZSS match-finding (window=32768, maxMatch=255, minMatch=3).
    //   2. Emit a single BTYPE=01 (fixed Huffman) block over all tokens.
    //   3. Literals → fixed LL Huffman code.
    //   4. Matches  → length LL code + extra bits + distance code + extra bits.
    //   5. End-of-block symbol 256.
    //
    // For empty input we emit a stored block (BTYPE=00) — the canonical
    // representation for zero bytes in raw DEFLATE.

    /**
     * Compresses bytes to raw RFC 1951 DEFLATE (no zlib wrapper).
     *
     * <p>Uses fixed Huffman (BTYPE=01) for non-empty input.  Empty input
     * produces a minimal stored block.</p>
     */
    static final class DeflateCompressor {

        private DeflateCompressor() {}

        /**
         * Compress {@code data} to raw DEFLATE bytes.
         *
         * @param data input bytes (may be empty)
         * @return raw DEFLATE-compressed bytes
         * @throws IOException on internal encoding error (should not occur for
         *                     valid inputs within the 3–255 match-length range)
         */
        static byte[] compress(byte[] data) throws IOException {
            BitWriter bw = new BitWriter();

            if (data.length == 0) {
                // Empty stored block: BFINAL=1, BTYPE=00, aligned, LEN=0, NLEN=0xFFFF.
                bw.addBits(1, 1); // BFINAL = 1 (this is the last block)
                bw.addBits(0, 2); // BTYPE  = 00 (stored)
                bw.flush();       // align to byte boundary before length fields
                bw.addBits(0x0000L, 16); // LEN  = 0
                bw.addBits(0xFFFFL, 16); // NLEN = one's complement of LEN
                return bw.toArray();
            }

            // Run LZSS tokenization.
            // Window = 32768 so every match distance fits in the RFC 1951 dist table.
            // maxMatch = 255 so every length fits in the length table without overflow.
            List<LzssToken> tokens = Lzss.encode(data, 32768, 255, 3);

            // Block header: BFINAL=1 (single block), BTYPE=01 (fixed Huffman).
            // Bits are written LSB-first: BFINAL in bit 0, BTYPE in bits 1-2.
            bw.addBits(1, 1); // BFINAL = 1
            bw.addBits(1, 1); // BTYPE bit 0 = 1  }
            bw.addBits(0, 1); // BTYPE bit 1 = 0  } → BTYPE = 01 (fixed Huffman)

            for (LzssToken token : tokens) {
                if (token instanceof LzssToken.Literal lit) {
                    // Literal byte: emit the fixed LL code for this byte value.
                    int byteVal = lit.value() & 0xFF;
                    int[] enc = FixedHuffman.encodeLL(byteVal);
                    bw.writeHuffman(enc[0], enc[1]);

                } else if (token instanceof LzssToken.Match match) {
                    // Length: find the LL symbol + extra bits.
                    int[] lenEnc = DeflateTable.encodeLength(match.length());
                    int[] llEnc = FixedHuffman.encodeLL(lenEnc[0]);
                    bw.writeHuffman(llEnc[0], llEnc[1]);
                    if (lenEnc[2] > 0) {
                        bw.addBits(match.length() - lenEnc[1], lenEnc[2]);
                    }

                    // Distance: the 5-bit fixed distance code equals the code number.
                    int[] distEnc = DeflateTable.encodeDist(match.offset());
                    bw.writeHuffman(distEnc[0], 5);
                    if (distEnc[2] > 0) {
                        bw.addBits(match.offset() - distEnc[1], distEnc[2]);
                    }
                }
            }

            // End-of-block symbol (256) — signals the decoder to stop.
            int[] eobEnc = FixedHuffman.encodeLL(256);
            bw.writeHuffman(eobEnc[0], eobEnc[1]);

            return bw.toArray();
        }
    }

    // =============================================================================
    // RFC 1951 DEFLATE — Decompressor
    // =============================================================================
    //
    // Handles stored blocks (BTYPE=00) and fixed Huffman blocks (BTYPE=01).
    // Dynamic Huffman blocks (BTYPE=10) throw IOException — we only write
    // BTYPE=01 ourselves, but stored blocks from other tools must be accepted.
    //
    // Security limits:
    //   - Maximum output: 256 MB (decompression bomb guard)
    //   - LEN/NLEN validation on stored blocks

    /**
     * Decompresses raw RFC 1951 DEFLATE bytes.
     *
     * <p>Security limits: output is capped at 256 MB to guard against
     * decompression bombs.  LEN/NLEN fields on stored blocks are validated.</p>
     */
    static final class DeflateDecompressor {

        /** Maximum allowed decompressed output size (decompression bomb guard). */
        private static final int MAX_OUTPUT_BYTES = 256 * 1024 * 1024;

        private DeflateDecompressor() {}

        /**
         * Decompress raw DEFLATE bytes into the original data.
         *
         * @param data raw DEFLATE-compressed bytes
         * @return decompressed bytes
         * @throws IOException for corrupt or unsupported input
         */
        static byte[] decompress(byte[] data) throws IOException {
            BitReader br = new BitReader(data);
            List<Byte> output = new ArrayList<>();

            while (true) {
                int bfinal = br.readLsb(1);
                if (bfinal < 0) throw new IOException("deflate: unexpected EOF reading BFINAL");
                int btype = br.readLsb(2);
                if (btype < 0) throw new IOException("deflate: unexpected EOF reading BTYPE");

                if (btype == 0b00) {
                    // ── Stored block ──────────────────────────────────────────
                    // Align to byte boundary before reading the length fields.
                    br.align();
                    int len  = br.readLsb(16);
                    int nlen = br.readLsb(16);
                    if (len < 0) throw new IOException("deflate: EOF reading stored LEN");
                    if (nlen < 0) throw new IOException("deflate: EOF reading stored NLEN");
                    // RFC 1951 §3.2.4: NLEN must be the one's complement of LEN.
                    if ((nlen ^ 0xFFFF) != len) {
                        throw new IOException(
                            "deflate: stored LEN/NLEN mismatch (" + len + " vs " + nlen + ")");
                    }
                    if (output.size() + len > MAX_OUTPUT_BYTES) {
                        throw new IOException("deflate: output size limit exceeded");
                    }
                    for (int i = 0; i < len; i++) {
                        int b = br.readLsb(8);
                        if (b < 0) throw new IOException("deflate: EOF inside stored block");
                        output.add((byte) b);
                    }

                } else if (btype == 0b01) {
                    // ── Fixed Huffman block ───────────────────────────────────
                    while (true) {
                        int sym = FixedHuffman.decodeLL(br);
                        if (sym < 0) throw new IOException("deflate: EOF decoding LL symbol");

                        if (sym >= 0 && sym <= 255) {
                            if (output.size() >= MAX_OUTPUT_BYTES) {
                                throw new IOException("deflate: output size limit exceeded");
                            }
                            output.add((byte) sym);

                        } else if (sym == 256) {
                            // End-of-block: leave the inner loop.
                            break;

                        } else if (sym >= 257 && sym <= 285) {
                            // Back-reference: decode length then distance.
                            int idx = sym - 257;
                            if (idx >= DeflateTable.LENGTH.length) {
                                throw new IOException("deflate: invalid length sym " + sym);
                            }

                            int baseLen = DeflateTable.LENGTH[idx][0];
                            int extraLenBits = DeflateTable.LENGTH[idx][1];
                            int extraLen = (extraLenBits > 0) ? br.readLsb(extraLenBits) : 0;
                            if (extraLen < 0) throw new IOException("deflate: EOF reading length extra");
                            int matchLen = baseLen + extraLen;

                            // Distance code is always 5 bits, read MSB-first.
                            int distCode = br.readMsb(5);
                            if (distCode < 0) throw new IOException("deflate: EOF reading distance code");
                            if (distCode >= DeflateTable.DIST.length) {
                                throw new IOException("deflate: invalid dist code " + distCode);
                            }

                            int baseDist = DeflateTable.DIST[distCode][0];
                            int extraDistBits = DeflateTable.DIST[distCode][1];
                            int extraDist = (extraDistBits > 0) ? br.readLsb(extraDistBits) : 0;
                            if (extraDist < 0) throw new IOException("deflate: EOF reading distance extra");
                            int offset = baseDist + extraDist;

                            if (offset > output.size()) {
                                throw new IOException(
                                    "deflate: back-ref offset " + offset +
                                    " > output len " + output.size());
                            }
                            if (output.size() + matchLen > MAX_OUTPUT_BYTES) {
                                throw new IOException("deflate: output size limit exceeded");
                            }

                            // Copy byte-by-byte to handle overlapping matches.
                            // Example: offset=1, length=4 expands one byte into a run of 4.
                            for (int i = 0; i < matchLen; i++) {
                                output.add(output.get(output.size() - offset));
                            }

                        } else {
                            throw new IOException("deflate: invalid LL symbol " + sym);
                        }
                    }

                } else if (btype == 0b10) {
                    throw new IOException(
                        "deflate: dynamic Huffman blocks (BTYPE=10) not supported");
                } else {
                    throw new IOException("deflate: reserved BTYPE=11");
                }

                if (bfinal == 1) break;
            }

            // Materialise List<Byte> → byte[].
            byte[] result = new byte[output.size()];
            for (int i = 0; i < result.length; i++) {
                result[i] = output.get(i);
            }
            return result;
        }
    }

    // =============================================================================
    // Public API — ZipEntry
    // =============================================================================

    /**
     * A single file or directory entry in a ZIP archive.
     *
     * <p>Directory entries have names ending with {@code '/'} and empty data.
     * File entries have names without trailing {@code '/'} and the uncompressed
     * file bytes in {@code data}.</p>
     *
     * @param name entry name (UTF-8, directory entries end with '/')
     * @param data uncompressed file bytes (empty for directories)
     */
    public record ZipEntry(String name, byte[] data) {}

    // =============================================================================
    // ZIP Write — ZipWriter
    // =============================================================================
    //
    // ZipWriter accumulates entries in memory: it writes a Local File Header +
    // data for each entry immediately, records metadata needed for the Central
    // Directory, and assembles the final archive in finish().
    //
    // Auto-compression policy (per-entry):
    //   - If compress=true and compressed < original → use method=8 (DEFLATE).
    //   - Otherwise → use method=0 (Stored).
    //   Common fall-back cases: empty files, already-compressed formats (JPEG,
    //   PNG, nested ZIP), random data.

    /**
     * Builds a ZIP archive incrementally in memory.
     *
     * <p>Call {@link #addFile} / {@link #addDirectory} for each entry, then
     * {@link #finish} to get the complete archive bytes.</p>
     *
     * <pre>{@code
     * ZipWriter w = new ZipWriter();
     * w.addFile("hello.txt", "hello, world!".getBytes(UTF_8), true);
     * w.addDirectory("mydir/");
     * byte[] archive = w.finish();
     * }</pre>
     */
    public static final class ZipWriter {

        // Central Directory records accumulated during addFile / addDirectory calls.
        private final List<CdRecord> entries = new ArrayList<>();

        // Raw bytes of the archive so far (Local Headers + file data).
        private final List<Byte> buf = new ArrayList<>();

        /** Metadata saved per entry for writing the Central Directory in finish(). */
        private static final class CdRecord {
            byte[] name;
            short method;
            int dosDt;
            long crc;          // unsigned 32-bit, stored in a long
            int compressedSize;
            int uncompressedSize;
            int localOffset;
            int externalAttrs;
        }

        /**
         * Add a file entry.
         *
         * <p>If {@code compress} is true, DEFLATE is attempted; the compressed
         * form is used only when it is strictly smaller than the original.
         * This prevents bloat for already-compressed or random data.</p>
         *
         * @param name     file path within the archive (UTF-8)
         * @param data     uncompressed file bytes
         * @param compress true to attempt DEFLATE compression
         * @throws IOException on DEFLATE encoding error (rare for valid data)
         */
        public void addFile(String name, byte[] data, boolean compress) throws IOException {
            if (name == null) throw new NullPointerException("name must not be null");
            if (data == null) throw new NullPointerException("data must not be null");
            addEntry(name, data, compress, UNIX_MODE_FILE);
        }

        /**
         * Add a directory entry.  {@code name} must end with {@code '/'}.
         *
         * @param name directory path within the archive (must end with '/')
         * @throws IOException on encoding error
         */
        public void addDirectory(String name) throws IOException {
            if (name == null) throw new NullPointerException("name must not be null");
            addEntry(name, new byte[0], false, UNIX_MODE_DIR);
        }

        /** Internal: write one entry (file or directory) with the given Unix mode. */
        private void addEntry(String name, byte[] data, boolean compress, int unixMode)
                throws IOException {

            byte[] nameBytes = name.getBytes(StandardCharsets.UTF_8);
            long crc = Crc32.compute(data);
            int uncompressedSize = data.length;

            // Decide compression: try DEFLATE, fall back to Stored if it doesn't help.
            short method;
            byte[] fileData;

            if (compress && data.length > 0) {
                byte[] compressed = DeflateCompressor.compress(data);
                if (compressed.length < data.length) {
                    method = METHOD_DEFLATE;
                    fileData = compressed;
                } else {
                    // DEFLATE made it larger (random or already-compressed data) — store raw.
                    method = METHOD_STORED;
                    fileData = data;
                }
            } else {
                method = METHOD_STORED;
                fileData = data;
            }

            int compressedSize = fileData.length;
            int localOffset = buf.size();
            short versionNeeded = (method == METHOD_DEFLATE) ? VERSION_DEFLATE : VERSION_STORED;

            // ── Local File Header (30 bytes fixed + variable name + data) ──────────
            // All integers are little-endian per the ZIP specification.
            writeU32(LOCAL_SIG);
            writeU16(versionNeeded);
            writeU16(FLAGS);                                           // bit 11 = UTF-8 filename
            writeU16(method);
            writeU16((short)(DOS_EPOCH & 0xFFFF));                    // mod_time
            writeU16((short)((DOS_EPOCH >> 16) & 0xFFFF));            // mod_date
            writeU32((int)(crc & 0xFFFFFFFFL));
            writeU32(compressedSize);
            writeU32(uncompressedSize);
            writeU16((short) nameBytes.length);
            writeU16((short) 0);                                      // extra_field_length = 0
            for (byte b : nameBytes) buf.add(b);
            for (byte b : fileData)  buf.add(b);

            // Save metadata for the Central Directory pass in finish().
            CdRecord rec = new CdRecord();
            rec.name = nameBytes;
            rec.method = method;
            rec.dosDt = DOS_EPOCH;
            rec.crc = crc;
            rec.compressedSize = compressedSize;
            rec.uncompressedSize = uncompressedSize;
            rec.localOffset = localOffset;
            rec.externalAttrs = unixMode << 16;
            entries.add(rec);
        }

        /**
         * Finish writing: append the Central Directory and EOCD record, then
         * return the complete archive as a byte array.
         *
         * @return complete ZIP archive bytes
         */
        public byte[] finish() {
            int cdOffset = buf.size();

            // ── Central Directory Headers ──────────────────────────────────────────
            // One 46-byte fixed record per entry, followed by the variable-length name.
            int cdStart = buf.size();
            for (CdRecord e : entries) {
                short versionNeeded = (e.method == METHOD_DEFLATE) ? VERSION_DEFLATE : VERSION_STORED;

                writeU32(CD_SIG);
                writeU16(VERSION_MADE_BY);
                writeU16(versionNeeded);
                writeU16(FLAGS);
                writeU16(e.method);
                writeU16((short)(e.dosDt & 0xFFFF));          // mod_time
                writeU16((short)((e.dosDt >> 16) & 0xFFFF));  // mod_date
                writeU32((int)(e.crc & 0xFFFFFFFFL));
                writeU32(e.compressedSize);
                writeU32(e.uncompressedSize);
                writeU16((short) e.name.length);
                writeU16((short) 0);                           // extra_len = 0
                writeU16((short) 0);                           // comment_len = 0
                writeU16((short) 0);                           // disk_start = 0
                writeU16((short) 0);                           // internal_attrs = 0
                writeU32(e.externalAttrs);
                writeU32(e.localOffset);
                for (byte b : e.name) buf.add(b);
                // (no extra field, no file comment)
            }
            int cdSize = buf.size() - cdStart;
            short numEntries = (short) entries.size();

            // ── End of Central Directory Record (22 bytes) ─────────────────────────
            writeU32(EOCD_SIG);
            writeU16((short) 0);          // disk_number = 0
            writeU16((short) 0);          // disk_with_cd_start = 0
            writeU16(numEntries);         // entries_on_this_disk
            writeU16(numEntries);         // entries_total
            writeU32(cdSize);             // Central Directory byte size
            writeU32(cdOffset);           // Central Directory byte offset from archive start
            writeU16((short) 0);          // comment_length = 0

            // Materialise List<Byte> → byte[].
            byte[] out = new byte[buf.size()];
            for (int i = 0; i < out.length; i++) {
                out[i] = buf.get(i);
            }
            return out;
        }

        // ── Little-endian helper writers ──────────────────────────────────────────

        private void writeU16(short v) {
            buf.add((byte)(v & 0xFF));
            buf.add((byte)((v >> 8) & 0xFF));
        }

        private void writeU16(int v) {
            buf.add((byte)(v & 0xFF));
            buf.add((byte)((v >> 8) & 0xFF));
        }

        private void writeU32(int v) {
            buf.add((byte)(v & 0xFF));
            buf.add((byte)((v >> 8) & 0xFF));
            buf.add((byte)((v >> 16) & 0xFF));
            buf.add((byte)((v >> 24) & 0xFF));
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
    //   4. Expose List<ZipEntry> of names (data is empty until read() is called).
    //   5. read(name): seek to Local Header via local_offset, skip name+extra,
    //      read compressed_size bytes, decompress, verify CRC-32.
    //
    // Security: use Central Directory as the authoritative source for sizes and
    // method.  Local Header is consulted only for name_len + extra_len skip.
    // This prevents malformed Local Headers from causing over-reads.

    /**
     * Reads entries from an in-memory ZIP archive.
     *
     * <p>The archive is parsed eagerly (all Central Directory entries) but file
     * data is decompressed lazily on demand via {@link #read}.</p>
     *
     * <pre>{@code
     * ZipReader r = new ZipReader(archiveBytes);
     * for (ZipEntry e : r.entries()) {
     *     System.out.println(e.name());
     * }
     * byte[] data = r.read("readme.txt");
     * }</pre>
     */
    public static final class ZipReader {

        // Raw archive bytes (kept alive for deferred reads).
        private final byte[] data;

        // Parsed entry metadata from the Central Directory.
        private final List<EntryMeta> meta = new ArrayList<>();

        // Public ZipEntry list (names only; data read on demand).
        private final List<ZipEntry> entryList;

        /** Internal: full metadata per entry needed for lazy reads. */
        private static final class EntryMeta {
            String name;
            int localOffset;
            short method;
            long crc;          // unsigned 32-bit stored in long
            int compressedSize;
            int uncompressedSize;
            boolean isDirectory;
        }

        /**
         * Parse an in-memory ZIP archive.
         *
         * @param data archive bytes
         * @throws IOException if no valid EOCD record is found or the archive is corrupt
         */
        public ZipReader(byte[] data) throws IOException {
            if (data == null) throw new NullPointerException("data must not be null");
            this.data = data;

            int eocdOffset = findEocd(data);
            if (eocdOffset < 0) {
                throw new IOException("zip: no End of Central Directory record found");
            }

            // Read EOCD fields: cd_size at +12, cd_offset at +16.
            int cdOffset = (int) readU32(data, eocdOffset + 16);
            int cdSize   = (int) readU32(data, eocdOffset + 12);

            if (cdOffset + cdSize > data.length) {
                throw new IOException(
                    "zip: Central Directory [" + cdOffset + ", " + (cdOffset + cdSize) +
                    ") out of bounds (file size " + data.length + ")");
            }

            // Parse Central Directory headers.
            int pos = cdOffset;
            while (pos + 4 <= cdOffset + cdSize) {
                long sig = readU32(data, pos);
                if (sig != (CD_SIG & 0xFFFFFFFFL)) break; // end of CD or padding

                short method         = (short) readU16(data, pos + 10);
                long crc             = readU32(data, pos + 16);
                int compressedSize   = (int) readU32(data, pos + 20);
                int uncompressedSize = (int) readU32(data, pos + 24);
                int nameLen          = readU16(data, pos + 28);
                int extraLen         = readU16(data, pos + 30);
                int commentLen       = readU16(data, pos + 32);
                int localOffset      = (int) readU32(data, pos + 42);

                int nameStart = pos + 46;
                int nameEnd   = nameStart + nameLen;
                if (nameEnd > data.length) {
                    throw new IOException("zip: CD entry name out of bounds");
                }

                String name = new String(data, nameStart, nameLen, StandardCharsets.UTF_8);

                EntryMeta em = new EntryMeta();
                em.name = name;
                em.localOffset = localOffset;
                em.method = method;
                em.crc = crc;
                em.compressedSize = compressedSize;
                em.uncompressedSize = uncompressedSize;
                em.isDirectory = name.endsWith("/");
                meta.add(em);

                pos = nameEnd + extraLen + commentLen;
            }

            // Build the public ZipEntry list (name only; data read on demand).
            entryList = new ArrayList<>();
            for (EntryMeta m : meta) {
                entryList.add(new ZipEntry(m.name, new byte[0]));
            }
        }

        /**
         * All entries in the archive (files and directories) in Central Directory
         * order.  The {@link ZipEntry#data()} field is empty until you call
         * {@link #read}.
         *
         * @return unmodifiable view of the entry list
         */
        public List<ZipEntry> entries() {
            return List.copyOf(entryList);
        }

        /**
         * Decompress and return the data for the named entry.
         *
         * @param name entry name as it appears in {@link #entries()}
         * @return decompressed file bytes
         * @throws IOException on CRC mismatch, corrupt data, or unknown entry name
         */
        public byte[] read(String name) throws IOException {
            EntryMeta found = null;
            for (EntryMeta m : meta) {
                if (m.name.equals(name)) {
                    found = m;
                    break;
                }
            }
            if (found == null) {
                throw new IOException("zip: entry '" + name + "' not found");
            }
            return readEntry(found);
        }

        /** Internal: read and decompress one entry using its local_offset. */
        private byte[] readEntry(EntryMeta em) throws IOException {
            if (em.isDirectory) return new byte[0];

            int lhOff = em.localOffset;

            // Reject encrypted entries (GP flag bit 0 = 1).
            int localFlags = readU16(data, lhOff + 6);
            if ((localFlags & 1) != 0) {
                throw new IOException(
                    "zip: entry '" + em.name + "' is encrypted; not supported");
            }

            // The Local Header name_len and extra_len can differ from the CD header,
            // so we must re-read them to find the actual start of the file data.
            int lhNameLen  = readU16(data, lhOff + 26);
            int lhExtraLen = readU16(data, lhOff + 28);
            int dataStart  = lhOff + 30 + lhNameLen + lhExtraLen;
            int dataEnd    = dataStart + em.compressedSize;

            if (dataEnd > data.length) {
                throw new IOException(
                    "zip: entry '" + em.name + "' data [" + dataStart + ", " + dataEnd +
                    ") out of bounds");
            }

            byte[] compressed = new byte[em.compressedSize];
            System.arraycopy(data, dataStart, compressed, 0, em.compressedSize);

            // Decompress according to method.
            byte[] decompressed;
            int method = em.method & 0xFFFF;
            if (method == 0) {
                // Stored — verbatim copy.
                decompressed = compressed;
            } else if (method == 8) {
                // DEFLATE.
                decompressed = DeflateDecompressor.decompress(compressed);
            } else {
                throw new IOException(
                    "zip: unsupported compression method " + method +
                    " for '" + em.name + "'");
            }

            // Trim to declared uncompressed size to guard against decompressor over-read.
            if (decompressed.length > em.uncompressedSize) {
                byte[] trimmed = new byte[em.uncompressedSize];
                System.arraycopy(decompressed, 0, trimmed, 0, em.uncompressedSize);
                decompressed = trimmed;
            }

            // Verify CRC-32 — detects corruption of the decompressed content.
            long actualCrc = Crc32.compute(decompressed);
            if (actualCrc != em.crc) {
                throw new IOException(
                    "zip: CRC-32 mismatch for '" + em.name + "': expected " +
                    Long.toHexString(em.crc).toUpperCase() +
                    ", got " + Long.toHexString(actualCrc).toUpperCase());
            }

            return decompressed;
        }

        // ── EOCD search ────────────────────────────────────────────────────────────
        //
        // Scan backwards from the end of the file for the EOCD signature.
        // Limit the scan to the last 65557 bytes (22-byte min EOCD + 65535-byte
        // max ZIP comment) to prevent unbounded searches on malformed archives.

        /**
         * Find the EOCD record offset, or -1 if not found.
         *
         * <p>We scan backwards from the file end and validate the {@code comment_len}
         * field to distinguish the real EOCD from a coincidental signature match
         * inside a file.</p>
         */
        private static int findEocd(byte[] data) {
            final int EOCD_MIN_SIZE = 22;
            final int MAX_COMMENT = 65535;

            if (data.length < EOCD_MIN_SIZE) return -1;

            int scanStart = Math.max(0, data.length - EOCD_MIN_SIZE - MAX_COMMENT);

            for (int i = data.length - EOCD_MIN_SIZE; i >= scanStart; i--) {
                try {
                    long sig = readU32(data, i);
                    if (sig != (EOCD_SIG & 0xFFFFFFFFL)) continue;

                    // Validate: comment_len at offset +20 must account for all remaining bytes.
                    int commentLen = readU16(data, i + 20);
                    if (i + EOCD_MIN_SIZE + commentLen == data.length) {
                        return i;
                    }
                } catch (IOException e) {
                    // Out-of-bounds read — skip this position.
                }
            }
            return -1;
        }

        // ── Little-endian readers ──────────────────────────────────────────────────

        private static int readU16(byte[] data, int offset) throws IOException {
            if (offset + 2 > data.length) {
                throw new IOException("zip: read U16 at " + offset + " out of bounds");
            }
            return (data[offset] & 0xFF) | ((data[offset + 1] & 0xFF) << 8);
        }

        private static long readU32(byte[] data, int offset) throws IOException {
            if (offset + 4 > data.length) {
                throw new IOException("zip: read U32 at " + offset + " out of bounds");
            }
            return ((long)(data[offset] & 0xFF))
                 | ((long)(data[offset + 1] & 0xFF) << 8)
                 | ((long)(data[offset + 2] & 0xFF) << 16)
                 | ((long)(data[offset + 3] & 0xFF) << 24);
        }
    }

    // =============================================================================
    // Convenience API
    // =============================================================================

    /**
     * Compress a list of {@link ZipEntry} objects into a ZIP archive.
     *
     * <p>Each entry is compressed with DEFLATE if that reduces size; otherwise
     * it is stored verbatim.  Directory entries (names ending with '/') are stored
     * uncompressed with no data.</p>
     *
     * @param entries list of entries to add
     * @return complete ZIP archive bytes
     * @throws IOException on DEFLATE encoding error
     */
    public static byte[] zip(List<ZipEntry> entries) throws IOException {
        if (entries == null) throw new NullPointerException("entries must not be null");
        ZipWriter writer = new ZipWriter();
        for (ZipEntry entry : entries) {
            if (entry.name().endsWith("/")) {
                writer.addDirectory(entry.name());
            } else {
                writer.addFile(entry.name(), entry.data(), true);
            }
        }
        return writer.finish();
    }

    /**
     * Extract all entries from a ZIP archive.
     *
     * <p>Directory entries are included with empty {@code data}.
     * Throws {@link IOException} on corrupt archives or CRC mismatches.</p>
     *
     * @param data archive bytes
     * @return list of all entries with fully decompressed data
     * @throws IOException on corrupt archive or CRC mismatch
     */
    public static List<ZipEntry> unzip(byte[] data) throws IOException {
        if (data == null) throw new NullPointerException("data must not be null");
        ZipReader reader = new ZipReader(data);
        List<ZipEntry> result = new ArrayList<>();
        for (ZipEntry entry : reader.entries()) {
            byte[] bytes = entry.name().endsWith("/")
                ? new byte[0]
                : reader.read(entry.name());
            result.add(new ZipEntry(entry.name(), bytes));
        }
        return result;
    }
}
