using CodingAdventures.CliBuilder;
using CodingAdventures.Cowsay;
using CodingAdventures.PaintInstructions;

namespace CodingAdventures.Cowsay.Tests;

/// <summary>
/// Regression test for a real bug this port's manual verification caught: CliBuilder's
/// Parser follows the C/Go argv convention (index 0 = program name, real tokens start
/// at index 1). C#'s `args` doesn't include the program name, so `Program.cs` must
/// prepend a placeholder before constructing Parser — otherwise the first real CLI
/// token is silently dropped. See the "C#" entry in lessons.md. This test drives the
/// actual Parser end-to-end (unlike the CowsayCli tests, which hand-build the
/// flags/arguments dictionaries and never touch Parser at all), because that's exactly
/// the gap that let the bug through unit tests the first time.
/// </summary>
public class ProgramArgvConventionTests
{
    private static readonly string SpecPath = Path.Combine(
        CowsayRenderer.FindRepoRoot(AppContext.BaseDirectory), "code", "specs", "cowsay.json");

    [Fact]
    public void SingleWordMessageIsNotDroppedWhenProgramNameIsPrepended()
    {
        var argv = new List<string> { "cowsay", "hello" };
        var result = Assert.IsType<ParseResult>(new Parser(SpecPath, argv).Parse());

        var message = CowsayCli.ResolveMessageFromArguments(result.Arguments);

        Assert.Equal("hello", message);
    }

    [Fact]
    public void MultiWordMessageKeepsItsFirstWordWhenProgramNameIsPrepended()
    {
        var argv = new List<string> { "cowsay", "hello", "world" };
        var result = Assert.IsType<ParseResult>(new Parser(SpecPath, argv).Parse());

        var message = CowsayCli.ResolveMessageFromArguments(result.Arguments);

        Assert.Equal("hello world", message);
    }
}

public class WrapTextTests
{
    [Fact]
    public void ShortTextIsNotWrapped()
    {
        Assert.Equal(["hello"], CowsayRenderer.WrapText("hello", 40));
    }

    [Fact]
    public void LongTextWrapsAtWordBoundaries()
    {
        var result = CowsayRenderer.WrapText("the quick brown fox jumps over", 10);
        Assert.Equal(["the quick", "brown fox", "jumps over"], result);
    }

    [Fact]
    public void EmptyTextReturnsEmptyLine()
    {
        Assert.Equal([""], CowsayRenderer.WrapText("", 40));
    }

    [Fact]
    public void SingleWordLongerThanWidthStaysWhole()
    {
        var result = CowsayRenderer.WrapText("supercalifragilisticexpialidocious", 5);
        Assert.Equal(["supercalifragilisticexpialidocious"], result);
    }

    [Fact]
    public void WhitespaceOnlyTextReturnsEmptyLine()
    {
        var result = CowsayRenderer.WrapText("     ", 3);
        Assert.Equal([""], result);
    }
}

public class FormatBubbleTests
{
    [Fact]
    public void EmptyLinesReturnsEmptyString()
    {
        Assert.Equal("", CowsayRenderer.FormatBubble([], isThink: false));
    }

    [Fact]
    public void SingleLineSpeechBubble()
    {
        var result = CowsayRenderer.FormatBubble(["hi"], isThink: false);
        Assert.Equal(" ____\n< hi >\n ----", result);
    }

    [Fact]
    public void SingleLineThoughtBubble()
    {
        var result = CowsayRenderer.FormatBubble(["hi"], isThink: true);
        Assert.Equal(" ____\n( hi )\n ----", result);
    }

    [Fact]
    public void MultiLineSpeechBubbleUsesSlashPipeBackslashBorders()
    {
        var result = CowsayRenderer.FormatBubble(["one", "two", "three"], isThink: false);
        Assert.Equal(
            " _______\n" +
            "/ one   \\\n" +
            "| two   |\n" +
            "\\ three /\n" +
            " -------",
            result);
    }

