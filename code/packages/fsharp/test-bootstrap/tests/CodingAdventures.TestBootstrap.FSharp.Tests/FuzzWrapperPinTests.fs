/// Pinned-value and differential tests of EVERY public F# fuzz wrapper (VER06m section 8).
///
/// fuzz-v1 pins no real signature hash and no energy value, and replays each mutator on a handful of
/// inputs, so the corpus alone would not notice a wrapper that altered an argument or a result. Each
/// test here pins literal values (taken from the C# engine's own pinned tests and the independent
/// Python oracle) and, for the mutators, compares the F# answer with the engine's over a grid that
/// uses asymmetric arguments, so a swapped or shifted argument changes the result.
module CodingAdventures.TestBootstrap.FSharp.Tests.FuzzWrapperPinTests

open System
open CodingAdventures.TestBootstrap.Prng
open CodingAdventures.TestBootstrap.FSharp
open CodingAdventures.TestBootstrap.FSharp.Fuzzing
open CodingAdventures.TestBootstrap.FSharp.Tests.FuzzSupport
open Xunit

let private huge = bg "100000000000000000000000000"
let private offsets = [ -9I; -1I; 0I; 1I; 2I; 3I; 7I; 100I; huge; -huge ]
let private counts = [ -3I; 0I; 1I; 2I; 5I; 17I; bg "1000000000000000000" ]
let private datas = [ [||]; hx "00"; hx "a53c"; hx "0123456789abcdef" ]
let private widths = [ -1; 0; 1; 2; 3; 4; 8; 9 ]
let private ends = [ Endian.Little; Endian.Big ]
let private cs (e: Endian) = match e with Endian.Little -> CsEndian.Little | Endian.Big -> CsEndian.Big

let private same (what: string) (want: byte[]) (got: byte[]) = Assert.True((want = got), what + ": want " + hex want + " got " + hex got)

