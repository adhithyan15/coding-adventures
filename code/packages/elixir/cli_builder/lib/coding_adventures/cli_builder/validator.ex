defmodule CodingAdventures.CliBuilder.Validator do
  @moduledoc """
  Standalone validation of CLI Builder JSON specifications.

  ## Why a separate module?

  `SpecLoader.load!/1` raises exceptions on invalid specs — useful when you
  want fail-fast behaviour at program startup.  But sometimes you need a
  gentler interface: a function that returns a result map instead of
  crashing.  That is what this module provides.

  ## How it works

  Both `validate_spec/1` and `validate_spec_string/1` delegate to the
  existing `SpecLoader` functions, wrapping them in a rescue block.  If the
  spec loads successfully, we return `%{valid: true, errors: []}`.  If a
  `SpecError` is raised, we catch it and return the error message in a list.

  This keeps validation logic in one place (SpecLoader) while offering two
  ergonomic interfaces — bang and non-bang — for different use cases.

  ## Examples

      # File-based validation
      result = Validator.validate_spec("path/to/cli.json")
      if result.valid, do: IO.puts("Spec is valid!")

      # String-based validation (handy in tests or REPLs)
      json = ~s({"cli_builder_spec_version": "1.0", "name": "hello", "description": "Hi"})
      %{valid: true, errors: []} = Validator.validate_spec_string(json)
  """

  alias CodingAdventures.CliBuilder.{SpecLoader, SpecError}

  # ---------------------------------------------------------------------------
  # Types
  # ---------------------------------------------------------------------------

  @typedoc """
  The result of a validation check.

  - `valid` — `true` when the spec passes all checks, `false` otherwise.
  - `errors` — a list of human-readable error strings.  Empty when valid.
  """
  @type validation_result :: %{valid: boolean(), errors: [String.t()]}

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Validate a CLI Builder spec file on disk.

  Reads the file, parses the JSON, and runs the full SpecLoader validation
  pipeline.  Returns a `validation_result` map rather than raising.

  ## Parameters

  - `spec_file_path` — absolute or relative path to a JSON spec file.

  ## Examples

      iex> result = Validator.validate_spec("nonexistent.json")
      iex> result.valid
      false
      iex> hd(result.errors) =~ "Cannot read spec file"
      true
  """
  @spec validate_spec(String.t()) :: validation_result()
  def validate_spec(spec_file_path) do
    # Delegate to SpecLoader.load!/1 which handles file reading, JSON
    # parsing, and full structural + semantic validation.  We wrap the
    # call so that any SpecError is caught and converted to a result map.
    SpecLoader.load!(spec_file_path)
    success_result()
  rescue
    error in SpecError ->
      failure_result(error.message)
  end

  @doc """
  Validate a CLI Builder spec from a raw JSON string.

  Useful when you already have the spec in memory — for example, when
  building a spec editor UI or running validation in tests without
  touching the filesystem.

  ## Parameters

  - `json_string` — a UTF-8 JSON string containing the spec.

  ## Examples

      iex> Validator.validate_spec_string("not json at all")
      %{valid: false, errors: ["Invalid JSON: unexpected byte at position 0: 0x6E (\"n\")"]}
  """
  @spec validate_spec_string(String.t()) :: validation_result()
  def validate_spec_string(json_string) do
    # Same pattern: delegate to the bang function, rescue on failure.
    SpecLoader.load_from_string!(json_string)
    success_result()
  rescue
    error in SpecError ->
      failure_result(error.message)
  end

  # ---------------------------------------------------------------------------
  # Result helpers
  # ---------------------------------------------------------------------------

  # Build a successful validation result.
  defp success_result, do: %{valid: true, errors: []}

  # Build a failed validation result with a single error message.
  # SpecLoader raises one SpecError per problem (it stops at the first
  # violation), so we always have exactly one error string.
  defp failure_result(message), do: %{valid: false, errors: [message]}
end
