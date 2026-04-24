/// Zstandard (ZStd) lossless compression algorithm — CMP07.
///
/// Zstandard (RFC 8878) is a high-ratio, fast compression format created by
/// Yann Collet at Facebook (2015). It combines:
///
///   - LZ77 back-references (via LZSS token generation) to exploit repetition
///     in the data — the same "copy from earlier in the output" trick as
///     DEFLATE, but with a 32 KB window.
///
///   - FSE (Finite State Entropy) coding instead of Huffman for the sequence
///     descriptor symbols. FSE is an asymmetric numeral system that approaches
///     the Shannon entropy limit in a single pass.
///
///   - Predefined decode tables (RFC 8878 Appendix B) so short frames need
///     no table description overhead.
///
/// Frame layout (RFC 8878 §3):
///
///   ┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
///   │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
///   │ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
///   └────────┴─────┴──────────────────────┴────────┴──────────────────┘
///
/// Each block has a 3-byte header:
///   bit 0        = Last_Block flag
///   bits [2:1]   = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
///   bits [23:3]  = Block_Size
///
/// Compression strategy (this implementation):
///   1. Split data into 128 KB blocks (maxBlockSize).
///   2. For each block, try:
///      a. RLE — all bytes identical → 4 bytes total.
///      b. Compressed (LZ77 + FSE) — if output < input length.
///      c. Raw — verbatim copy as fallback.
///
/// Series:
///   CMP00 (LZ77)    — Sliding-window back-references
///   CMP01 (LZ78)    — Explicit dictionary (trie)
///   CMP02 (LZSS)    — LZ77 + flag bits
///   CMP03 (LZW)     — LZ78 + pre-initialised alphabet; GIF
///   CMP04 (Huffman) — Entropy coding
///   CMP05 (DEFLATE) — LZ77 + Huffman; ZIP/gzip/PNG/zlib
///   CMP06 (Brotli)  — DEFLATE + context modelling + static dict
///   CMP07 (ZStd)    — LZ77 + FSE; high ratio + speed  ← this package
module CodingAdventures.Zstd.FSharp

open System
open CodingAdventures.Lzss.FSharp

// ─── Constants ────────────────────────────────────────────────────────────────

/// ZStd magic number: 0xFD2FB528 (little-endian bytes: 28 B5 2F FD).
/// Every valid ZStd frame starts with these 4 bytes.
let private magic : uint32 = 0xFD2FB528u

/// Maximum block size: 128 KB.
/// ZStd allows blocks up to 128 KB. Larger inputs are split across multiple blocks.
let private maxBlockSize = 128 * 1024

// ─── LL / ML / OF code tables (RFC 8878 §3.1.1.3) ────────────────────────────
//
// These tables map a *code number* to a (baseline, extra_bits) pair.
//
// For example, LL code 17 means literal_length = 18 + read(1 extra bit),
// so it covers literal lengths 18 and 19.
//
// The FSE state machine tracks one code number per field; extra bits are
// read directly from the bitstream after state transitions.

/// Literal Length code table: (baseline, extra_bits) for codes 0..=35.
///
/// Literal length 0..15 each have their own code (0 extra bits).
/// Larger lengths are grouped with increasing ranges.
let private llCodes : (uint32 * byte) array =
    [| (0u,0uy); (1u,0uy); (2u,0uy); (3u,0uy); (4u,0uy); (5u,0uy)
       (6u,0uy); (7u,0uy); (8u,0uy); (9u,0uy); (10u,0uy); (11u,0uy)
       (12u,0uy); (13u,0uy); (14u,0uy); (15u,0uy)
       // Grouped ranges start at code 16
       (16u,1uy); (18u,1uy); (20u,1uy); (22u,1uy)
       (24u,2uy); (28u,2uy)
       (32u,3uy); (40u,3uy)
       (48u,4uy); (64u,6uy)
       (128u,7uy); (256u,8uy); (512u,9uy); (1024u,10uy); (2048u,11uy); (4096u,12uy)
       (8192u,13uy); (16384u,14uy); (32768u,15uy); (65536u,16uy) |]

/// Match Length code table: (baseline, extra_bits) for codes 0..=52.
///
/// Minimum match length in ZStd is 3 (not 0). Code 0 = match length 3.
let private mlCodes : (uint32 * byte) array =
    [| // codes 0..31: individual values 3..34
       (3u,0uy); (4u,0uy); (5u,0uy); (6u,0uy); (7u,0uy); (8u,0uy)
       (9u,0uy); (10u,0uy); (11u,0uy); (12u,0uy); (13u,0uy); (14u,0uy)
       (15u,0uy); (16u,0uy); (17u,0uy); (18u,0uy); (19u,0uy); (20u,0uy)
       (21u,0uy); (22u,0uy); (23u,0uy); (24u,0uy); (25u,0uy); (26u,0uy)
       (27u,0uy); (28u,0uy); (29u,0uy); (30u,0uy); (31u,0uy); (32u,0uy)
       (33u,0uy); (34u,0uy)
       // codes 32+: grouped ranges
       (35u,1uy); (37u,1uy); (39u,1uy); (41u,1uy)
       (43u,2uy); (47u,2uy)
       (51u,3uy); (59u,3uy)
       (67u,4uy); (83u,4uy)
       (99u,5uy); (131u,7uy)
       (259u,8uy); (515u,9uy); (1027u,10uy); (2051u,11uy)
       (4099u,12uy); (8195u,13uy); (16387u,14uy); (32771u,15uy); (65539u,16uy) |]

// ─── FSE predefined distributions (RFC 8878 Appendix B) ──────────────────────
//
// "Predefined_Mode" means no per-frame table description is transmitted.
// The decoder builds the same table from these fixed distributions.
//
// Entries of -1 mean "probability 1/table_size" — these symbols get one slot
// in the decode table and their encoder state never needs extra bits.

/// Predefined normalised distribution for Literal Length FSE.
/// Table accuracy log = 6 → 64 slots.
let private llNorm : int16 array =
    [| 4s; 3s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 1s; 1s; 1s
       2s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 3s; 2s; 1s; 1s; 1s; 1s; 1s
       -1s; -1s; -1s; -1s |]
