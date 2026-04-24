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
//   │  ...                                                 │
//   │  ══════════ Central Directory ══════════             │
//   │  [Central Dir Header]  ← entry 1 (has local offset) │
//   │  [Central Dir Header]  ← entry 2                    │
//   │  [End of Central Directory Record]                   │
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

namespace CodingAdventures.Zip.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic
open System.IO
open System.Text
open CodingAdventures.Lzss.FSharp

// =============================================================================
// Wire Format Constants
// =============================================================================
//
// ZIP uses four-byte "magic number" signatures to identify each structural
// region.  All integers in the wire format are little-endian.
//
// The signatures are written as human-readable ASCII for the first two bytes
// ("PK") and control bytes for the remaining two.  Unzipping tools use these
// signatures for format detection without relying on file extensions.

[<RequireQualifiedAccess>]
module ZipConstants =
    // Local File Header signature: "PK\x03\x04"
    let [<Literal>] LocalSig = 0x04034B50u

    // Central Directory Header signature: "PK\x01\x02"
    let [<Literal>] CdSig = 0x02014B50u

    // End of Central Directory Record signature: "PK\x05\x06"
    let [<Literal>] EocdSig = 0x06054B50u

    // Fixed timestamp: 1980-01-01 00:00:00
    // DOS date = (0<<9)|(1<<5)|1 = 0x0021; time = 0 → combined 0x00210000
    let [<Literal>] DosEpoch = 0x00210000u

    // General Purpose Bit Flag: bit 11 = UTF-8 filename encoding
    let [<Literal>] Flags = 0x0800us

    // Compression methods
    let [<Literal>] MethodStored  = 0us
    let [<Literal>] MethodDeflate = 8us

    // Version needed: 2.0 for DEFLATE (20 = version 2.0), 1.0 for Stored
    let [<Literal>] VersionDeflate = 20us
    let [<Literal>] VersionStored  = 10us

    // Version made by: 0x031E = Unix OS (high byte 3), spec version 30 (low byte 0x1E)
    let [<Literal>] VersionMadeBy = 0x031Eus

    // Unix file modes embedded in Central Directory external_attrs (shifted left 16 bits).
    // 0o100644 = regular file, rw-r--r-- (octal); 0o040755 = directory, rwxr-xr-x.
    let [<Literal>] UnixModeFile = 33188u  // 0o100644 decimal
    let [<Literal>] UnixModeDir  = 16877u  // 0o040755 decimal

// =============================================================================
// CRC-32
// =============================================================================
//
// CRC-32 uses polynomial 0xEDB88320 (reflected form of 0x04C11DB7).
// It is computed over the *uncompressed* bytes and stored in the headers so
// extractors can verify integrity after decompression.
//
// CRC-32 is NOT a cryptographic hash — it detects accidental corruption only.
// For tamper-detection, use AES-GCM or a signed manifest.
//
// How the table works:
//   For every possible byte value 0-255, we precompute the CRC-32 of just that
//   single byte.  Then, for a multi-byte message, we can look up each byte's
//   contribution in the table and XOR it in — O(n) with a small constant.

[<RequireQualifiedAccess>]
module Crc32 =
    // Precomputed 256-entry lookup table.  Each entry is the CRC-32 of a single
    // byte value, using the reflected polynomial 0xEDB88320.  Building it once
    // at module-load time amortises the cost across all archives.
    let private table : uint[] =
        let t = Array.zeroCreate 256
        for i in 0u .. 255u do
            let mutable c = i
            for _ in 0 .. 7 do
                // If the LSB is set, XOR with the polynomial (reflected form).
                // This is the "table-driven CRC" algorithm from RFC 1952 §8.
                c <- if (c &&& 1u) <> 0u then (0xEDB88320u ^^^ (c >>> 1)) else (c >>> 1)
            t[int i] <- c
        t

    /// Compute CRC-32 over `data`.  Pass `initial` = 0u for a fresh hash,
    /// or the previous result to continue an incremental computation.
    let compute (data: byte[]) (initial: uint) : uint =
        // XOR the initial value in (for the first call initial=0 → crc starts at 0xFFFFFFFF).
        let mutable crc = initial ^^^ 0xFFFFFFFFu
        for b in data do
            crc <- table[int ((crc ^^^ uint b) &&& 0xFFu)] ^^^ (crc >>> 8)
        // XOR out to produce the final CRC.
        crc ^^^ 0xFFFFFFFFu

// =============================================================================
// RFC 1951 DEFLATE — Bit I/O (BitWriter)
// =============================================================================
//
// RFC 1951 packs bits LSB-first within bytes.  Huffman codes are logically
// MSB-first, so before writing one we bit-reverse it and write the reversed
// value LSB-first into the stream.  Extra bits (length/distance extras, stored
// block headers) are written directly in LSB-first order without reversal.

