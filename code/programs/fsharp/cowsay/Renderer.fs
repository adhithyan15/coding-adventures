// cowsay — routed through paint-vm-ascii (F# port)
//
// This is the second program in the repository that renders through the
// paint-vm-ascii backend (see code/specs/cowsay-paintvm-pipeline.md), after
// the C# pilot. Everything up through composing the bubble+cow text block
// is ordinary string formatting, ported from the reference implementation
// at code/programs/go/cowsay/main.go. The one thing that's different from
// that reference: instead of printing the composed text directly, buildScene
// converts it into a PaintScene of PaintGlyphRun instructions (one glyph
// placement per non-space character, positioned on an 8x16 character grid),
// and PaintVmAscii.renderToAscii turns that scene back into the terminal
// string we print. The round trip must reproduce the same bytes a direct
// print would have produced.
module CodingAdventures.Cowsay.Renderer

open System
open System.IO
open System.Text.RegularExpressions
open CodingAdventures.PaintInstructions
open CodingAdventures.PaintVmAscii

/// paint-vm-ascii's documented default scale factors (P2D02-paint-vm-ascii.md).
[<Literal>]
let ScaleX = 8.0

[<Literal>]
let ScaleY = 16.0

/// The resolved set of inputs needed to render one cowsay invocation, after
/// CLI flags and mode shortcuts have been reconciled into concrete values.
type CowsayInvocation =
    { Message: string
      Eyes: string
      Tongue: string
      ActiveModes: string list
      NoWrap: bool
      Width: int
      Think: bool
      CowFile: string }

/// Splits text into lines no longer than `width`, breaking on word
/// boundaries. A single word longer than the width is kept whole (never
/// split mid-word).
let wrapText (text: string) (width: int) : string list =
    if text.Length <= width then
        [ text ]
    else
        let words = text.Split(' ') |> Array.filter (fun w -> w.Length > 0)

        if words.Length = 0 then
            [ "" ]
        else
            let lines = ResizeArray<string>()
            let mutable current = ""

            for word in words do
                if current.Length + word.Length + 1 <= width then
                    current <- (if current.Length = 0 then word else current + " " + word)
                else
                    if current.Length > 0 then
                        lines.Add current

                    current <- word

            if current.Length > 0 then
                lines.Add current

            lines |> List.ofSeq

/// Draws the speech/thought bubble around the given lines. A single line
/// gets "&lt; ... &gt;" (or "( ... )" for a thought bubble); multiple lines
/// get "/ ... \", "| ... |", "\ ... /" (or "( ... )" on every line for a
/// thought bubble).
let formatBubble (lines: string list) (isThink: bool) : string =
    if List.isEmpty lines then
        ""
    else
        let maxLen = lines |> List.map String.length |> List.max
        let borderTop = " " + String('_', maxLen + 2)
        let borderBottom = " " + String('-', maxLen + 2)
        let padded (s: string) = s.PadRight(maxLen)

        let bodyLines =
            match lines with
            | [ only ] ->
                let startChar, endChar = if isThink then "(", ")" else "<", ">"
                [ sprintf "%s %s %s" startChar (padded only) endChar ]
            | many ->
                let count = List.length many

                many
                |> List.mapi (fun i line ->
                    let startChar, endChar =
                        if isThink then "(", ")"
                        elif i = 0 then "/", "\\"
                        elif i = count - 1 then "\\", "/"
                        else "|", "|"

                    sprintf "%s %s %s" startChar (padded line) endChar)

        String.Join("\n", borderTop :: bodyLines @ [ borderBottom ])

/// Pads or truncates a mode string (eyes/tongue) to exactly two characters,
/// matching cowsay's convention that eyes/tongue are always a 2-char glyph.
let normalizeTwoChars (value: string) : string =
    if value.Length < 2 then (value + "  ").Substring(0, 2)
    elif value.Length > 2 then value.Substring(0, 2)
    else value

let private modeOverrides: Map<string, string * string option> =
    Map.ofList
        [ "borg", ("==", None)
          "dead", ("XX", Some "U ")
          "greedy", ("$$", None)
          "paranoid", ("@@", None)
          "stoned", ("xx", Some "U ")
          "tired", ("--", None)
          "wired", ("OO", None)
          "youthful", ("..", None) ]

/// Applies mode shortcuts (--borg, --dead, etc.) on top of the base
/// eyes/tongue flag values, then normalizes both to two characters. Modes
/// are mutually exclusive per cowsay.json, but this accepts any set for
/// robustness.
let resolveEyesAndTongue (baseEyes: string) (baseTongue: string) (activeModes: string seq) : string * string =
    let mutable eyes = baseEyes
    let mutable tongue = baseTongue

    for mode in activeModes do
        match Map.tryFind mode modeOverrides with
        | Some(modeEyes, modeTongue) ->
            eyes <- modeEyes

            match modeTongue with
            | Some t -> tongue <- t
            | None -> ()
        | None -> ()

    normalizeTwoChars eyes, normalizeTwoChars tongue

let private cowBodyPattern = Regex(@"<<EOC;\n(.*?)EOC", RegexOptions.Singleline)

