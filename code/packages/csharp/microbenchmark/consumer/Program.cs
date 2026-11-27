using CodingAdventures.Microbenchmark;
using CodingAdventures.RankStatistics;

if (RankStatistics.Median([3.0, 1.0, 2.0]) != 2.0) return 1;
var report = Microbenchmark.Analyze(
    [new Pair { BaselineNs = 10, CandidateNs = 9, Order = "baseline-first" }], 95, 1.05);
if (report.Verdict != "inconclusive") return 2;
if (args.Length > 0)
{
    if (args.Length != 1) return 3;
    var native = Report.FromJson(args[0]);
    if (native.Pairs.Count != 40 || native.Pairs.Any(pair => pair.BaselineNs <= 0 || pair.CandidateNs <= 0)) return 4;
    Console.WriteLine("installed C# package replayed the native 40-pair report");
}
Console.WriteLine("installed C# packages consumed successfully");
return 0;
