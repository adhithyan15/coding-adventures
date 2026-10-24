using System.Security.Cryptography;
using System.Text.Json;
using CodingAdventures.DocumentAst;
using Pipeline = CodingAdventures.Commonmark.Commonmark;

namespace CodingAdventures.Commonmark.Tests;

/// <summary>
/// The official corpus is data, not source code.  Loading it at runtime keeps
/// the case roster reviewable and ensures each record enters through the same
/// public facade APIs a caller uses.
/// </summary>
public sealed class CommonmarkSpecTests
{
    private const int ExpectedCaseCount = 652;
    private const string ExpectedSha256 = "d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20";

    public static IEnumerable<object[]> Cases()
        => LoadCases().Select(testCase => new object[] { testCase });

    [Theory]
    [MemberData(nameof(Cases))]
    public void Commonmark_ExecutesOfficial0312Case(CommonmarkCase testCase)
    {
        var document = Pipeline.Parse(testCase.Markdown);
        var html = Pipeline.ToHtml(document);

        Assert.Equal(testCase.Html, html);
    }

    [Fact]
    public void Fixture_HasExactVersionedRosterAndDigest()
    {
        var path = FixturePath();
        var cases = LoadCases();
        var digest = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path))).ToLowerInvariant();

        Assert.Equal(ExpectedCaseCount, cases.Count);
        Assert.Equal(Enumerable.Range(1, ExpectedCaseCount), cases.Select(testCase => testCase.Example));
        Assert.Equal(ExpectedSha256, digest);
    }

    [Fact]
    public void Facade_ExposesParserAndRendererWithoutHidingTheAst()
    {
        DocumentNode document = Pipeline.Parse("# Hello\n");
        Assert.IsType<HeadingNode>(Assert.Single(document.Children));
        Assert.Equal("<h1>Hello</h1>\n", Pipeline.ToHtml(document));
    }

    private static IReadOnlyList<CommonmarkCase> LoadCases()
    {
        using var stream = File.OpenRead(FixturePath());
        return JsonSerializer.Deserialize<List<CommonmarkCase>>(
            stream,
            new JsonSerializerOptions { PropertyNameCaseInsensitive = true })
            ?? throw new InvalidDataException("The CommonMark fixture must contain a JSON array.");
    }

    private static string FixturePath()
        => Path.Combine(AppContext.BaseDirectory, "fixtures", "spec.json");
}

public sealed record CommonmarkCase(
    string Markdown,
    string Html,
    int Example,
    int StartLine,
    int EndLine,
    string Section)
{
    public override string ToString() => $"CommonMark 0.31.2 example {Example}: {Section}";
}
