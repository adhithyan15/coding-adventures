namespace CodingAdventures.Zstd.FSharp.Tests

open System
open System.Collections.Generic
open System.Diagnostics
open System.IO
open System.Text
open CodingAdventures.Zstd.FSharp
open Xunit

/// TC-9 CLI interoperability helpers: shells out to the real `zstd` binary
/// (github.com/facebook/zstd) so our encoder/decoder are checked against an
/// independent implementation, not just against themselves. This is the
/// class of test that actually catches wire-format bugs: two of the three
/// FSE sequences-codec bugs documented in lessons.md Lesson 96 were
/// internally self-consistent (our own decoder correctly read our own
/// encoder's output) and were invisible to every round-trip-only test in
/// this file — they were only caught by decompressing our output with a
/// genuinely independent decoder.
module private ZstdCli =
    /// True if a `zstd` binary answering `--version` is reachable on PATH.
    let isAvailable () =
        try
            use process' = new Process()
            process'.StartInfo.FileName <- "zstd"
            process'.StartInfo.ArgumentList.Add "--version"
            process'.StartInfo.RedirectStandardOutput <- true
            process'.StartInfo.RedirectStandardError <- true
            process'.StartInfo.UseShellExecute <- false
            process'.Start() |> ignore
            process'.StandardOutput.ReadToEnd() |> ignore
            process'.StandardError.ReadToEnd() |> ignore
            process'.WaitForExit()
            process'.ExitCode = 0
        with _ -> false

    /// Runs `zstd` with the given arguments, feeding `input` on stdin and
    /// returning whatever it wrote to stdout. Throws with the captured
    /// stderr text if the process exits non-zero.
    let run (arguments: string list) (input: byte array) =
        use process' = new Process()
        process'.StartInfo.FileName <- "zstd"
        for argument in arguments do
            process'.StartInfo.ArgumentList.Add argument
        process'.StartInfo.RedirectStandardInput <- true
        process'.StartInfo.RedirectStandardOutput <- true
        process'.StartInfo.RedirectStandardError <- true
        process'.StartInfo.UseShellExecute <- false
        process'.Start() |> ignore

        use stdout = new MemoryStream()
        let copyTask = process'.StandardOutput.BaseStream.CopyToAsync stdout

        process'.StandardInput.BaseStream.Write(input, 0, input.Length)
        process'.StandardInput.BaseStream.Close()
        copyTask.Wait()
        let stderrText = process'.StandardError.ReadToEnd()
        process'.WaitForExit()

        if process'.ExitCode <> 0 then
            let message = "zstd exited " + string process'.ExitCode + ": " + stderrText
            failwith message

        stdout.ToArray()