type private BitWriter() =
    let buf = ResizeArray<byte>()
    let mutable reg  : uint64 = 0UL  // accumulator holding up to 63 unflushable bits
    let mutable bits : int    = 0    // how many bits are currently valid in reg

    /// Write the `n` low-order bits of `v` into the stream, LSB-first.
    /// Used for extra bits and block headers.
    member _.AddBits(v: uint64, n: int) =
        // OR the new bits into the accumulator at the current fill position.
        reg <- reg ||| ((v &&& ((1UL <<< n) - 1UL)) <<< bits)
        bits <- bits + n
        // Drain complete bytes from the accumulator.
        while bits >= 8 do
            buf.Add(byte (reg &&& 0xFFUL))
            reg  <- reg >>> 8
            bits <- bits - 8

    /// Reverse the bottom `nbits` bits of `code`.
    /// Example: reverseBits 0b110u 3 = 0b011u
    static member private ReverseBits(code: uint, nbits: int) =
        let mutable rev = 0u
        let mutable c   = code
        for _ in 0 .. nbits - 1 do
            rev <- (rev <<< 1) ||| (c &&& 1u)
            c   <- c >>> 1
        rev

    /// Write a Huffman code of `nbits` bits.
    /// Huffman codes are MSB-first logically, so we bit-reverse before storing.
    member bw.WriteHuffman(code: uint, nbits: int) =
        let reversed = BitWriter.ReverseBits(code, nbits)
        bw.AddBits(uint64 reversed, nbits)

    /// Flush any partial byte to the buffer (zero-pad the remaining bits).
    /// Required before writing stored-block headers (must be byte-aligned).
    member _.Flush() =
        if bits > 0 then
            buf.Add(byte (reg &&& 0xFFUL))
            reg  <- 0UL
            bits <- 0

    /// Return the completed byte array.  Flushes any partial byte first.
    member bw.ToArray() =
        bw.Flush()
        buf.ToArray()

// =============================================================================
// RFC 1951 DEFLATE — Bit I/O (BitReader)
// =============================================================================
//
// Mirrors BitWriter: fill an accumulator from bytes in the source array,
// reading LSB-first.  Huffman code decoding reads MSB-first by bit-reversing
// the extracted value.

type private BitReader(data: byte[]) =
    let mutable pos  : int    = 0    // next byte to consume from data
    let mutable buf  : uint64 = 0UL  // bit accumulator
    let mutable bits : int    = 0    // valid bits in buf

    /// Ensure the accumulator holds at least `need` bits.
    /// Returns false if the source is exhausted.
    member private _.Fill(need: int) =
        let mutable ok = true
        while bits < need && ok do
            if pos >= data.Length then ok <- false
            else
                buf  <- buf ||| (uint64 data[pos] <<< bits)
                pos  <- pos + 1
                bits <- bits + 8
        ok

    /// Read `nbits` bits from the stream, LSB-first.  Returns None on EOF.
    member br.ReadLsb(nbits: int) =
        if nbits = 0 then Some 0
        elif not (br.Fill(nbits)) then None
        else
            let mask = (1UL <<< nbits) - 1UL
            let v    = int (buf &&& mask)
            buf  <- buf >>> nbits
            bits <- bits - nbits
            Some v

    /// Read `nbits` bits and bit-reverse the result.
    /// Used when decoding Huffman codes (logically MSB-first).
    member br.ReadMsb(nbits: int) =
        match br.ReadLsb(nbits) with
        | None   -> None
        | Some v ->
            let mutable rev = 0u
            let mutable u   = uint v
            for _ in 0 .. nbits - 1 do
                rev <- (rev <<< 1) ||| (u &&& 1u)
                u   <- u >>> 1
            Some (int rev)

    /// Discard any partial-byte bits, aligning to the next byte boundary.
    /// Required before reading stored-block length fields.
    member _.Align() =
        let discard = bits % 8
        if discard > 0 then
            buf  <- buf >>> discard
            bits <- bits - discard

// =============================================================================
// RFC 1951 DEFLATE — Fixed Huffman Tables
// =============================================================================
//
// RFC 1951 §3.2.6 defines a canonical "fixed" Huffman alphabet that both
// encoder and decoder know in advance.  Using BTYPE=01 (fixed Huffman) means
// we never need to transmit code-length tables, keeping the implementation
// simple.
//
// Literal/Length code lengths:
//   Symbols   0–143: 8-bit codes, base 0x30 (0b00110000)
//   Symbols 144–255: 9-bit codes, base 0x190 (0b110010000)
//   Symbols 256–279: 7-bit codes, base 0x00
//   Symbols 280–287: 8-bit codes, base 0xC0 (0b11000000)
//
// Distance codes: 5-bit codes equal to the code number (0–29).

