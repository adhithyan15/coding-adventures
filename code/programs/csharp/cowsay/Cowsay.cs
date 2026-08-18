// cowsay — routed through paint-vm-ascii
//
// This is the first program in the repository that renders through the
// paint-vm-ascii backend (see code/specs/cowsay-paintvm-pipeline.md for the
// full design rationale). Everything up through composing the bubble+cow
// text block is ordinary string formatting, ported from the reference
// implementation at code/programs/go/cowsay/main.go. The one thing that's
// different from that reference: instead of printing the composed text
// directly, BuildScene converts it into a PaintScene of PaintGlyphRun
// instructions (one glyph placement per non-space character, positioned on
// an 8x16 character grid), and PaintVmAscii.RenderToAscii turns that scene
// back into the terminal string we print. The round trip must reproduce the
// same bytes a direct print would have produced.
using System.Text.RegularExpressions;
using CodingAdventures.PaintInstructions;
using CodingAdventures.PaintVmAscii;

namespace CodingAdventures.Cowsay;

/// <summary>
/// The resolved set of inputs needed to render one cowsay invocation, after
/// CLI flags and mode shortcuts have been reconciled into concrete values.
/// </summary>
public sealed record CowsayInvocation(
    string Message,
    string Eyes,
    string Tongue,
    IReadOnlyList<string> ActiveModes,
    bool NoWrap,
    int Width,
    bool Think,
    string CowFile);

public static class CowsayRenderer
{
    // paint-vm-ascii's documented default scale factors (P2D02-paint-vm-ascii.md).
    public const double ScaleX = 8.0;
    public const double ScaleY = 16.0;

    private static readonly IReadOnlyDictionary<string, (string Eyes, string? Tongue)> ModeOverrides =
        new Dictionary<string, (string, string?)>(StringComparer.Ordinal)
        {
            ["borg"] = ("==", null),
            ["dead"] = ("XX", "U "),
            ["greedy"] = ("$$", null),
            ["paranoid"] = ("@@", null),
            ["stoned"] = ("xx", "U "),
            ["tired"] = ("--", null),
            ["wired"] = ("OO", null),
            ["youthful"] = ("..", null),
        };

    /// <summary>
    /// Splits text into lines no longer than <paramref name="width"/>, breaking on word
    /// boundaries. A single word longer than the width is kept whole (never split mid-word).
    /// </summary>
    public static IReadOnlyList<string> WrapText(string text, int width)
    {
        if (text.Length <= width)
        {
            return [text];
        }

        var words = text.Split(' ', StringSplitOptions.RemoveEmptyEntries);
        if (words.Length == 0)
        {
            return [""];
        }

        var lines = new List<string>();
        var current = "";
        foreach (var word in words)
        {
            if (current.Length + word.Length + 1 <= width)
            {
                current = current.Length == 0 ? word : $"{current} {word}";
            }
            else
            {
                if (current.Length > 0)
                {
                    lines.Add(current);
                }

                current = word;
            }
        }

        if (current.Length > 0)
        {
            lines.Add(current);
        }

        return lines;
    }

    /// <summary>
    /// Draws the speech/thought bubble around the given lines. A single line gets
    /// "&lt; ... &gt;" (or "( ... )" for a thought bubble); multiple lines get
    /// "/ ... \", "| ... |", "\ ... /" (or "( ... )" on every line for a thought bubble).
    /// </summary>
    public static string FormatBubble(IReadOnlyList<string> lines, bool isThink)
    {
        if (lines.Count == 0)
        {
            return "";
        }

        var maxLen = lines.Max(line => line.Length);
        var borderTop = " " + new string('_', maxLen + 2);
        var borderBottom = " " + new string('-', maxLen + 2);

        var result = new List<string> { borderTop };

        if (lines.Count == 1)
        {
            var (start, end) = isThink ? ("(", ")") : ("<", ">");
            result.Add($"{start} {lines[0].PadRight(maxLen)} {end}");
        }
        else
        {
            for (var i = 0; i < lines.Count; i++)
            {
                string start, end;
                if (isThink)
                {
                    (start, end) = ("(", ")");
                }
                else if (i == 0)
                {
                    (start, end) = ("/", "\\");
                }
                else if (i == lines.Count - 1)
                {
                    (start, end) = ("\\", "/");
                }
                else
                {
                    (start, end) = ("|", "|");
                }

                result.Add($"{start} {lines[i].PadRight(maxLen)} {end}");
            }
        }

        result.Add(borderBottom);
        return string.Join("\n", result);
    }

    /// <summary>
    /// Pads or truncates a mode string (eyes/tongue) to exactly two characters,
    /// matching cowsay's convention that eyes/tongue are always a 2-char glyph.
    /// </summary>
    public static string NormalizeTwoChars(string value)
    {
        if (value.Length < 2)
        {
            return (value + "  ")[..2];
        }

        return value.Length > 2 ? value[..2] : value;
    }

