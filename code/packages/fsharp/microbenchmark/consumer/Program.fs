open CodingAdventures.Microbenchmark
open CodingAdventures.RankStatistics

[<EntryPoint>]
let main arguments =
    let nativeReportIsValid =
        match arguments with
        | [||] -> true
        | [| json |] ->
            match Microbenchmark.fromJson json with
            | Ok report ->
                report.pairs.Length = 40
                && (report.pairs |> List.forall (fun pair -> pair.baselineNs > 0L && pair.candidateNs > 0L))
            | Error _ -> false
        | _ -> false
    match RankStatistics.median [ 3.0; 1.0; 2.0 ], Microbenchmark.analyze [ { baselineNs = 10L; candidateNs = 9L; order = BaselineFirst } ] 95 1.05 with
    | 2.0, Ok { verdict = Inconclusive } when nativeReportIsValid ->
        if arguments.Length = 1 then printfn "installed F# package replayed the native 40-pair report"
        printfn "installed F# packages consumed successfully"
        0
    | _ -> 1
