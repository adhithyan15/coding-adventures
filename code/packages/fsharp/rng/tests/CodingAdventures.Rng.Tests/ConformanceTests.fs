namespace CodingAdventures.Rng.Tests

// ConformanceTests.fs — the shared rng corpus (spec rng.md, "Conformance corpus")
//
// code/specs/fixtures/rng-v1/cases.txt is the flat form of the corpus every
// language's rng package must reproduce; its results come from the independent
// reference code/scripts/rng_conformance.py. Each line is whitespace-separated:
//
//     case <id> <generator> <seed>
//     next_u32 <result>
//     next_u64 <result>
//     next_float <16 hex digits of the IEEE-754 bits>
//     next_int_in_range <min> <max> <result | error>
//     end
//
// The file is found by walking up from the test assembly to the directory that
// contains code/specs, because dotnet test runs from bin/<config>/<tfm>/.

open System
open System.Globalization
open System.IO
open CodingAdventures.Rng
open Xunit

/// The three generators behind one set of functions.
type private AnyRng =
    { U32: unit -> uint32
      U64: unit -> uint64
      F64: unit -> float
      InRange: int64 -> int64 -> int64 }

module private Corpus =
    let make name (seed: uint64) =
        match name with
        | "lcg" ->
            let g = Lcg seed
            { U32 = g.NextU32; U64 = g.NextU64; F64 = g.NextFloat; InRange = fun a b -> g.NextIntInRange(a, b) }
        | "xorshift64" ->
            let g = Xorshift64 seed
            { U32 = g.NextU32; U64 = g.NextU64; F64 = g.NextFloat; InRange = fun a b -> g.NextIntInRange(a, b) }
        | "pcg32" ->
            let g = Pcg32 seed
            { U32 = g.NextU32; U64 = g.NextU64; F64 = g.NextFloat; InRange = fun a b -> g.NextIntInRange(a, b) }
        | other -> failwithf "unknown generator %s" other

    let path () =
        let mutable dir = DirectoryInfo(AppContext.BaseDirectory)
        while not (isNull dir) && not (Directory.Exists(Path.Combine(dir.FullName, "code", "specs"))) do
            dir <- dir.Parent
        Assert.NotNull dir
        Path.Combine(dir.FullName, "code", "specs", "fixtures", "rng-v1", "cases.txt")

type ConformanceTests() =
    [<Fact>]
    member _.EveryCorpusCaseMatches() =
        let inv = CultureInfo.InvariantCulture
        let mutable cases = 0
        let mutable calls = 0
        let mutable id = ""
        let mutable g = None

        for line in File.ReadAllLines(Corpus.path ()) do
            if line.Length > 0 && not (line.StartsWith "#") then
                let t = line.Split ' '
                match t.[0] with
                | "case" ->
                    id <- t.[1]
                    g <- Some(Corpus.make t.[2] (UInt64.Parse(t.[3], inv)))
                    cases <- cases + 1
                | "end" -> g <- None
                | "next_u32" ->
                    calls <- calls + 1
                    Assert.True(UInt32.Parse(t.[1], inv) = g.Value.U32(), id)
                | "next_u64" ->
                    calls <- calls + 1
                    Assert.True(UInt64.Parse(t.[1], inv) = g.Value.U64(), id)
                | "next_float" ->
                    calls <- calls + 1
                    let bits = uint64 (BitConverter.DoubleToInt64Bits(g.Value.F64()))
                    Assert.True(UInt64.Parse(t.[1], NumberStyles.HexNumber, inv) = bits, id)
                | "next_int_in_range" ->
                    calls <- calls + 1
                    let lo = Int64.Parse(t.[1], inv)
                    let hi = Int64.Parse(t.[2], inv)
                    let gen = g.Value
                    if t.[3] = "error" then
                        // Thrown before any draw, so the sequence carries on.
                        Assert.Throws<ArgumentException>(fun () -> gen.InRange lo hi |> ignore) |> ignore
                    else
                        Assert.True(Int64.Parse(t.[3], inv) = gen.InRange lo hi, id)
                | _ -> Assert.Fail $"malformed corpus line: {line}"

        // Guard against a truncated or empty file passing vacuously.
        Assert.True(cases >= 27, $"cases: {cases}")
        Assert.True(calls >= 150, $"calls: {calls}")

    // ── Regressions for the v1 range bug ─────────────────────────────────────

    [<Fact>]
    member _.WholeInt64DomainNoLongerDividesByZero() =
        let g = Lcg 3UL
        let values = List.init 64 (fun _ -> g.NextIntInRange(Int64.MinValue, Int64.MaxValue))
        Assert.True(List.exists (fun v -> v < 0L) values && List.exists (fun v -> v > 0L) values)

    [<Fact>]
    member _.WideRangesReachBeyondTwoToThe32() =
        let g = Pcg32 3UL
        let values = List.init 64 (fun _ -> g.NextIntInRange(0L, (1L <<< 40)))
        Assert.True(List.exists (fun v -> v >= (1L <<< 32)) values)

    [<Fact>]
    member _.HalfDomainPlusOneTerminates() =
        let g = Xorshift64 3UL
        for _ in 1..100 do
            Assert.True(g.NextIntInRange(Int64.MinValue, 0L) <= 0L)
