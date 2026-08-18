// cowsay (C#) — entry point
//
// Thin CLI wiring: parse argv against code/specs/cowsay.json via CliBuilder,
// resolve the parsed flags/arguments into a CowsayInvocation, and hand off to
// CowsayRenderer (Cowsay.cs) for the actual formatting + paint-vm-ascii
// render. See code/specs/cowsay-paintvm-pipeline.md for the design.
using CodingAdventures.CliBuilder;
using CodingAdventures.Cowsay;

var repoRoot = CowsayRenderer.FindRepoRoot(Directory.GetCurrentDirectory());
var specPath = Path.Combine(repoRoot, "code", "specs", "cowsay.json");
var cowsDir = Path.Combine(repoRoot, "code", "specs", "cows");

// CliBuilder's Parser follows the C/Go argv convention, where index 0 is the
// program name and real arguments start at index 1 (see Parser.Parse(),
// "for (var index = 1; index < _argv.Count; index++)"). C#'s top-level
// `args` does NOT include the program name, so it must be prepended here —
// passing `args` directly silently drops the first real argument.
var argv = new List<string> { "cowsay" };
argv.AddRange(args);

ParserResult result;
try
{
    var parser = new Parser(specPath, argv);
    result = parser.Parse();
}
catch (CliBuilderError error)
{
    Console.Error.WriteLine(error.Message);
    Environment.Exit(1);
    return;
}

switch (result)
{
    case HelpResult help:
        Console.WriteLine(help.Text);
        return;

    case VersionResult version:
        Console.WriteLine(version.Version);
        return;

    case ParseResult parseResult:
        Run(parseResult, cowsDir);
        return;
}

static void Run(ParseResult result, string cowsDir)
{
    var flags = result.Flags;
    var arguments = result.Arguments;

    if (CowsayCli.IsListRequested(flags))
    {
        foreach (var name in CowsayCli.ListCowFiles(cowsDir))
        {
            Console.WriteLine(name);
        }

        return;
    }

    var message = CowsayCli.ResolveMessageFromArguments(arguments);
    if (message is null)
    {
        if (!Console.IsInputRedirected)
        {
            return;
        }

        message = Console.In.ReadToEnd().Trim();
    }

    if (message.Length == 0)
    {
        return;
    }

    var invocation = CowsayCli.BuildInvocation(message, flags);
    Console.WriteLine(CowsayRenderer.Render(invocation, cowsDir));
}