    [Fact]
    public void MultiLineThoughtBubbleUsesParensOnEveryLine()
    {
        var result = CowsayRenderer.FormatBubble(["one", "two"], isThink: true);
        Assert.Equal(
            " _____\n" +
            "( one )\n" +
            "( two )\n" +
            " -----",
            result);
    }
}

public class NormalizeTwoCharsTests
{
    [Theory]
    [InlineData("o", "o ")]
    [InlineData("", "  ")]
    [InlineData("oo", "oo")]
    [InlineData("ooo", "oo")]
    public void NormalizesToExactlyTwoCharacters(string input, string expected)
    {
        Assert.Equal(expected, CowsayRenderer.NormalizeTwoChars(input));
    }
}

public class ResolveEyesAndTongueTests
{
    [Fact]
    public void NoActiveModesKeepsBaseValues()
    {
        var (eyes, tongue) = CowsayRenderer.ResolveEyesAndTongue("oo", "  ", []);
        Assert.Equal("oo", eyes);
        Assert.Equal("  ", tongue);
    }

    [Theory]
    [InlineData("borg", "==", "  ")]
    [InlineData("dead", "XX", "U ")]
    [InlineData("greedy", "$$", "  ")]
    [InlineData("paranoid", "@@", "  ")]
    [InlineData("stoned", "xx", "U ")]
    [InlineData("tired", "--", "  ")]
    [InlineData("wired", "OO", "  ")]
    [InlineData("youthful", "..", "  ")]
    public void EachModeOverridesEyesAndSometimesTongue(string mode, string expectedEyes, string expectedTongue)
    {
        var (eyes, tongue) = CowsayRenderer.ResolveEyesAndTongue("oo", "  ", [mode]);
        Assert.Equal(expectedEyes, eyes);
        Assert.Equal(expectedTongue, tongue);
    }

    [Fact]
    public void UnknownModeIsIgnored()
    {
        var (eyes, tongue) = CowsayRenderer.ResolveEyesAndTongue("oo", "  ", ["not-a-real-mode"]);
        Assert.Equal("oo", eyes);
        Assert.Equal("  ", tongue);
    }
}

public class LoadCowTests : IDisposable
{
    private readonly string _tempDir = Directory.CreateTempSubdirectory("cowsay-tests-").FullName;

    public void Dispose() => Directory.Delete(_tempDir, recursive: true);

    [Fact]
    public void LoadsBodyBetweenHeredocMarkers()
    {
        File.WriteAllText(Path.Combine(_tempDir, "default.cow"),
            "$the_cow = <<EOC;\n  $thoughts   ^__^\n   ($eyes)\nEOC\n");

        var body = CowsayRenderer.LoadCow("default", _tempDir);

        Assert.Equal("  $thoughts   ^__^\n   ($eyes)\n", body);
    }

    [Fact]
    public void FallsBackToDefaultWhenNamedCowIsMissing()
    {
        File.WriteAllText(Path.Combine(_tempDir, "default.cow"), "$the_cow = <<EOC;\nfallback\nEOC\n");

        var body = CowsayRenderer.LoadCow("does-not-exist", _tempDir);

        Assert.Equal("fallback\n", body);
    }

    [Theory]
    [InlineData("../../../../../../etc/passwd")]
    [InlineData("..\\..\\..\\secret")]
    [InlineData("../outside")]
    public void FallsBackToDefaultInsteadOfEscapingCowsDirViaTraversal(string maliciousCowName)
    {
        File.WriteAllText(Path.Combine(_tempDir, "default.cow"), "$the_cow = <<EOC;\nfallback\nEOC\n");
        var outsideDir = Directory.CreateTempSubdirectory("cowsay-outside-").FullName;
        try
        {
            File.WriteAllText(Path.Combine(outsideDir, "secret.cow"), "$the_cow = <<EOC;\nSECRET\nEOC\n");
            File.WriteAllText(Path.Combine(outsideDir, "outside.cow"), "$the_cow = <<EOC;\nSECRET\nEOC\n");

            var body = CowsayRenderer.LoadCow(maliciousCowName, _tempDir);

            Assert.Equal("fallback\n", body);
        }
        finally
        {
            Directory.Delete(outsideDir, recursive: true);
        }
    }

