// cowsay (F#) — entry point
//
// Thin CLI wiring: parse argv against code/specs/cowsay.json via CliBuilder,
// resolve the parsed flags/arguments into a CowsayInvocation, and hand off
// to Renderer.render for the actual formatting + paint-vm-ascii render. See
// code/specs/cowsay-paintvm-pipeline.md for the design.
module CodingAdventures.Cowsay.Program

open System
open System.IO
open CodingAdventures.CliBuilder.FSharp
open CodingAdventures.Cowsay.Renderer
open CodingAdventures.Cowsay.Cli

let private run (result: ParseResult) (cowsDir: string) =
    let flags = result.Flags
    let arguments = result.Arguments

    if isListRequested flags then
        for name in listCowFiles cowsDir do
            printfn "%s" name
    else
        let message =
            match resolveMessageFromArguments arguments with
            | Some m -> Some m
            | None -> if Console.IsInputRedirected then Some(Console.In.ReadToEnd().Trim()) else None

        match message with
        | Some m when m.Length > 0 ->
            let invocation = buildInvocation m flags
            printfn "%s" (render invocation cowsDir)
        | _ -> ()

[<EntryPoint>]
let main argv =
    let repoRoot = findRepoRoot (Directory.GetCurrentDirectory())
    let specPath = Path.Combine(repoRoot, "code", "specs", "cowsay.json")
    let cowsDir = Path.Combine(repoRoot, "code", "specs", "cows")

    // CliBuilder.Parser follows the C/Go argv convention where index 0 is
    // the program name (Parser.Parse() iterates from index 1). F#'s `argv`
    // here does NOT include the program name -- passing it straight
    // through would silently drop the first real CLI token, exactly the
    // bug found and fixed in the C# pilot (see lessons.md, "C#" section).
    let fullArgv = ResizeArray<string>()
    fullArgv.Add("cowsay")
    fullArgv.AddRange(argv)

    try
        let parser = Parser(specPath, fullArgv)

        match parser.Parse() with
        | :? HelpResult as help -> printfn "%s" help.Text
        | :? VersionResult as version -> printfn "%s" version.Version
        | :? ParseResult as parseResult -> run parseResult cowsDir
        | _ -> ()

        0
    with :? CliBuilderError as error ->
        eprintfn "%s" error.Message
        1
