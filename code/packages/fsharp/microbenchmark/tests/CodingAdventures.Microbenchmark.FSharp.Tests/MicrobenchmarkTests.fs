namespace CodingAdventures.Microbenchmark.FSharp.Tests

open System
open System.Collections.Generic
open System.IO
open System.Text.Json
open Xunit
open CodingAdventures.Microbenchmark

module private Helpers =
    let unwrap = function
        | Ok value -> value
        | Error error -> failwithf "%A" error

    let error = function
        | Error value -> value
        | Ok _ -> failwith "expected an error result"

    let pair baseline candidate =
        { baselineNs = baseline
          candidateNs = candidate
          order = BaselineFirst }

type MicrobenchmarkTests() =
    [<Fact>]
    member _.``array strategies read the supplied mutable array``() =
        let values = [| 91; -7; 204; 0 |]
        Assert.Equal(288L, ArrayWorkload.indexedBatch values 1L)
        Assert.Equal(288L, ArrayWorkload.iterationBatch values 1L)
        values[0] <- 2
        Assert.Equal(199L, ArrayWorkload.indexedBatch values 1L)
        Assert.Equal(199L, ArrayWorkload.iterationBatch values 1L)

    [<Fact>]
    member _.``CLI output propagates broken writer failures``() =
        let writer =
            { new StringWriter() with
                override _.Flush() = raise (IOException "closed output") }
        Assert.Throws<IOException>(fun () -> Microbenchmark.writeAndFlush writer "complete report") |> ignore

    [<Fact>]
    member _.``fixed array run produces metadata and uses native clock``() =
        let config = { Microbenchmark.defaultConfig with pairs = 1; targetNs = 1L; maxBatch = 1L }
        let mutable tick = 0L
        let clock () = let now = tick in tick <- tick + 1L; now
        let report = ArrayWorkload.run config [| "array-microbenchmark"; "--pairs"; "1" |] "test-revision" "debug" clock |> Helpers.unwrap
        Assert.Equal(ArrayWorkload.checksum, ArrayWorkload.indexedBatch (ArrayWorkload.createArray()) 1L)
        Assert.Equal(ArrayWorkload.checksum, ArrayWorkload.iterationBatch (ArrayWorkload.createArray()) 1L)
        Assert.Equal("test-revision", report.metadata.Value.revision)
        let nativeClock = ArrayWorkload.createClock()
        Assert.True(nativeClock() >= 0L)
        Assert.True(nativeClock() >= 0L)

    [<Fact>]
    member _.``collector warms calibrates pairs and repeats seeded schedule``() =
        let config = { Microbenchmark.defaultConfig with pairs = 7; targetNs = 1L; maxBatch = 1L; seed = -71L }
        let events = ResizeArray<string>()
        let baseline count = events.Add(sprintf "b%d" count); Ok(11L * count)
        let candidate count = events.Add(sprintf "c%d" count); Ok(11L * count)
        let mutable tick = 0L
        let report = Microbenchmark.collect config 11L baseline candidate (fun () -> let now = tick in tick <- tick + 1L; now) |> Helpers.unwrap

        Assert.Equal(7, report.pairs.Length)
        Assert.Equal(4, report.pairs |> List.filter (fun item -> item.order = BaselineFirst) |> List.length)
        for index in 0 .. report.pairs.Length - 1 do
            let first = if events[4 + 2 * index][0] = 'b' then BaselineFirst else CandidateFirst
            Assert.Equal(first, report.pairs[index].order)

        let mutable replayTick = 0L
        let replay = Microbenchmark.collect config 11L baseline candidate (fun () -> let now = replayTick in replayTick <- replayTick + 1L; now) |> Helpers.unwrap
        Assert.Equal<PairOrder list>(report.pairs |> List.map _.order, replay.pairs |> List.map _.order)
        Assert.Equal(1L, report.batchSize)

    [<Fact>]
    member _.``collector calibrates to first common target batch``() =
        let config = { Microbenchmark.defaultConfig with pairs = 1; targetNs = 4L; maxBatch = 4L }
        let ticks = Queue<int64>([ 0L; 1L; 2L; 3L; 4L; 6L; 7L; 9L; 10L; 14L; 15L; 19L; 20L; 24L; 25L; 29L ])
        let identity count = Ok count
        let report = Microbenchmark.collect config 1L identity identity (fun () -> ticks.Dequeue()) |> Helpers.unwrap
        Assert.Equal(4L, report.batchSize)

    [<Fact>]
    member _.``collector reports callback checksum clock zero duration overflow and exhaustion``() =
        let config = { Microbenchmark.defaultConfig with pairs = 1; targetNs = 1L; maxBatch = 1L }
        let identity count = Ok count
        let mutable calls = 0
        let candidate count = calls <- calls + 1; Ok count
        let failed = Microbenchmark.collect config 1L (fun _ -> Error "broken") candidate (fun () -> 0L)
        Assert.Equal(WorkloadFailure "broken", Helpers.error failed)
        Assert.Equal(0, calls)
        Assert.Equal(ChecksumMismatch "baseline one-operation checksum mismatch", Helpers.error (Microbenchmark.collect config 1L (fun _ -> Ok 0L) candidate (fun () -> 0L)))
        let regressing = Queue<int64>([ 0L; 2L; 1L ])
        Assert.Equal(ClockFailure "clock ticks must be nonnegative and nonregressing", Helpers.error (Microbenchmark.collect config 1L identity identity (fun () -> regressing.Dequeue())))
        let zero = Queue<int64>([ 0L; 1L; 2L; 3L; 3L; 3L ])
        Assert.Equal(InvalidObservation "measured duration must be positive", Helpers.error (Microbenchmark.collect config 1L identity identity (fun () -> zero.Dequeue())))
        let overflowConfig = { config with maxBatch = 2L; targetNs = 2L }
        let huge count = if count = 1L then Ok Int64.MaxValue else Ok 0L
        Assert.Equal(ArithmeticOverflow "batch checksum arithmetic overflow", Helpers.error (Microbenchmark.collect overflowConfig Int64.MaxValue huge huge (fun () -> 0L)))
        let exhaustedConfig = { config with targetNs = 2L }
        Assert.Equal(CalibrationExhausted, Helpers.error (Microbenchmark.collect exhaustedConfig 1L identity identity (fun () -> 0L)))
        Assert.Equal(InvalidConfiguration "pairs must be from 1 through 10000", Helpers.error (Microbenchmark.collect { config with pairs = 0 } 1L identity identity (fun () -> 0L)))

    [<Fact>]
    member _.``analyzer uses portable verdict boundaries and validates observations``() =
        let faster = Microbenchmark.analyze [ for _ in 1 .. 6 -> Helpers.pair 100L 80L ] 95 1.0625 |> Helpers.unwrap
        let slower = Microbenchmark.analyze [ for _ in 1 .. 6 -> Helpers.pair 100L 120L ] 95 1.0625 |> Helpers.unwrap
        let insufficient = Microbenchmark.analyze [ Helpers.pair Int64.MaxValue Int64.MaxValue ] 95 1.05 |> Helpers.unwrap
        Assert.Equal(Faster, faster.verdict)
        Assert.Equal(Slower, slower.verdict)
        Assert.Equal(Inconclusive, insufficient.verdict)
        Assert.True((Microbenchmark.analyze [] 95 1.05).IsError)
        Assert.True((Microbenchmark.analyze [ Helpers.pair 0L 1L ] 95 1.05).IsError)

    [<Fact>]
    member _.``json replay preserves int64 and rejects unknown duplicate unbalanced and forged reports``() =
        let config = { Microbenchmark.defaultConfig with pairs = 3; targetNs = 1L; maxBatch = 1L }
        let mutable tick = 0L
        let identity count = Ok count
        let report = Microbenchmark.collect config 1L identity identity (fun () -> let now = tick in tick <- tick + 1L; now) |> Helpers.unwrap
        let metadata =
            { workload = "indexed-array-sum-4096"
              invocation = [| "array-microbenchmark"; "--pairs"; "3" |]
              revision = "abc123"
              runtime = ".NET 9"
              os = "windows"
              arch = "x64"
              profile = Some "release" }
        let json = Microbenchmark.toJson { report with metadata = Some metadata }
        Assert.Equal(3, (Microbenchmark.fromJson json |> Helpers.unwrap).pairs.Length)
        Assert.Contains("\"profile\": \"release\"", json)

        let maximum = """{"config":{"pairs":1,"confidence_percent":95,"minimum_factor":1.05,"seed":0,"target_ns":1,"max_batch":9223372036854775807},"batch_size":1,"pairs":[{"baseline_ns":9223372036854775807,"candidate_ns":9223372036854775807,"order":"baseline-first"}],"analysis":{"ratio":1,"interval":null,"verdict":"inconclusive"}}"""
        let parsed = Microbenchmark.fromJson maximum |> Helpers.unwrap
        Assert.Equal(Int64.MaxValue, parsed.pairs.Head.baselineNs)
        Assert.Contains("9223372036854775807", Microbenchmark.toJson parsed)
        Assert.True((Microbenchmark.fromJson "{\"x\":1,\"x\":2}").IsError)
        Assert.True((Microbenchmark.fromJson (json.Replace("\"analysis\":", "\"unknown\":true,\"analysis\":", StringComparison.Ordinal))).IsError)
        Assert.True((Microbenchmark.fromJson (json.Replace("baseline-first", "candidate-first", StringComparison.Ordinal))).IsError)
        Assert.True((Microbenchmark.fromJson (json.Replace("\"ratio\": 1", "\"ratio\": 2", StringComparison.Ordinal))).IsError)
        Assert.True((Microbenchmark.fromJson (maximum.Replace("9223372036854775807", "9223372036854775808", StringComparison.Ordinal))).IsError)

    [<Fact>]
    member _.``shared fixture matches canonical and analyzer``() =
        let local = Path.Combine(AppContext.BaseDirectory, "testdata", "microbenchmark-pairs-v1.json") |> File.ReadAllBytes
        let mutable current = DirectoryInfo(AppContext.BaseDirectory)
        let mutable canonical: string option = None
        while not (isNull current) && canonical.IsNone do
            let path = Path.Combine(current.FullName, "code", "specs", "fixtures", "microbenchmark-pairs-v1.json")
            if File.Exists path then canonical <- Some path
            current <- current.Parent
        canonical |> Option.iter (fun path -> Assert.Equal<byte array>(File.ReadAllBytes path, local))
        use document = JsonDocument.Parse local
        let root = document.RootElement
        let confidence = root.GetProperty("confidence_percent").GetInt32()
        let factor = root.GetProperty("minimum_factor").GetDouble()
        for item in root.GetProperty("cases").EnumerateArray() do
            let baseline = item.GetProperty("baseline_ns").GetInt64()
            let pairs =
                item.GetProperty("candidate_ns").EnumerateArray()
                |> Seq.mapi (fun index value -> { baselineNs = baseline; candidateNs = value.GetInt64(); order = if index % 2 = 0 then BaselineFirst else CandidateFirst })
                |> Seq.toList
            let actual = Microbenchmark.analyze pairs confidence factor |> Helpers.unwrap
            Assert.Equal(item.GetProperty("ratio").GetDouble(), actual.ratio)
            Assert.Equal(item.GetProperty("verdict").GetString(), sprintf "%A" actual.verdict |> fun text -> text.ToLowerInvariant())

    [<Fact>]
    member _.``generated observations keep confidence endpoints in the samples``() =
        let random = Random(0x4d454449)
        for _ in 0 .. 511 do
            let pairs =
                [ for index in 0 .. random.Next(1, 40) - 1 ->
                    { baselineNs = int64 (100000 + random.Next(1, 100000))
                      candidateNs = int64 (100000 + random.Next(1, 100000))
                      order = if index % 2 = 0 then BaselineFirst else CandidateFirst } ]
            match Microbenchmark.analyze pairs (random.Next(1, 100)) 1.05 |> Helpers.unwrap with
            | { interval = Some(low, high) } ->
                let ratios = pairs |> List.map (fun pair -> float pair.candidateNs / float pair.baselineNs)
                Assert.Contains(low, ratios)
                Assert.Contains(high, ratios)
            | _ -> ()

    [<Fact>]
    member _.``replays maximum report and deterministic JSON mutations safely``() =
        let config = { Microbenchmark.defaultConfig with pairs = 10000; targetNs = 1L; maxBatch = 1L }
        let pairs =
            [ for index in 0 .. config.pairs - 1 ->
                { baselineNs = int64 (100 + index % 7)
                  candidateNs = int64 (99 + index % 9)
                  order = if index % 2 = 0 then BaselineFirst else CandidateFirst } ]
        let analysis = Microbenchmark.analyze pairs config.confidencePercent config.minimumFactor |> Helpers.unwrap
        let report = { config = config; batchSize = 1L; pairs = pairs; analysis = analysis; metadata = None }
        let json = Microbenchmark.toJson report
        Assert.Equal(10000, (Microbenchmark.fromJson json |> Helpers.unwrap).pairs.Length)

        let random = Random(0x4a534f4e)
        for _ in 0 .. 255 do
            let chars = json.ToCharArray()
            let position = random.Next(chars.Length)
            chars[position] <- match chars[position] with | '0' -> '1' | '1' -> '0' | '{' -> '[' | '[' -> '{' | _ -> 'x'
            match Microbenchmark.fromJson (String chars) with
            | Ok parsed ->
                let replay = Microbenchmark.toJson parsed |> Microbenchmark.fromJson |> Helpers.unwrap
                Assert.Equal(parsed.pairs.Length, replay.pairs.Length)
            | Error _ -> ()
