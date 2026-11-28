# C# paired microbenchmarks

`CodingAdventures.Microbenchmark.CSharp` provides paired collection,
analysis, and strict JSON report replay. Its rank-statistics implementation
is a separate NuGet dependency. Replay accepts workload labels containing a
non-NUL character outside ASCII whitespace (U+0020 and U+0009 through U+000D);
other Unicode whitespace, including U+2003, remains valid.

```csharp
using CodingAdventures.Microbenchmark;

long tick = 0;
long Work(long n) => 10 * n;
var report = Microbenchmark.Collect(
    new Config { Pairs = 1, TargetNs = 1, MaxBatch = 1 },
    10, Work, Work, () => ++tick);
var replayed = Report.FromJson(report.ToJson());
```

The native indexed-array CLI is in `cli/` and runs with
`dotnet run --project cli/CodingAdventures.ArrayMicrobenchmark.CSharp.csproj
-- --pairs 40`. It compares indexed access with `foreach` over the same
4,096-element array. It reports only the standard metadata fields. Timings are
advisory and do not prove physical cache behavior.

Reports are limited to 10,000 pairs. Replay rejects JSON above 4 MiB, nesting above 128 levels, or more than 65,536 JSON values. It also rejects duplicate or unknown fields and checks balanced pair order. Run `./verify-consumer.ps1` to pack both NuGet packages and execute an installed consumer using a temporary local feed.

Replay accepts standard metadata either as a complete set or not at all;
`profile` requires that complete set. Blank revision environment values fall
back from `GIT_REVISION` to `GITHUB_SHA` to `unknown`, while a blank explicit
`--revision` fails. Public array batch helpers reject negative operation counts
and return zero for zero operations.
