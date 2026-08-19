namespace CodingAdventures.Cowsay.Tests

open System
open System.Collections.Generic
open System.IO
open Xunit
open CodingAdventures.CliBuilder.FSharp
open CodingAdventures.PaintInstructions
open CodingAdventures.PaintVmAscii
open CodingAdventures.Cowsay.Renderer
open CodingAdventures.Cowsay.Cli

module private Helpers =
    /// dict [...] returns an IDictionary, not the IReadOnlyDictionary CowsayCli's
    /// functions expect -- wrap it in a concrete Dictionary, which implements both.
    let flagsOf (pairs: (string * obj) list) : IReadOnlyDictionary<string, obj> =
        Dictionary<string, obj>(dict pairs) :> IReadOnlyDictionary<string, obj>

open Helpers

type WrapTextTests() =
    [<Fact>]
    member _.``Short text is not wrapped``() =
        Assert.Equal<string list>([ "hello" ], wrapText "hello" 40)

    [<Fact>]
    member _.``Long text wraps at word boundaries``() =
        let result = wrapText "the quick brown fox jumps over" 10
        Assert.Equal<string list>([ "the quick"; "brown fox"; "jumps over" ], result)

    [<Fact>]
    member _.``Empty text returns empty line``() =
        Assert.Equal<string list>([ "" ], wrapText "" 40)

    [<Fact>]
    member _.``Single word longer than width stays whole``() =
        let result = wrapText "supercalifragilisticexpialidocious" 5
        Assert.Equal<string list>([ "supercalifragilisticexpialidocious" ], result)

    [<Fact>]
    member _.``Whitespace only text returns empty line``() =
        Assert.Equal<string list>([ "" ], wrapText "     " 3)

type FormatBubbleTests() =
    [<Fact>]
    member _.``Empty lines returns empty string``() =
        Assert.Equal("", formatBubble [] false)

    [<Fact>]
    member _.``Single line speech bubble``() =
        Assert.Equal(" ____\n< hi >\n ----", formatBubble [ "hi" ] false)

    [<Fact>]
    member _.``Single line thought bubble``() =
        Assert.Equal(" ____\n( hi )\n ----", formatBubble [ "hi" ] true)

    [<Fact>]
    member _.``Multi line speech bubble uses slash pipe backslash borders``() =
        let result = formatBubble [ "one"; "two"; "three" ] false

        Assert.Equal(" _______\n" + "/ one   \\\n" + "| two   |\n" + "\\ three /\n" + " -------", result)

    [<Fact>]
    member _.``Multi line thought bubble uses parens on every line``() =
        let result = formatBubble [ "one"; "two" ] true

        Assert.Equal(" _____\n" + "( one )\n" + "( two )\n" + " -----", result)

type NormalizeTwoCharsTests() =
    [<Theory>]
    [<InlineData("o", "o ")>]
    [<InlineData("", "  ")>]
    [<InlineData("oo", "oo")>]
    [<InlineData("ooo", "oo")>]
    member _.``Normalizes to exactly two characters``(input: string, expected: string) =
        Assert.Equal(expected, normalizeTwoChars input)

type ResolveEyesAndTongueTests() =
    [<Fact>]
    member _.``No active modes keeps base values``() =
        let eyes, tongue = resolveEyesAndTongue "oo" "  " []
        Assert.Equal("oo", eyes)
        Assert.Equal("  ", tongue)

    [<Theory>]
    [<InlineData("borg", "==", "  ")>]
    [<InlineData("dead", "XX", "U ")>]
    [<InlineData("greedy", "$$", "  ")>]
    [<InlineData("paranoid", "@@", "  ")>]
    [<InlineData("stoned", "xx", "U ")>]
    [<InlineData("tired", "--", "  ")>]
    [<InlineData("wired", "OO", "  ")>]
    [<InlineData("youthful", "..", "  ")>]
    member _.``Each mode overrides eyes and sometimes tongue``(mode: string, expectedEyes: string, expectedTongue: string) =
        let eyes, tongue = resolveEyesAndTongue "oo" "  " [ mode ]
        Assert.Equal(expectedEyes, eyes)
        Assert.Equal(expectedTongue, tongue)

    [<Fact>]
    member _.``Unknown mode is ignored``() =
        let eyes, tongue = resolveEyesAndTongue "oo" "  " [ "not-a-real-mode" ]
        Assert.Equal("oo", eyes)
        Assert.Equal("  ", tongue)