    [Fact]
    public void FallsBackToDefaultInsteadOfFollowingARootedPathOverride()
    {
        File.WriteAllText(Path.Combine(_tempDir, "default.cow"), "$the_cow = <<EOC;\nfallback\nEOC\n");
        var outsideDir = Directory.CreateTempSubdirectory("cowsay-outside-").FullName;
        try
        {
            var rootedTarget = Path.Combine(outsideDir, "win");
            File.WriteAllText(rootedTarget + ".cow", "$the_cow = <<EOC;\nSECRET\nEOC\n");

            var body = CowsayRenderer.LoadCow(rootedTarget, _tempDir);

            Assert.Equal("fallback\n", body);
        }
        finally
        {
            Directory.Delete(outsideDir, recursive: true);
        }
    }
}

public class ComposeContentTests : IDisposable
{
    private readonly string _tempDir = Directory.CreateTempSubdirectory("cowsay-tests-").FullName;

    public ComposeContentTests()
    {
        File.WriteAllText(Path.Combine(_tempDir, "default.cow"), "$the_cow = <<EOC;\n$thoughts $eyes $tongue\nEOC\n");
    }

    public void Dispose() => Directory.Delete(_tempDir, recursive: true);

    [Fact]
    public void ComposesBubbleAndCowWithSubstitutions()
    {
        var invocation = new CowsayInvocation(
            Message: "hi",
            Eyes: "oo",
            Tongue: "  ",
            ActiveModes: [],
            NoWrap: false,
            Width: 40,
            Think: false,
            CowFile: "default");

        var content = CowsayRenderer.ComposeContent(invocation, _tempDir);

        Assert.Equal(" ____\n< hi >\n ----\n\\ oo   \n", content);
    }

    [Fact]
    public void ThinkModeUsesOForThoughtsAndParenBubble()
    {
        var invocation = new CowsayInvocation(
            Message: "hi",
            Eyes: "oo",
            Tongue: "  ",
            ActiveModes: [],
            NoWrap: false,
            Width: 40,
            Think: true,
            CowFile: "default");

        var content = CowsayRenderer.ComposeContent(invocation, _tempDir);

        Assert.Equal(" ____\n( hi )\n ----\no oo   \n", content);
    }

    [Fact]
    public void ModeFlagOverridesEyesInCowTemplate()
    {
        var invocation = new CowsayInvocation(
            Message: "hi",
            Eyes: "oo",
            Tongue: "  ",
            ActiveModes: ["dead"],
            NoWrap: false,
            Width: 40,
            Think: false,
            CowFile: "default");

        var content = CowsayRenderer.ComposeContent(invocation, _tempDir);

        Assert.Equal(" ____\n< hi >\n ----\n\\ XX U \n", content);
    }
}

public class BuildSceneTests
{
    [Fact]
    public void OneGlyphRunPerNonBlankLineWithCorrectPlacements()
    {
        var scene = CowsayRenderer.BuildScene("hi\n\nyo");

        var glyphRuns = scene.Instructions.Cast<PaintGlyphRun>().ToList();
        Assert.Equal(2, glyphRuns.Count);

        Assert.Equal(2, glyphRuns[0].Glyphs.Count);
        Assert.Equal(new PaintGlyphPlacement('h', 0, 0), glyphRuns[0].Glyphs[0]);
        Assert.Equal(new PaintGlyphPlacement('i', CowsayRenderer.ScaleX, 0), glyphRuns[0].Glyphs[1]);

        Assert.Equal(2, glyphRuns[1].Glyphs.Count);
        Assert.Equal(new PaintGlyphPlacement('y', 0, 2 * CowsayRenderer.ScaleY), glyphRuns[1].Glyphs[0]);
        Assert.Equal(new PaintGlyphPlacement('o', CowsayRenderer.ScaleX, 2 * CowsayRenderer.ScaleY), glyphRuns[1].Glyphs[1]);
    }

