namespace CodingAdventures.Lz77.FSharp.Tests

open System
open System.Diagnostics
open System.Text
open CodingAdventures.Lz77.FSharp
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

    [<Fact>]
    member _.``max count header only``() =
        Assert.Empty(quick (fun () -> Lz77.Decompress(b [ 255; 255; 255; 255 ])))

    [<Fact>]
    member _.``count larger than remaining bytes yields present tokens``() =
        let d = b [ 255; 255; 255; 255; 0; 0; 0; 104; 0; 0; 0; 105 ]
        Assert.Equal<byte array>(str "hi", quick (fun () -> Lz77.Decompress d))
        Assert.Equal<byte array>(str "hi", Lz77.Decompress(Array.append d (b [ 0; 0; 0 ])))

    [<Fact>]
    member _.``count smaller than remaining bytes stops at count``() =
        Assert.Equal<byte array>(str "h", Lz77.Decompress(b [ 0; 0; 0; 1; 0; 0; 0; 104; 0; 0; 0; 105 ]))

    [<Fact>]
    member _.``invalid offsets are rejected``() =
        Assert.Throws<InvalidOperationException>(fun () -> Lz77.Decompress(b [ 0; 0; 0; 1; 0; 5; 3; 120 ]) |> ignore) |> ignore
        Assert.Throws<InvalidOperationException>(fun () -> Lz77.Decompress(b [ 0; 0; 0; 1; 0; 0; 3; 120 ]) |> ignore) |> ignore

    [<Fact>]
    member _.``round trip every input of length up to 2``() =
        let inputs = ResizeArray<byte array>()
        inputs.Add [||]
        for a in 0 .. 255 do
            inputs.Add [| byte a |]
            for c in 0 .. 255 do
                inputs.Add [| byte a; byte c |]
        for d in inputs do
            Assert.Equal<byte array>(d, Lz77.Decompress(Lz77.Compress(d)))

    [<Fact>]
    member _.``round trip seeded random inputs``() =
        let rng = Random(0x5EED)
        let alphabets = [| 2; 4; 16; 256 |]
        for i in 0 .. 299 do
            let d = Array.init (3 + rng.Next(597)) (fun _ -> byte (rng.Next(alphabets[i % 4])))
            Assert.Equal<byte array>(d, Lz77.Decompress(Lz77.Compress(d)))