type LoadCowTests() =
    let tempDir = Directory.CreateTempSubdirectory("cowsay-tests-").FullName
    interface IDisposable with
        member _.Dispose() = Directory.Delete(tempDir, true)

    [<Fact>]
    member _.``Loads body between heredoc markers``() =
        File.WriteAllText(Path.Combine(tempDir, "default.cow"), "$the_cow = <<EOC;\n  $thoughts   ^__^\n   ($eyes)\nEOC\n")

        let body = loadCow "default" tempDir

        Assert.Equal("  $thoughts   ^__^\n   ($eyes)\n", body)

    [<Fact>]
    member _.``Falls back to default when named cow is missing``() =
        File.WriteAllText(Path.Combine(tempDir, "default.cow"), "$the_cow = <<EOC;\nfallback\nEOC\n")

        let body = loadCow "does-not-exist" tempDir

        Assert.Equal("fallback\n", body)

    [<Theory>]
    [<InlineData("../../../../../../etc/passwd")>]
    [<InlineData("..\\..\\..\\secret")>]
    [<InlineData("../outside")>]
    member _.``Falls back to default instead of escaping cowsDir via traversal``(maliciousCowName: string) =
        File.WriteAllText(Path.Combine(tempDir, "default.cow"), "$the_cow = <<EOC;\nfallback\nEOC\n")
        let outsideDir = Directory.CreateTempSubdirectory("cowsay-outside-").FullName

        try
            File.WriteAllText(Path.Combine(outsideDir, "secret.cow"), "$the_cow = <<EOC;\nSECRET\nEOC\n")
            File.WriteAllText(Path.Combine(outsideDir, "outside.cow"), "$the_cow = <<EOC;\nSECRET\nEOC\n")

            let body = loadCow maliciousCowName tempDir

            Assert.Equal("fallback\n", body)
        finally
            Directory.Delete(outsideDir, true)

    [<Fact>]
    member _.``Falls back to default instead of following a rooted path override``() =
        File.WriteAllText(Path.Combine(tempDir, "default.cow"), "$the_cow = <<EOC;\nfallback\nEOC\n")
        let outsideDir = Directory.CreateTempSubdirectory("cowsay-outside-").FullName

        try
            let rootedTarget = Path.Combine(outsideDir, "win")
            File.WriteAllText(rootedTarget + ".cow", "$the_cow = <<EOC;\nSECRET\nEOC\n")

            let body = loadCow rootedTarget tempDir

            Assert.Equal("fallback\n", body)
        finally
            Directory.Delete(outsideDir, true)

type ComposeContentTests() =
    let tempDir = Directory.CreateTempSubdirectory("cowsay-tests-").FullName
    do File.WriteAllText(Path.Combine(tempDir, "default.cow"), "$the_cow = <<EOC;\n$thoughts $eyes $tongue\nEOC\n")

    interface IDisposable with
        member _.Dispose() = Directory.Delete(tempDir, true)

    [<Fact>]
    member _.``Composes bubble and cow with substitutions``() =
        let invocation =
            { Message = "hi"
              Eyes = "oo"
              Tongue = "  "
              ActiveModes = []
              NoWrap = false
              Width = 40
              Think = false
              CowFile = "default" }

        let content = composeContent invocation tempDir

        Assert.Equal(" ____\n< hi >\n ----\n\\ oo   \n", content)

    [<Fact>]
    member _.``Think mode uses o for thoughts and paren bubble``() =
        let invocation =
            { Message = "hi"
              Eyes = "oo"
              Tongue = "  "
              ActiveModes = []
              NoWrap = false
              Width = 40
              Think = true
              CowFile = "default" }

        let content = composeContent invocation tempDir

        Assert.Equal(" ____\n( hi )\n ----\no oo   \n", content)

    [<Fact>]
    member _.``Mode flag overrides eyes in cow template``() =
        let invocation =
            { Message = "hi"
              Eyes = "oo"
              Tongue = "  "
              ActiveModes = [ "dead" ]
              NoWrap = false
              Width = 40
              Think = false
              CowFile = "default" }

        let content = composeContent invocation tempDir

        Assert.Equal(" ____\n< hi >\n ----\n\\ XX U \n", content)

