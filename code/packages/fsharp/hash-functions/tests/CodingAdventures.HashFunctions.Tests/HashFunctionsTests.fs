namespace CodingAdventures.HashFunctions.Tests

open System
open System.Text
open Xunit
open CodingAdventures.HashFunctions.FSharp

module HashFunctionsTests =
    let bytes (text: string) = Encoding.UTF8.GetBytes text

    [<Fact>]
    let ``fnv1a known vectors`` () =
        Assert.Equal(0x811C9DC5u, HashFunctions.fnv1a32Bytes Array.empty<byte>)
        Assert.Equal(0xE40C292Cu, HashFunctions.fnv1a32 "a")
        Assert.Equal(0x1A47E90Bu, HashFunctions.fnv1a32 "abc")
        Assert.Equal(1_335_831_723u, HashFunctions.fnv1a32 "hello")
        Assert.Equal(3_214_735_720u, HashFunctions.fnv1a32 "foobar")

        Assert.Equal(0xCBF29CE484222325UL, HashFunctions.fnv1a64Bytes Array.empty<byte>)
        Assert.Equal(0xAF63DC4C8601EC8CUL, HashFunctions.fnv1a64 "a")
        Assert.Equal(0xE71FA2190541574BUL, HashFunctions.fnv1a64 "abc")
        Assert.Equal(0xA430D84680AABD0BUL, HashFunctions.fnv1a64 "hello")

    [<Fact>]
    let ``djb2 and polynomial known vectors`` () =
        Assert.Equal(5_381UL, HashFunctions.djb2Bytes Array.empty<byte>)
        Assert.Equal(177_670UL, HashFunctions.djb2 "a")
        Assert.Equal(193_485_963UL, HashFunctions.djb2 "abc")

        Assert.Equal(0UL, HashFunctions.polynomialRollingBytes Array.empty<byte> HashFunctions.polynomialRollingDefaultBase HashFunctions.polynomialRollingDefaultModulus)
        Assert.Equal(97UL, HashFunctions.polynomialRolling "a")
        Assert.Equal(3_105UL, HashFunctions.polynomialRolling "ab")
        Assert.Equal(96_354UL, HashFunctions.polynomialRolling "abc")
        Assert.Equal(((97UL * 37UL + 98UL) * 37UL + 99UL), HashFunctions.polynomialRollingWithParams "abc" 37UL 1_000_000_007UL)
        Assert.Throws<ArgumentException>(fun () -> HashFunctions.polynomialRollingWithParams "abc" 31UL 0UL |> ignore) |> ignore

    [<Fact>]
    let ``murmur3 known vectors`` () =
        Assert.Equal(0u, HashFunctions.murmur3_32Bytes Array.empty<byte>)
        Assert.Equal(0x514E28B7u, HashFunctions.murmur3_32BytesWithSeed Array.empty<byte> 1u)
        Assert.Equal(0x3C2569B2u, HashFunctions.murmur3_32 "a")
        Assert.Equal(0xB3DD93FAu, HashFunctions.murmur3_32 "abc")
        Assert.Equal(0x43ED676Au, HashFunctions.murmur3_32 "abcd")

    [<Fact>]
    let ``siphash vectors and string helpers`` () =
        let key = [| for value in 0 .. 15 -> byte value |]

        Assert.Equal(0x726FDB47DD0E0E31UL, HashFunctions.sipHash24 Array.empty<byte> key)
        Assert.Equal(0x74F839C593DC67FDUL, HashFunctions.sipHash24 [| 0uy |] key)
        Assert.Equal(HashFunctions.fnv1a32 "hello", HashFunctions.hashStringFnv1a32 "hello")
        Assert.Equal(HashFunctions.sipHash24 (bytes "hello") key, HashFunctions.hashStringSipHash "hello" key)
        Assert.Throws<ArgumentException>(fun () -> HashFunctions.sipHash24 Array.empty<byte> (Array.zeroCreate<byte> 8) |> ignore) |> ignore

    [<Fact>]
    let ``strategy types forward to free functions`` () =
        let strategies: HashFunction array =
            [|
                Fnv1a32() :> HashFunction
                Fnv1a64() :> HashFunction
                Djb2Hash() :> HashFunction
                PolynomialRollingHash() :> HashFunction
                Murmur3_32() :> HashFunction
                SipHash24(Array.zeroCreate<byte> 16) :> HashFunction
            |]

        let input = bytes "abc"

        Assert.Equal(uint64 (HashFunctions.fnv1a32Bytes input), strategies.[0].Hash input)
        Assert.Equal(HashFunctions.fnv1a64Bytes input, strategies.[1].Hash input)
        Assert.Equal(HashFunctions.djb2Bytes input, strategies.[2].Hash input)
        Assert.Equal(HashFunctions.polynomialRollingBytes input HashFunctions.polynomialRollingDefaultBase HashFunctions.polynomialRollingDefaultModulus, strategies.[3].Hash input)
        Assert.Equal(uint64 (HashFunctions.murmur3_32Bytes input), strategies.[4].Hash input)
        Assert.Equal(HashFunctions.sipHash24 input (Array.zeroCreate<byte> 16), strategies.[5].Hash input)
        Assert.Equal<int array>([| 32; 64; 64; 64; 32; 64 |], strategies |> Array.map (fun strategy -> strategy.OutputBits))

    [<Fact>]
    let ``distribution test matches exact constant hash math`` () =
        let inputs = [| bytes "a"; bytes "b"; bytes "c"; bytes "d" |]

        Assert.Equal(12.0, HashFunctions.distributionTest (fun _ -> 0UL) inputs 4)
        Assert.Throws<ArgumentException>(fun () -> HashFunctions.distributionTest (fun _ -> 0UL) inputs 0 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> HashFunctions.distributionTest (fun _ -> 0UL) Array.empty<byte array> 4 |> ignore) |> ignore

    [<Fact>]
    let ``avalanche score handles small samples`` () =
        Assert.Equal(0.0, HashFunctions.avalancheScore (fun _ -> 0UL) 32 1)
        Assert.Throws<ArgumentException>(fun () -> HashFunctions.avalancheScore (fun _ -> 0UL) 65 1 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> HashFunctions.avalancheScore (fun _ -> 0UL) 32 0 |> ignore) |> ignore