let private llAccLog : int = 6 // table_size = 64

/// Predefined normalised distribution for Match Length FSE.
/// Table accuracy log = 6 → 64 slots.
let private mlNorm : int16 array =
    [| 1s; 4s; 3s; 2s; 2s; 2s; 2s; 2s; 2s; 1s; 1s; 1s; 1s; 1s; 1s; 1s
       1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s
       1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; -1s; -1s
       -1s; -1s; -1s; -1s; -1s |]
let private mlAccLog : int = 6

/// Predefined normalised distribution for Offset FSE.
/// Table accuracy log = 5 → 32 slots.
let private ofNorm : int16 array =
    [| 1s; 1s; 1s; 1s; 1s; 1s; 2s; 2s; 2s; 1s; 1s; 1s; 1s; 1s; 1s; 1s
       1s; 1s; 1s; 1s; 1s; 1s; 1s; 1s; -1s; -1s; -1s; -1s; -1s |]
let private ofAccLog : int = 5 // table_size = 32

// ─── FSE decode table entry ───────────────────────────────────────────────────

/// One cell in the FSE decode table.
///
/// To decode a symbol from state S:
///   1. sym is the output symbol.
///   2. Read nb bits from the bitstream as bits.
///   3. New state = base + bits.
[<Struct>]
type FseDe =
    { /// Decoded symbol.
      Sym: byte
      /// Number of extra bits to read for next state.
      Nb: byte
      /// Base value for next state computation.
      Base: uint16 }

// ─── FSE encode symbol table entry ───────────────────────────────────────────

/// Encode transform for one symbol.
///
/// Given encoder state S for symbol s:
///   nb_out = (S + delta_nb) >> 16   (number of bits to emit)
///   emit low nb_out bits of S
///   new_S  = state_tbl[(S >> nb_out) + delta_fs]
[<Struct>]
type FseEe =
    { /// (max_bits_out << 16) - (count << max_bits_out)
      /// Used to derive nb_out: nb_out = (state + delta_nb) >> 16
      DeltaNb: uint32
      /// cumulative_count_before_sym - count (may be negative)
      /// Used to index state_tbl: new_S = state_tbl[(S >> nb_out) + delta_fs]
      DeltaFs: int }

// ─── Helper: integer log2 ─────────────────────────────────────────────────────

/// Count leading zeros of an unsigned 32-bit integer.
/// Returns 32 for v=0, which avoids a special case in callers.
let private leadingZeros32 (v: uint32) : int =
    if v = 0u then 32
    else
        let mutable n = 0
        let mutable x = v
        if (x &&& 0xFFFF0000u) = 0u then n <- n + 16; x <- x <<< 16
        if (x &&& 0xFF000000u) = 0u then n <- n + 8;  x <- x <<< 8
        if (x &&& 0xF0000000u) = 0u then n <- n + 4;  x <- x <<< 4
        if (x &&& 0xC0000000u) = 0u then n <- n + 2;  x <- x <<< 2
        if (x &&& 0x80000000u) = 0u then n <- n + 1
        n

// ─── FSE table construction ───────────────────────────────────────────────────

/// Build an FSE decode table from a normalised probability distribution.
///
/// The algorithm:
///   1. Place symbols with probability -1 (very rare) at the top of the table.
///   2. Spread remaining symbols using a deterministic step function derived
///      from the table size. This ensures each symbol occupies the correct
///      fraction of slots.
///   3. Assign nb (number of state bits to read) and base to each slot so
///      that the decoder can reconstruct the next state.
///
/// The step function step = (sz >> 1) + (sz >> 3) + 3 is co-prime to sz when
/// sz is a power of two (which it always is in ZStd), ensuring that the walk
/// visits every slot exactly once.
let buildDecodeTable (norm: int16 array) (accLog: int) : FseDe array =
    let sz = 1 <<< accLog
    let step = (sz >>> 1) + (sz >>> 3) + 3
    let tbl = Array.zeroCreate<FseDe> sz
    let symNext = Array.zeroCreate<uint16> norm.Length

    // Phase 1: symbols with probability -1 go at the top (high indices).
    // These symbols each get exactly 1 slot, and their state transition uses
    // the full acc_log bits (they can go to any state).
    let mutable high = sz - 1
    for s in 0 .. norm.Length - 1 do
        if norm[s] = -1s then
            tbl[high] <- { tbl[high] with Sym = byte s }
            if high > 0 then high <- high - 1
            symNext[s] <- 1us

    // Phase 2: spread remaining symbols into the lower portion of the table.
    // Two-pass approach: first symbols with count > 1, then count == 1.
    // This matches the reference implementation's deterministic ordering.
    let mutable pos = 0
    for pass in 0 .. 1 do
        for s in 0 .. norm.Length - 1 do
            if norm[s] > 0s then
                let cnt = int norm[s]
                if (pass = 0) = (cnt > 1) then
                    symNext[s] <- uint16 cnt
                    for _ in 0 .. cnt - 1 do
                        tbl[pos] <- { tbl[pos] with Sym = byte s }
                        pos <- (pos + step) &&& (sz - 1)
                        while pos > high do
                            pos <- (pos + step) &&& (sz - 1)

    // Phase 3: assign nb (number of state bits to read) and base.
    //
    // For a symbol with count cnt occupying slots i₀, i₁, ...:
    //   The next_state counter starts at cnt and increments.
    //   nb = acc_log - floor(log2(next_state))
    //   base = next_state * (1 << nb) - sz
    //
    // This ensures that when we reconstruct state = base + read(nb bits),
    // we land in the range [sz, 2*sz), which is the valid encoder state range.
    let sn = Array.copy symNext
    for i in 0 .. sz - 1 do
        let s = int tbl[i].Sym
        let ns = uint32 sn[s]
        sn[s] <- sn[s] + 1us
        // floor(log2(ns)) = 31 - leadingZeros32(ns)
        let nb = accLog - (31 - leadingZeros32 ns)
        // base = ns * (1 << nb) - sz
        let baseVal = uint16 (int (ns <<< nb) - sz)
        tbl[i] <- { Sym = tbl[i].Sym; Nb = byte nb; Base = baseVal }

    tbl

