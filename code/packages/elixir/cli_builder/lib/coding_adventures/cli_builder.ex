defmodule CodingAdventures.CliBuilder do
  @moduledoc """
  Declarative CLI argument parsing driven by directed graphs and state machines.

  ## Overview

  CLI Builder separates *what a CLI accepts* (the JSON spec) from *what it does*
  (the application logic). A developer writes a JSON specification file describing
  the CLI's structure — subcommands, flags, arguments, constraints — and this
  library handles all parsing, validation, help generation, and error messaging.

  ## Architecture

  ```
  JSON Spec File
       │
       ▼
  SpecLoader ──── validates, normalises, builds G_flag ──► SpecError on failure
       │
       ▼
  Parser.parse/2
       ├── Phase 1: Routing (G_cmd DirectedGraph)
       ├── Phase 2: Scanning (TokenClassifier + ModalStateMachine)
       └── Phase 3: Validation (PositionalResolver + FlagValidator)
            │
            ▼
       ParseResult | HelpResult | VersionResult | ParseErrors
  ```

  ## Quick Start

  Given a spec file `my_tool.json`:

      {:ok, result} = CodingAdventures.CliBuilder.parse("my_tool.json", System.argv())

      case result do
        %ParseResult{} -> run_tool(result.flags, result.arguments)
        %HelpResult{text: text} -> IO.puts(text)
        %VersionResult{version: v} -> IO.puts(v)
      end

  Or, from a JSON string (useful in tests):

      spec_json = File.read!("my_tool.json")
      {:ok, result} = CodingAdventures.CliBuilder.parse_string(spec_json, argv)

  ## Error Handling

  On parse failure the library returns `{:error, %ParseErrors{}}` containing a
  list of `%ParseError{}` structs. Each error has a machine-readable `error_type`,
  a human-readable `message`, an optional `suggestion`, and a `context` (the
  command path at the point of failure).

  Spec errors (invalid JSON spec file) raise `SpecError` immediately — these are
  programmer errors, not user errors.
  """

  alias CodingAdventures.CliBuilder.Parser

  @doc """
  Parse `argv` against the spec at `spec_file_path`.

  See `CodingAdventures.CliBuilder.Parser.parse/2` for full documentation.
  """
  defdelegate parse(spec_file_path, argv), to: Parser

  @doc """
  Parse `argv` against a spec supplied as a JSON string.

  See `CodingAdventures.CliBuilder.Parser.parse_string/2` for full documentation.
  """
  defdelegate parse_string(spec_json, argv), to: Parser
end