type MutatorPins() =
    inherit FuzzSeamTest()

    // ---- literals ------------------------------------------------------------------------------

    [<Fact>]
    member _.``bitFlip and byteFlip literals``() =
        let data = [| 0uy; 0uy |]
        Assert.Equal("feff", hex (Mutate.bitFlip data 0I 17I))
        Assert.Equal("0002", hex (Mutate.bitFlip data 9I 1I))
        Assert.Equal("0100", hex (Mutate.bitFlip data 0I 1I))
        Assert.Equal("3c00", hex (Mutate.bitFlip data 2I 4I))
        Assert.Equal("ffff", hex (Mutate.bitFlip data 0I 16I))
        Assert.Equal("0000", hex (Mutate.bitFlip data 0I 32I))
        Assert.Equal("0000", hex (Mutate.bitFlip data 5I 0I))
        Assert.Equal("", hex (Mutate.bitFlip [||] 5I 3I))
        Assert.Equal("0001", hex (Mutate.bitFlip data -8I 1I))
        Assert.Equal("0080", hex (Mutate.bitFlip data -1I 1I))
        Assert.Equal("0000", hex (Mutate.bitFlip data 5I -3I))
        Assert.Equal("edcba9", hex (Mutate.byteFlip (hx "123456") 0I 3I))
        Assert.Equal("12cba9", hex (Mutate.byteFlip (hx "123456") 1I 2I))
        Assert.Equal("12cba9", hex (Mutate.byteFlip (hx "123456") -2I 2I))
        Assert.Equal("1234a9", hex (Mutate.byteFlip (hx "123456") 0I 5I))
        Assert.Equal("123456", hex (Mutate.byteFlip (hx "123456") 7I 0I))

    [<Fact>]
    member _.``byteArithmetic literals``() =
        let f d o dl w e = hex (Mutate.byteArithmetic (hx d) o dl w e)
        Assert.Equal("00", f "ff" 0I 1I 1 Endian.Little)
        Assert.Equal("0001", f "ff00" 0I 1I 2 Endian.Little)
        Assert.Equal("0100", f "00ff" 0I 1I 2 Endian.Big)
        Assert.Equal("00000000", f "ffffffff" 0I 1I 4 Endian.Big)
        Assert.Equal("0000000000000000", f "ffffffffffffffff" 0I 1I 8 Endian.Little)
        Assert.Equal("ffffffffffffffff", f "0000000000000000" 0I -1I 8 Endian.Big)
        Assert.Equal("fe", f "00" 0I -2I 1 Endian.Little)
        Assert.Equal("0000ff", f "0000fe" -1I 1I 1 Endian.Little)
        Assert.Equal("ff0000", f "fe0000" 3I 1I 1 Endian.Little)
        Assert.Equal("ffff", f "ffff" 0I 65536I 2 Endian.Little)
        Assert.Equal("ab", f "ab" 0I 5I 3 Endian.Little)
        Assert.Equal("ab", f "ab" 0I 5I 2 Endian.Little)
        Assert.Equal("ab", f "ab" 0I 5I 0 Endian.Little)
        Assert.Equal("ab", f "ab" 0I 1I 9 Endian.Big)
        // endianness and the delta are not interchangeable
        Assert.Equal("2300", f "0000" 0I 35I 2 Endian.Little)
        Assert.Equal("0023", f "0000" 0I 35I 2 Endian.Big)

    [<Fact>]
    member _.``boundaryInsert literals``() =
        let f d o w v e = hex (Mutate.boundaryInsert (hx d) o w v e)
        Assert.Equal("00000080", f "00000000" 0I 4 2147483648I Endian.Little)
        Assert.Equal("80000000", f "00000000" 0I 4 2147483648I Endian.Big)
        Assert.Equal("00ffff", f "000000" 1I 2 65535I Endian.Big)
        Assert.Equal("ffff", f "0000" 0I 2 (bigint UInt64.MaxValue) Endian.Little)
        Assert.Equal("ff", f "00" 0I 1 -1I Endian.Little)
        Assert.Equal("00", f "ab" 0I 1 256I Endian.Little)
        Assert.Equal("ab", f "ab" 0I 3 1I Endian.Little)
        Assert.Equal("ab", f "ab" 0I 2 1I Endian.Little)
        Assert.Equal("ab", f "ab" 0I 0 1I Endian.Little)
        Assert.Equal("00ab", f "0000" -1I 1 171I Endian.Little)
        Assert.Equal("0102", f "0000" 0I 2 513I Endian.Little)
        Assert.Equal("0201", f "0000" 0I 2 513I Endian.Big)

    [<Fact>]
    member _.``block mutator literals``() =
        Assert.Equal("01020506", hex (Mutate.blockDelete (hx "010203040506") 2I 2I))
        Assert.Equal("0102", hex (Mutate.blockDelete (hx "01020304") 2I 100I))
        Assert.Equal("0304", hex (Mutate.blockDelete (hx "01020304") -4I 2I))
        Assert.Equal("01020304", hex (Mutate.blockDelete (hx "01020304") 1I 0I))
        Assert.Equal("", hex (Mutate.blockDelete (hx "0102") 0I huge))
        Assert.Equal("01020304", hex (Mutate.blockDelete (hx "01020304") 0I -1I))
        Assert.Equal("0102ff03", hex (Mutate.blockInsert (hx "010203") 2I (hx "ff")))
        Assert.Equal("010203ff", hex (Mutate.blockInsert (hx "010203") 3I (hx "ff")))
        Assert.Equal("ff010203", hex (Mutate.blockInsert (hx "010203") 4I (hx "ff")))
        Assert.Equal("ff", hex (Mutate.blockInsert [||] 9I (hx "ff")))
        Assert.Equal("01ff0203", hex (Mutate.blockInsert (hx "010203") (bg "1000000000000000000000000001") (hx "ff")))
        Assert.Equal("01010203", hex (Mutate.blockDuplicate (hx "010203") 0I 1I 1I))
        Assert.Equal("0102030203", hex (Mutate.blockDuplicate (hx "010203") 1I 100I 3I))
        Assert.Equal("010203", hex (Mutate.blockDuplicate (hx "010203") 1I 0I 3I))
        Assert.Equal("", hex (Mutate.blockDuplicate [||] 1I 1I 1I))
        Assert.Equal("010203", hex (Mutate.blockDuplicate (hx "010203") 0I -1I 1I))
        Assert.Equal("010203010203", hex (Mutate.blockDuplicate (hx "010203") 0I huge 3I))
        // src, length and dst are not interchangeable
        Assert.Equal("0101020203", hex (Mutate.blockDuplicate (hx "010203") 0I 2I 1I))
        Assert.Equal("01020102", hex (Mutate.blockDuplicate (hx "0102") 0I 2I 2I))

    [<Fact>]
    member _.``splice literals``() =
        let t = hx "01020304"
        let d = hx "05060708"
        Assert.Equal("0108", hex (Mutate.splice t d 1I 3I))
        Assert.Equal("010203060708", hex (Mutate.splice t d 3I 1I))
        Assert.Equal("0506070304", hex (Mutate.splice d t 3I 2I))
        let longer = hx "0506070809"
        Assert.Equal("010203040809", hex (Mutate.splice t longer 4I 3I))
        Assert.Equal("01020309", hex (Mutate.splice t longer 3I 4I))
        Assert.Equal("010506070809", hex (Mutate.splice t longer 1I 0I))
        Assert.Equal("050607" + "04", hex (Mutate.splice longer t 3I 3I))
        Assert.Equal("0102", hex (Mutate.splice (hx "0102") (hx "0304") 99I 99I))
        Assert.Equal("010204", hex (Mutate.splice (hx "0102") (hx "0304") 99I 1I))
        Assert.Equal("0304", hex (Mutate.splice (hx "0102") (hx "0304") 0I 0I))
        Assert.Equal("04", hex (Mutate.splice [||] (hx "0304") 7I 1I))
        Assert.Equal("0304", hex (Mutate.splice (hx "0102") (hx "0304") -5I 0I))

    [<Fact>]
    member _.``sequence literals``() =
        Assert.Equal("00ddb08a3e0c0753ff7ffd0100c6fd", hex (Mutate.sequence (hx "b9892d5158ae8d3dfdfa") 42UL 40 1048576))
        Assert.Equal("72bfca0b54", hex (Mutate.sequence (hx "72cb54") 42UL 4 1048576))
        Assert.Equal("012bff013b", hex (Mutate.sequence (hx "5d296d413b") UInt64.MaxValue 8 5))
        Assert.Equal("7f82", hex (Mutate.sequence (hx "4a8b6329b5") 0UL 16 1))
        Assert.Equal("03ff00000000bf080000", hex (Mutate.sequence (hx "0000000000000000") 42UL 4 (1 <<< 20)))
        Assert.Equal("00fff6", hex (Mutate.sequence (hx "0000") 7UL 6 (1 <<< 20)))
        Assert.Equal("", hex (Mutate.sequence [||] 5UL 0 64))
        Assert.Equal("0102030405", hex (Mutate.sequence (hx "0102030405") 7UL 0 100))
        Assert.Equal("0102030405", hex (Mutate.sequence (hx "0102030405") 7UL -3 100))
        Assert.Throws<ArgumentException>(fun () -> Mutate.sequence (hx "00") 1UL 1_000_001 10 |> ignore) |> ignore

    [<Fact>]
    member _.``no mutator modifies its argument or returns its alias``() =
        let d = hx "0102030405060708"
        let keep = Array.copy d
        let results =
            [ Mutate.bitFlip d 3I 5I; Mutate.bitFlip d 3I 0I; Mutate.byteFlip d 1I 2I; Mutate.byteFlip d 1I 0I
              Mutate.byteArithmetic d 1I 5I 2 Endian.Big; Mutate.byteArithmetic d 1I 5I 3 Endian.Big
              Mutate.boundaryInsert d 1I 2 9I Endian.Little; Mutate.boundaryInsert d 1I 3 9I Endian.Little
              Mutate.blockDelete d 1I 2I; Mutate.blockDelete d 1I 0I; Mutate.blockInsert d 1I (hx "aa")
              Mutate.blockDuplicate d 1I 2I 3I; Mutate.blockDuplicate d 1I 0I 3I; Mutate.splice d (hx "aa") 1I 0I
              Mutate.sequence d 7UL 0 100 ]
        Assert.Equal<byte[]>(keep, d)
        for r in results do
            Assert.False(obj.ReferenceEquals(d, r))

    // ---- differential: the F# answer is the engine's, over an asymmetric grid ----------------------

    [<Fact>]
    member _.``flips match the engine over a grid``() =
        for data in datas do
            for o in offsets do
                for c in counts do
                    same "bitFlip" (CsMutate.BitFlipBig(data, o, c)) (Mutate.bitFlip data o c)
                    same "byteFlip" (CsMutate.ByteFlipBig(data, o, c)) (Mutate.byteFlip data o c)

    [<Fact>]
    member _.``arithmetic and boundary match the engine over a grid``() =
        let deltas = [ -35I; -1I; 0I; 1I; 35I; bigint.Pow(2I, 64) + 5I ]
        let values = [ -1I; 0I; 1I; 255I; 256I; bigint.Pow(2I, 64); bigint.Pow(2I, 70) + 7I ]
        for data in datas do
            for o in offsets do
                for w in widths do
                    for e in ends do
                        for dl in deltas do
                            same "byteArithmetic" (CsMutate.ByteArithmeticBig(data, o, dl, w, cs e)) (Mutate.byteArithmetic data o dl w e)
                        for v in values do
                            same "boundaryInsert" (CsMutate.BoundaryInsertBig(data, o, w, v, cs e)) (Mutate.boundaryInsert data o w v e)

    [<Fact>]
    member _.``block mutators and splice match the engine over a grid``() =
        let inserts = [ [||]; [| 0xaauy |]; [| 1uy; 2uy |] ]
        for data in datas do
            for o in offsets do
                for c in counts do
                    same "blockDelete" (CsMutate.BlockDeleteBig(data, o, c)) (Mutate.blockDelete data o c)
                    for dst in [ -1I; 0I; 1I; 3I; 100I ] do
                        same "blockDuplicate" (CsMutate.BlockDuplicateBig(data, o, c, dst)) (Mutate.blockDuplicate data o c dst)
                for ins in inserts do
                    same "blockInsert" (CsMutate.BlockInsertBig(data, o, ins)) (Mutate.blockInsert data o ins)
                for donor in datas do
                    for ds in offsets do
                        same "splice" (CsMutate.SpliceBig(data, donor, o, ds)) (Mutate.splice data donor o ds)

    [<Fact>]
    member _.``sequence matches the engine over a grid``() =
        for data in datas do
            for seed in [ 0UL; 42UL; UInt64.MaxValue; 0x9E3779B97F4A7C15UL ] do
                for steps in [ 0; 1; 5; 40 ] do
                    for maxLen in [ 1; 5; 1 <<< 20 ] do
                        same "sequence" (CsMutate.MutateSequence(data, seed, steps, maxLen)) (Mutate.sequence data seed steps maxLen)

