namespace CodingAdventures.Microbenchmark

open System
open System.Collections.Generic
open System.IO
open System.Text
open System.Text.Json
open CodingAdventures.RankStatistics

/// Pair execution order recorded in each report row.
type PairOrder =
    | BaselineFirst
    | CandidateFirst

/// The four policy outcomes for a paired comparison.
type Verdict =
    | Faster
    | Slower
    | Equivalent
    | Inconclusive

/// Values fixed before collection starts.
type Config =
    { pairs: int
      confidencePercent: int
      minimumFactor: float
      seed: int64
      targetNs: int64
      maxBatch: int64 }

/// One baseline/candidate observation pair.
type Pair =
    { baselineNs: int64
      candidateNs: int64
      order: PairOrder }

/// Median estimate, exact rank interval, and practical verdict.
type Analysis =
    { ratio: float
      interval: (float * float) option
      verdict: Verdict }

/// Standard command metadata; no benchmark-private fields are serialized.
type Metadata =
    { workload: string
      invocation: string array
      revision: string
      runtime: string
      os: string
      arch: string
      profile: string option }

/// Full replayable report with optional command metadata.
type Report =
    { config: Config
      batchSize: int64
      pairs: Pair list
      analysis: Analysis
      metadata: Metadata option }

/// Failures are values so callers can compose collection in F# result workflows.
type BenchmarkError =
    | InvalidConfiguration of string
    | InvalidObservation of string
    | ChecksumMismatch of string
    | WorkloadFailure of string
    | ClockFailure of string
    | CalibrationExhausted
    | ArithmeticOverflow of string
    | InvalidReport of string