[<RequireQualifiedAccess>]
module private FixedHuffman =

    /// Return the (code, nbits) pair for encoding literal/length symbol 0–287.
    let encodeLL (sym: int) : uint * int =
        if   sym >= 0   && sym <= 143 then uint (sym + 0x30),        8
        elif sym >= 144 && sym <= 255 then uint (sym - 144 + 0x190), 9
        elif sym >= 256 && sym <= 279 then uint (sym - 256),         7
        elif sym >= 280 && sym <= 287 then uint (sym - 280 + 0xC0),  8
        else raise (InvalidDataException(sprintf "FixedHuffman.encodeLL: invalid symbol %d" sym))

    /// Decode one literal/length symbol from `br` using the RFC 1951 fixed
    /// Huffman table.  Reads incrementally (7 → 8 → 9 bits).
    /// Returns None on end-of-input.
    let decodeLL (br: BitReader) : int option =
        // Try 7 bits first (covers symbols 256–279, codes 0–23).
        match br.ReadMsb(7) with
        | None     -> None
        | Some v7  ->
            if v7 <= 23 then
                // 7-bit code → symbols 256–279.
                Some (v7 + 256)
            else
                // Need one more bit to reach 8-bit codes.
                match br.ReadLsb(1) with
                | None       -> None
                | Some extra1 ->
                    let v8 = (v7 <<< 1) ||| extra1
                    if   v8 >= 48  && v8 <= 191 then Some (v8 - 48)    // literals 0–143
                    elif v8 >= 192 && v8 <= 199 then Some (v8 + 88)    // symbols 280–287
                    else
                        // Need one more bit for 9-bit codes (literals 144–255).
                        match br.ReadLsb(1) with
                        | None        -> None
                        | Some extra2 ->
                            let v9 = (v8 <<< 1) ||| extra2
                            if v9 >= 400 && v9 <= 511 then Some (v9 - 256) // literals 144–255
                            else None // malformed

// =============================================================================
// RFC 1951 DEFLATE — Length / Distance Tables
// =============================================================================
//
// Match lengths (3–258 bytes) map to LL symbols 257–285 plus extra bits.
// Match distances (1–32768 bytes) map to distance codes 0–29 plus extra bits.
// The tables below come directly from RFC 1951 §3.2.5.

[<RequireQualifiedAccess>]
module private DeflateTable =

    // (base_length, extra_bits) indexed by (LL_symbol - 257).
    // Symbol 285 has a special-case base of 258 with 0 extra bits.
    let lengths : (int * int)[] =
        [| (3,0); (4,0); (5,0); (6,0); (7,0); (8,0); (9,0); (10,0)  // 257–264
           (11,1); (13,1); (15,1); (17,1)                             // 265–268
           (19,2); (23,2); (27,2); (31,2)                             // 269–272
           (35,3); (43,3); (51,3); (59,3)                             // 273–276
           (67,4); (83,4); (99,4); (115,4)                            // 277–280
           (131,5); (163,5); (195,5); (227,5); (258,0) |]             // 281–285

    // (base_distance, extra_bits) indexed by distance code 0–29.
    let dists : (int * int)[] =
        [| (1,0); (2,0); (3,0); (4,0)
           (5,1); (7,1); (9,2); (13,2)
           (17,3); (25,3); (33,4); (49,4)
           (65,5); (97,5); (129,6); (193,6)
           (257,7); (385,7); (513,8); (769,8)
           (1025,9); (1537,9); (2049,10); (3073,10)
           (4097,11); (6145,11); (8193,12); (12289,12)
           (16385,13); (24577,13) |]

    /// Encode a match `length` (3–258) as an RFC 1951 LL symbol plus extra bits.
    /// Returns (ll_symbol, base, extra_bit_count).
    let encodeLength (length: int) =
        let mutable result = None
        let mutable i = lengths.Length - 1
        while i >= 0 && result.IsNone do
            let (baseLen, extra) = lengths[i]
            if length >= baseLen then
                result <- Some (257 + i, baseLen, extra)
            i <- i - 1
        match result with
        | Some r -> r
        | None   -> raise (InvalidDataException(sprintf "encodeLength: unreachable for length=%d" length))

    /// Encode a match `distance` (1–32768) as an RFC 1951 distance code plus extra bits.
    /// Returns (dist_code, base, extra_bit_count).
    let encodeDist (distance: int) =
        let mutable result = None
        let mutable i = dists.Length - 1
        while i >= 0 && result.IsNone do
            let (baseDist, extra) = dists[i]
            if distance >= baseDist then
                result <- Some (i, baseDist, extra)
            i <- i - 1
        match result with
        | Some r -> r
        | None   -> raise (InvalidDataException(sprintf "encodeDist: unreachable for distance=%d" distance))

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