type AdmissionEnergyPins() =
    inherit FuzzSeamTest()

    [<Fact>]
    member _.``admit literals``() =
        Assert.Equal({ Admitted = true; NewEdges = 1 }, Admission.admit [ 1UL; 2UL; 3UL ] [ 2UL; 4UL ])
        Assert.Equal({ Admitted = false; NewEdges = 0 }, Admission.admit [ 1UL; 2UL; 3UL ] [ 1UL; 2UL ])
        // existing and trace are not interchangeable
        Assert.Equal({ Admitted = true; NewEdges = 2 }, Admission.admit [ 2UL; 4UL ] [ 1UL; 2UL; 3UL ])
        // a novel edge repeated three times counts once
        Assert.Equal({ Admitted = true; NewEdges = 1 }, Admission.admit [ 1UL ] [ 9UL; 9UL; 9UL ])
        Assert.Equal({ Admitted = true; NewEdges = 2 }, Admission.admit [] [ 5UL; 6UL; 5UL ])
        Assert.Equal({ Admitted = false; NewEdges = 0 }, Admission.admit [ 1UL ] [])
        // ids are full 64-bit words: nothing is reduced mod 65,536
        Assert.Equal({ Admitted = true; NewEdges = 1 }, Admission.admit [ 1UL ] [ 65537UL ])
        Assert.Equal({ Admitted = true; NewEdges = 1 }, Admission.admit [ UInt64.MaxValue ] [ UInt64.MaxValue; 0UL ])
        Assert.Equal({ Admitted = false; NewEdges = 0 }, Admission.admit [ UInt64.MaxValue; 0UL ] [ 0UL; UInt64.MaxValue ])

    [<Theory>]
    [<InlineData(10, 4, 3, 6, 8)>]
    [<InlineData(5, 4, 0, 0, 3)>]
    [<InlineData(10, 1, 5, 0, 10)>]
    [<InlineData(0, 1000000, 1, 1, 1)>]
    [<InlineData(10, 0, 1, 1, 20)>]
    [<InlineData(10, 100, 1, 2, 2)>]
    [<InlineData(10, 9, 2, 4, 5)>]
    [<InlineData(10, 16, 1, 3, 3)>]
    [<InlineData(7, 2, 1, 2, 7)>]
    [<InlineData(100, 25, 5, 10, 30)>]
    [<InlineData(10, 1, 6, 6, 20)>]
    [<InlineData(3, 1, 1, 2, 5)>]
    [<InlineData(1, 1, 0, 5, 1)>]
    [<InlineData(10, 2, 1, 1, 14)>]
    member _.``energy schedule rows``(baseEnergy: int, len: int, newEdges: int, total: int, expected: int) =
        Assert.Equal(expected, Energy.calculate baseEnergy len newEdges total)

    [<Fact>]
    member _.``energy clamps, is monotone and is not argument-order blind``() =
        Assert.Equal(1_000_000, Energy.calculate Int32.MaxValue 1 1 1)
        Assert.True(Energy.calculate 10 4 6 6 > Energy.calculate 10 4 0 6)
        Assert.True(Energy.calculate 10 100 1 2 < Energy.calculate 10 1 1 2)
        // (base, len, new, total) = (10, 4, 3, 6) is 8; swapping new and total gives 6
        Assert.Equal(8, Energy.calculate 10 4 3 6)
        Assert.Equal(15, Energy.calculate 10 4 6 3)