/// Build FSE encode tables from a normalised distribution.
///
/// Returns:
///   ee[sym]: the FseEe transform for each symbol.
///   st[slot]: the encoder state table (slot → output state in [sz, 2*sz)).
///
/// The encode/decode symmetry:
///   The FSE decoder assigns (sym, nb, base) to each table cell in INDEX ORDER.
///   For symbol s, the j-th cell (in ascending index order) has:
///     ns = count[s] + j
///     nb = acc_log - floor(log2(ns))
///     base = ns * (1<<nb) - sz
///
///   The FSE encoder must use the SAME indexing: slot cumul[s]+j maps to the
///   j-th table cell for symbol s (in ascending index order).
let buildEncodeTable (norm: int16 array) (accLog: int) : FseEe array * uint16 array =
    let sz = 1 <<< accLog

    // Step 1: compute cumulative sums.
    let cumul = Array.zeroCreate<uint32> norm.Length
    let mutable total = 0u
    for s in 0 .. norm.Length - 1 do
        cumul[s] <- total
        let cnt = if norm[s] = -1s then 1u elif norm[s] <= 0s then 0u else uint32 norm[s]
        total <- total + cnt

    // Step 2: build the spread table (which symbol occupies each table slot).
    //
    // This uses the same spreading algorithm as buildDecodeTable, producing
    // a mapping from table index to symbol.
    let step = (sz >>> 1) + (sz >>> 3) + 3
    let spread = Array.zeroCreate<byte> sz
    let mutable idxHigh = sz - 1

    // Phase 1: probability -1 symbols at the high end
    for s in 0 .. norm.Length - 1 do
        if norm[s] = -1s then
            spread[idxHigh] <- byte s
            if idxHigh > 0 then idxHigh <- idxHigh - 1
    let idxLimit = idxHigh

    // Phase 2: spread remaining symbols using the step function
    let mutable pos2 = 0
    for pass in 0 .. 1 do
        for s in 0 .. norm.Length - 1 do
            if norm[s] > 0s then
                let cnt = int norm[s]
                if (pass = 0) = (cnt > 1) then
                    for _ in 0 .. cnt - 1 do
                        spread[pos2] <- byte s
                        pos2 <- (pos2 + step) &&& (sz - 1)
                        while pos2 > idxLimit do
                            pos2 <- (pos2 + step) &&& (sz - 1)

    // Step 3: build the state table by iterating spread in INDEX ORDER.
    //
    // For each table index i (in ascending order), determine which
    // occurrence of symbol s = spread[i] this is (j = 0, 1, 2, ...).
    // The encode slot is cumul[s] + j, and the encoder output state is
    // i + sz (so the decoder, in state i, will decode symbol s).
    let symOcc = Array.zeroCreate<uint32> norm.Length
    let st = Array.zeroCreate<uint16> sz

    for i in 0 .. sz - 1 do
        let s = int spread[i]
        let j = int symOcc[s]
        symOcc[s] <- symOcc[s] + 1u
        let slot = int cumul[s] + j
        st[slot] <- uint16 (i + sz)

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
    let ee = Array.zeroCreate<FseEe> norm.Length
    for s in 0 .. norm.Length - 1 do
        let cnt = if norm[s] = -1s then 1u elif norm[s] <= 0s then 0u else uint32 norm[s]
        if cnt > 0u then
            let mbo =
                if cnt = 1u then uint32 accLog
                else uint32 accLog - uint32 (31 - leadingZeros32 cnt)
            let deltaNb = (mbo <<< 16) - (cnt <<< int mbo)
            let deltaFs = int cumul[s] - int cnt
            ee[s] <- { DeltaNb = deltaNb; DeltaFs = deltaFs }

    (ee, st)

// ─── Reverse bit-writer ───────────────────────────────────────────────────────
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

/// Accumulates bits LSB-first and produces a backward-compatible byte stream.
type RevBitWriter() =
    let buf = ResizeArray<byte>()
    let mutable reg : uint64 = 0UL  // accumulation register (bits fill from LSB)
    let mutable bits : int = 0      // number of valid bits in reg

    /// Add nb low-order bits of val to the stream.
    member _.AddBits(v: uint64, nb: int) =
        if nb > 0 then
            let msk = if nb = 64 then System.UInt64.MaxValue else (1UL <<< nb) - 1UL
            reg <- reg ||| ((v &&& msk) <<< bits)
            bits <- bits + nb
            while bits >= 8 do
                buf.Add(byte (reg &&& 0xFFUL))
                reg <- reg >>> 8
                bits <- bits - 8

    /// Flush remaining bits with a sentinel and mark the stream end.
    ///
    /// The sentinel is a 1 bit placed at position bits in the last byte.
    /// The decoder locates it with leading-zeros arithmetic.
    member _.Flush() =
        let sentinel = byte (1 <<< bits) // bit above all remaining data bits
        let lastByte = byte (reg &&& 0xFFUL) ||| sentinel
        buf.Add(lastByte)
        reg <- 0UL
        bits <- 0

    /// Return the accumulated byte array.
    member _.Finish() : byte array = buf.ToArray()

// ─── Reverse bit-reader ───────────────────────────────────────────────────────
//
// Mirrors RevBitWriter: reads bits from the END of the buffer going backwards.
// The stream is laid out so that the LAST bits written by the encoder are at
// the END of the byte buffer (in the sentinel-containing last byte). The
// reader initialises at the last byte and reads backward toward byte 0.
//
// Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side).
// ReadBits(n) extracts the top n bits and shifts the register left by n.
//
// Why left-aligned? The writer accumulates bits LSB-first. Within each flushed
// byte, bit 0 = earliest written, bit N = latest written. To read the LATEST
// bits first (which were in the highest byte positions and in the high bits of
// each byte), we need a left-aligned register so that reading from the top
// gives the highest-position bits first.

