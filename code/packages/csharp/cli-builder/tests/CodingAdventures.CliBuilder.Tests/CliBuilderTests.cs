namespace CodingAdventures.CliBuilder.Tests;

using JsonNode = CodingAdventures.JsonValue.JsonValue;

public sealed class CliBuilderTests
{
    [Fact]
    public void SpecLoaderLoadsAndNormalizesBasicSpec()
    {
        var spec = LoadSpec(
            """
            {
              "cli_builder_spec_version": "1.0",
              "name": "demo",
              "description": "Demo tool",
              "flags": [
                { "id": "verbose", "short": "v", "long": "verbose", "description": "Verbose output", "type": "boolean" }
              ],
              "arguments": [
                { "id": "input", "display_name": "INPUT", "description": "Input file", "type": "string", "required": true }
              ]
            }
            """);

        Assert.Equal("demo", spec.Name);
        Assert.Single(spec.Flags);
        Assert.Single(spec.Arguments);
        Assert.Equal(ParsingMode.Gnu, spec.ParsingMode);
    }

    [Fact]
    public void SpecLoaderRejectsCircularRequiresGraph()
    {
        var exception = Assert.Throws<SpecError>(() => LoadSpec(
            """
            {
              "cli_builder_spec_version": "1.0",
              "name": "demo",
              "description": "Demo tool",
              "flags": [
                { "id": "a", "long": "a", "description": "A", "type": "boolean", "requires": ["b"] },
                { "id": "b", "long": "b", "description": "B", "type": "boolean", "requires": ["a"] }
              ]
            }
            """));

        Assert.Contains("Circular requires dependency", exception.Message);
    }

    [Fact]
    public void ParserParsesFlagsAndArguments()
    {
        var spec = LoadSpec(
            """
            {
              "cli_builder_spec_version": "1.0",
              "name": "demo",
              "description": "Demo tool",
              "flags": [
                { "id": "verbose", "short": "v", "long": "verbose", "description": "Verbose output", "type": "boolean" },
                { "id": "count", "long": "count", "description": "Count value", "type": "integer" }
              ],
              "arguments": [
                { "id": "input", "display_name": "INPUT", "description": "Input file", "type": "string", "required": true }
              ]
            }
            """);

        var result = Assert.IsType<ParseResult>(new Parser(spec, ["demo", "-v", "--count=3", "file.txt"]).Parse());

        Assert.Equal(["demo"], result.CommandPath);
        Assert.Equal(true, result.Flags["verbose"]);
        Assert.Equal(3L, result.Flags["count"]);
        Assert.Equal("file.txt", result.Arguments["input"]);
    }

    [Fact]
    public void ParserRoutesToSubcommandsAndCarriesGlobalFlags()
    {
        var spec = LoadSpec(
            """
            {
              "cli_builder_spec_version": "1.0",
              "name": "demo",
              "description": "Demo tool",
              "global_flags": [
                { "id": "verbose", "short": "v", "long": "verbose", "description": "Verbose output", "type": "boolean" }
              ],
              "commands": [
                {
                  "id": "serve",
                  "name": "serve",
                  "description": "Serve files",
                  "arguments": [
                    { "id": "port", "display_name": "PORT", "description": "Port", "type": "integer", "required": true }
                  ]
                }
              ]
            }
            """);

        var result = Assert.IsType<ParseResult>(new Parser(spec, ["demo", "serve", "-v", "8080"]).Parse());

        Assert.Equal(["demo", "serve"], result.CommandPath);
        Assert.Equal(true, result.Flags["verbose"]);
        Assert.Equal(8080L, result.Arguments["port"]);
    }

    [Fact]
    public void ParserReturnsHelpAndVersionResults()
    {
        var spec = LoadSpec(
            """
            {
              "cli_builder_spec_version": "1.0",
              "name": "demo",
              "description": "Demo tool",
              "version": "1.2.3"
            }
            """);

        var help = Assert.IsType<HelpResult>(new Parser(spec, ["demo", "--help"]).Parse());
        var version = Assert.IsType<VersionResult>(new Parser(spec, ["demo", "--version"]).Parse());

        Assert.Contains("USAGE", help.Text);
        Assert.Equal("1.2.3", version.Version);
    }

    [Fact]
    public void ParserReportsValidationErrors()
    {
        var spec = LoadSpec(
            """
            {
              "cli_builder_spec_version": "1.0",
              "name": "demo",
              "description": "Demo tool",
              "flags": [
                { "id": "force", "long": "force", "description": "Force", "type": "boolean", "conflicts_with": ["dry-run"] },
                { "id": "dry-run", "long": "dry-run", "description": "Dry run", "type": "boolean", "conflicts_with": ["force"] }
              ],
              "arguments": [
                { "id": "input", "display_name": "INPUT", "description": "Input", "type": "string", "required": true }
              ]
            }
            """);

        var exception = Assert.Throws<ParseErrors>(() => new Parser(spec, ["demo", "--force", "--dry-run"]).Parse());

        Assert.Contains(exception.Errors, error => error.ErrorType == "conflicting_flags");
        Assert.Contains(exception.Errors, error => error.ErrorType == "missing_required_argument");
    }

    [Fact]
    public void ParserSupportsRepeatableAndStackedFlags()
    {
        var spec = LoadSpec(
            """
            {
              "cli_builder_spec_version": "1.0",
              "name": "demo",
              "description": "Demo tool",
              "flags": [
                { "id": "verbose", "short": "v", "description": "Verbose", "type": "count" },
                { "id": "tag", "short": "t", "long": "tag", "description": "Tag", "type": "string", "repeatable": true }
              ]
            }
            """);

        var result = Assert.IsType<ParseResult>(new Parser(spec, ["demo", "-vv", "--tag=one", "--tag=two"]).Parse());

        Assert.Equal(2L, result.Flags["verbose"]);
        var tags = Assert.IsType<List<object?>>(result.Flags["tag"]);
        Assert.Equal([ "one", "two" ], tags.Cast<string>().ToArray());
    }

    private static CliSpec LoadSpec(string text)
    {
        return new SpecLoader("<memory>").LoadFromObject((Dictionary<string, object?>)JsonNode.ParseNative(text)!);
    }
}