type BuildSceneTests() =
    [<Fact>]
    member _.``One glyph run per non blank line with correct placements``() =
        let scene = buildScene "hi\n\nyo"

        let glyphRuns =
            scene.Instructions
            |> List.choose (function
                | GlyphRun run -> Some run
                | _ -> None)

        Assert.Equal(2, glyphRuns.Length)

        Assert.Equal(2, glyphRuns.[0].Glyphs.Length)
        Assert.Equal({ GlyphId = int 'h'; X = 0.0; Y = 0.0 }, glyphRuns.[0].Glyphs.[0])
        Assert.Equal({ GlyphId = int 'i'; X = ScaleX; Y = 0.0 }, glyphRuns.[0].Glyphs.[1])

        Assert.Equal(2, glyphRuns.[1].Glyphs.Length)
        Assert.Equal({ GlyphId = int 'y'; X = 0.0; Y = 2.0 * ScaleY }, glyphRuns.[1].Glyphs.[0])
        Assert.Equal({ GlyphId = int 'o'; X = ScaleX; Y = 2.0 * ScaleY }, glyphRuns.[1].Glyphs.[1])

    [<Fact>]
    member _.``Spaces are skipped not placed``() =
        let scene = buildScene "a b"

        let glyphRuns =
            scene.Instructions
            |> List.choose (function
                | GlyphRun run -> Some run
                | _ -> None)

        let glyphRun = Assert.Single(glyphRuns)
        Assert.Equal(2, glyphRun.Glyphs.Length)
        Assert.Equal('a', char glyphRun.Glyphs.[0].GlyphId)
        Assert.Equal('b', char glyphRun.Glyphs.[1].GlyphId)

    [<Fact>]
    member _.``Scene dimensions cover all lines``() =
        let scene = buildScene "abc\nde"

        Assert.Equal(3.0 * ScaleX, scene.Width)
        Assert.Equal(2.0 * ScaleY, scene.Height)

type RenderRoundTripTests() =
    [<Theory>]
    [<InlineData("hi")>]
    [<InlineData("hello\nworld")>]
    [<InlineData(" ____\n< hi >\n ----\n\\   ^__^\n")>]
    member _.``Rendering through paint-vm-ascii reproduces the original text``(content: string) =
        let scene = buildScene content
        let output = PaintVmAscii.renderToAscii scene

        let expected =
            content.Split('\n')
            |> Array.map (fun line -> line.TrimEnd())
            |> String.concat "\n"
            |> fun s -> s.TrimEnd('\n')

        Assert.Equal(expected, output)

type CowsayCliTests() =
    [<Fact>]
    member _.``IsListRequested true when flag present``() =
        let flags = flagsOf [ "list", box true ]
        Assert.True(isListRequested flags)

    [<Fact>]
    member _.``IsListRequested false when flag absent``() =
        let flags = flagsOf []
        Assert.False(isListRequested flags)

    [<Fact>]
    member _.``IsListRequested false when flag explicitly false``() =
        let flags = flagsOf [ "list", box false ]
        Assert.False(isListRequested flags)

    [<Fact>]
    member _.``ResolveMessage joins positional words``() =
        let items = ResizeArray<obj>([ box "hello"; box "there" ])
        let arguments = flagsOf [ "message", box items ]

        Assert.Equal(Some "hello there", resolveMessageFromArguments arguments)

    [<Fact>]
    member _.``ResolveMessage returns none when arguments dictionary is empty``() =
        let arguments = flagsOf []
        Assert.Equal(None, resolveMessageFromArguments arguments)

    [<Fact>]
    member _.``ResolveMessage returns none when message list is empty``() =
        let arguments = flagsOf [ "message", box (ResizeArray<obj>()) ]
        Assert.Equal(None, resolveMessageFromArguments arguments)

    [<Fact>]
    member _.``BuildInvocation uses defaults when no flags set``() =
        let invocation = buildInvocation "hi" (flagsOf [])

        Assert.Equal("hi", invocation.Message)
        Assert.Equal("oo", invocation.Eyes)
        Assert.Equal("  ", invocation.Tongue)
        Assert.Equal("default", invocation.CowFile)
        Assert.False(invocation.NoWrap)
        Assert.False(invocation.Think)
        Assert.Equal(40, invocation.Width)
        Assert.Empty(invocation.ActiveModes)

    [<Fact>]
    member _.``BuildInvocation honors explicit flags``() =
        let flags =
            flagsOf
                [ "eyes", box "^^"
                  "tongue", box "vv"
                  "cowfile", box "dragon"
                  "nowrap", box true
                  "think", box true
                  "width", box 20
                  "borg", box true ]

        let invocation = buildInvocation "hi" flags

        Assert.Equal("^^", invocation.Eyes)
        Assert.Equal("vv", invocation.Tongue)
        Assert.Equal("dragon", invocation.CowFile)
        Assert.True(invocation.NoWrap)
        Assert.True(invocation.Think)
        Assert.Equal(20, invocation.Width)
        Assert.Equal<string list>([ "borg" ], invocation.ActiveModes)

    [<Fact>]
    member _.``BuildInvocation accepts long width and clamps to positive``() =
        let flags = flagsOf [ "width", box 99_999_999_999L ]
        Assert.Equal(Int32.MaxValue, (buildInvocation "hi" flags).Width)

        let negativeFlags = flagsOf [ "width", box -5L ]
        Assert.Equal(1, (buildInvocation "hi" negativeFlags).Width)

    [<Fact>]
    member _.``ListCowFiles returns sorted basenames``() =
        let tempDir = Directory.CreateTempSubdirectory("cowsay-cli-tests-").FullName

        try
            File.WriteAllText(Path.Combine(tempDir, "tux.cow"), "")
            File.WriteAllText(Path.Combine(tempDir, "default.cow"), "")
            File.WriteAllText(Path.Combine(tempDir, "dragon.cow"), "")

            Assert.Equal<string list>([ "default"; "dragon"; "tux" ], listCowFiles tempDir)
        finally
            Directory.Delete(tempDir, true)

