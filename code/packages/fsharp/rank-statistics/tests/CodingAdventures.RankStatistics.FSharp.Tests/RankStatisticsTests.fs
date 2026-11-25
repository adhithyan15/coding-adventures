namespace CodingAdventures.RankStatistics.FSharp.Tests

open System
open System.IO
open System.Numerics
open System.Text.Json
open Xunit
open CodingAdventures.RankStatistics

module private Oracle =
    let rank n confidence =
        let assignments = 1 <<< n
        let mutable best = 0
        for rank in 1 .. n do
            let mutable below = 0
            for mask in 0 .. assignments - 1 do
                if BitOperations.PopCount(uint32 mask) <= rank - 1 then below <- below + 1
            if 200 * below <= (100 - confidence) * assignments then best <- rank
        best

type RankStatisticsTests() =
    [<Fact>]
    member _.``median preserves input and handles exact floating boundaries``() =
        let source = [| 9.0; 1.0; 5.0; 3.0 |]
        Assert.Equal(4.0, RankStatistics.median source)
        Assert.Equal<double array>([| 9.0; 1.0; 5.0; 3.0 |], source)
        Assert.Equal(7.0, RankStatistics.median [ 7.0 ])

        let denorm = Double.Epsilon
        Assert.Equal(2.0 * denorm, RankStatistics.median [ denorm; 2.0 * denorm ])
        Assert.Equal(4.0 * denorm, RankStatistics.median [ 3.0 * denorm; 4.0 * denorm ])
        Assert.Equal(-2.0 * denorm, RankStatistics.median [ -2.0 * denorm; -denorm ])
        Assert.Equal(-2.0 * denorm, RankStatistics.median [ -3.0 * denorm; -2.0 * denorm ])
        let minNormal = Math.ScaleB(1.0, -1022)
        Assert.Equal(minNormal, RankStatistics.median [ Math.BitDecrement minNormal; minNormal ])
        Assert.Equal(Double.MaxValue, RankStatistics.median [ Double.MaxValue; Double.MaxValue ])
        Assert.Equal(0.0, RankStatistics.median [ -Double.MaxValue; Double.MaxValue ])
        Assert.Equal(Double.MaxValue * 0.75, RankStatistics.median [ Double.MaxValue / 2.0; Double.MaxValue ])

    [<Fact>]
    member _.``rejects empty nonfinite and invalid confidence inputs``() =
        Assert.Throws<ArgumentException>(fun () -> RankStatistics.median Seq.empty |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> RankStatistics.median [ 1.0; Double.NaN ] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> RankStatistics.medianInterval Seq.empty 95 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> RankStatistics.medianInterval [ 1.0; Double.PositiveInfinity ] 95 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> RankStatistics.medianInterval [ 1.0 ] 0 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> RankStatistics.medianInterval [ 1.0 ] 100 |> ignore) |> ignore

    [<Fact>]
    member _.``exact cutoff matches bounded sign assignment oracle``() =
        for n in 1 .. 10 do
            let values = [| for i in 0 .. n - 1 -> float i |]
            for confidence in 1 .. 99 do
                let rank = Oracle.rank n confidence
                let interval = RankStatistics.medianInterval values confidence
                if rank = 0 then Assert.Equal<MedianInterval option>(None, interval)
                else Assert.Equal(Some { Low = float (rank - 1); High = float (n - rank) }, interval)

    [<Fact>]
    member _.``wide cutoff values match frozen implementation vectors``() =
        let cases =
            [| 64, 95, 23, 40
               65, 99, 21, 43
               127, 95, 51, 75
               128, 95, 52, 75
               256, 99, 106, 149
               1000, 95, 468, 531
               10000, 50, 4965, 5034
               10000, 95, 4901, 5098
               10000, 99, 4870, 5129 |]
        for n, confidence, low, high in cases do
            let values = [| for i in 0 .. n - 1 -> float i |]
            Assert.Equal(Some { Low = float low; High = float high }, RankStatistics.medianInterval values confidence)

    [<Fact>]
    member _.``fixture copy matches canonical bytes and rank cases``() =
        let localPath = Path.Combine(AppContext.BaseDirectory, "testdata", "microbenchmark-pairs-v1.json")
        let local = File.ReadAllBytes localPath
        let mutable current = DirectoryInfo(AppContext.BaseDirectory)
        let mutable canonicalPath: string option = None
        while not (isNull current) && canonicalPath.IsNone do
            let candidate = Path.Combine(current.FullName, "code", "specs", "fixtures", "microbenchmark-pairs-v1.json")
            if File.Exists candidate then canonicalPath <- Some candidate
            current <- current.Parent
        match canonicalPath with
        | Some path -> Assert.Equal<byte array>(File.ReadAllBytes path, local)
        | None -> ()

        use document = JsonDocument.Parse local
        let root = document.RootElement
        let confidence = root.GetProperty("confidence_percent").GetInt32()
        for item in root.GetProperty("cases").EnumerateArray() do
            let baseline = item.GetProperty("baseline_ns").GetInt64()
            let ratios =
                item.GetProperty("candidate_ns").EnumerateArray()
                |> Seq.map (fun value -> float (value.GetInt64()) / float baseline)
                |> Seq.toArray
            Assert.Equal(item.GetProperty("ratio").GetDouble(), RankStatistics.median ratios)
            let interval = RankStatistics.medianInterval ratios confidence
            let expected = item.GetProperty("interval")
            if expected.ValueKind = JsonValueKind.Null then Assert.Equal<MedianInterval option>(None, interval)
            else
                let bounds = expected.EnumerateArray() |> Seq.map _.GetDouble() |> Seq.toArray
                Assert.Equal(Some { Low = bounds[0]; High = bounds[1] }, interval)

    [<Fact>]
    member _.``seeded generated samples preserve permutation and endpoints``() =
        let random = Random(0x4d454449)
        for _ in 0 .. 511 do
            let values = Array.init (random.Next(1, 48)) (fun _ -> float (random.Next(-100000, 100001)) / 17.0)
            Assert.Equal(RankStatistics.median values, RankStatistics.median (Array.rev values))
            let interval = RankStatistics.medianInterval values (random.Next(1, 100))
            match interval with
            | None -> ()
            | Some result ->
                Assert.Contains(result.Low, values)
                Assert.Contains(result.High, values)
