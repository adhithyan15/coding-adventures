# F# paired microbenchmarks

`CodingAdventures.Microbenchmark.FSharp` exposes an F#-native paired
collector (`Result` errors, records, discriminated unions, and options), exact
analysis, and JSON replay. Its separate F# rank-statistics package has no C#
runtime dependency. Replay accepts workload labels containing a non-NUL
character outside ASCII whitespace (U+0020 and U+0009 through U+000D); other
Unicode whitespace, including U+2003, remains valid.

```fsharp
open CodingAdventures.Microbenchmark

let baseline count = Ok (10L * count)
let candidate count = Ok (10L * count)
let mutable tick = 0L
let clock () = tick <- tick + 1L; tick
let report = Microbenchmark.collect { Microbenchmark.defaultConfig with pairs = 1; targetNs = 1L; maxBatch = 1L } 10L baseline candidate clock
```

The native CLI under `cli/` compares indexed array access with F# array
iteration over the same 4,096 values. Timings are advisory and do not prove
physical cache behavior.

Reports are limited to 10,000 pairs. Replay rejects JSON above 4 MiB, nesting above 128 levels, or more than 65,536 JSON values. It also rejects duplicate or unknown fields and checks balanced pair order. Run `./verify-consumer.ps1` to pack both NuGet packages and execute an installed consumer using a temporary local feed.

Replay accepts standard metadata either as a complete set or not at all;
`profile` requires that complete set. Blank revision environment values fall
back from `GIT_REVISION` to `GITHUB_SHA` to `unknown`, while a blank explicit
`--revision` fails. Public array batch helpers reject negative operation counts
and return zero for zero operations.