type SignaturePins() =
    inherit FuzzSeamTest()

    [<Fact>]
    member _.``crash signatures are pinned against the independent oracle``() =
        Assert.Equal("4eca7ae8", Signature.crash "panic" [ "eval"; "step" ])
        Assert.Equal("4e61ea76", Signature.crash "timeout" [])
        Assert.Equal("7146a410", Signature.crash "oom" [])
        Assert.Equal("43a76e9e", Signature.crash "invalid" [])
        Assert.Equal("35a55e82", Signature.crash "k" [ "f" ])
        Assert.Equal("258cf594", Signature.crash "System.InvalidOperationException" [ "Parser.Read"; "Parser.Run" ])
        // the documented, unescaped ambiguity
        Assert.Equal("557f548e", Signature.crash "x" [ "a:b" ])
        Assert.Equal("557f548e", Signature.crash "x" [ "a"; "b" ])
        Assert.Equal(
            (SplitMix64.Fnv1a64("panic:eval:step") &&& 0xFFFFFFFFUL).ToString("x8"),
            Signature.crash "panic" [ "eval"; "step" ])

    [<Fact>]
    member _.``every frame and the order of frames matter``() =
        Assert.NotEqual<string>(Signature.crash "panic" [ "eval"; "step" ], Signature.crash "panic" [ "eval" ])
        Assert.NotEqual<string>(Signature.crash "panic" [ "eval"; "step" ], Signature.crash "panic" [ "step"; "eval" ])
        Assert.NotEqual<string>(Signature.crash "panic" [ "eval"; "step" ], Signature.crash "segv" [ "eval"; "step" ])
        Assert.NotEqual<string>(Signature.crash "k" [ "f" ], Signature.crash "k" [])

    [<Fact>]
    member _.``dedup literals``() =
        Assert.True(Signature.isDuplicate "panic" [ "eval"; "step" ] "panic" [ "eval"; "step" ])
        Assert.False(Signature.isDuplicate "panic" [ "eval"; "step" ] "segv" [ "alloc"; "write" ])
        Assert.False(Signature.isDuplicate "panic" [ "eval"; "step" ] "panic" [ "step"; "eval" ])
        Assert.False(Signature.isDuplicate "panic" [ "eval" ] "panic" [ "eval"; "step" ])
        Assert.False(Signature.isDuplicate "panic" [ "eval"; "step" ] "oom" [ "eval"; "step" ])
        Assert.True(Signature.isDuplicate "x" [ "a:b" ] "x" [ "a"; "b" ])
        // kindA/framesA and kindB/framesB are paired, not crossed
        Assert.False(Signature.isDuplicate "panic" [ "a" ] "segv" [ "a" ])
        Assert.False(Signature.isDuplicate "k" [ "f" ] "f" [ "k" ])