/// Paired collection, analysis and JSON replay operations.
module Microbenchmark =
    let maximumPairs = 10000
    let maximumJsonBytes = 4 * 1024 * 1024
    let maximumJsonDepth = 128
    let maximumJsonNodes = 65536

    /// Writes and flushes all CLI output, preserving I/O failures for the caller.
    let writeAndFlush (writer: TextWriter) (text: string) =
        if isNull writer then nullArg "writer"
        writer.Write text
        writer.Flush()
    let baselineFirst = BaselineFirst
    let candidateFirst = CandidateFirst

    let defaultConfig =
        { pairs = 40
          confidencePercent = 95
          minimumFactor = 1.05
          seed = 0L
          targetNs = 100000L
          maxBatch = 65536L }

    let private validateConfig (config: Config) : Result<Config, BenchmarkError> =
        if config.pairs < 1 || config.pairs > maximumPairs then Error(InvalidConfiguration "pairs must be from 1 through 10000")
        elif config.confidencePercent < 1 || config.confidencePercent > 99 then Error(InvalidConfiguration "confidence must be from 1 through 99")
        elif not (Double.IsFinite config.minimumFactor) || config.minimumFactor <= 1.0 then Error(InvalidConfiguration "minimum_factor must be finite and greater than 1")
        elif config.targetNs <= 0L then Error(InvalidConfiguration "target_ns must be positive")
        elif config.maxBatch <= 0L then Error(InvalidConfiguration "max_batch must be positive")
        else Ok config

    let private orderText (order: PairOrder) =
        match order with
        | BaselineFirst -> "baseline-first"
        | CandidateFirst -> "candidate-first"

    let private verdictText (verdict: Verdict) =
        match verdict with
        | Faster -> "faster"
        | Slower -> "slower"
        | Equivalent -> "equivalent"
        | Inconclusive -> "inconclusive"

    let private parseOrder (text: string) : Result<PairOrder, BenchmarkError> =
        match text with
        | "baseline-first" -> Ok BaselineFirst
        | "candidate-first" -> Ok CandidateFirst
        | _ -> Error(InvalidReport "invalid pair order")

    let private parseVerdict (text: string) : Result<Verdict, BenchmarkError> =
        match text with
        | "faster" -> Ok Faster
        | "slower" -> Ok Slower
        | "equivalent" -> Ok Equivalent
        | "inconclusive" -> Ok Inconclusive
        | _ -> Error(InvalidReport "invalid verdict")

    /// Analyzes existing positive signed-64-bit observations.
    let analyze (pairs: Pair list) (confidencePercent: int) (minimumFactor: float) : Result<Analysis, BenchmarkError> =
        if List.isEmpty pairs then Error(InvalidObservation "pairs must not be empty")
        elif confidencePercent < 1 || confidencePercent > 99 then Error(InvalidConfiguration "confidence must be from 1 through 99")
        elif not (Double.IsFinite minimumFactor) || minimumFactor <= 1.0 then Error(InvalidConfiguration "minimum factor must be finite and greater than 1")
        elif pairs |> List.exists (fun pair -> pair.baselineNs <= 0L || pair.candidateNs <= 0L) then Error(InvalidObservation "pair durations must be positive")
        else
            let ratios = pairs |> List.map (fun pair -> float pair.candidateNs / float pair.baselineNs) |> List.toArray
            let ratio = RankStatistics.median ratios
            let interval = RankStatistics.medianInterval ratios confidencePercent |> Option.map (fun value -> value.Low, value.High)
            let verdict =
                match interval with
                | None -> Inconclusive
                | Some(low, high) ->
                    let fastBoundary = 1.0 / minimumFactor
                    if high < fastBoundary then Faster
                    elif low > minimumFactor then Slower
                    elif low >= fastBoundary && high <= minimumFactor then Equivalent
                    else Inconclusive
            Ok { ratio = ratio; interval = interval; verdict = verdict }

    type private SplitMix64(seed: uint64) =
        let mutable state = seed
        member _.Next() =
            state <- state + 0x9E3779B97F4A7C15UL
            let mutable value = state
            value <- (value ^^^ (value >>> 30)) * 0xBF58476D1CE4E5B9UL
            value <- (value ^^^ (value >>> 27)) * 0x94D049BB133111EBUL
            value ^^^ (value >>> 31)
        member this.Bounded(bound: uint64) =
            let threshold = (0UL - bound) % bound
            let mutable value = this.Next()
            while value < threshold do value <- this.Next()
            value % bound

    let private schedule (pairs: int) (seed: int64) : PairOrder array =
        let orders = Array.append (Array.create ((pairs + 1) / 2) BaselineFirst) (Array.create (pairs / 2) CandidateFirst)
        let random = SplitMix64(uint64 seed)
        for index in orders.Length - 1 .. -1 .. 1 do
            let other = int (random.Bounded(uint64 (index + 1)))
            let temporary = orders[index]
            orders[index] <- orders[other]
            orders[other] <- temporary
        orders

    /// Collects a complete paired run, returning no partial report after failure.
    let collect (config: Config) (expectedChecksum: int64) (baseline: int64 -> Result<int64, string>) (candidate: int64 -> Result<int64, string>) (clock: unit -> int64) : Result<Report, BenchmarkError> =
        match validateConfig config with
        | Error error -> Error error
        | Ok config ->
            let verify (name: string) (callback: int64 -> Result<int64, string>) : Result<unit, BenchmarkError> =
                try
                    match callback 1L with
                    | Error message -> Error(WorkloadFailure message)
                    | Ok actual when actual <> expectedChecksum -> Error(ChecksumMismatch(sprintf "%s one-operation checksum mismatch" name))
                    | Ok _ -> Ok()
                with error -> Error(WorkloadFailure error.Message)
            match verify "baseline" baseline with
            | Error error -> Error error
            | Ok() ->
                match verify "candidate" candidate with
                | Error error -> Error error
                | Ok() ->
                    let mutable previousTick: int64 option = None
                    let readClock () : Result<int64, BenchmarkError> =
                        try
                            let tick = clock()
                            if tick < 0L || (previousTick |> Option.exists (fun old -> tick < old)) then Error(ClockFailure "clock ticks must be nonnegative and nonregressing")
                            else
                                previousTick <- Some tick
                                Ok tick
                        with error -> Error(ClockFailure error.Message)

                    let timed (name: string) (callback: int64 -> Result<int64, string>) (size: int64) (calibration: bool) : Result<int64, BenchmarkError> =
                        try
                            let expected = Checked.(*) expectedChecksum size
                            match readClock() with
                            | Error error -> Error error
                            | Ok started ->
                                let result = try callback size with error -> Error error.Message
                                match result with
                                | Error message -> Error(WorkloadFailure message)
                                | Ok actual ->
                                    match readClock() with
                                    | Error error -> Error error
                                    | Ok finished ->
                                        if actual <> expected then Error(ChecksumMismatch(sprintf "%s batch checksum mismatch" name))
                                        else
                                            let duration = finished - started
                                            if duration < 0L || (not calibration && duration = 0L) then Error(InvalidObservation "measured duration must be positive")
                                            else Ok duration
                        with :? OverflowException -> Error(ArithmeticOverflow "batch checksum arithmetic overflow")

                    let rec calibrate (batch: int64) : Result<int64, BenchmarkError> =
                        match timed "baseline" baseline batch true with
                        | Error error -> Error error
                        | Ok baselineNs ->
                            match timed "candidate" candidate batch true with
                            | Error error -> Error error
                            | Ok candidateNs when baselineNs >= config.targetNs && candidateNs >= config.targetNs -> Ok batch
                            | Ok _ when batch >= config.maxBatch -> Error CalibrationExhausted
                            | Ok _ -> calibrate (if batch > config.maxBatch / 2L then config.maxBatch else batch * 2L)

                    match calibrate 1L with
                    | Error error -> Error error
                    | Ok batch ->
                        let collectPair (order: PairOrder) : Result<Pair, BenchmarkError> =
                            let firstName, first, secondName, second =
                                if order = BaselineFirst then "baseline", baseline, "candidate", candidate
                                else "candidate", candidate, "baseline", baseline
                            match timed firstName first batch false with
                            | Error error -> Error error
                            | Ok firstNs ->
                                match timed secondName second batch false with
                                | Error error -> Error error
                                | Ok secondNs ->
                                    if order = BaselineFirst then Ok { baselineNs = firstNs; candidateNs = secondNs; order = order }
                                    else Ok { baselineNs = secondNs; candidateNs = firstNs; order = order }
                        let orders = schedule config.pairs config.seed
                        let rec gather (index: int) (acc: Pair list) : Result<Pair list, BenchmarkError> =
                            if index = orders.Length then Ok(List.rev acc)
                            else
                                match collectPair orders[index] with
                                | Error error -> Error error
                                | Ok pair -> gather (index + 1) (pair :: acc)
                        match gather 0 [] with
                        | Error error -> Error error
                        | Ok pairs ->
                            match analyze pairs config.confidencePercent config.minimumFactor with
                            | Error error -> Error error
                            | Ok analysis -> Ok { config = config; batchSize = batch; pairs = pairs; analysis = analysis; metadata = None }

    let private writeInt64 (writer: Utf8JsonWriter) (name: string) (value: int64) = writer.WriteNumber(name, value)
    let private writeConfig (writer: Utf8JsonWriter) (config: Config) =
        writer.WriteStartObject("config")
        writer.WriteNumber("pairs", config.pairs)
        writer.WriteNumber("confidence_percent", config.confidencePercent)
        writer.WriteNumber("minimum_factor", config.minimumFactor)
        writer.WriteNumber("seed", config.seed)
        writeInt64 writer "target_ns" config.targetNs
        writeInt64 writer "max_batch" config.maxBatch
        writer.WriteEndObject()

    /// Serializes a report using exact JSON int64 tokens and portable field names.
    let toJson (report: Report) =
        use stream = new MemoryStream()
        use writer = new Utf8JsonWriter(stream, JsonWriterOptions(Indented = true))
        writer.WriteStartObject()
        writeConfig writer report.config
        writeInt64 writer "batch_size" report.batchSize
        writer.WriteStartArray("pairs")
        for pair in report.pairs do
            writer.WriteStartObject()
            writeInt64 writer "baseline_ns" pair.baselineNs
            writeInt64 writer "candidate_ns" pair.candidateNs
            writer.WriteString("order", orderText pair.order)
            writer.WriteEndObject()
        writer.WriteEndArray()
        writer.WriteStartObject("analysis")
        writer.WriteNumber("ratio", report.analysis.ratio)
        match report.analysis.interval with
        | None -> writer.WriteNull("interval")
        | Some(low, high) ->
            writer.WriteStartArray("interval")
            writer.WriteNumberValue low
            writer.WriteNumberValue high
            writer.WriteEndArray()
        writer.WriteString("verdict", verdictText report.analysis.verdict)
        writer.WriteEndObject()
        match report.metadata with
        | None -> ()
        | Some metadata ->
            writer.WriteString("workload", metadata.workload)
            writer.WriteStartArray("invocation")
            for argument in metadata.invocation do writer.WriteStringValue argument
            writer.WriteEndArray()
            writer.WriteString("revision", metadata.revision)
            writer.WriteString("runtime", metadata.runtime)
            writer.WriteString("os", metadata.os)
            writer.WriteString("arch", metadata.arch)
            metadata.profile |> Option.iter (fun profile -> writer.WriteString("profile", profile))
        writer.WriteEndObject()
        writer.Flush()
        Encoding.UTF8.GetString(stream.ToArray())

    let private ensureOnly (allowed: string list) (element: JsonElement) : unit =
        if element.ValueKind <> JsonValueKind.Object then invalidOp "expected JSON object"
        for property in element.EnumerateObject() do
            if not (List.contains property.Name allowed) then invalidOp ("unknown JSON property: " + property.Name)

    let private required (name: string) (element: JsonElement) : JsonElement =
        match element.TryGetProperty name with
        | true, value -> value
        | false, _ -> invalidOp ("missing JSON property: " + name)

    let private readInt32 (name: string) (element: JsonElement) : int =
        match element.TryGetInt32() with true, value -> value | false, _ -> invalidOp (name + " must be an int32")
    let private readInt64 (name: string) (element: JsonElement) : int64 =
        match element.TryGetInt64() with true, value -> value | false, _ -> invalidOp (name + " must be an int64")
    let private readString (name: string) (element: JsonElement) : string =
        if element.ValueKind <> JsonValueKind.String then invalidOp (name + " must be a string")
        let value = element.GetString()
        if String.IsNullOrEmpty value then invalidOp (name + " must not be empty")
        value

    let private isValidWorkload (value: string) : bool =
        not (value.Contains('\000'))
        && (value |> Seq.exists (fun character -> character <> ' ' && (character < '\t' || character > '\r')))

    let private checkTree (root: JsonElement) : unit =
        let mutable nodes = 0
        let rec visit (depth: int) (element: JsonElement) : unit =
            nodes <- nodes + 1
            if nodes > maximumJsonNodes then invalidOp "report exceeds the JSON node limit"
            if depth > maximumJsonDepth then invalidOp "report exceeds the JSON depth limit"
            match element.ValueKind with
            | JsonValueKind.Object ->
                let names = HashSet<string>(StringComparer.Ordinal)
                for property in element.EnumerateObject() do
                    if not (names.Add property.Name) then invalidOp ("duplicate JSON property: " + property.Name)
                    visit (depth + 1) property.Value
            | JsonValueKind.Array -> for item in element.EnumerateArray() do visit (depth + 1) item
            | _ -> ()
        visit 0 root

    /// Validates a JSON report, including its raw orders and recomputed analysis.
    let fromJson (json: string) : Result<Report, BenchmarkError> =
        try
            if isNull json then invalidOp "report JSON must not be null"
            if Encoding.UTF8.GetByteCount json > maximumJsonBytes then invalidOp "report exceeds the 4 MiB JSON limit"
            use document = JsonDocument.Parse(json, JsonDocumentOptions(MaxDepth = maximumJsonDepth))
            let root = document.RootElement
            checkTree root
            ensureOnly [ "config"; "batch_size"; "pairs"; "analysis"; "workload"; "invocation"; "revision"; "runtime"; "os"; "arch"; "profile" ] root
            let configElement = required "config" root
            ensureOnly [ "pairs"; "confidence_percent"; "minimum_factor"; "seed"; "target_ns"; "max_batch" ] configElement
            let factorElement = required "minimum_factor" configElement
            let config =
                { pairs = required "pairs" configElement |> readInt32 "pairs"
                  confidencePercent = required "confidence_percent" configElement |> readInt32 "confidence_percent"
                  minimumFactor = factorElement.GetDouble()
                  seed = required "seed" configElement |> readInt64 "seed"
                  targetNs = required "target_ns" configElement |> readInt64 "target_ns"
                  maxBatch = required "max_batch" configElement |> readInt64 "max_batch" }
            match validateConfig config with Error error -> invalidOp (sprintf "%A" error) | Ok _ -> ()
            let batchSize = required "batch_size" root |> readInt64 "batch_size"
            if batchSize <= 0L || batchSize > config.maxBatch then invalidOp "batch_size is outside max_batch"
            let pairsElement = required "pairs" root
            if pairsElement.ValueKind <> JsonValueKind.Array || pairsElement.GetArrayLength() <> config.pairs then invalidOp "pair count does not match config"
            let pairs =
                pairsElement.EnumerateArray()
                |> Seq.map (fun item ->
                    ensureOnly [ "baseline_ns"; "candidate_ns"; "order" ] item
                    let baselineNs = required "baseline_ns" item |> readInt64 "baseline_ns"
                    let candidateNs = required "candidate_ns" item |> readInt64 "candidate_ns"
                    let order = required "order" item |> readString "order" |> parseOrder |> Result.defaultWith (fun error -> invalidOp (sprintf "%A" error))
                    { baselineNs = baselineNs; candidateNs = candidateNs; order = order })
                |> Seq.toList
            if pairs |> List.exists (fun pair -> pair.baselineNs <= 0L || pair.candidateNs <= 0L) then invalidOp "pair durations must be positive"
            if pairs |> List.filter (fun pair -> pair.order = BaselineFirst) |> List.length <> (config.pairs + 1) / 2 then invalidOp "pair order is not balanced"
            let analysisElement = required "analysis" root
            ensureOnly [ "ratio"; "interval"; "verdict" ] analysisElement
            let ratioElement = required "ratio" analysisElement
            let ratio = ratioElement.GetDouble()
            if not (Double.IsFinite ratio) then invalidOp "ratio must be finite"
            let intervalElement = required "interval" analysisElement
            let interval =
                if intervalElement.ValueKind = JsonValueKind.Null then None
                elif intervalElement.ValueKind = JsonValueKind.Array && intervalElement.GetArrayLength() = 2 then Some(intervalElement[0].GetDouble(), intervalElement[1].GetDouble())
                else invalidOp "interval must be null or contain two values"
            let verdict = required "verdict" analysisElement |> readString "verdict" |> parseVerdict |> Result.defaultWith (fun error -> invalidOp (sprintf "%A" error))
            let claimed = { ratio = ratio; interval = interval; verdict = verdict }
            let metadataNames = [ "workload"; "invocation"; "revision"; "runtime"; "os"; "arch" ]
            let present = metadataNames |> List.filter (fun name -> root.TryGetProperty name |> fst)
            let metadata =
                if present.IsEmpty then
                    if root.TryGetProperty("profile") |> fst then invalidOp "profile requires standard metadata fields"
                    None
                elif present.Length <> metadataNames.Length then invalidOp "metadata must include every standard field"
                else
                    let invocationElement = required "invocation" root
                    if invocationElement.ValueKind <> JsonValueKind.Array then invalidOp "invocation must be an array"
                    let invocation = invocationElement.EnumerateArray() |> Seq.map (readString "invocation entry") |> Seq.toArray
                    let profile = if root.TryGetProperty("profile") |> fst then Some(required "profile" root |> readString "profile") else None
                    let workload = required "workload" root |> readString "workload"
                    if not (isValidWorkload workload) then invalidOp "workload must contain a non-ASCII-whitespace character and no NUL"
                    Some
                        { workload = workload
                          invocation = invocation
                          revision = required "revision" root |> readString "revision"
                          runtime = required "runtime" root |> readString "runtime"
                          os = required "os" root |> readString "os"
                          arch = required "arch" root |> readString "arch"
                          profile = profile }
            let computed = analyze pairs config.confidencePercent config.minimumFactor |> Result.defaultWith (fun error -> invalidOp (sprintf "%A" error))
            if computed <> claimed then invalidOp "reported analysis disagrees with raw pairs"
            Ok { config = config; batchSize = batchSize; pairs = pairs; analysis = computed; metadata = metadata }
        with error -> Error(InvalidReport error.Message)
