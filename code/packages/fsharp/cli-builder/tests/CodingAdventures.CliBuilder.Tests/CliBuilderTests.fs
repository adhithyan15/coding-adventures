namespace CodingAdventures.CliBuilder.FSharp.Tests

open System.Collections.Generic
open CodingAdventures.CliBuilder.FSharp
open CodingAdventures.JsonValue.FSharp
open Xunit

type CliBuilderTests() =
    static member private LoadSpec(text: string) =
        let raw = JsonValue.ParseNative(text) :?> Dictionary<string, obj>
        SpecLoader("<memory>").LoadFromObject(raw)

    [<Fact>]
    member _.``SpecLoader loads and normalizes basic specs``() =
        let spec =
            CliBuilderTests.LoadSpec(
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
                """)

        Assert.Equal("demo", spec.Name)
        Assert.Single(spec.Flags) |> ignore
        Assert.Single(spec.Arguments) |> ignore
        Assert.Equal(ParsingMode.Gnu, spec.ParsingMode)

    [<Fact>]
    member _.``SpecLoader rejects circular requires graphs``() =
        let ex =
            Assert.Throws<SpecError>(fun () ->
                CliBuilderTests.LoadSpec(
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
                    """)
                |> ignore)

        Assert.Contains("Circular requires dependency", ex.Message)

    [<Fact>]
    member _.``Parser parses flags and arguments``() =
        let spec =
            CliBuilderTests.LoadSpec(
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
                """)

        let result = Parser(spec, [ "demo"; "-v"; "--count=3"; "file.txt" ]).Parse() |> Assert.IsType<ParseResult>

        Assert.Equal<string list>([ "demo" ], result.CommandPath |> Seq.toList)
        Assert.Equal(box true, result.Flags.["verbose"])
        Assert.Equal(box 3L, result.Flags.["count"])
        Assert.Equal(box "file.txt", result.Arguments.["input"])

    [<Fact>]
    member _.``Parser routes to subcommands and carries global flags``() =
        let spec =
            CliBuilderTests.LoadSpec(
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
                """)

        let result = Parser(spec, [ "demo"; "serve"; "-v"; "8080" ]).Parse() |> Assert.IsType<ParseResult>

        Assert.Equal<string list>([ "demo"; "serve" ], result.CommandPath |> Seq.toList)
        Assert.Equal(box true, result.Flags.["verbose"])
        Assert.Equal(box 8080L, result.Arguments.["port"])

    [<Fact>]
    member _.``Parser returns help and version results``() =
        let spec =
            CliBuilderTests.LoadSpec(
                """
                {
                  "cli_builder_spec_version": "1.0",
                  "name": "demo",
                  "description": "Demo tool",
                  "version": "1.2.3"
                }
                """)

        let help = Parser(spec, [ "demo"; "--help" ]).Parse() |> Assert.IsType<HelpResult>
        let version = Parser(spec, [ "demo"; "--version" ]).Parse() |> Assert.IsType<VersionResult>

        Assert.Contains("USAGE", help.Text)
        Assert.Equal("1.2.3", version.Version)

    [<Fact>]
    member _.``Parser reports validation errors``() =
        let spec =
            CliBuilderTests.LoadSpec(
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
                """)

        let ex =
            Assert.Throws<ParseErrors>(fun () ->
                Parser(spec, [ "demo"; "--force"; "--dry-run" ]).Parse() |> ignore)

        Assert.Contains(ex.Errors, fun error -> error.ErrorType = "conflicting_flags")
        Assert.Contains(ex.Errors, fun error -> error.ErrorType = "missing_required_argument")

    [<Fact>]
    member _.``Parser supports repeatable and stacked flags``() =
        let spec =
            CliBuilderTests.LoadSpec(
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
                """)

        let result = Parser(spec, [ "demo"; "-vv"; "--tag=one"; "--tag=two" ]).Parse() |> Assert.IsType<ParseResult>

        Assert.Equal(box 2L, result.Flags.["verbose"])
        let tags = result.Flags.["tag"] :?> ResizeArray<obj>
        Assert.Equal<string list>([ "one"; "two" ], tags |> Seq.cast<string> |> Seq.toList)