    /// <summary>
    /// Applies mode shortcuts (--borg, --dead, etc.) on top of the base eyes/tongue
    /// flag values, then normalizes both to two characters. Modes are mutually
    /// exclusive per cowsay.json, but this accepts any set for robustness.
    /// </summary>
    public static (string Eyes, string Tongue) ResolveEyesAndTongue(
        string baseEyes, string baseTongue, IEnumerable<string> activeModes)
    {
        var eyes = baseEyes;
        var tongue = baseTongue;

        foreach (var mode in activeModes)
        {
            if (!ModeOverrides.TryGetValue(mode, out var overrideValue))
            {
                continue;
            }

            eyes = overrideValue.Eyes;
            if (overrideValue.Tongue is not null)
            {
                tongue = overrideValue.Tongue;
            }
        }

        return (NormalizeTwoChars(eyes), NormalizeTwoChars(tongue));
    }

    /// <summary>
    /// Walks up from <paramref name="startDir"/> looking for CLAUDE.md, the repo-root
    /// sentinel file. CLAUDE.md (not code/specs/cowsay.json itself) is used deliberately —
    /// it's a more robust marker than reaching for the very file being located, and this
    /// exact fix was called out as a lesson from a prior, reverted cowsay Lua port's CI
    /// pathing problems (PR #1535).
    /// </summary>
    public static string FindRepoRoot(string startDir)
    {
        var dir = startDir;
        for (var i = 0; i < 24; i++)
        {
            if (File.Exists(Path.Combine(dir, "CLAUDE.md")))
            {
                return dir;
            }

            var parent = Directory.GetParent(dir);
            if (parent is null)
            {
                break;
            }

            dir = parent.FullName;
        }

        return startDir;
    }

    private static readonly Regex CowBodyPattern = new(@"<<EOC;\n(.*?)EOC", RegexOptions.Singleline);

    /// <summary>
    /// Loads a .cow template's body from <paramref name="cowsDir"/>, falling back to
    /// default.cow when the requested file doesn't exist. The template is a Perl
    /// heredoc (`$the_cow = &lt;&lt;EOC; ... EOC`); only the body between the heredoc
    /// markers is returned.
    ///
    /// <paramref name="cowName"/> comes from the user-supplied -f/--file flag, so it
    /// is treated as untrusted: only a bare filename (no directory separators, no
    /// rooted/absolute path) is accepted, and the resolved path is verified to stay
    /// inside <paramref name="cowsDir"/> before it's read — otherwise this falls back
    /// to default.cow instead of reading an arbitrary file the caller pointed at via
    /// "..", a rooted override, or similar. A rooted `cowName` matters because
    /// Path.Combine ignores its first argument entirely when the second is already
    /// rooted (e.g. "C:\Windows\win" + ".cow"), which would otherwise let a crafted
    /// -f value escape cowsDir outright, not just traverse out of it.
    /// </summary>
    public static string LoadCow(string cowName, string cowsDir)
    {
        var cowsRoot = Path.GetFullPath(cowsDir);
        var safeName = Path.GetFileName(cowName);
        var cowPath = safeName.Length > 0 && !Path.IsPathRooted(cowName)
            ? Path.GetFullPath(Path.Combine(cowsRoot, safeName + ".cow"))
            : null;

        var isWithinCowsDir = cowPath is not null &&
            cowPath.StartsWith(cowsRoot + Path.DirectorySeparatorChar, StringComparison.Ordinal);

        if (cowPath is null || !isWithinCowsDir || !File.Exists(cowPath))
        {
            cowPath = Path.Combine(cowsRoot, "default.cow");
        }

        var content = File.ReadAllText(cowPath);
        var match = CowBodyPattern.Match(content);
        return match.Success ? match.Groups[1].Value : content;
    }

    /// <summary>
    /// Composes the full bubble+cow text block for one invocation — everything up
    /// to (but not including) the paint-vm-ascii render step.
    /// </summary>
    public static string ComposeContent(CowsayInvocation invocation, string cowsDir)
    {
        var (eyes, tongue) = ResolveEyesAndTongue(invocation.Eyes, invocation.Tongue, invocation.ActiveModes);

        var lines = new List<string>();
        foreach (var rawLine in invocation.Message.Split('\n'))
        {
            if (rawLine.Length == 0)
            {
                lines.Add("");
            }
            else if (invocation.NoWrap)
            {
                lines.Add(rawLine);
            }
            else
            {
                lines.AddRange(WrapText(rawLine, invocation.Width));
            }
        }

        var thoughts = invocation.Think ? "o" : "\\";
        var bubble = FormatBubble(lines, invocation.Think);

        var cowTemplate = LoadCow(invocation.CowFile, cowsDir);
        var cow = cowTemplate
            .Replace("$eyes", eyes)
            .Replace("$tongue", tongue)
            .Replace("$thoughts", thoughts)
            .Replace("\\\\", "\\");

        return bubble + "\n" + cow;
    }

