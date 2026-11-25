open CodingAdventures.Microbenchmark
open CodingAdventures.RankStatistics

[<EntryPoint>]
let main _ =
    match RankStatistics.median [ 3.0; 1.0; 2.0 ], Microbenchmark.analyze [ { baselineNs = 10L; candidateNs = 9L; order = BaselineFirst } ] 95 1.05 with
    | 2.0, Ok { verdict = Inconclusive } ->
        printfn "installed F# packages consumed successfully"
        0
    | _ -> 1