    [Fact]
    public void SpacesAreSkippedNotPlaced()
    {
        var scene = CowsayRenderer.BuildScene("a b");

        var glyphRun = Assert.Single(scene.Instructions.Cast<PaintGlyphRun>());
        Assert.Equal(2, glyphRun.Glyphs.Count);
        Assert.Equal('a', (char)glyphRun.Glyphs[0].GlyphId);
        Assert.Equal('b', (char)glyphRun.Glyphs[1].GlyphId);
    }

    [Fact]
    public void SceneDimensionsCoverAllLines()
    {
        var scene = CowsayRenderer.BuildScene("abc\nde");

        Assert.Equal(3 * CowsayRenderer.ScaleX, scene.Width);
        Assert.Equal(2 * CowsayRenderer.ScaleY, scene.Height);
    }
}

public class RenderRoundTripTests
{
    [Theory]
    [InlineData("hi")]
    [InlineData("hello\nworld")]
    [InlineData(" ____\n< hi >\n ----\n\\   ^__^\n")]
    public void RenderingThroughPaintVmAsciiReproducesTheOriginalText(string content)
    {
        var rendered = CowsayRenderer.BuildScene(content);
        var output = CodingAdventures.PaintVmAscii.PaintVmAscii.RenderToAscii(rendered);

        var expectedLines = content.Split('\n').Select(line => line.TrimEnd());
        var expected = string.Join("\n", expectedLines).TrimEnd('\n');

        Assert.Equal(expected, output);
    }
}

public class CowsayCliTests
{
    [Fact]
    public void IsListRequestedTrueWhenFlagPresent()
    {
        Assert.True(CowsayCli.IsListRequested(new Dictionary<string, object?> { ["list"] = true }));
    }

    [Theory]
    [MemberData(nameof(NotListRequestedCases))]
    public void IsListRequestedFalseOtherwise(IReadOnlyDictionary<string, object?> flags)
    {
        Assert.False(CowsayCli.IsListRequested(flags));
    }

    public static IEnumerable<object[]> NotListRequestedCases()
    {
        yield return [new Dictionary<string, object?>()];
        yield return [new Dictionary<string, object?> { ["list"] = false }];
    }

    [Fact]
    public void ResolveMessageJoinsPositionalWords()
    {
        var arguments = new Dictionary<string, object?>
        {
            ["message"] = new List<object?> { "hello", "there" },
        };

        Assert.Equal("hello there", CowsayCli.ResolveMessageFromArguments(arguments));
    }

    [Theory]
    [MemberData(nameof(NoMessageCases))]
    public void ResolveMessageReturnsNullWhenNoPositionalMessage(IReadOnlyDictionary<string, object?> arguments)
    {
        Assert.Null(CowsayCli.ResolveMessageFromArguments(arguments));
    }

    public static IEnumerable<object[]> NoMessageCases()
    {
        yield return [new Dictionary<string, object?>()];
        yield return [new Dictionary<string, object?> { ["message"] = new List<object?>() }];
    }

    [Fact]
    public void BuildInvocationUsesDefaultsWhenNoFlagsSet()
    {
        var invocation = CowsayCli.BuildInvocation("hi", new Dictionary<string, object?>());

        Assert.Equal("hi", invocation.Message);
        Assert.Equal("oo", invocation.Eyes);
        Assert.Equal("  ", invocation.Tongue);
        Assert.Equal("default", invocation.CowFile);
        Assert.False(invocation.NoWrap);
        Assert.False(invocation.Think);
        Assert.Equal(40, invocation.Width);
        Assert.Empty(invocation.ActiveModes);
    }