type TapFailureIdPins() =
    inherit FuzzSeamTest()

    [<Fact>]
    member _.``tap format literals``() =
        Assert.Equal("not ok 1 - ct_compare [fuzz-crash:3a8f19b2]", Tap.format 1 "ct_compare" "fuzz-crash" "3a8f19b2")
        Assert.Equal("not ok 42 - a/b.c-d_e [fuzz-oom:00000000]", Tap.format 42 "a/b.c-d_e" "fuzz-oom" "00000000")
        Assert.Equal("not ok 7 - t [fuzz-timeout:4e61ea76]", Tap.format 7 "t" "fuzz-timeout" "4e61ea76")
        Assert.Equal("not ok 0 - t [fuzz-leak:abcdef01]", Tap.format 0 "t" "fuzz-leak" "abcdef01")
        Assert.Equal("not ok -3 - t [fuzz-panic:abcdef01]", Tap.format -3 "t" "fuzz-panic" "abcdef01")
        // failure id and sig hash are not interchangeable, and the target is a separate field
        Assert.Equal("not ok 1 - fuzz-crash [fuzz-oom:3a8f19b2]", Tap.format 1 "fuzz-crash" "fuzz-oom" "3a8f19b2")

    [<Fact>]
    member _.``tap format sanitises the target and refuses bad ids and hashes``() =
        let line = Tap.format 1 "evil\nok 2 - forged [x]\r\n" "fuzz-crash" "3a8f19b2"
        Assert.DoesNotContain("\n", line)
        Assert.DoesNotContain("\r", line)
        Assert.Equal("not ok 1 - a__b [fuzz-crash:3a8f19b2]", Tap.format 1 "aéb" "fuzz-crash" "3a8f19b2")
        Assert.Equal("not ok 1 - " + String('x', 128) + " [fuzz-crash:3a8f19b2]", Tap.format 1 (String('x', 500)) "fuzz-crash" "3a8f19b2")
        Assert.Equal("not ok 1 -  [fuzz-crash:3a8f19b2]", Tap.format 1 "" "fuzz-crash" "3a8f19b2")
        Assert.Equal("not ok 1 -  [fuzz-crash:3a8f19b2]", Tap.format 1 null "fuzz-crash" "3a8f19b2")
        Assert.Throws<ArgumentException>(fun () -> Tap.format 1 "t" "fuzz-bogus" "3a8f19b2" |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Tap.format 1 "t" null "3a8f19b2" |> ignore) |> ignore
        for bad in [ ""; "abc"; "3A8F19B2"; "3a8f19b"; "3a8f19b22"; "3a8f19bg"; null ] do
            Assert.Throws<ArgumentException>(fun () -> Tap.format 1 "t" "fuzz-crash" bad |> ignore) |> ignore

    [<Fact>]
    member _.``failure id vocabulary``() =
        Assert.Equal("fuzz-crash", FuzzFailureId.crash)
        Assert.Equal("fuzz-timeout", FuzzFailureId.timeout)
        Assert.Equal("fuzz-oom", FuzzFailureId.oom)
        Assert.Equal("fuzz-leak", FuzzFailureId.leak)
        Assert.Equal("fuzz-panic", FuzzFailureId.panic)
        Assert.Equal<string list>([ "fuzz-crash"; "fuzz-timeout"; "fuzz-oom"; "fuzz-leak"; "fuzz-panic" ], FuzzFailureId.all)
        for id in FuzzFailureId.all do
            Assert.True(FuzzFailureId.isValid id)
        for bad in [ "fuzz-arbitrary-error"; ""; "FUZZ-CRASH"; "fuzz-crash "; " fuzz-crash"; "property-panic"; null ] do
            Assert.False(FuzzFailureId.isValid bad, "accepted " + string bad)

    [<Fact>]
    member _.``the strict TFV validator``() =
        Assert.True(FuzzTfv.isValid "tfv 1\nr R1 Kop S706274\n")
        Assert.False(FuzzTfv.isValid "bad header\n")
        Assert.False(FuzzTfv.isValid "")