[<RequireQualifiedAccess>]
module private DeflateCompressor =

    /// Compress `data` to raw RFC 1951 DEFLATE (no zlib wrapper).
    /// Uses fixed Huffman (BTYPE=01) for non-empty input.
    let compress (data: byte[]) : byte[] =
        let bw = BitWriter()

        if data.Length = 0 then
            // Empty stored block: BFINAL=1, BTYPE=00, aligned, LEN=0, NLEN=0xFFFF.
            bw.AddBits(1UL, 1)      // BFINAL = 1 (last block)
            bw.AddBits(0UL, 2)      // BTYPE = 00 (stored)
            bw.Flush()              // align to byte boundary (RFC 1951 §3.2.4)
            bw.AddBits(0x0000UL, 16) // LEN = 0
            bw.AddBits(0xFFFFUL, 16) // NLEN = one's complement of LEN
            bw.ToArray()
        else
            // Run LZSS tokenization.  Window = 32768 so every match distance fits
            // in the RFC 1951 distance table.  Max match = 255 to fit the length table.
            let tokens = Lzss.Encode(data, windowSize = 32768, maxMatch = 255, minMatch = 3)

            // Block header: BFINAL=1 (single block), BTYPE=01 (fixed Huffman).
            // Bits are written LSB-first: BFINAL in bit 0, BTYPE in bits 1-2.
            bw.AddBits(1UL, 1) // BFINAL = 1
            bw.AddBits(1UL, 1) // BTYPE bit 0 = 1  }
            bw.AddBits(0UL, 1) // BTYPE bit 1 = 0  } → BTYPE = 01 (fixed Huffman)

            for token in tokens do
                match token with
                | Literal b ->
                    // Literal byte: emit the fixed Huffman code for symbol `b`.
                    let (code, nbits) = FixedHuffman.encodeLL (int b)
                    bw.WriteHuffman(code, nbits)

                | Match(offset, length) ->
                    // Length: find the LL symbol + extra bits, then emit them.
                    let (lenSym, lenBase, lenExtra) = DeflateTable.encodeLength length
                    let (lenCode, lenBits) = FixedHuffman.encodeLL lenSym
                    bw.WriteHuffman(lenCode, lenBits)
                    if lenExtra > 0 then
                        bw.AddBits(uint64 (length - lenBase), lenExtra)

                    // Distance: the 5-bit fixed distance code equals the code number.
                    let (distCode, distBase, distExtra) = DeflateTable.encodeDist offset
                    bw.WriteHuffman(uint distCode, 5)
                    if distExtra > 0 then
                        bw.AddBits(uint64 (offset - distBase), distExtra)

            // End-of-block symbol (256) — signals the decoder to stop.
            let (eobCode, eobBits) = FixedHuffman.encodeLL 256
            bw.WriteHuffman(eobCode, eobBits)
            bw.ToArray()

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