/// Reads bits in reverse order from a backward bit stream.
type RevBitReader(data: byte array) =
    do
        if data.Length = 0 then
            raise (System.IO.InvalidDataException("empty bitstream"))

    let last = data[data.Length - 1]
    do
        if last = 0uy then
            raise (System.IO.InvalidDataException("bitstream last byte is zero (no sentinel)"))

    // sentinel_pos = bit index (0 = LSB) of the sentinel in the last byte.
    // The sentinel is the HIGHEST set bit in the last byte. We scan from bit 7
    // down to 0 and stop at the first set bit.
    let sentinelPos =
        let mutable p = 0
        let mutable found = false
        let mutable b = 7
        while not found && b >= 0 do
            if (last &&& byte (1 <<< b)) <> 0uy then
                p <- b
                found <- true
            b <- b - 1
        p
    let validBits = sentinelPos // number of data bits below the sentinel

    let initMask = if validBits = 0 then 0UL else (1UL <<< validBits) - 1UL
    let initReg =
        if validBits = 0 then 0UL
        else (uint64 last &&& initMask) <<< (64 - validBits)

    let mutable reg : uint64 = initReg    // shift register, valid bits at TOP (MSB side)
    let mutable bitsLoaded : int = validBits // how many valid bits are loaded (from MSB)
    let mutable pos : int = data.Length - 1 // next byte to load (decrements toward 0)

    /// Load more bytes into the register from the stream going backward.
    ///
    /// Each new byte is placed just BELOW the currently loaded bits (in the
    /// left-aligned register, that means at position 64 - bits - 8).
    let reload () =
        while bitsLoaded <= 56 && pos > 0 do
            pos <- pos - 1
            let shift = 64 - bitsLoaded - 8
            reg <- reg ||| (uint64 data[pos] <<< shift)
            bitsLoaded <- bitsLoaded + 8

    do reload ()

    /// Read nb bits from the top of the register (returns 0 if nb = 0).
    ///
    /// This returns the most recently written bits first (highest stream
    /// positions first), mirroring the encoder's backward order.
    member _.ReadBits(nb: int) : uint64 =
        if nb = 0 then 0UL
        else
            let v = reg >>> (64 - nb)
            reg <- if nb = 64 then 0UL else reg <<< nb
            bitsLoaded <- max 0 (bitsLoaded - nb)
            if bitsLoaded < 24 then reload ()
            v

// ─── FSE encode/decode helpers ────────────────────────────────────────────────

/// Encode one symbol into the backward bitstream, updating the FSE state.
///
/// The encoder maintains state in [sz, 2*sz). To emit symbol sym:
///   1. Compute how many bits to flush: nb = (state + delta_nb) >> 16
///   2. Write the low nb bits of state to the bitstream.
///   3. New state = st[(state >> nb) + delta_fs]
///
/// After all symbols are encoded, the final state (minus sz) is written as
/// acc_log bits to allow the decoder to initialise.
let private fseEncodeSym (state: uint32 byref) (sym: int) (ee: FseEe array) (st: uint16 array) (bw: RevBitWriter) =
    let e = ee[sym]
    let nb = int ((state + e.DeltaNb) >>> 16)
    bw.AddBits(uint64 state, nb)
    let slotI = int (state >>> nb) + e.DeltaFs
    let slot = max 0 slotI
    state <- uint32 st[slot]

/// Decode one symbol from the backward bitstream, updating the FSE state.
///
///   1. Look up de[state] to get sym, nb, and base.
///   2. New state = base + read(nb bits).
let private fseDecodeSym (state: uint16 byref) (de: FseDe array) (br: RevBitReader) : byte =
    let e = de[int state]
    let sym = e.Sym
    state <- uint16 (uint32 e.Base + uint32 (br.ReadBits(int e.Nb)))
    sym

// ─── LL/ML/OF code number computation ────────────────────────────────────────

/// Map a literal length value to its LL code number (0..35).
///
/// Codes 0..15 are identity; codes 16+ cover ranges via lookup.
/// Simple linear scan — codes are in increasing baseline order, so the last
/// code whose baseline ≤ ll is the correct code.
let llToCode (ll: uint32) : int =
    let mutable code = 0
    let mutable i = 0
    while i < llCodes.Length do
        let (baseVal, _) = llCodes[i]
        if baseVal <= ll then
            code <- i
            i <- i + 1
        else
            i <- llCodes.Length // break
    code

/// Map a match length value to its ML code number (0..52).
let mlToCode (ml: uint32) : int =
    let mutable code = 0
    let mutable i = 0
    while i < mlCodes.Length do
        let (baseVal, _) = mlCodes[i]
        if baseVal <= ml then
            code <- i
            i <- i + 1
        else
            i <- mlCodes.Length // break
    code

// ─── Sequence struct ──────────────────────────────────────────────────────────

/// One ZStd sequence: (literal_length, match_length, match_offset).
///
/// A sequence means: emit ll literal bytes from the literals section,
/// then copy ml bytes starting off positions back in the output buffer.
/// After all sequences, any remaining literals are appended.
[<Struct>]
type private Seq =
    { /// Literal length: bytes to copy from literal section before this match.
      Ll: uint32
      /// Match length: bytes to copy from output history.
      Ml: uint32
      /// Match offset (1-indexed: 1 = last byte written).
      Off: uint32 }

/// Convert LZSS tokens into ZStd sequences + a flat literals buffer.
///
/// LZSS produces a stream of Literal(byte) and Match{offset, length}.
/// ZStd groups consecutive literals before each match into a single sequence.
/// Any trailing literals (after the last match) go into the literals buffer
/// without a corresponding sequence entry.
let private tokensToSeqs (tokens: LzssToken list) : byte array * Seq list =
    let lits = ResizeArray<byte>()
    let seqs = ResizeArray<Seq>()
    let mutable litRun = 0u

    for tok in tokens do
        match tok with
        | Literal b ->
            lits.Add(b)
            litRun <- litRun + 1u
        | Match(offset, length) ->
            seqs.Add({ Ll = litRun; Ml = uint32 length; Off = uint32 offset })
            litRun <- 0u
    // Trailing literals stay in lits; no sequence for them.
    (lits.ToArray(), List.ofSeq seqs)

