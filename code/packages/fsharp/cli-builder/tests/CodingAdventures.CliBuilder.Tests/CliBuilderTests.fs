namespace CliBuilderFacadeTests

open System.Collections.Generic
open global.CodingAdventures.CliBuilder.FSharp
open Xunit

type JsonNode = global.CodingAdventures.JsonValue.JsonValue

type CliBuilderTests() =
    [<Fact>]
    member _.``SpecLoader loads specs through the F# facade``() =
        let raw : Dictionary<string, obj> =
            JsonNode.ParseNative(
                """
                {
                  "cli_builder_spec_version": "1.0",
                  "name": "demo",
                  "description": "Demo tool",
                  "flags": [
                    { "id": "verbose", "short": "v", "long": "verbose", "description": "Verbose output", "type": "boolean" }
                  ]
                }
                """)
            :?> Dictionary<string, obj>

        let spec = SpecLoader("<memory>").LoadFromObject(raw)

        Assert.Equal("demo", spec.Name)
        Assert.Single(spec.Flags) |> ignore

    [<Fact>]
    member _.``Parser parses flags and arguments through the F# facade``() =
        let raw : Dictionary<string, obj> =
            JsonNode.ParseNative(
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
            :?> Dictionary<string, obj>

        let spec = SpecLoader("<memory>").LoadFromObject(raw)
        let result = Parser(spec, [ "demo"; "-v"; "file.txt" ]).Parse() |> Assert.IsType<ParseResult>

        Assert.Equal(box true, result.Flags.["verbose"])
        Assert.Equal(box "file.txt", result.Arguments.["input"])

    [<Fact>]
    member _.``Parser returns help results through the F# facade``() =
        let raw : Dictionary<string, obj> =
            JsonNode.ParseNative(
                """
                {
                  "cli_builder_spec_version": "1.0",
                  "name": "demo",
                  "description": "Demo tool",
                  "version": "1.2.3"
                }
                """)
            :?> Dictionary<string, obj>

        let spec = SpecLoader("<memory>").LoadFromObject(raw)
        let help = Parser(spec, [ "demo"; "--help" ]).Parse() |> Assert.IsType<HelpResult>

        Assert.Contains("USAGE", help.Text)