[<RequireQualifiedAccess>]
module private DeflateDecompressor =

    let private maxOutputBytes = 256 * 1024 * 1024

    /// Decompress raw RFC 1951 DEFLATE bytes into the original data.
    /// Throws InvalidDataException for corrupt or unsupported input.
    let decompress (data: byte[]) : byte[] =
        let br     = BitReader(data)
        let output = ResizeArray<byte>()
        let mutable finished = false

        while not finished do
            let bfinal =
                match br.ReadLsb(1) with
                | Some v -> v
                | None   -> raise (InvalidDataException "deflate: unexpected EOF reading BFINAL")
            let btype =
                match br.ReadLsb(2) with
                | Some v -> v
                | None   -> raise (InvalidDataException "deflate: unexpected EOF reading BTYPE")

            match btype with
            | 0b00 ->
                // ── Stored block ────────────────────────────────────────────
                // Align to byte boundary before reading the length fields.
                br.Align()
                let len =
                    match br.ReadLsb(16) with
                    | Some v -> v
                    | None   -> raise (InvalidDataException "deflate: EOF reading stored LEN")
                let nlen =
                    match br.ReadLsb(16) with
                    | Some v -> v
                    | None   -> raise (InvalidDataException "deflate: EOF reading stored NLEN")
                // RFC 1951 §3.2.4: NLEN is the one's complement of LEN.
                if (nlen ^^^ 0xFFFF) <> len then
                    raise (InvalidDataException(sprintf "deflate: stored LEN/NLEN mismatch (%d vs %d)" len nlen))
                if output.Count + len > maxOutputBytes then
                    raise (InvalidDataException "deflate: output size limit exceeded")
                for _ in 0 .. len - 1 do
                    match br.ReadLsb(8) with
                    | Some b -> output.Add(byte b)
                    | None   -> raise (InvalidDataException "deflate: EOF inside stored block")

            | 0b01 ->
                // ── Fixed Huffman block ──────────────────────────────────────
                let mutable blockDone = false
                while not blockDone do
                    let sym =
                        match FixedHuffman.decodeLL br with
                        | Some v -> v
                        | None   -> raise (InvalidDataException "deflate: EOF decoding LL symbol")

                    if sym >= 0 && sym <= 255 then
                        if output.Count >= maxOutputBytes then
                            raise (InvalidDataException "deflate: output size limit exceeded")
                        output.Add(byte sym)
                    elif sym = 256 then
                        // End-of-block: leave the inner loop.
                        blockDone <- true
                    elif sym >= 257 && sym <= 285 then
                        // Back-reference: decode length, then distance.
                        let idx = sym - 257
                        if idx >= DeflateTable.lengths.Length then
                            raise (InvalidDataException(sprintf "deflate: invalid length sym %d" sym))

                        let (baseLen, extraLenBits) = DeflateTable.lengths[idx]
                        let extraLen =
                            if extraLenBits > 0 then
                                match br.ReadLsb(extraLenBits) with
                                | Some v -> v
                                | None   -> raise (InvalidDataException "deflate: EOF reading length extra")
                            else 0
                        let matchLen = baseLen + extraLen

                        // Distance code is always 5 bits, read MSB-first.
                        let distCode =
                            match br.ReadMsb(5) with
                            | Some v -> v
                            | None   -> raise (InvalidDataException "deflate: EOF reading distance code")
                        if distCode >= DeflateTable.dists.Length then
                            raise (InvalidDataException(sprintf "deflate: invalid dist code %d" distCode))

                        let (baseDist, extraDistBits) = DeflateTable.dists[distCode]
                        let extraDist =
                            if extraDistBits > 0 then
                                match br.ReadLsb(extraDistBits) with
                                | Some v -> v
                                | None   -> raise (InvalidDataException "deflate: EOF reading distance extra")
                            else 0
                        let matchOffset = baseDist + extraDist

                        if matchOffset > output.Count then
                            raise (InvalidDataException(
                                sprintf "deflate: back-ref offset %d > output len %d" matchOffset output.Count))
                        if output.Count + matchLen > maxOutputBytes then
                            raise (InvalidDataException "deflate: output size limit exceeded")

                        // Copy byte-by-byte to handle overlapping matches.
                        // Example: offset=1, length=4 expands a single byte into a run of 4.
                        for _ in 0 .. matchLen - 1 do
                            output.Add(output[output.Count - matchOffset])
                    else
                        raise (InvalidDataException(sprintf "deflate: invalid LL symbol %d" sym))

            | 0b10 ->
                raise (InvalidDataException "deflate: dynamic Huffman blocks (BTYPE=10) not supported")
            | _ ->
                raise (InvalidDataException "deflate: reserved BTYPE=11")

            if bfinal = 1 then finished <- true

        output.ToArray()

// =============================================================================
// Public API — ZipEntry
// =============================================================================

/// A single file or directory entry in a ZIP archive.
/// Directory entries have `Name` ending with '/' and empty `Data`.
type ZipEntry =
    { /// The entry name (UTF-8). Directory entries end with '/'.
      Name : string
      /// The uncompressed file bytes. Empty for directories.
      Data : byte[] }

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

// Central Directory record saved during AddFile / AddDirectory calls.
[<AllowNullLiteral>]
type private CdRecord() =
    member val Name             : byte[]  = [||]  with get, set
    member val Method           : uint16  = 0us   with get, set
    member val DosDt            : uint    = 0u    with get, set
    member val Crc              : uint    = 0u    with get, set
    member val CompressedSize   : uint    = 0u    with get, set
    member val UncompressedSize : uint    = 0u    with get, set
    member val LocalOffset      : uint    = 0u    with get, set
    member val ExternalAttrs    : uint    = 0u    with get, set

