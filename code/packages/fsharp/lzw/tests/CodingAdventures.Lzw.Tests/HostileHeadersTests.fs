namespace CodingAdventures.Lzw.FSharp.Tests

open System
open System.Diagnostics
open System.Text
open CodingAdventures.Lzw.FSharp
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
    member _.``max header only``() =
        for n in [ 4; 8; 64 ] do
            try
                quick (fun () -> Lzw.Decompress(Array.create n 255uy)) |> ignore
            with :? InvalidOperationException ->
                ()

    [<Fact>]
    member _.``max declared length on small stream``() =
        let c = Lzw.Compress(str "hello hello hello")
        Array.blit (u32 0xFFFFFFFFu) 0 c 0 4
        Assert.Equal<byte array>(str "hello hello hello", quick (fun () -> Lzw.Decompress c))

    [<Fact>]
    member _.``declared length truncates output``() =
        let c = Lzw.Compress(str "hello hello hello")
        Array.blit (u32 5u) 0 c 0 4
        Assert.Equal<byte array>(str "hello", quick (fun () -> Lzw.Decompress c))

    [<Fact>]
    member _.``round trip every input of length up to 2``() =
        let inputs = ResizeArray<byte array>()
        inputs.Add [||]
        for a in 0 .. 255 do
            inputs.Add [| byte a |]
            for c in 0 .. 255 do
                inputs.Add [| byte a; byte c |]
        for d in inputs do
            Assert.Equal<byte array>(d, Lzw.Decompress(Lzw.Compress(d)))

    [<Fact>]
    member _.``round trip seeded random inputs``() =
        let rng = Random(0x5EED)
        let alphabets = [| 2; 4; 16; 256 |]
        for i in 0 .. 299 do
            let d = Array.init (3 + rng.Next(597)) (fun _ -> byte (rng.Next(alphabets[i % 4])))
            Assert.Equal<byte array>(d, Lzw.Decompress(Lzw.Compress(d)))
