namespace CodingAdventures.Lzss.FSharp.Tests

open System
open System.Diagnostics
open System.Text
open CodingAdventures.Lzss.FSharp
open Xunit

/// Security regression tests: hostile headers must not drive work or memory.
/// Each case is a few bytes whose header claims an enormous count or length;
/// they must finish quickly with the data actually present.  Also round-trips
/// every input of length <= 2 and a seeded random sample.
type HostileHeadersTests() =
    let b (v: int list) : byte array = v |> List.map byte |> Array.ofList
    let u32 (v: uint32) : byte array =
        b [ int (v >>> 24) &&& 255; int (v >>> 16) &&& 255; int (v >>> 8) &&& 255; int v &&& 255 ]
    let str (s: string) : byte array = Encoding.Latin1.GetBytes s

    let quick (body: unit -> 'T) : 'T =
        let sw = Stopwatch.StartNew()
        let result = body ()
        Assert.True(sw.Elapsed < TimeSpan.FromSeconds 10.0, "hostile input took too long")
        result

    let hdr (len: uint32) (blocks: uint32) = Array.append (u32 len) (u32 blocks)

    [<Fact>]
    member _.``max header only``() =
        Assert.Empty(quick (fun () -> Lzss.Decompress(Array.create 8 255uy)))

    [<Fact>]
    member _.``max length one literal``() =
        Assert.Equal<byte array>(str "x", quick (fun () -> Lzss.Decompress(Array.append (hdr 0xFFFFFFFFu 1u) (b [ 0; 120 ]))))

    [<Fact>]
    member _.``block count larger than remaining bytes``() =
        Assert.Equal<byte array>(str "hi", quick (fun () -> Lzss.Decompress(Array.append (hdr 2u 0xFFFFFFFFu) (b [ 0; 104; 105 ]))))

    [<Fact>]
    member _.``length inconsistent with tokens``() =
        Assert.Equal<byte array>(str "h", Lzss.Decompress(Array.append (hdr 1u 1u) (b [ 0; 104; 105 ])))

    [<Fact>]
    member _.``invalid match offsets are rejected``() =
        Assert.Throws<InvalidOperationException>(fun () -> Lzss.Decompress(Array.append (hdr 100u 1u) (b [ 1; 0; 0; 5 ])) |> ignore) |> ignore
        Assert.Throws<InvalidOperationException>(fun () -> Lzss.Decompress(Array.append (hdr 100u 1u) (b [ 1; 255; 255; 255 ])) |> ignore) |> ignore

    [<Fact>]
    member _.``round trip every input of length up to 2``() =
        let inputs = ResizeArray<byte array>()
        inputs.Add [||]
        for a in 0 .. 255 do
            inputs.Add [| byte a |]
            for c in 0 .. 255 do
                inputs.Add [| byte a; byte c |]
        for d in inputs do
            Assert.Equal<byte array>(d, Lzss.Decompress(Lzss.Compress(d)))

    [<Fact>]
    member _.``round trip seeded random inputs``() =
        let rng = Random(0x5EED)
        let alphabets = [| 2; 4; 16; 256 |]
        for i in 0 .. 299 do
            let d = Array.init (3 + rng.Next(597)) (fun _ -> byte (rng.Next(alphabets[i % 4])))
            Assert.Equal<byte array>(d, Lzss.Decompress(Lzss.Compress(d)))