type OptionsPins() =
    inherit FuzzSeamTest()

    [<Fact>]
    member _.``defaults are the engine's``() =
        let d = FuzzOptions.defaults
        Assert.Equal(1000, d.MaxIterations)
        Assert.Equal(42UL, d.Seed)
        Assert.Equal(4096, d.MaxInputLen)
        Assert.Equal(10, d.BaseEnergy)
        Assert.Equal(2000, d.MinimizeBudget)
        Assert.Equal(200_000, d.StepBudget)
        Assert.Equal(64 <<< 20, d.MemBudget)
        Assert.Equal(100_000_000, FuzzOptions.maxStepBudget)
        Assert.Equal(1 <<< 30, FuzzOptions.maxMemBudget)
        Assert.Equal(100_000, FuzzOptions.maxMinimizeBudget)

    [<Fact>]
    member _.``every field reaches the engine in its own slot``() =
        let o =
            { MaxIterations = 11; Seed = 22UL; MaxInputLen = 33; BaseEnergy = 44; MinimizeBudget = 55; StepBudget = 66; MemBudget = 77 }
        let e = o.ToEngine()
        Assert.Equal(11, e.MaxIterations)
        Assert.Equal(22UL, e.Seed)
        Assert.Equal(33, e.MaxInputLen)
        Assert.Equal(44, e.BaseEnergy)
        Assert.Equal(55, e.MinimizeBudget)
        Assert.Equal(66, e.StepBudget)
        Assert.Equal(77, e.MemBudget)
        Assert.Equal("FuzzOptions{maxIterations=11 maxInputLen=33 baseEnergy=44 minimizeBudget=55 stepBudget=66 memBudget=77}", string o)

    [<Fact>]
    member _.``each field is validated against its own ceiling``() =
        let d = FuzzOptions.defaults
        FuzzOptions.validate d
        let bad : (string * FuzzOptions) list =
            [ "MaxIterations", { d with MaxIterations = 1_000_001 }
              "MaxIterations", { d with MaxIterations = -1 }
              "MaxInputLen", { d with MaxInputLen = (1 <<< 20) + 1 }
              "BaseEnergy", { d with BaseEnergy = 1_000_001 }
              "MinimizeBudget", { d with MinimizeBudget = 100_001 }
              "StepBudget", { d with StepBudget = 100_000_001 }
              "MemBudget", { d with MemBudget = (1 <<< 30) + 1 } ]
        for (field, o) in bad do
            let e = Assert.Throws<ArgumentException>(fun () -> FuzzOptions.validate o)
            Assert.Contains(field, e.Message)
        // the ceilings themselves are valid
        FuzzOptions.validate { d with MaxIterations = 1_000_000; MaxInputLen = 1 <<< 20; BaseEnergy = 1_000_000; MinimizeBudget = 100_000; StepBudget = 100_000_000; MemBudget = 1 <<< 30 }

    [<Fact>]
    member _.``handle edge ids are the stable FNV-1a hash``() =
        Assert.Equal(SplitMix64.Fnv1a64 "a", Handle.EdgeId "a")
        Assert.Equal(0xaf63dc4c8601ec8cUL, Handle.EdgeId "a")
        Assert.Equal(Handle.EdgeId "parse.header", Handle.edgeId "parse.header")
        Assert.NotEqual(Handle.EdgeId "a", Handle.EdgeId "b")
