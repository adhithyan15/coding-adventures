namespace CodingAdventures.Lz78.FSharp.Tests

open System
open System.Diagnostics
open System.Text
open CodingAdventures.Lz78.FSharp
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

    let hdr (len: uint32) (count: uint32) = Array.append (u32 len) (u32 count)

    [<Fact>]
    member _.``max header only``() =
        Assert.Empty(quick (fun () -> Lz78.Decompress(Array.create 8 255uy)))

    [<Fact>]
    member _.``max length zero tokens``() =
        Assert.Empty(quick (fun () -> Lz78.Decompress(hdr 0xFFFFFFFFu 0u)))

    [<Fact>]
    member _.``count larger than remaining bytes yields present tokens``() =
        let d = Array.append (hdr 2u 0xFFFFFFFFu) (b [ 0; 0; 104; 0; 0; 0; 105; 0 ])
        Assert.Equal<byte array>(str "hi", quick (fun () -> Lz78.Decompress d))

    [<Fact>]
    member _.``length larger than tokens supply``() =
        let d = Array.append (hdr 0xFFFFFFFFu 1u) (b [ 0; 0; 104; 0 ])
        Assert.Equal<byte array>(str "h", quick (fun () -> Lz78.Decompress d))

    [<Fact>]
    member _.``dangling dict index is rejected``() =
        let d = Array.append (hdr 10u 2u) (b [ 0; 0; 97; 0; 0x27; 0x0F; 98; 0 ])
        Assert.Throws<InvalidOperationException>(fun () -> Lz78.Decompress d |> ignore) |> ignore

    [<Fact>]
    member _.``chain tokens stop at declared length``() =
        let n = 20000
        let body = [| for i in 0 .. n - 1 do yield! [| byte (i >>> 8); byte (i &&& 255); 97uy; 0uy |] |]
        let d = Array.append (hdr 1000u (uint32 n)) body
        Assert.Equal(1000, (quick (fun () -> Lz78.Decompress d)).Length)

    [<Fact>]
    member _.``round trip every input of length up to 2``() =
        let inputs = ResizeArray<byte array>()
        inputs.Add [||]
        for a in 0 .. 255 do
            inputs.Add [| byte a |]
            for c in 0 .. 255 do
                inputs.Add [| byte a; byte c |]
        for d in inputs do
            Assert.Equal<byte array>(d, Lz78.Decompress(Lz78.Compress(d)))

    [<Fact>]
    member _.``round trip seeded random inputs``() =
        let rng = Random(0x5EED)
        let alphabets = [| 2; 4; 16; 256 |]
        for i in 0 .. 299 do
            let d = Array.init (3 + rng.Next(597)) (fun _ -> byte (rng.Next(alphabets[i % 4])))
            Assert.Equal<byte array>(d, Lz78.Decompress(Lz78.Compress(d)))