/// Loads a .cow template's body from `cowsDir`, falling back to
/// default.cow when the requested file doesn't exist. The template is a
/// Perl heredoc (`$the_cow = &lt;&lt;EOC; ... EOC`); only the body between
/// the heredoc markers is returned.
///
/// `cowName` comes from the user-supplied -f/--file flag, so it is treated
/// as untrusted: only a bare filename (no directory separators, no
/// rooted/absolute path) is accepted, and the resolved path is verified to
/// stay inside `cowsDir` before it's read — otherwise this falls back to
/// default.cow instead of reading an arbitrary file the caller pointed at
/// via "..", a rooted override, or similar (mirrors the fix applied to the
/// C# pilot's LoadCow after /security-review flagged it there).
let loadCow (cowName: string) (cowsDir: string) : string =
    let cowsRoot = Path.GetFullPath(cowsDir)
    let safeName = Path.GetFileName(cowName)

    let candidatePath =
        if safeName.Length > 0 && not (Path.IsPathRooted(cowName)) then
            Some(Path.GetFullPath(Path.Combine(cowsRoot, safeName + ".cow")))
        else
            None

    let isWithinCowsDir =
        match candidatePath with
        | Some p -> p.StartsWith(cowsRoot + string Path.DirectorySeparatorChar, StringComparison.Ordinal)
        | None -> false

    let cowPath =
        match candidatePath with
        | Some p when isWithinCowsDir && File.Exists(p) -> p
        | _ -> Path.Combine(cowsRoot, "default.cow")

    let content = File.ReadAllText(cowPath)
    let m = cowBodyPattern.Match(content)
    if m.Success then m.Groups.[1].Value else content

/// Walks up from `startDir` looking for CLAUDE.md, the repo-root sentinel
/// file. CLAUDE.md (not code/specs/cowsay.json itself) is used deliberately
/// — it's a more robust marker than reaching for the very file being
/// located, and this exact fix was called out as a lesson from a prior,
/// reverted cowsay Lua port's CI pathing problems (PR #1535).
let findRepoRoot (startDir: string) : string =
    let rec loop (dir: string) (remaining: int) =
        if remaining <= 0 then
            None
        elif File.Exists(Path.Combine(dir, "CLAUDE.md")) then
            Some dir
        else
            match Directory.GetParent(dir) with
            | null -> None
            | parent -> loop parent.FullName (remaining - 1)

    loop startDir 24 |> Option.defaultValue startDir

/// Composes the full bubble+cow text block for one invocation — everything
/// up to (but not including) the paint-vm-ascii render step.
let composeContent (invocation: CowsayInvocation) (cowsDir: string) : string =
    let eyes, tongue =
        resolveEyesAndTongue invocation.Eyes invocation.Tongue invocation.ActiveModes

    let lines =
        invocation.Message.Split('\n')
        |> Array.collect (fun rawLine ->
            if rawLine.Length = 0 then [| "" |]
            elif invocation.NoWrap then [| rawLine |]
            else wrapText rawLine invocation.Width |> Array.ofList)
        |> List.ofArray

    let thoughts = if invocation.Think then "o" else "\\"
    let bubble = formatBubble lines invocation.Think

    let cowTemplate = loadCow invocation.CowFile cowsDir

    let cow =
        cowTemplate
            .Replace("$eyes", eyes)
            .Replace("$tongue", tongue)
            .Replace("$thoughts", thoughts)
            .Replace("\\\\", "\\")

    bubble + "\n" + cow

/// Converts a composed text block into a PaintScene: one PaintGlyphRun per
/// line, one PaintGlyphPlacement per non-space character. See
/// code/specs/cowsay-paintvm-pipeline.md §3 for the full contract, including
/// why glyph_id is a literal Unicode code point here (an ASCII-backend-only
/// relaxation of the general PaintGlyphRun contract).
let buildScene (text: string) : PaintScene =
    let lines = text.Replace("\r\n", "\n").Split('\n')
    let mutable maxWidth = 0
    let instructions = ResizeArray<PaintInstruction>()

    for row in 0 .. lines.Length - 1 do
        let line = lines.[row]

        if line.Length > maxWidth then
            maxWidth <- line.Length

        let placements = ResizeArray<PaintGlyphPlacement>()

        for col in 0 .. line.Length - 1 do
            let ch = line.[col]

            if ch <> ' ' then
                placements.Add(
                    { GlyphId = int ch
                      X = float col * ScaleX
                      Y = float row * ScaleY }
                )

        if placements.Count > 0 then
            let glyphRun: PaintGlyphRun =
                { Base = { Id = None; Metadata = None }
                  Glyphs = placements |> List.ofSeq
                  FontRef = "terminal-mono"
                  FontSize = ScaleY
                  Fill = Some "#000000" }

            instructions.Add(GlyphRun glyphRun)

    let width = float (max 1 maxWidth) * ScaleX
    let height = float (max 1 lines.Length) * ScaleY

    { Width = width
      Height = height
      Background = "transparent"
      Instructions = instructions |> List.ofSeq
      Id = None
      Metadata = None }

/// End-to-end: compose the bubble+cow text, build a PaintScene from it, and
/// render that scene through paint-vm-ascii.
let render (invocation: CowsayInvocation) (cowsDir: string) : string =
    let content = composeContent invocation cowsDir
    let scene = buildScene content
    PaintVmAscii.renderToAscii scene