/// Builds a ZIP archive incrementally in memory.
///
/// Example:
///   let w = ZipWriter()
///   w.AddFile("hello.txt", Encoding.UTF8.GetBytes("hello, world!"))
///   w.AddDirectory("mydir/")
///   let archive = w.Finish()
type ZipWriter() =
    // Central Directory records accumulated during AddFile / AddDirectory calls.
    let entries = ResizeArray<CdRecord>()

    // Raw bytes of the archive so far (Local Headers + file data).
    let buf = ResizeArray<byte>()

    // ── Little-endian write helpers ────────────────────────────────────────────

    let writeU16 (v: uint16) =
        let tmp = Array.zeroCreate<byte> 2
        BinaryPrimitives.WriteUInt16LittleEndian(Span tmp, v)
        buf.Add(tmp[0]); buf.Add(tmp[1])

    let writeU32 (v: uint) =
        let tmp = Array.zeroCreate<byte> 4
        BinaryPrimitives.WriteUInt32LittleEndian(Span tmp, v)
        buf.Add(tmp[0]); buf.Add(tmp[1]); buf.Add(tmp[2]); buf.Add(tmp[3])

    // Internal: write one entry (file or directory) with the given Unix mode.
    let addEntry (name: string) (data: byte[]) (compress: bool) (unixMode: uint) =
        let nameBytes        = Encoding.UTF8.GetBytes(name)
        let crc              = Crc32.compute data 0u
        let uncompressedSize = uint data.Length

        // Decide compression: try DEFLATE and fall back to Stored if it doesn't help.
        let (method, fileData) =
            if compress && data.Length > 0 then
                let compressed = DeflateCompressor.compress data
                if compressed.Length < data.Length then
                    ZipConstants.MethodDeflate, compressed
                else
                    // DEFLATE made it larger (random or already-compressed data) — store raw.
                    ZipConstants.MethodStored, data
            else
                ZipConstants.MethodStored, data

        let compressedSize = uint fileData.Length
        let localOffset    = uint buf.Count
        let versionNeeded  =
            if method = ZipConstants.MethodDeflate then ZipConstants.VersionDeflate
            else ZipConstants.VersionStored

        // ── Local File Header (30 bytes fixed + variable name + data) ───────────
        // All integers are little-endian per the ZIP specification.
        writeU32 ZipConstants.LocalSig
        writeU16 versionNeeded
        writeU16 ZipConstants.Flags                                    // bit 11 = UTF-8 filename
        writeU16 method
        writeU16 (uint16 (ZipConstants.DosEpoch &&& 0xFFFFu))         // mod_time
        writeU16 (uint16 (ZipConstants.DosEpoch >>> 16))              // mod_date
        writeU32 crc
        writeU32 compressedSize
        writeU32 uncompressedSize
        writeU16 (uint16 nameBytes.Length)
        writeU16 0us                                                   // extra_field_length = 0
        buf.AddRange(nameBytes)
        buf.AddRange(fileData)

        // Save metadata for the Central Directory pass in Finish().
        let rec_ = CdRecord()
        rec_.Name             <- nameBytes
        rec_.Method           <- method
        rec_.DosDt            <- ZipConstants.DosEpoch
        rec_.Crc              <- crc
        rec_.CompressedSize   <- compressedSize
        rec_.UncompressedSize <- uncompressedSize
        rec_.LocalOffset      <- localOffset
        rec_.ExternalAttrs    <- unixMode <<< 16
        entries.Add(rec_)

    /// Add a file entry.
    /// If `compress` is true, DEFLATE is attempted; the compressed form is used
    /// only if it is strictly smaller than the original.
    member _.AddFile(name: string, data: byte[], ?compress: bool) =
        if isNull name then nullArg "name"
        if isNull data then nullArg "data"
        let doCompress = defaultArg compress true
        addEntry name data doCompress ZipConstants.UnixModeFile

    /// Add a directory entry. `name` must end with '/'.
    member _.AddDirectory(name: string) =
        if isNull name then nullArg "name"
        addEntry name [||] false ZipConstants.UnixModeDir

    /// Finish writing: append the Central Directory and EOCD record, then
    /// return the complete archive as a byte array.
    member _.Finish() : byte[] =
        let cdOffset = uint buf.Count

        // ── Central Directory Headers ─────────────────────────────────────────
        // One 46-byte fixed record per entry, followed by the variable-length name.
        let cdStart = buf.Count
        for e in entries do
            let versionNeeded =
                if e.Method = ZipConstants.MethodDeflate then ZipConstants.VersionDeflate
                else ZipConstants.VersionStored

            writeU32 ZipConstants.CdSig
            writeU16 ZipConstants.VersionMadeBy
            writeU16 versionNeeded
            writeU16 ZipConstants.Flags
            writeU16 e.Method
            writeU16 (uint16 (e.DosDt &&& 0xFFFFu))    // mod_time
            writeU16 (uint16 (e.DosDt >>> 16))          // mod_date
            writeU32 e.Crc
            writeU32 e.CompressedSize
            writeU32 e.UncompressedSize
            writeU16 (uint16 e.Name.Length)
            writeU16 0us                                // extra_len = 0
            writeU16 0us                                // comment_len = 0
            writeU16 0us                                // disk_start = 0
            writeU16 0us                                // internal_attrs = 0
            writeU32 e.ExternalAttrs
            writeU32 e.LocalOffset
            buf.AddRange(e.Name)
            // (no extra field, no file comment)

        let cdSize     = uint (buf.Count - cdStart)
        let numEntries = uint16 entries.Count

        // ── End of Central Directory Record (22 bytes) ────────────────────────
        writeU32 ZipConstants.EocdSig
        writeU16 0us             // disk_number = 0
        writeU16 0us             // disk_with_cd_start = 0
        writeU16 numEntries      // entries_on_this_disk
        writeU16 numEntries      // entries_total
        writeU32 cdSize          // Central Directory byte size
        writeU32 cdOffset        // Central Directory byte offset from archive start
        writeU16 0us             // comment_length = 0

        buf.ToArray()