    [Fact]
    public void BuildInvocationHonorsExplicitFlags()
    {
        var flags = new Dictionary<string, object?>
        {
            ["eyes"] = "^^",
            ["tongue"] = "vv",
            ["cowfile"] = "dragon",
            ["nowrap"] = true,
            ["think"] = true,
            ["width"] = 20,
            ["borg"] = true,
        };

        var invocation = CowsayCli.BuildInvocation("hi", flags);

        Assert.Equal("^^", invocation.Eyes);
        Assert.Equal("vv", invocation.Tongue);
        Assert.Equal("dragon", invocation.CowFile);
        Assert.True(invocation.NoWrap);
        Assert.True(invocation.Think);
        Assert.Equal(20, invocation.Width);
        Assert.Equal(["borg"], invocation.ActiveModes);
    }

    [Fact]
    public void BuildInvocationAcceptsLongWidthAndClampsToPositive()
    {
        var flags = new Dictionary<string, object?> { ["width"] = 99_999_999_999L };
        Assert.Equal(int.MaxValue, CowsayCli.BuildInvocation("hi", flags).Width);

        var negativeFlags = new Dictionary<string, object?> { ["width"] = -5L };
        Assert.Equal(1, CowsayCli.BuildInvocation("hi", negativeFlags).Width);
    }

    [Fact]
    public void ListCowFilesReturnsSortedBasenames()
    {
        var tempDir = Directory.CreateTempSubdirectory("cowsay-cli-tests-").FullName;
        try
        {
            File.WriteAllText(Path.Combine(tempDir, "tux.cow"), "");
            File.WriteAllText(Path.Combine(tempDir, "default.cow"), "");
            File.WriteAllText(Path.Combine(tempDir, "dragon.cow"), "");

            Assert.Equal(["default", "dragon", "tux"], CowsayCli.ListCowFiles(tempDir));
        }
        finally
        {
            Directory.Delete(tempDir, recursive: true);
        }
    }
}

public class EndToEndGoldenTests
{
    private static readonly string CowsDir = Path.Combine(
        CowsayRenderer.FindRepoRoot(AppContext.BaseDirectory), "code", "specs", "cows");

    [Fact]
    public void FindRepoRootLocatesTheRealCowsDirectory()
    {
        Assert.True(Directory.Exists(CowsDir), $"expected cows dir at {CowsDir}; BaseDirectory={AppContext.BaseDirectory}");
        Assert.True(File.Exists(Path.Combine(CowsDir, "default.cow")));
    }

    [Fact]
    public void DefaultCowSpeakingHelloWorld()
    {
        var invocation = new CowsayInvocation(
            Message: "Hello, World!",
            Eyes: "oo",
            Tongue: "  ",
            ActiveModes: [],
            NoWrap: false,
            Width: 40,
            Think: false,
            CowFile: "default");

        var output = CowsayRenderer.Render(invocation, CowsDir);

        Assert.Equal(
            " _______________\n" +
            "< Hello, World! >\n" +
            " ---------------\n" +
            "        \\   ^__^\n" +
            "         \\  (oo)\\_______\n" +
            "            (__)\\       )\\/\\\n" +
            "                ||----w |\n" +
            "                ||     ||",
            output);
    }

    [Fact]
    public void BorgModeThinkingWithDefaultCow()
    {
        var invocation = new CowsayInvocation(
            Message: "beep",
            Eyes: "oo",
            Tongue: "  ",
            ActiveModes: ["borg"],
            NoWrap: false,
            Width: 40,
            Think: true,
            CowFile: "default");

        var output = CowsayRenderer.Render(invocation, CowsDir);

        Assert.Equal(
            " ______\n" +
            "( beep )\n" +
            " ------\n" +
            "        o   ^__^\n" +
            "         o  (==)\\_______\n" +
            "            (__)\\       )\\/\\\n" +
            "                ||----w |\n" +
            "                ||     ||",
            output);
    }
}
