using CodingAdventures.Microbenchmark;
using CodingAdventures.RankStatistics;

if (RankStatistics.Median([3.0, 1.0, 2.0]) != 2.0) return 1;
var report = Microbenchmark.Analyze(
    [new Pair { BaselineNs = 10, CandidateNs = 9, Order = "baseline-first" }], 95, 1.05);
if (report.Verdict != "inconclusive") return 2;
Console.WriteLine("installed C# packages consumed successfully");
return 0;
