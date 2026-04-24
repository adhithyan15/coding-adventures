/// ZStd (CMP07) — xunit test suite (F# port).
///
/// Every test round-trips data through Zstd.Compress → Zstd.Decompress and
/// checks bit-for-bit equality with the original. Several tests also verify
/// specific compression-ratio guarantees to ensure the LZ77 + FSE pipeline
/// is actually compressing and not just copying.
module CodingAdventures.Zstd.Tests.ZstdTests

open System
open System.Text
open Xunit
open CodingAdventures.Zstd.FSharp

// ── Helper ────────────────────────────────────────────────────────────────────

/// Round-trip: compress then decompress.
let private rt (data: byte array) : byte array =
    Zstd.Decompress(Zstd.Compress(data))

// ── TC-1: empty input ─────────────────────────────────────────────────────────

[<Fact>]
let Tc1_Empty () =
    // An empty input must produce a valid ZStd frame and decompress back
    // to empty bytes without panic or error.
    Assert.Equal<byte array>(Array.empty, rt Array.empty)

// ── TC-2: literal-only short input ────────────────────────────────────────────

[<Fact>]
let Tc2_Literal () =
    // The smallest non-empty input: one byte, and a short string.
    Assert.Equal<byte array>([| 0x42uy |], rt [| 0x42uy |])
    let hello = Encoding.UTF8.GetBytes("hello")
    Assert.Equal<byte array>(hello, rt hello)

// ── TC-3: all 256 byte values ─────────────────────────────────────────────────

[<Fact>]
let Tc3_AllBytes () =
    // Every possible byte value 0x00..=0xFF in order. This exercises
    // literal encoding of non-ASCII and zero bytes.
    let input = Array.init 256 byte
    Assert.Equal<byte array>(input, rt input)

// ── TC-4: RLE block ───────────────────────────────────────────────────────────

[<Fact>]
let Tc4_RleBlock () =
    // 1024 identical bytes should be detected as an RLE block.
    // Expected compressed size: 4 (magic) + 1 (FHD) + 8 (FCS) + 3 (block header)
    //                         + 1 (RLE byte) = 17 bytes < 30.
    let input = Array.create 1024 (byte 'A')
    let compressed = Zstd.Compress(input)
    Assert.Equal<byte array>(input, Zstd.Decompress(compressed))
    Assert.True(
        (compressed.Length < 30),
        sprintf "RLE of 1024 bytes compressed to %d (expected < 30)" compressed.Length)

// ── TC-5: English prose ───────────────────────────────────────────────────────

[<Fact>]
let Tc5_Prose () =
    // Repeated English text has strong LZ77 matches. Must achieve ≥ 20%
    // compression (output ≤ 80% of input size).
    let text = String.concat "" (List.replicate 25 "the quick brown fox jumps over the lazy dog ")
    let input = Encoding.UTF8.GetBytes(text)
    let compressed = Zstd.Compress(input)
    Assert.Equal<byte array>(input, Zstd.Decompress(compressed))
    let threshold = input.Length * 80 / 100
    Assert.True(
        (compressed.Length < threshold),
        sprintf "prose: compressed %d bytes (input %d), expected < %d (80%%)"
            compressed.Length input.Length threshold)

// ── TC-6: pseudo-random data (deterministic) ──────────────────────────────────

[<Fact>]
let Tc6_Random () =
    // 1000 bytes incrementing mod 251 (deterministic "random").
    // No significant compression expected, but round-trip must be exact.
    let input = Array.init 1000 (fun i -> byte (i % 251))
    Assert.Equal<byte array>(input, rt input)

// ── TC-7: multi-block (300 KB) ────────────────────────────────────────────────

[<Fact>]
let Tc7_Multiblock () =
    // 300 KB > maxBlockSize (128 KB), so this requires at least 2 blocks.
    // Use a repeating pattern so it's actually compressible.
    let pattern = Encoding.UTF8.GetBytes("Hello World ZStd! ")
    let input = Array.init (300 * 1024) (fun i -> pattern[i % pattern.Length])
    Assert.Equal<byte array>(input, rt input)

// ── TC-8: repeat-offset pattern ───────────────────────────────────────────────