/// Regression test for a real bug the C# pilot's manual verification caught: CliBuilder's
/// Parser follows the C/Go argv convention (index 0 = program name, real tokens start at
/// index 1). Neither C# nor F# `argv`/`args` include the program name, so the caller must
/// prepend a placeholder before constructing Parser -- otherwise the first real CLI token
/// is silently dropped. See the "C#" entry in lessons.md. This test drives the actual
/// Parser end-to-end (unlike the CowsayCli tests, which hand-build the flags/arguments
/// dictionaries and never touch Parser at all), because that's exactly the gap that let
/// the bug through unit tests the first time in the C# port.
type ProgramArgvConventionTests() =
    static let specPath =
        Path.Combine(findRepoRoot AppContext.BaseDirectory, "code", "specs", "cowsay.json")

    [<Fact>]
    member _.``Single word message is not dropped when program name is prepended``() =
        let argv = ResizeArray<string>([ "cowsay"; "hello" ])
        let result = Assert.IsType<ParseResult>(Parser(specPath, argv).Parse())

        let message = resolveMessageFromArguments result.Arguments

        Assert.Equal(Some "hello", message)

    [<Fact>]
    member _.``Multi word message keeps its first word when program name is prepended``() =
        let argv = ResizeArray<string>([ "cowsay"; "hello"; "world" ])
        let result = Assert.IsType<ParseResult>(Parser(specPath, argv).Parse())

        let message = resolveMessageFromArguments result.Arguments

        Assert.Equal(Some "hello world", message)

type EndToEndGoldenTests() =
    static let cowsDir =
        Path.Combine(findRepoRoot AppContext.BaseDirectory, "code", "specs", "cows")

    [<Fact>]
    member _.``FindRepoRoot locates the real cows directory``() =
        Assert.True(Directory.Exists(cowsDir), sprintf "expected cows dir at %s" cowsDir)
        Assert.True(File.Exists(Path.Combine(cowsDir, "default.cow")))

    [<Fact>]
    member _.``Default cow speaking hello world``() =
        let invocation =
            { Message = "Hello, World!"
              Eyes = "oo"
              Tongue = "  "
              ActiveModes = []
              NoWrap = false
              Width = 40
              Think = false
              CowFile = "default" }

        let output = render invocation cowsDir

        Assert.Equal(
            " _______________\n"
            + "< Hello, World! >\n"
            + " ---------------\n"
            + "        \\   ^__^\n"
            + "         \\  (oo)\\_______\n"
            + "            (__)\\       )\\/\\\n"
            + "                ||----w |\n"
            + "                ||     ||",
            output
        )

    [<Fact>]
    member _.``Borg mode thinking with default cow``() =
        let invocation =
            { Message = "beep"
              Eyes = "oo"
              Tongue = "  "
              ActiveModes = [ "borg" ]
              NoWrap = false
              Width = 40
              Think = true
              CowFile = "default" }

        let output = render invocation cowsDir

        Assert.Equal(
            " ______\n"
            + "( beep )\n"
            + " ------\n"
            + "        o   ^__^\n"
            + "         o  (==)\\_______\n"
            + "            (__)\\       )\\/\\\n"
            + "                ||----w |\n"
            + "                ||     ||",
            output
        )