type ZstdTests() =
    let blockTypes (frame: byte array) =
        let mutable position = 13
        let result = ResizeArray<int>()
        let mutable last = false

        while not last do
            let header = int frame[position] ||| (int frame[position + 1] <<< 8) ||| (int frame[position + 2] <<< 16)
            position <- position + 3
            last <- (header &&& 1) <> 0
            let blockType = (header >>> 1) &&& 3
            let size = header >>> 3
            result.Add blockType
            position <- position + (if blockType = 1 then 1 else size)

        result.ToArray()

    [<Fact>]
    member _.``empty input round trips and produces a frame``() =
        let compressed = Zstd.Compress [||]
        Assert.Empty(Zstd.Decompress compressed)
        Assert.Equal(16, compressed.Length)
        Assert.Equal<byte array>([| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy |], compressed[..3])

    [<Theory>]
    [<InlineData(0)>]
    [<InlineData(0x42)>]
    [<InlineData(0xFF)>]
    member _.``single bytes round trip``(value: int) =
        let input = [| byte value |]
        Assert.Equal<byte array>(input, Zstd.Decompress(Zstd.Compress input))

    [<Fact>]
    member _.``all byte values round trip as raw data``() =
        let input = Array.init 256 byte
        let compressed = Zstd.Compress input
        Assert.Equal<byte array>(input, Zstd.Decompress compressed)
        Assert.Equal<int array>([| 0 |], blockTypes compressed)

    [<Theory>]
    [<InlineData(0)>]
    [<InlineData(0x41)>]
    [<InlineData(0xFF)>]
    member _.``repeated bytes use RLE``(value: int) =
        let input = Array.create 1024 (byte value)
        let compressed = Zstd.Compress input
        Assert.Equal<byte array>(input, Zstd.Decompress compressed)
        Assert.True(compressed.Length < 30)
        Assert.Equal<int array>([| 1 |], blockTypes compressed)

    [<Fact>]
    member _.``English prose uses compressed blocks``() =
        let input = Encoding.UTF8.GetBytes(String.concat "" (List.replicate 25 "the quick brown fox jumps over the lazy dog "))
        let compressed = Zstd.Compress input
        Assert.Equal<byte array>(input, Zstd.Decompress compressed)
        Assert.True(float compressed.Length < float input.Length * 0.8)
        Assert.Equal<int array>([| 2 |], blockTypes compressed)

    [<Fact>]
    member _.``deterministic binary data round trips``() =
        let mutable seed = 42u
        let input = Array.zeroCreate<byte> 512

        for index in 0 .. input.Length - 1 do
            seed <- seed * 1_664_525u + 1_013_904_223u
            input[index] <- byte seed

        Assert.Equal<byte array>(input, Zstd.Decompress(Zstd.Compress input))

    [<Fact>]
    member _.``multi-block RLE frame round trips``() =
        let input = Array.create (200 * 1024) (byte 'x')
        let compressed = Zstd.Compress input
        Assert.Equal<byte array>(input, Zstd.Decompress compressed)
        Assert.Equal<int array>([| 1; 1 |], blockTypes compressed)
        Assert.True(compressed.Length < 50)

    [<Fact>]
    member _.``multi-block compressed frame round trips``() =
        let input = Encoding.ASCII.GetBytes(String.concat "" (List.replicate 10_000 "ABCDEFGHIJKLMNOP"))
        let compressed = Zstd.Compress input
        let types = blockTypes compressed
        Assert.Equal<byte array>(input, Zstd.Decompress compressed)
        Assert.True(types.Length > 1)
        Assert.All<int>(types, fun blockType -> Assert.Equal(2, blockType))

    [<Fact>]
    member _.``repeat distance pattern compresses efficiently``() =
        let bytes = ResizeArray<byte>(Encoding.ASCII.GetBytes "ABCDEFGH")

        for _ in 0..9 do
            bytes.AddRange(Array.create 128 (byte 'X'))
            bytes.AddRange(Encoding.ASCII.GetBytes "ABCDEFGH")

        let input = bytes.ToArray()
        let compressed = Zstd.Compress input
        Assert.Equal<byte array>(input, Zstd.Decompress compressed)
        Assert.True(float compressed.Length < float input.Length * 0.7)

    [<Fact>]
    member _.``compression is deterministic``() =
        let input = Encoding.ASCII.GetBytes(String.concat "" (List.replicate 50 "hello zstd world! "))
        Assert.Equal<byte array>(Zstd.Compress input, Zstd.Compress input)

    [<Fact>]
    member _.``hand-crafted raw frame decodes``() =
        let frame = [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0x05uy; 0x29uy; 0uy; 0uy; byte 'h'; byte 'e'; byte 'l'; byte 'l'; byte 'o' |]
        Assert.Equal<byte array>(Encoding.ASCII.GetBytes "hello", Zstd.Decompress frame)

    [<Fact>]
    member _.``hand-crafted RLE frame decodes``() =
        let frame = [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0x0Auy; 0x53uy; 0uy; 0uy; 0x41uy |]
        Assert.Equal<byte array>(Array.create 10 (byte 'A'), Zstd.Decompress frame)

    [<Fact>]
    member _.``multi-segment headers and checksums are consumed``() =
        // FHD 0x04 sets Content_Checksum_flag (bit 2, RFC 8878 §3.1.1.1) —
        // NOT bit 4 (0x10), which is Unused_bit. See lessons.md Lesson 95.
        let frame = [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x04uy; 0uy; 0x09uy; 0uy; 0uy; byte 'x'; 1uy; 2uy; 3uy; 4uy |]
        Assert.Equal<byte array>([| byte 'x' |], Zstd.Decompress frame)

    [<Fact>]
    member _.``unused bit 4 is ignored, not enforced zero``() =
        // Bit 4 is Unused_bit per RFC 8878, not Content_Checksum_flag (that's
        // bit 2) and not Reserved_bit (that's bit 3) — a decoder must not
        // reject a frame merely because it's set, nor mistake it for a
        // checksum flag. This is the exact descriptor byte (0x10) an
        // earlier, buggy revision of this codec used to mean "checksum
        // present"; here it correctly means nothing at all, and no trailing
        // checksum bytes follow the block. See lessons.md Lesson 95.
        let frame = [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x10uy; 0uy; 0x09uy; 0uy; 0uy; byte 'x' |]
        Assert.Equal<byte array>([| byte 'x' |], Zstd.Decompress frame)

    [<Fact>]
    member _.``dictionary ID and content-size forms are skipped``() =
        for descriptor in [| 0x21uy; 0x62uy; 0xA3uy |] do
            let dictionaryBytes = match descriptor &&& 3uy with | 1uy -> 1 | 2uy -> 2 | _ -> 4
            let contentBytes = match descriptor >>> 6 with | 0uy -> 1 | 1uy -> 2 | 2uy -> 4 | _ -> 8
            let frame = ResizeArray<byte>([| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; descriptor |])
            frame.AddRange(Array.zeroCreate (dictionaryBytes + contentBytes))
            frame.AddRange([| 1uy; 0uy; 0uy |])
            Assert.Empty(Zstd.Decompress(frame.ToArray()))

    [<Fact>]
    member _.``null inputs are rejected``() =
        let nullBytes = Unchecked.defaultof<byte array>
        Assert.Throws<ArgumentNullException>(fun () -> Zstd.Compress nullBytes |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Zstd.Decompress nullBytes |> ignore) |> ignore

    // ─── TC-9: real `zstd` CLI interoperability ────────────────────────────
    //
    // xunit 2.x has no built-in "skip at runtime" mechanism (that arrived in
    // xunit v3's Assert.Skip); these tests approximate JUnit's
    // Assumptions.assumeTrue by returning early — with no assertions run —
    // when `zstd` isn't reachable on PATH, so CI environments without the
    // CLI installed neither fail nor falsely report the interop path as
    // exercised.
    [<Fact>]
    member _.``TC-9: our compressed output decompresses with the real zstd CLI``() =
        if ZstdCli.isAvailable () then
            let input =
                Encoding.ASCII.GetBytes(
                    String.concat "" (List.replicate 200 "the quick brown fox jumps over the lazy dog ABCDEFGH "))

            let compressed = Zstd.Compress input
            let decompressedByCli = ZstdCli.run [ "-d"; "-c" ] compressed
            Assert.Equal<byte array>(input, decompressedByCli)

    [<Fact>]
    member _.``TC-9: real zstd CLI output decompresses here``() =
        if ZstdCli.isAvailable () then
            let input =
                Encoding.ASCII.GetBytes(
                    String.concat "" (List.replicate 200 "the quick brown fox jumps over the lazy dog ABCDEFGH "))

            // --no-compress-literals: this educational decoder only supports
            // Raw_Literals_Block (RFC 8878 §3.1.1.3.1, Literals_Block_Type
            // 0), not Huffman-coded literals (type 2/3) — matching the
            // documented scope of this and every sibling zstd port in this
            // repo. Real `zstd`'s default heuristic picks Huffman literals
            // once the literals section is large/complex enough (it does
            // for this input without the flag), which is an intentional
            // out-of-scope limitation, not a sequences-codec bug.
            let compressedByCli = ZstdCli.run [ "-c"; "--no-compress-literals" ] input
            Assert.Equal<byte array>(input, Zstd.Decompress compressedByCli)

    [<Fact>]
    member _.``TC-9: high sequence count round trips through the real zstd CLI``() =
        // Regression coverage for the specific bug class in lessons.md
        // Lesson 96: a block with many sequences exercises the FSE
        // state-update/skip-update transition many times, and also crosses
        // the sequence-count wire encoding's 1-byte -> 2-byte boundary (128)
        // — exactly where an earlier revision of this codec (and the
        // shared-design Rust/Java/Kotlin ports) diverged from the real wire
        // format while still round-tripping against itself.
        //
        // One direction only (ours compresses, real `zstd -d` decodes):
        // real zstd's own encoder heuristic switches away from predefined
        // FSE tables (RFC 8878's Predefined_Mode) to custom per-frame
        // tables once a block has "enough" sequences with a non-trivial
        // symbol distribution — which this codec deliberately does not
        // support (see the "Educational simplification" note in README.md).
        // That's a separate, intentional scope limit unrelated to the
        // sequences-codec conformance bug this test targets, so the reverse
        // direction isn't exercised here. A repeating 6-byte cycle over
        // 50,000 bytes gives LZSS plenty of short, distinct matches — our
        // own encoder emits 197 sequences for this input (verified: it
        // crosses the sequence-count wire encoding's 128-sequence 1-byte ->
        // 2-byte boundary), while its narrow LL/ML/OF code distribution
        // keeps real zstd's own encoder inside Predefined_Mode on the
        // forward direction covered by the other TC-9 tests above.
        if ZstdCli.isAvailable () then
            let cycle = Encoding.ASCII.GetBytes "ABCDEF"
            let input = Array.init 50_000 (fun index -> cycle[index % cycle.Length])

            let compressed = Zstd.Compress input
            let decompressedByCli = ZstdCli.run [ "-d"; "-c" ] compressed
            Assert.Equal<byte array>(input, decompressedByCli)

    [<Fact>]
    member _.``TC-9: real zstd CLI Repeated-Offset (R1/R2/R3) sequences decode correctly``() =
        // Regression coverage for lessons.md Lesson 98 (see PR #9941,
        // c/zstd): this package's own encoder is, by design, incapable of
        // emitting an Offset_Value <= 3 (the minimum LZSS match offset is
        // 1, so `rawOffset = offset + 3 >= 4` always) — so a self round
        // trip, and every existing TC-9 test above (which only compresses
        // WITH this package's own encoder, or decompresses a real-CLI frame
        // built from non-repetitive prose), never exercises the decoder's
        // handling of Offset_Value 1/2/3 as a Repeated-Offset reference
        // (RFC 8878 §3.1.1.3.2.1.1) rather than a literal `code - 3`.
        //
        // Real `zstd`'s own encoder uses repeat offsets constantly — long
        // constant-byte runs are an easy, reliable way to force it: 4713
        // bytes of a single repeated byte compresses (at the default level,
        // WITHOUT --no-check so the interop path also covers a real
        // checksum trailer) to a single Compressed block whose one sequence
        // is 2 literal bytes ("ZZ") + a match with Offset_Value=1 — i.e.
        // "reuse Repeated_Offset1", which starts at its RFC-mandated
        // default of 1. Before the Lesson-98 fix, DecompressBlock computed
        // `matchOffset = rawOffset - 3` unconditionally, so `rawOffset = 1`
        // underflowed to a huge bogus offset that the existing
        // offset-bounds check (`matchOffset > output.Count`) correctly
        // rejected as malformed — even though the frame is perfectly valid,
        // just encoded via a mechanism the decoder didn't understand yet.
        if ZstdCli.isAvailable () then
            let input = Array.create 4713 (byte 'Z')
            let compressedByCli = ZstdCli.run [ "-c" ] input
            Assert.Equal<byte array>(input, Zstd.Decompress compressedByCli)

    [<Fact>]
    member _.``TC-9: real zstd CLI Repeated-Offset sequences decode correctly across a periodic pattern``() =
        // A second, independent Repeated-Offset repro using a periodic
        // (non-constant) pattern rather than a single repeated byte, so the
        // regression coverage doesn't depend on real zstd's RLE-via-
        // repeat-offset heuristic for constant-byte runs specifically.
        // Six-byte cycles over many repetitions give real zstd's encoder
        // many equal-distance matches, which its cost model strongly
        // favours encoding as Repeated-Offset (R1) references once the same
        // offset has been used twice in a row.
        if ZstdCli.isAvailable () then
            let cycle = Encoding.ASCII.GetBytes "ABCDEF"
            let input = Array.init 3000 (fun index -> cycle[index % cycle.Length])
            let compressedByCli = ZstdCli.run [ "-c" ] input
            Assert.Equal<byte array>(input, Zstd.Decompress compressedByCli)

    [<Fact>]
    member _.``malformed frames are rejected``() =
        let malformed =
            [| [||]
               [| 0x28uy; 0xB5uy; 0x2Fuy |]
               Encoding.ASCII.GetBytes "not zstd data"
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x2Cuy |]
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy |]
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0uy; 0x29uy; 0uy; 0uy; byte 'h' |]
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0uy; 0x53uy; 0uy; 0uy |]
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0uy; 0x07uy; 0uy; 0uy |]
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0uy; 0x01uy; 0uy; 0uy; 0xFFuy |]
               [| 0x28uy; 0xB5uy; 0x2Fuy; 0xFDuy; 0x20uy; 0uy; 0x05uy; 0uy; 0uy |] |]

        for frame in malformed do
            Assert.Throws<InvalidDataException>(fun () -> Zstd.Decompress frame |> ignore) |> ignore
