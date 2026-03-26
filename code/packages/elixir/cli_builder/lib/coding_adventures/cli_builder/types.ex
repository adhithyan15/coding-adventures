defmodule CodingAdventures.CliBuilder.ParseResult do
  @moduledoc """
  The successful result of parsing argv against a CLI spec.

  ## Fields

  - `:program` — `argv[0]`: the program name as invoked (e.g. `"git"`).
  - `:command_path` — full path from root to resolved leaf command
    (e.g. `["git", "remote", "add"]`). For root-level invocations with no
    subcommands this is just `["program-name"]`.
  - `:flags` — map from flag `id` to parsed (and coerced) value. Every flag in
    scope is present: absent booleans are `false`, absent non-booleans are `nil`
    (or the flag's `default` value if set). Count-type flags default to `0`.
  - `:arguments` — map from argument `id` to parsed value. Variadic arguments
    produce lists.
  - `:explicit_flags` — list of flag IDs that the user explicitly set on the
    command line (v1.1). This lets callers distinguish between "the user typed
    `--verbose`" and "verbose defaulted to false". A flag ID appears once per
    occurrence, so a count flag passed three times will appear three times.

  ## Example

      %ParseResult{
        program: "git",
        command_path: ["git", "remote", "add"],
        flags: %{"verbose" => false, "dry-run" => false},
        arguments: %{"name" => "origin", "url" => "https://github.com/user/repo"},
        explicit_flags: []
      }
  """

  @enforce_keys [:program, :command_path, :flags, :arguments]
  defstruct [:program, :command_path, :flags, :arguments, explicit_flags: []]

  @type t :: %__MODULE__{
          program: String.t(),
          command_path: [String.t()],
          flags: %{String.t() => term()},
          arguments: %{String.t() => term()},
          explicit_flags: [String.t()]
        }
end

defmodule CodingAdventures.CliBuilder.HelpResult do
  @moduledoc """
  Returned instead of a `ParseResult` when `--help` or `-h` is encountered.

  The caller should print `:text` to stdout and exit 0.

  ## Fields

  - `:text` — the fully-rendered help string for the deepest resolved command.
  - `:command_path` — path at which help was requested.
  """

  @enforce_keys [:text, :command_path]
  defstruct [:text, :command_path]

  @type t :: %__MODULE__{
          text: String.t(),
          command_path: [String.t()]
        }
end

defmodule CodingAdventures.CliBuilder.VersionResult do
  @moduledoc """
  Returned instead of a `ParseResult` when `--version` is encountered.

  The caller should print `:version` to stdout and exit 0.

  ## Fields

  - `:version` — the `version` string from the spec.
  """

  @enforce_keys [:version]
  defstruct [:version]

  @type t :: %__MODULE__{version: String.t()}
end