[<Fact>]
let Tc8_RepeatOffset () =
    // "abcabc..." 1000 bytes — strong LZ77 match potential.
    let input = Array.init 1000 (fun i -> byte (int 'a' + i % 3))
    Assert.Equal<byte array>(input, rt input)

// ── TC-9: deterministic output ────────────────────────────────────────────────

[<Fact>]
let Tc9_Deterministic () =
    // Compressing the same data twice must produce identical bytes.
    // This is required for reproducible builds and cache invalidation.
    let data = Encoding.UTF8.GetBytes(String.concat "" (List.replicate 50 "hello, ZStd world! "))
    Assert.Equal<byte array>(Zstd.Compress(data), Zstd.Compress(data))

// ── RtRepeatedPattern ─────────────────────────────────────────────────────────

[<Fact>]
let RtRepeatedPattern () =
    // "Hello World! " × 1000
    let data = Encoding.UTF8.GetBytes(String.concat "" (List.replicate 1000 "Hello World! "))
    Assert.Equal<byte array>(data, rt data)

// ── RtBinaryData ─────────────────────────────────────────────────────────────

[<Fact>]
let RtBinaryData () =
    // bytes 0..255 repeated 10×
    let input = Array.init 2560 (fun i -> byte (i % 256))
    Assert.Equal<byte array>(input, rt input)

// ── TestRevBitWriterRevBitReaderRoundtrip ─────────────────────────────────────

[<Fact>]
let TestRevBitWriterRevBitReaderRoundtrip () =
    // The backward bit stream stores bits so the LAST-written bits are
    // read FIRST by the decoder. This mirrors how ZStd's sequence codec
    // writes the initial FSE states last (so the decoder reads them first).
    //
    // Write order:  A=0b101 (3 bits), B=0b11001100 (8 bits), C=0b1 (1 bit)
    // Read order:   C first, then B, then A  (reversed)
    let bw = RevBitWriter()
    bw.AddBits(0b101UL, 3)       // A — written first → read last
    bw.AddBits(0b11001100UL, 8)  // B
    bw.AddBits(0b1UL, 1)         // C — written last → read first
    bw.Flush()
    let buf = bw.Finish()

    let br = RevBitReader(buf)
    Assert.Equal(0b1UL,        br.ReadBits(1)) // C: last written, first read
    Assert.Equal(0b11001100UL, br.ReadBits(8)) // B
    Assert.Equal(0b101UL,      br.ReadBits(3)) // A: first written, last read

// ── TestFseDecodeTableCoverage ────────────────────────────────────────────────

[<Fact>]
let TestFseDecodeTableCoverage () =
    // Every slot in the decode table should be reachable (sym is valid).
    // We use the LL table as the representative case.
    let llNormLocal : int16 array =
        [| 4s; 3s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 1s; 1s; 1s
           2s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 2s; 3s; 2s; 1s; 1s; 1s; 1s; 1s
           -1s; -1s; -1s; -1s |]
    let dt = buildDecodeTable llNormLocal 6
    Assert.Equal(1 <<< 6, dt.Length)
    for cell in dt do
        Assert.True(
            (int cell.Sym < llNormLocal.Length),
            sprintf "sym=%d out of range (norm.Length=%d)" cell.Sym llNormLocal.Length)

// ── TestLlToCodeSmall ─────────────────────────────────────────────────────────

[<Fact>]
let TestLlToCodeSmall () =
    // First 16 LL code mappings are identity: ll=0 → code 0, ll=1 → code 1, etc.
    for i in 0 .. 15 do
        let got = llToCode (uint32 i)
        Assert.True((got = i), sprintf "LL code for literal length %d: expected %d got %d" i i got)

// ── TestMlToCodeSmall ─────────────────────────────────────────────────────────

[<Fact>]
let TestMlToCodeSmall () =
    // ML code 0 maps to match length 3 (minimum match in ZStd).
    Assert.Equal(0, mlToCode 3u)
    // Lengths 3..34 are identity-mapped: ml=3 → code 0, ml=4 → code 1, ...
    for i in 3 .. 34 do
        let got = mlToCode (uint32 i)
        let expected = i - 3
        Assert.True((got = expected),
            sprintf "ML code for match length %d: expected %d got %d" i expected got)