    /// <summary>
    /// Converts a composed text block into a PaintScene: one PaintGlyphRun per
    /// line, one PaintGlyphPlacement per non-space character. See
    /// code/specs/cowsay-paintvm-pipeline.md §3 for the full contract, including
    /// why glyph_id is a literal Unicode code point here (an ASCII-backend-only
    /// relaxation of the general PaintGlyphRun contract).
    /// </summary>
    public static PaintScene BuildScene(string text)
    {
        var lines = text.Replace("\r\n", "\n").Split('\n');
        var instructions = new List<PaintInstructionBase>();
        var maxWidth = 0;

        for (var row = 0; row < lines.Length; row++)
        {
            var line = lines[row];
            if (line.Length > maxWidth)
            {
                maxWidth = line.Length;
            }

            var placements = new List<PaintGlyphPlacement>();
            for (var col = 0; col < line.Length; col++)
            {
                var ch = line[col];
                if (ch == ' ')
                {
                    continue;
                }

                placements.Add(new PaintGlyphPlacement(ch, col * ScaleX, row * ScaleY));
            }

            if (placements.Count > 0)
            {
                instructions.Add(new PaintGlyphRun(placements, "terminal-mono", ScaleY) { Fill = "#000000" });
            }
        }

        var width = Math.Max(1, maxWidth) * ScaleX;
        var height = Math.Max(1, lines.Length) * ScaleY;
        return new PaintScene(width, height, "transparent", instructions);
    }

    /// <summary>
    /// End-to-end: compose the bubble+cow text, build a PaintScene from it, and
    /// render that scene through paint-vm-ascii.
    /// </summary>
    public static string Render(CowsayInvocation invocation, string cowsDir)
    {
        var content = ComposeContent(invocation, cowsDir);
        var scene = BuildScene(content);
        return CodingAdventures.PaintVmAscii.PaintVmAscii.RenderToAscii(scene, new AsciiOptions { ScaleX = ScaleX, ScaleY = ScaleY });
    }
}

/// <summary>
/// The CLI-facing glue between a CliBuilder ParseResult's flags/arguments dictionaries
/// and CowsayRenderer's typed inputs. Pulled out of Program.cs so it's directly
/// unit-testable without spawning a process or driving a real Parser.
/// </summary>
public static class CowsayCli
{
    private static readonly string[] ModeFlagIds =
        ["borg", "dead", "greedy", "paranoid", "stoned", "tired", "wired", "youthful"];

    public static bool IsListRequested(IReadOnlyDictionary<string, object?> flags) =>
        flags.TryGetValue("list", out var listFlag) && listFlag is true;

    /// <summary>Cow file basenames under <paramref name="cowsDir"/>, sorted ordinally.</summary>
    public static IReadOnlyList<string> ListCowFiles(string cowsDir) =>
        Directory.EnumerateFiles(cowsDir, "*.cow")
            .Select(Path.GetFileNameWithoutExtension)
            .Where(name => name is not null)
            .Select(name => name!)
            .OrderBy(name => name, StringComparer.Ordinal)
            .ToArray();

    /// <summary>
    /// Resolves the message from the parsed "message" positional argument. Returns
    /// null when no message was given on argv — the caller should fall back to stdin.
    /// </summary>
    public static string? ResolveMessageFromArguments(IReadOnlyDictionary<string, object?> arguments)
    {
        if (arguments.TryGetValue("message", out var messageValue) &&
            messageValue is List<object?> { Count: > 0 } messageParts)
        {
            return string.Join(" ", messageParts.Select(part => part?.ToString() ?? ""));
        }

        return null;
    }

    /// <summary>
    /// Builds a CowsayInvocation from a resolved message and the parsed flags
    /// dictionary, applying cowsay.json's documented defaults for any flag that
    /// wasn't explicitly set.
    /// </summary>
    public static CowsayInvocation BuildInvocation(string message, IReadOnlyDictionary<string, object?> flags)
    {
        var eyes = flags.TryGetValue("eyes", out var eyesValue) && eyesValue is string eyesString ? eyesString : "oo";
        var tongue = flags.TryGetValue("tongue", out var tongueValue) && tongueValue is string tongueString ? tongueString : "  ";
        var cowFile = flags.TryGetValue("cowfile", out var cowFileValue) && cowFileValue is string cowFileString ? cowFileString : "default";
        var noWrap = flags.TryGetValue("nowrap", out var noWrapValue) && noWrapValue is true;
        var think = flags.TryGetValue("think", out var thinkValue) && thinkValue is true;

        var width = 40;
        if (flags.TryGetValue("width", out var widthValue))
        {
            width = widthValue switch
            {
                long longWidth => (int)Math.Clamp(longWidth, 1, int.MaxValue),
                int intWidth => Math.Max(1, intWidth),
                _ => width,
            };
        }

        var activeModes = ModeFlagIds
            .Where(mode => flags.TryGetValue(mode, out var modeValue) && modeValue is true)
            .ToArray();

        return new CowsayInvocation(message, eyes, tongue, activeModes, noWrap, width, think, cowFile);
    }
}