// =============================================================================
// ZIP Read — ZipReader
// =============================================================================
//
// Strategy (EOCD-first):
//   1. Scan backwards from end of archive for the EOCD signature 0x06054B50.
//      Limit scan to the last 65557 bytes (EOCD 22 + max ZIP comment 65535).
//   2. Read cd_offset + cd_size from EOCD.
//   3. Parse all Central Directory headers into internal metadata.
//   4. Expose a ZipEntry list of names (Data = [||]).
//   5. Read(name): seek to Local Header via local_offset, skip name+extra,
//      read compressed_size bytes, decompress, verify CRC-32.
//
// Security: use Central Directory as the authoritative source for sizes and
// method.  Local Header is consulted only for the variable-length name_len +
// extra_len skip.  This prevents malformed Local Headers from causing over-reads.

// Internal: full metadata per entry needed for lazy reads.
type private ZipEntryMeta =
    { Name             : string
      LocalOffset      : uint
      Method           : uint16
      Crc              : uint
      CompressedSize   : uint
      UncompressedSize : uint
      IsDirectory      : bool }

/// Reads entries from an in-memory ZIP archive.
type ZipReader(data: byte[]) =
    do if isNull data then nullArg "data"

    // ── Little-endian read helpers ─────────────────────────────────────────────

    static let readU16 (d: byte[]) (offset: int) =
        if offset + 2 > d.Length then
            raise (InvalidDataException(sprintf "zip: read U16 at %d out of bounds" offset))
        BinaryPrimitives.ReadUInt16LittleEndian(ReadOnlySpan(d, offset, 2))

    static let readU32 (d: byte[]) (offset: int) =
        if offset + 4 > d.Length then
            raise (InvalidDataException(sprintf "zip: read U32 at %d out of bounds" offset))
        BinaryPrimitives.ReadUInt32LittleEndian(ReadOnlySpan(d, offset, 4))

    // ── EOCD search ────────────────────────────────────────────────────────────
    //
    // Scan backwards from the end of the file for the EOCD signature 0x06054B50.
    // Limit the scan to the last 65557 bytes (22-byte minimum EOCD + 65535-byte
    // maximum ZIP comment) to prevent unbounded searches on malformed archives.

    static let findEocd (d: byte[]) : int option =
        let eocdMinSize = 22
        let maxComment  = 65535

        if d.Length < eocdMinSize then None
        else
            let scanStart = max 0 (d.Length - eocdMinSize - maxComment)
            let mutable result = None
            let mutable i = d.Length - eocdMinSize
            while i >= scanStart && result.IsNone do
                if readU32 d i = ZipConstants.EocdSig then
                    // Validate: comment_len at offset +20 must account for all remaining bytes.
                    let commentLen = int (readU16 d (i + 20))
                    if i + eocdMinSize + commentLen = d.Length then
                        result <- Some i
                i <- i - 1
            result

    // ── Parse Central Directory ────────────────────────────────────────────────

    let meta : ZipEntryMeta list =
        let eocdOffset =
            match findEocd data with
            | Some v -> v
            | None   -> raise (InvalidDataException "zip: no End of Central Directory record found")

        // Read EOCD fields: cd_size at +12, cd_offset at +16.
        let cdOffset = int (readU32 data (eocdOffset + 16))
        let cdSize   = int (readU32 data (eocdOffset + 12))

        if cdOffset + cdSize > data.Length then
            raise (InvalidDataException(
                sprintf "zip: Central Directory [%d, %d) out of bounds (file size %d)"
                    cdOffset (cdOffset + cdSize) data.Length))

        // Parse Central Directory headers.
        let acc = ResizeArray<ZipEntryMeta>()
        let mutable pos = cdOffset
        let mutable cont = true
        while cont && pos + 4 <= cdOffset + cdSize do
            let entryMagic = readU32 data pos
            if entryMagic <> ZipConstants.CdSig then
                cont <- false
            else
                let method           = readU16 data (pos + 10)
                let crc              = readU32 data (pos + 16)
                let compressedSize   = readU32 data (pos + 20)
                let uncompressedSize = readU32 data (pos + 24)
                let nameLen          = int (readU16 data (pos + 28))
                let extraLen         = int (readU16 data (pos + 30))
                let commentLen       = int (readU16 data (pos + 32))
                let localOffset      = readU32 data (pos + 42)

                let nameStart = pos + 46
                let nameEnd   = nameStart + nameLen
                if nameEnd > data.Length then
                    raise (InvalidDataException "zip: CD entry name out of bounds")

                let name = Encoding.UTF8.GetString(data, nameStart, nameLen)
                acc.Add {
                    Name             = name
                    LocalOffset      = localOffset
                    Method           = method
                    Crc              = crc
                    CompressedSize   = compressedSize
                    UncompressedSize = uncompressedSize
                    IsDirectory      = name.EndsWith('/')
                }
                pos <- nameEnd + extraLen + commentLen

        List.ofSeq acc

    // Build the public entry list (name only; data read on demand).
    let entries : ZipEntry list =
        meta |> List.map (fun m -> { Name = m.Name; Data = [||] })

    // ── Internal: read and decompress one entry ────────────────────────────────

    let readEntry (m: ZipEntryMeta) : byte[] =
        if m.IsDirectory then [||]
        else
            let lhOff = int m.LocalOffset

            // Reject encrypted entries (GP flag bit 0 = 1).
            let localFlags = readU16 data (lhOff + 6)
            if (localFlags &&& 1us) <> 0us then
                raise (InvalidDataException(sprintf "zip: entry '%s' is encrypted; not supported" m.Name))

            // The Local Header name_len and extra_len can differ from the CD header,
            // so we must re-read them to find the actual start of the file data.
            let lhNameLen  = int (readU16 data (lhOff + 26))
            let lhExtraLen = int (readU16 data (lhOff + 28))
            let dataStart  = lhOff + 30 + lhNameLen + lhExtraLen
            let dataEnd    = dataStart + int m.CompressedSize

            if dataEnd > data.Length then
                raise (InvalidDataException(
                    sprintf "zip: entry '%s' data [%d, %d) out of bounds" m.Name dataStart dataEnd))

            let compressed = data[dataStart .. dataEnd - 1]

            // Decompress according to method.
            let decompressed =
                match int m.Method with
                | 0 -> compressed  // Stored — verbatim
                | 8 -> DeflateDecompressor.decompress compressed
                | meth ->
                    raise (InvalidDataException(
                        sprintf "zip: unsupported compression method %d for '%s'" meth m.Name))

            // Trim to declared uncompressed size to guard against decompressor over-read.
            let decompressed =
                if decompressed.Length > int m.UncompressedSize then
                    decompressed[.. int m.UncompressedSize - 1]
                else decompressed

            // Verify CRC-32 — this detects corruption of the decompressed content.
            let actualCrc = Crc32.compute decompressed 0u
            if actualCrc <> m.Crc then
                raise (InvalidDataException(
                    sprintf "zip: CRC-32 mismatch for '%s': expected %08X, got %08X" m.Name m.Crc actualCrc))

            decompressed

    // ── Public API ─────────────────────────────────────────────────────────────

    /// All entries in the archive (files and directories) in Central Directory order.
    /// The `Data` field is empty until you call `Read`.
    member _.Entries : ZipEntry list = entries

    /// Decompress and return the data for the named entry.
    /// Throws InvalidDataException on CRC mismatch or corrupt data.
    member _.Read(name: string) : byte[] =
        match meta |> List.tryFind (fun m -> m.Name = name) with
        | None   -> raise (InvalidDataException(sprintf "zip: entry '%s' not found" name))
        | Some m -> readEntry m

// =============================================================================
// Convenience API — ZipArchive
// =============================================================================

/// Convenience functions for one-shot ZIP archive creation and extraction.
[<RequireQualifiedAccess>]
module ZipArchive =

    /// Compress a collection of ZipEntry objects into a ZIP archive.
    /// Each entry is compressed with DEFLATE if that reduces size, otherwise stored.
    let zip (entries: ZipEntry seq) : byte[] =
        if isNull (box entries) then nullArg "entries"
        let writer = ZipWriter()
        for entry in entries do
            if entry.Name.EndsWith('/') then
                writer.AddDirectory(entry.Name)
            else
                writer.AddFile(entry.Name, entry.Data)
        writer.Finish()

    /// Extract all entries from a ZIP archive.
    /// Directory entries are included with empty Data.
    /// Throws InvalidDataException on corrupt archives.
    let unzip (data: byte[]) : ZipEntry list =
        if isNull data then nullArg "data"
        let reader = ZipReader(data)
        reader.Entries
        |> List.map (fun entry ->
            let bytes =
                if entry.Name.EndsWith('/') then [||]
                else reader.Read(entry.Name)
            { Name = entry.Name; Data = bytes })
