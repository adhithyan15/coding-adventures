namespace CodingAdventures.Hamurabi.Tests;

using System;
using System.IO;
using System.Text.Json;
using Xunit;

public class ConformanceTests
{
    private static string GetCasesJsonPath()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir != null && !Directory.Exists(Path.Combine(dir.FullName, "code", "specs")))
        {
            dir = dir.Parent;
        }
        Assert.NotNull(dir);
        var path = Path.Combine(dir!.FullName, "code", "specs", "fixtures", "hamurabi-v1", "cases.json");
        Assert.True(File.Exists(path), $"cases.json not found at {path}");
        return path;
    }

    private static GameStatus ParseStatus(string s)
    {
        if (s == "Playing")
        {
            return new GameStatus.Playing();
        }
        if (s == "GameOverExtinction")
        {
            return new GameStatus.GameOverExtinction();
        }
        if (s.StartsWith("Won(") && s.EndsWith(")"))
        {
            string rStr = s[4..^1];
            var rating = Enum.Parse<Rating>(rStr);
            return new GameStatus.Won(rating);
        }
        if (s.StartsWith("Impeached(") && s.EndsWith(")"))
        {
            // format: Impeached(starved=50, percent=50)
            string inner = s[10..^1];
            var parts = inner.Split(',');
            uint starved = uint.Parse(parts[0].Split('=')[1].Trim());
            uint percent = uint.Parse(parts[1].Split('=')[1].Trim());
            return new GameStatus.Impeached(starved, percent);
        }
        throw new ArgumentException($"Unknown status string: {s}");
    }

    private static HamurabiState ParseState(JsonElement el)
    {
        return new HamurabiState(
            Year: el.GetProperty("year").GetByte(),
            Population: el.GetProperty("population").GetUInt32(),
            Bushels: el.GetProperty("bushels").GetUInt32(),
            Acres: el.GetProperty("acres").GetUInt32(),
            LandPrice: el.GetProperty("land_price").GetUInt32(),
            HarvestPerAcre: el.GetProperty("harvest_per_acre").GetUInt32(),
            RatsEaten: el.GetProperty("rats_eaten").GetUInt32(),
            StarvedLastYear: el.GetProperty("starved_last_year").GetUInt32(),
            StarvedTotal: el.GetProperty("starved_total").GetUInt32(),
            PercentStarvedSum: el.GetProperty("percent_starved_sum").GetUInt32(),
            ImmigrantsLastYear: el.GetProperty("immigrants_last_year").GetUInt32(),
            PlagueStruck: el.GetProperty("plague_struck").GetBoolean(),
            Status: ParseStatus(el.GetProperty("status").GetString()!),
            RngState: el.GetProperty("rng_state").GetUInt64(),
            RngInc: el.GetProperty("rng_inc").GetUInt64()
        );
    }

    private static TurnOrder ParseOrder(JsonElement el)
    {
        return new TurnOrder(
            AcresDelta: el.GetProperty("acres_delta").GetInt32(),
            BushelsToFeed: el.GetProperty("bushels_to_feed").GetUInt32(),
            AcresToPlant: el.GetProperty("acres_to_plant").GetUInt32()
        );
    }

    private static HamurabiError ParseError(string s) => Enum.Parse<HamurabiError>(s);

    private static void AssertStatesEqual(HamurabiState expected, HamurabiState actual, string context)
    {
        Assert.True(expected.Year == actual.Year, $"{context}: Year expected {expected.Year}, got {actual.Year}");
        Assert.True(expected.Population == actual.Population, $"{context}: Population expected {expected.Population}, got {actual.Population}");
        Assert.True(expected.Bushels == actual.Bushels, $"{context}: Bushels expected {expected.Bushels}, got {actual.Bushels}");
        Assert.True(expected.Acres == actual.Acres, $"{context}: Acres expected {expected.Acres}, got {actual.Acres}");
        Assert.True(expected.LandPrice == actual.LandPrice, $"{context}: LandPrice expected {expected.LandPrice}, got {actual.LandPrice}");
        Assert.True(expected.HarvestPerAcre == actual.HarvestPerAcre, $"{context}: HarvestPerAcre expected {expected.HarvestPerAcre}, got {actual.HarvestPerAcre}");
        Assert.True(expected.RatsEaten == actual.RatsEaten, $"{context}: RatsEaten expected {expected.RatsEaten}, got {actual.RatsEaten}");
        Assert.True(expected.StarvedLastYear == actual.StarvedLastYear, $"{context}: StarvedLastYear expected {expected.StarvedLastYear}, got {actual.StarvedLastYear}");
        Assert.True(expected.StarvedTotal == actual.StarvedTotal, $"{context}: StarvedTotal expected {expected.StarvedTotal}, got {actual.StarvedTotal}");
        Assert.True(expected.PercentStarvedSum == actual.PercentStarvedSum, $"{context}: PercentStarvedSum expected {expected.PercentStarvedSum}, got {actual.PercentStarvedSum}");
        Assert.True(expected.ImmigrantsLastYear == actual.ImmigrantsLastYear, $"{context}: ImmigrantsLastYear expected {expected.ImmigrantsLastYear}, got {actual.ImmigrantsLastYear}");
        Assert.True(expected.PlagueStruck == actual.PlagueStruck, $"{context}: PlagueStruck expected {expected.PlagueStruck}, got {actual.PlagueStruck}");
        Assert.True(expected.Status.ToString() == actual.Status.ToString(), $"{context}: Status expected {expected.Status}, got {actual.Status}");
        Assert.True(expected.RngState == actual.RngState, $"{context}: RngState expected {expected.RngState}, got {actual.RngState}");
        Assert.True(expected.RngInc == actual.RngInc, $"{context}: RngInc expected {expected.RngInc}, got {actual.RngInc}");
    }

    [Fact]
    public void TestAllCasesInConformanceCorpus()
    {
        string jsonPath = GetCasesJsonPath();
        string jsonText = File.ReadAllText(jsonPath);
        using var doc = JsonDocument.Parse(jsonText);

        var cases = doc.RootElement.GetProperty("cases").EnumerateArray();
        int caseCount = 0;

        foreach (var c in cases)
        {
            caseCount++;
            string id = c.GetProperty("id").GetString()!;
            var initialState = ParseState(c.GetProperty("initial_state"));

            bool hasTurns = c.TryGetProperty("turns", out var turnsEl);

            if (hasTurns &&
                c.TryGetProperty("seed", out var seedEl) &&
                seedEl.ValueKind == JsonValueKind.Number &&
                initialState.Year == 1 &&
                initialState.Population == 100 &&
                initialState.Bushels == 2800 &&
                initialState.Acres == 1000 &&
                initialState.Status is GameStatus.Playing)
            {
                ulong seed = seedEl.GetUInt64();
                var newGameState = Hamurabi.NewGame(seed);
                AssertStatesEqual(initialState, newGameState, $"NewGame({seed}) vs initial_state in case '{id}'");
            }

            if (hasTurns)
            {
                var currentState = initialState;
                foreach (var turnEl in turnsEl.EnumerateArray())
                {
                    int turnNum = turnEl.GetProperty("turn").GetInt32();
                    var order = ParseOrder(turnEl.GetProperty("order"));
                    var expectedState = ParseState(turnEl.GetProperty("expected_state"));
                    string expectedReport = turnEl.GetProperty("expected_report").GetString()!;

                    var valRes = Hamurabi.ValidateOrder(currentState, order);
                    Assert.True(valRes.IsOk, $"Case '{id}' Turn {turnNum} ValidateOrder failed: {valRes.Error}");

                    var stepRes = Hamurabi.StepYear(currentState, order);
                    Assert.True(stepRes.IsOk, $"Case '{id}' Turn {turnNum} StepYear failed: {stepRes.Error}");

                    var nextState = stepRes.Value;
                    AssertStatesEqual(expectedState, nextState, $"Case '{id}' Turn {turnNum}");

                    string report = Hamurabi.FormatReport(nextState);
                    Assert.Equal(expectedReport, report);

                    currentState = nextState;
                }
            }
            else if (c.TryGetProperty("expected_error", out var errorEl))
            {
                var order = ParseOrder(c.GetProperty("order"));
                var expectedErr = ParseError(errorEl.GetString()!);

                var valRes = Hamurabi.ValidateOrder(initialState, order);
                Assert.False(valRes.IsOk, $"Case '{id}' expected error {expectedErr}, but ValidateOrder succeeded");
                Assert.Equal(expectedErr, valRes.Error);

                var stepRes = Hamurabi.StepYear(initialState, order);
                Assert.False(stepRes.IsOk, $"Case '{id}' expected error {expectedErr}, but StepYear succeeded");
                Assert.Equal(expectedErr, stepRes.Error);
            }
            else
            {
                Assert.Fail($"Case '{id}' has neither turns nor expected_error");
            }
        }

        Assert.Equal(31, caseCount);
    }
}
