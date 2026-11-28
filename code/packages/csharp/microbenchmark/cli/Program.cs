using System.Globalization;
using CodingAdventures.Microbenchmark;

return ProgramEntry.Run(args);

public static class ProgramEntry
{
    public static int Run(string[] args)
    {
        try
        {
            if (args.Contains("--help", StringComparer.Ordinal) || args.Contains("-h", StringComparer.Ordinal))
            {
                WriteAndFlush("Usage: array-microbenchmark [--pairs N] [--confidence P] [--factor F] [--seed N] [--target-ns N] [--max-batch N] [--revision ID]\n");
                return 0;
            }

            var config = new Config();
            var revision = Environment.GetEnvironmentVariable("GIT_REVISION");
            if (string.IsNullOrWhiteSpace(revision)) revision = Environment.GetEnvironmentVariable("GITHUB_SHA");
            if (string.IsNullOrWhiteSpace(revision)) revision = "unknown";
            for (var i = 0; i < args.Length; i++)
            {
                var flag = args[i];
                if (i + 1 >= args.Length) throw new ArgumentException($"{flag} requires a value");
                var value = args[++i];
                if (flag == "--revision")
                {
                    if (string.IsNullOrWhiteSpace(value)) throw new ArgumentException("--revision must not be blank");
                    revision = value;
                    continue;
                }
                config = flag switch
                {
                    "--pairs" => config with { Pairs = int.Parse(value, NumberStyles.None, CultureInfo.InvariantCulture) },
                    "--confidence" => config with { ConfidencePercent = int.Parse(value, NumberStyles.None, CultureInfo.InvariantCulture) },
                    "--factor" => config with { MinimumFactor = double.Parse(value, NumberStyles.Float, CultureInfo.InvariantCulture) },
                    "--seed" => config with { Seed = long.Parse(value, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture) },
                    "--target-ns" => config with { TargetNs = long.Parse(value, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture) },
                    "--max-batch" => config with { MaxBatch = long.Parse(value, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture) },
                    _ => throw new ArgumentException($"unknown option: {flag}"),
                };
            }

            string[] invocation = [Environment.ProcessPath ?? "array-microbenchmark", .. args];
#if DEBUG
            const string profile = "debug";
#else
            const string profile = "release";
#endif
            var report = ArrayWorkload.Run(config, ArrayWorkload.CreateClock(), invocation, revision, profile);
            WriteAndFlush(report.ToJson() + "\n");
            return 0;
        }
        catch (Exception error)
        {
            Console.Error.WriteLine($"array-microbenchmark: {error.Message}");
            return 1;
        }
    }

    private static void WriteAndFlush(string text)
    {
        CliOutput.WriteAndFlush(Console.Out, text);
    }
}