// ─── Literals section encoding ────────────────────────────────────────────────
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

/// Encode raw literals as a Raw_Literals section.
let private encodeLiteralsSection (lits: byte array) : byte array =
    let n = lits.Length
    let header = ResizeArray<byte>(n + 3)

    // Raw_Literals header format (RFC 8878 §3.1.1.2.1):
    // bits [1:0] = Literals_Block_Type = 00 (Raw)
    // bits [3:2] = Size_Format: 00 or 10 = 1-byte, 01 = 2-byte, 11 = 3-byte
    //
    // 1-byte:  size in bits [7:3] (5 bits) — header = (size << 3) | 0b000
    // 2-byte:  size in bits [11:4] (12 bits) — header = (size << 4) | 0b0100
    // 3-byte:  size in bits [19:4] (16 bits) — header = (size << 4) | 0b1100
    if n <= 31 then
        // 1-byte header: size_format=00, type=00
        header.Add(byte ((n <<< 3) &&& 0xFF))
    elif n <= 4095 then
        // 2-byte header: size_format=01, type=00 → 0b0100
        let hdr = uint32 (n <<< 4) ||| 0b0100u
        header.Add(byte (hdr &&& 0xFFu))
        header.Add(byte ((hdr >>> 8) &&& 0xFFu))
    else
        // 3-byte header: size_format=11, type=00 → 0b1100
        let hdr = uint32 (n <<< 4) ||| 0b1100u
        header.Add(byte (hdr &&& 0xFFu))
        header.Add(byte ((hdr >>> 8) &&& 0xFFu))
        header.Add(byte ((hdr >>> 16) &&& 0xFFu))

    header.AddRange(lits)
    header.ToArray()

/// Decode literals section, returning (literals, bytes_consumed).
let private decodeLiteralsSection (data: byte array) (startPos: int) : byte array * int =
    if startPos >= data.Length then
        raise (System.IO.InvalidDataException("empty literals section"))

    let b0 = data[startPos]
    let ltype = int b0 &&& 0b11 // bottom 2 bits = Literals_Block_Type

    if ltype <> 0 then
        raise (System.IO.InvalidDataException(
            sprintf "unsupported literals type %d (only Raw=0 supported)" ltype))

    // Decode size_format from bits [3:2] of b0
    let sizeFormat = (int b0 >>> 2) &&& 0b11

    // Decode the literal length and header byte count from size_format.
    //
    // Raw_Literals size_format encoding (RFC 8878 §3.1.1.2.1):
    //   0b00 or 0b10 → 1-byte header: size = b0[7:3] (5 bits, values 0..31)
    //   0b01          → 2-byte LE header: size in bits [11:4] (12 bits, values 0..4095)
    //   0b11          → 3-byte LE header: size in bits [19:4] (20 bits, values 0..1MB)
    let n, headerBytes =
        match sizeFormat with
        | 0 | 2 ->
            // 1-byte header: size in bits [7:3] (5 bits = values 0..31)
            int b0 >>> 3, 1
        | 1 ->
            // 2-byte header: 12-bit size
            if startPos + 2 > data.Length then
                raise (System.IO.InvalidDataException("truncated literals header (2-byte)"))
            let nn = ((int b0 >>> 4) &&& 0xF) ||| (int data[startPos + 1] <<< 4)
            nn, 2
        | 3 ->
            // 3-byte header: 20-bit size
            if startPos + 3 > data.Length then
                raise (System.IO.InvalidDataException("truncated literals header (3-byte)"))
            let nn = ((int b0 >>> 4) &&& 0xF) ||| (int data[startPos + 1] <<< 4) ||| (int data[startPos + 2] <<< 12)
            nn, 3
        | _ -> raise (System.IO.InvalidDataException("impossible size_format"))

    let dataStart = startPos + headerBytes
    let dataEnd = dataStart + n
    if dataEnd > data.Length then
        raise (System.IO.InvalidDataException(
            sprintf "literals data truncated: need %d, have %d" dataEnd data.Length))

    data[dataStart .. dataEnd - 1], dataEnd - startPos

// ─── Sequences section encoding ───────────────────────────────────────────────
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
// Mode 0 = Predefined. We always write 0x00.
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

/// Encode the Number_of_Sequences field per RFC 8878 §3.1.1.1.2.
let private encodeSeqCount (count: int) : byte array =
    if count < 128 then
        [| byte count |]
    elif count < 0x8000 then // 128..32767
        let delta = count - 128
        let b0 = byte (0x80 ||| (delta >>> 8))
        let b1 = byte (delta &&& 0xFF)
        [| b0; b1 |]
    else
        // 32768..131071: 3-byte encoding
        let r = count - 0x7F00
        [| 0xFFuy; byte (r &&& 0xFF); byte ((r >>> 8) &&& 0xFF) |]

/// Decode the Number_of_Sequences field, returning (count, bytes_consumed).
let private decodeSeqCount (data: byte array) (pos: int) : int * int =
    if pos >= data.Length then
        raise (System.IO.InvalidDataException("empty sequence count"))
    let b0 = data[pos]
    if b0 < 128uy then
        int b0, 1
    elif b0 < 0xFFuy then
        // 2-byte: delta = ((b0 & 0x7F) << 8) | b1; count = delta + 128
        if pos + 2 > data.Length then
            raise (System.IO.InvalidDataException("truncated sequence count"))
        let delta = (int (b0 &&& 0x7Fuy) <<< 8) ||| int data[pos + 1]
        delta + 128, 2
    else
        // 3-byte encoding: byte0=0xFF, then (count - 0x7F00) as LE u16
        if pos + 3 > data.Length then
            raise (System.IO.InvalidDataException("truncated sequence count (3-byte)"))
        let count2 = 0x7F00 + int data[pos + 1] + (int data[pos + 2] <<< 8)
        count2, 3

