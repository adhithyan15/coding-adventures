defmodule CodingAdventures.CliBuilder.ParseError do
  @moduledoc """
  Represents a single parse error with a machine-readable type, human-readable
  message, an optional suggestion (e.g. a fuzzy match for typos), and a
  `context` list that records the `command_path` at the point the error
  was detected.

  ## Fields

  - `:error_type` — snake_case string (e.g. `"unknown_flag"`, `"missing_required_flag"`).
    See §8.2 of the spec for the full catalogue.
  - `:message` — a sentence the user can read in a terminal.
  - `:suggestion` — optional corrective hint; `nil` when there is nothing useful to say.
  - `:context` — the `command_path` list at error detection time.

  ## Example

      %ParseError{
        error_type: "unknown_flag",
        message: "Unknown flag '--mesage'. Did you mean '--message'?",
        suggestion: "--message",
        context: ["git", "commit"]
      }
  """

  @enforce_keys [:error_type, :message]
  defstruct [:error_type, :message, :suggestion, :context]

  @type t :: %__MODULE__{
          error_type: String.t(),
          message: String.t(),
          suggestion: String.t() | nil,
          context: [String.t()]
        }
end

defmodule CodingAdventures.CliBuilder.SpecError do
  @moduledoc """
  Raised when the JSON spec file itself is invalid.

  Examples include: circular `requires` dependencies, duplicate flag IDs,
  missing `enum_values` when `type` is `"enum"`, unrecognised spec version,
  and similar structural problems discovered at load time.

  This is an *exception* (raised via `raise/1`) rather than a value, because
  spec errors are programmer errors that should stop the application from
  starting — there is no useful way to continue parsing against an invalid spec.
  """

  defexception [:message]
end

defmodule CodingAdventures.CliBuilder.ParseErrors do
  @moduledoc """
  Exception raised (or returned as `{:error, t()}`) when one or more parse
  errors accumulate during argv processing.

  Collecting all errors at once gives the user a full picture of what is wrong
  in a single invocation, which is far more usable than fail-fast behaviour.

  ## Fields

  - `:errors` — a non-empty list of `ParseError` structs.
  - `:message` — a pre-rendered string joining all error messages with newlines.
    (This is the format that `Exception.message/1` returns.)

  ## Example

      {:error, %ParseErrors{errors: [
        %ParseError{error_type: "unknown_flag", message: "...", ...},
        %ParseError{error_type: "missing_required_flag", message: "...", ...}
      ]}}
  """

  defexception [:errors, :message]

  @type t :: %__MODULE__{
          errors: [CodingAdventures.CliBuilder.ParseError.t()],
          message: String.t()
        }

  @doc """
  Render all error messages joined by newlines.
  Called automatically by `Exception.message/1`.
  """
  @impl true
  def message(%{errors: errors}) do
    Enum.map_join(errors, "\n", & &1.message)
  end
end