/// Encode the sequences section using predefined FSE tables.
let private encodeSequencesSection (seqs: Seq list) : byte array =
    // Build encode tables (precomputed from the predefined distributions).
    let (eeLl, stLl) = buildEncodeTable llNorm llAccLog
    let (eeMl, stMl) = buildEncodeTable mlNorm mlAccLog
    let (eeOf, stOf) = buildEncodeTable ofNorm ofAccLog

    let szLl = 1u <<< llAccLog
    let szMl = 1u <<< mlAccLog
    let szOf = 1u <<< ofAccLog

    // FSE encoder states start at table_size (= sz).
    // The state range [sz, 2*sz) maps to slot range [0, sz).
    let mutable stateLl = szLl
    let mutable stateMl = szMl
    let mutable stateOf = szOf

    let bw = RevBitWriter()

    // Encode sequences in reverse order.
    let seqArr = Array.ofList seqs
    for si in (seqArr.Length - 1) .. -1 .. 0 do
        let seq = seqArr[si]
        let llCode = llToCode seq.Ll
        let mlCode = mlToCode seq.Ml

        // Offset encoding: raw = offset + 3 (RFC 8878 §3.1.1.3.2.1)
        // code = floor(log2(raw)); extra = raw - (1 << code)
        let rawOff = seq.Off + 3u
        let ofCode =
            if rawOff <= 1u then 0
            else 31 - leadingZeros32 rawOff
        let ofExtra = rawOff - (1u <<< ofCode)

        // Write extra bits (OF, ML, LL in this order for backward stream).
        bw.AddBits(uint64 ofExtra, ofCode)
        let (mlBase, mlBits) = mlCodes[mlCode]
        let mlExtra = seq.Ml - mlBase
        bw.AddBits(uint64 mlExtra, int mlBits)
        let (llBase, llBits) = llCodes[llCode]
        let llExtra = seq.Ll - llBase
        bw.AddBits(uint64 llExtra, int llBits)

        // FSE encode symbols in the order that the backward bitstream reverses
        // to match the decoder's read order (LL first, OF second, ML third).
        //
        // Since the backward stream reverses write order, we write the REVERSE
        // of the decode order: ML → OF → LL (LL is written last = at the top
        // of the bitstream = read first by the decoder).
        //
        // Decode order: LL, OF, ML
        // Encode order (reversed): ML, OF, LL
        fseEncodeSym &stateMl mlCode eeMl stMl bw
        fseEncodeSym &stateOf ofCode eeOf stOf bw
        fseEncodeSym &stateLl llCode eeLl stLl bw

    // Flush final states (low acc_log bits of state - sz).
    bw.AddBits(uint64 (stateOf - szOf), ofAccLog)
    bw.AddBits(uint64 (stateMl - szMl), mlAccLog)
    bw.AddBits(uint64 (stateLl - szLl), llAccLog)
    bw.Flush()

    bw.Finish()

// ─── Block-level compress ─────────────────────────────────────────────────────

/// Compress one block into ZStd compressed block format.
///
/// Returns None if the compressed form is larger than the input (in which
/// case the caller should use a Raw block instead).
let private compressBlock (block: byte array) : byte array option =
    // Use LZSS to generate LZ77 tokens.
    // Window = 32 KB, max match = 255, min match = 3
    let tokens = Lzss.Encode(block, 32768, 255, 3)

    // Convert tokens to ZStd sequences.
    let (lits, seqs) = tokensToSeqs tokens

    // If no sequences were found, LZ77 had nothing to compress.
    if seqs.IsEmpty then None
    else

    let result = ResizeArray<byte>()

    // Encode literals section (Raw_Literals).
    result.AddRange(encodeLiteralsSection lits)

    // Encode sequences section.
    result.AddRange(encodeSeqCount seqs.Length)
    result.Add(0x00uy) // Symbol_Compression_Modes = all Predefined

    let bitstream = encodeSequencesSection seqs
    result.AddRange(bitstream)

    if result.Count >= block.Length then None
    else Some (result.ToArray())

/// Decompress one ZStd compressed block.
///
/// Reads the literals section, sequences section, and applies the sequences
/// to the output buffer to reconstruct the original data.
let private decompressBlock (data: byte array) (blockStart: int) (blockLen: int) (output: ResizeArray<byte>) : unit =
    // ── Literals section ─────────────────────────────────────────────────
    let (lits, litConsumed) = decodeLiteralsSection data blockStart
    let mutable pos = blockStart + litConsumed
    let blockEnd = blockStart + blockLen

    // ── Sequences count ──────────────────────────────────────────────────
    if pos >= blockEnd then
        // Block has only literals, no sequences.
        output.AddRange(lits)
    else

    let (nSeqs, scBytes) = decodeSeqCount data pos
    pos <- pos + scBytes

    if nSeqs = 0 then
        // No sequences — all content is in literals.
        output.AddRange(lits)
    else

    // ── Symbol compression modes ─────────────────────────────────────────
    if pos >= blockEnd then
        raise (System.IO.InvalidDataException("missing symbol compression modes byte"))
    let modesByte = data[pos]
    pos <- pos + 1

    // Check that all modes are Predefined (0).
    let llMode = (int modesByte >>> 6) &&& 3
    let ofMode = (int modesByte >>> 4) &&& 3
    let mlMode = (int modesByte >>> 2) &&& 3
    if llMode <> 0 || ofMode <> 0 || mlMode <> 0 then
        raise (System.IO.InvalidDataException(
            sprintf "unsupported FSE modes: LL=%d OF=%d ML=%d (only Predefined=0 supported)"
                llMode ofMode mlMode))

    // ── FSE bitstream ────────────────────────────────────────────────────
    let bsLen = blockEnd - pos
    if bsLen <= 0 then
        raise (System.IO.InvalidDataException("missing FSE bitstream"))

    let bitstreamSlice = data[pos .. blockEnd - 1]
    let br = RevBitReader(bitstreamSlice)

    // Build decode tables from predefined distributions.
    let dtLl = buildDecodeTable llNorm llAccLog
    let dtMl = buildDecodeTable mlNorm mlAccLog
    let dtOf = buildDecodeTable ofNorm ofAccLog

    // Initialise FSE states from the bitstream.
    // The encoder wrote: state_ll, state_ml, state_of (each as acc_log bits).
    // The decoder reads them in the same order.
    let mutable stateLl = uint16 (br.ReadBits(llAccLog))
    let mutable stateMl = uint16 (br.ReadBits(mlAccLog))
    let mutable stateOf = uint16 (br.ReadBits(ofAccLog))

    // Track position in the literals buffer.
    let mutable litPos = 0

    // Apply each sequence.
    for _ in 0 .. nSeqs - 1 do
        // Decode symbols (state transitions) — order: LL, OF, ML.
        let llCode = fseDecodeSym &stateLl dtLl br
        let ofCode = fseDecodeSym &stateOf dtOf br
        let mlCode = fseDecodeSym &stateMl dtMl br

        // Validate codes.
        if int llCode >= llCodes.Length then
            raise (System.IO.InvalidDataException(sprintf "invalid LL code %d" llCode))
        if int mlCode >= mlCodes.Length then
            raise (System.IO.InvalidDataException(sprintf "invalid ML code %d" mlCode))

        let (llBase, llBits) = llCodes[int llCode]
        let (mlBase, mlBits) = mlCodes[int mlCode]

        let ll = llBase + uint32 (br.ReadBits(int llBits))
        let ml = mlBase + uint32 (br.ReadBits(int mlBits))
        // Offset: raw = (1 << of_code) | extra_bits; offset = raw - 3
        let ofRaw = (1u <<< int ofCode) ||| uint32 (br.ReadBits(int ofCode))
        if ofRaw < 3u then
            raise (System.IO.InvalidDataException(
                sprintf "decoded offset underflow: of_raw=%d" ofRaw))
        let offset = ofRaw - 3u

        // Emit ll literal bytes from the literals buffer.
        let litEnd = litPos + int ll
        if litEnd > lits.Length then
            raise (System.IO.InvalidDataException(
                sprintf "literal run %d overflows literals buffer (pos=%d len=%d)"
                    ll litPos lits.Length))
        output.AddRange(lits[litPos .. litEnd - 1])
        litPos <- litEnd

        // Copy ml bytes from offset back in the output buffer.
        if offset = 0u || int offset > output.Count then
            raise (System.IO.InvalidDataException(
                sprintf "bad match offset %d (output len %d)" offset output.Count))
        let copyStart = output.Count - int offset
        for j in 0 .. int ml - 1 do
            output.Add(output[copyStart + j])

    // Any remaining literals after the last sequence.
    if litPos < lits.Length then
        output.AddRange(lits[litPos ..])

// ─── Public API ───────────────────────────────────────────────────────────────

/// Pure F# ZStd compression and decompression (RFC 8878 / CMP07).
[<AbstractClass; Sealed>]
type Zstd private () =

    /// Compress data to ZStd format (RFC 8878).
    ///
    /// The output is a valid ZStd frame that can be decompressed by the
    /// zstd CLI tool or any conforming implementation.
    ///
    /// Example:
    ///   let compressed = Zstd.Compress(System.Text.Encoding.UTF8.GetBytes("hello!"))
    ///   let original   = Zstd.Decompress(compressed)
    static member Compress(data: byte array) : byte array =
        if isNull data then nullArg "data"
        let out = ResizeArray<byte>()

        // ── ZStd frame header ─────────────────────────────────────────────────
        // Magic number (4 bytes LE).
        let magicBytes = BitConverter.GetBytes(magic)
        if BitConverter.IsLittleEndian then
            out.AddRange(magicBytes)
        else
            out.AddRange(Array.rev magicBytes)

        // Frame Header Descriptor (FHD):
        //   bit 7-6: FCS_Field_Size flag = 11 → 8-byte FCS
        //   bit 5:   Single_Segment_Flag = 1 (no Window_Descriptor follows)
        //   bit 4:   Content_Checksum_Flag = 0
        //   bit 3-2: reserved = 0
        //   bit 1-0: Dict_ID_Flag = 0
        // = 0b1110_0000 = 0xE0
        out.Add(0xE0uy)

        // Frame_Content_Size (8 bytes LE) — the uncompressed size.
        // A decoder can use this to pre-allocate the output buffer.
        let fcsBytes = BitConverter.GetBytes(uint64 data.Length)
        if BitConverter.IsLittleEndian then
            out.AddRange(fcsBytes)
        else
            out.AddRange(Array.rev fcsBytes)

        // ── Blocks ────────────────────────────────────────────────────────────
        // Handle the special case of completely empty input: emit one empty raw block.
        if data.Length = 0 then
            // Last=1, Type=Raw(00), Size=0 → header = 0b0000_0001 = 0x01
            out.Add(0x01uy)
            out.Add(0x00uy)
            out.Add(0x00uy)
            out.ToArray()
        else

        let mutable offset = 0
        while offset < data.Length do
            let endIdx = min (offset + maxBlockSize) data.Length
            let blockLen = endIdx - offset
            let last = endIdx = data.Length

            // ── Try RLE block ─────────────────────────────────────────────────
            // If all bytes in the block are identical, a single-byte RLE block
            // encodes it in just 1 byte (plus 3-byte header = 4 bytes total).
            let firstByte = data[offset]
            let allSame =
                let mutable same = true
                let mutable i = offset + 1
                while i < endIdx && same do
                    if data[i] <> firstByte then same <- false
                    i <- i + 1
                same

            if allSame then
                // RLE block header: type=01, size=blockLen, last=1/0
                let hdr = (uint32 blockLen <<< 3) ||| (0b01u <<< 1) ||| (if last then 1u else 0u)
                out.Add(byte (hdr &&& 0xFFu))
                out.Add(byte ((hdr >>> 8) &&& 0xFFu))
                out.Add(byte ((hdr >>> 16) &&& 0xFFu))
                out.Add(firstByte)
            else
                // ── Try compressed block ──────────────────────────────────────
                let block = data[offset .. endIdx - 1]
                match compressBlock block with
                | Some compressed ->
                    let hdr = (uint32 compressed.Length <<< 3) ||| (0b10u <<< 1) ||| (if last then 1u else 0u)
                    out.Add(byte (hdr &&& 0xFFu))
                    out.Add(byte ((hdr >>> 8) &&& 0xFFu))
                    out.Add(byte ((hdr >>> 16) &&& 0xFFu))
                    out.AddRange(compressed)
                | None ->
                    // ── Raw block (fallback) ──────────────────────────────────
                    let hdr = (uint32 blockLen <<< 3) ||| (0b00u <<< 1) ||| (if last then 1u else 0u)
                    out.Add(byte (hdr &&& 0xFFu))
                    out.Add(byte ((hdr >>> 8) &&& 0xFFu))
                    out.Add(byte ((hdr >>> 16) &&& 0xFFu))
                    out.AddRange(data[offset .. endIdx - 1])

            offset <- endIdx

        out.ToArray()

    /// Decompress a ZStd frame, returning the original data.
    ///
    /// Accepts any valid ZStd frame with Raw, RLE, or Compressed blocks using
    /// Predefined FSE modes.
    ///
    /// Raises InvalidDataException if the input is truncated, has a bad magic
    /// number, or contains unsupported features (non-predefined FSE tables,
    /// Huffman literals, reserved block types).
    static member Decompress(data: byte array) : byte array =
        if isNull data then nullArg "data"
        if data.Length < 5 then
            raise (System.IO.InvalidDataException("frame too short"))

        // ── Validate magic ────────────────────────────────────────────────────
        let magicRead =
            uint32 data[0] |||
            (uint32 data[1] <<< 8) |||
            (uint32 data[2] <<< 16) |||
            (uint32 data[3] <<< 24)
        if magicRead <> magic then
            raise (System.IO.InvalidDataException(
                sprintf "bad magic: 0x%08X (expected 0x%08X)" magicRead magic))

        let mutable pos = 4

        // ── Parse Frame Header Descriptor ─────────────────────────────────────
        // FHD encodes several flags that control the header layout.
        let fhd = data[pos]
        pos <- pos + 1

        // FCS_Field_Size: bits [7:6] of FHD.
        //   00 → 0 bytes if Single_Segment=0, else 1 byte
        //   01 → 2 bytes
        //   10 → 4 bytes
        //   11 → 8 bytes
        let fcsFlag = (int fhd >>> 6) &&& 3

        // Single_Segment_Flag: bit 5. When set, the window descriptor is omitted.
        let singleSeg = (int fhd >>> 5) &&& 1

        // Dict_ID_Flag: bits [1:0]. Indicates how many bytes the dict ID occupies.
        let dictFlag = int fhd &&& 3

        // ── Window Descriptor ─────────────────────────────────────────────────
        // Present only if Single_Segment_Flag = 0. We skip it.
        if singleSeg = 0 then pos <- pos + 1

        // ── Dict ID ───────────────────────────────────────────────────────────
        let dictIdBytes =
            match dictFlag with
            | 0 -> 0 | 1 -> 1 | 2 -> 2 | _ -> 4
        pos <- pos + dictIdBytes

        // ── Frame Content Size ────────────────────────────────────────────────
        // We read but don't validate FCS.
        let fcsBytes =
            match fcsFlag with
            | 0 -> if singleSeg = 1 then 1 else 0
            | 1 -> 2 | 2 -> 4 | _ -> 8
        pos <- pos + fcsBytes

        // ── Blocks ────────────────────────────────────────────────────────────
        // Guard against decompression bombs: cap total output at 256 MB.
        let maxOutput = 256 * 1024 * 1024
        let output = ResizeArray<byte>()

        let mutable keepGoing = true
        while keepGoing do
            if pos + 3 > data.Length then
                raise (System.IO.InvalidDataException("truncated block header"))

            // 3-byte little-endian block header.
            let hdr =
                uint32 data[pos] |||
                (uint32 data[pos + 1] <<< 8) |||
                (uint32 data[pos + 2] <<< 16)
            pos <- pos + 3

            let last = (hdr &&& 1u) <> 0u
            let btype = int ((hdr >>> 1) &&& 3u)
            let bsize = int (hdr >>> 3)

            match btype with
            | 0 ->
                // Raw block: bsize bytes of verbatim content.
                if pos + bsize > data.Length then
                    raise (System.IO.InvalidDataException(
                        sprintf "raw block truncated: need %d bytes at pos %d" bsize pos))
                if output.Count + bsize > maxOutput then
                    raise (System.IO.InvalidDataException(
                        sprintf "decompressed size exceeds limit of %d bytes" maxOutput))
                output.AddRange(data[pos .. pos + bsize - 1])
                pos <- pos + bsize
            | 1 ->
                // RLE block: 1 byte repeated bsize times.
                if pos >= data.Length then
                    raise (System.IO.InvalidDataException("RLE block missing byte"))
                if output.Count + bsize > maxOutput then
                    raise (System.IO.InvalidDataException(
                        sprintf "decompressed size exceeds limit of %d bytes" maxOutput))
                let rleByte = data[pos]
                pos <- pos + 1
                for _ in 0 .. bsize - 1 do
                    output.Add(rleByte)
            | 2 ->
                // Compressed block.
                if pos + bsize > data.Length then
                    raise (System.IO.InvalidDataException(
                        sprintf "compressed block truncated: need %d bytes" bsize))
                decompressBlock data pos bsize output
                pos <- pos + bsize
            | 3 ->
                raise (System.IO.InvalidDataException("reserved block type 3"))
            | _ ->
                raise (System.IO.InvalidDataException(sprintf "unknown block type %d" btype))

            if last then keepGoing <- false

        output.ToArray()
