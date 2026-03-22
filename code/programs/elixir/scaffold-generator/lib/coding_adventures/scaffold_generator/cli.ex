# =========================================================================
# CodingAdventures.ScaffoldGenerator.CLI
# =========================================================================
#
# Command-line interface for the scaffold generator. Uses Elixir's built-in
# OptionParser module for argument parsing instead of the cli-builder
# package, keeping this program dependency-free.
#
# Usage:
#
#   ./scaffold-generator my-package [options]
#
# Options:
#
#   -t, --type TYPE         Package type: "library" or "program" (default: "library")
#   -l, --language LANGS    Comma-separated languages or "all" (default: "all")
#   -d, --depends-on DEPS   Comma-separated dependency names (kebab-case)
#       --layer N           Layer number in the computing stack (default: 0)
#       --description TEXT   Short description of the package
#       --dry-run            Show what would be created without creating it
#   -h, --help              Show help message
#   -v, --version           Show version
#
# =========================================================================

defmodule CodingAdventures.ScaffoldGenerator.CLI do
  @moduledoc """
  CLI entry point for the scaffold generator.

  Parses command-line arguments using OptionParser and delegates to the
  main ScaffoldGenerator module for the actual work.
  """

  alias CodingAdventures.ScaffoldGenerator
  alias CodingAdventures.ScaffoldGenerator.Config

  @version "0.1.0"

  # =========================================================================
  # Main entry point
  # =========================================================================
  #
  # This is the function called by the escript. It parses argv, validates
  # inputs, and dispatches to the scaffold function for each language.

  @doc """
  Main entry point for the CLI. Parses arguments and runs the scaffolder.
  Called by escript with the raw argv list.
  """
  @spec main([String.t()]) :: no_return()
  def main(argv) do
    argv
    |> parse_args()
    |> execute()
    |> System.halt()
  end

  # =========================================================================
  # Argument parsing
  # =========================================================================
  #
  # We use OptionParser with strict mode to catch unknown flags early.
  # The package name is a positional argument (appears in the "remaining"
  # list after options are parsed).
  #
  # OptionParser returns {parsed_opts, remaining_args, invalid_opts}.
  #   - parsed_opts: keyword list of recognized flags
  #   - remaining_args: positional arguments
  #   - invalid_opts: flags that didn't match our spec

  @doc """
  Parses command-line arguments into a structured map.
  Returns `{:ok, config}`, `{:help}`, `{:version}`, or `{:error, message}`.
  """
  @spec parse_args([String.t()]) ::
          {:ok, Config.t()} | {:help} | {:version} | {:error, String.t()}
  def parse_args(argv) do
    switches = [
      type: :string,
      language: :string,
      depends_on: :string,
      layer: :integer,
      description: :string,
      dry_run: :boolean,
      help: :boolean,
      version: :boolean
    ]

    aliases = [
      t: :type,
      l: :language,
      d: :depends_on,
      h: :help,
      v: :version
    ]

    case OptionParser.parse(argv, strict: switches, aliases: aliases) do
      {opts, remaining, []} ->
        cond do
          Keyword.get(opts, :help, false) ->
            {:help}

          Keyword.get(opts, :version, false) ->
            {:version}

          true ->
            build_config(opts, remaining)
        end

      {_opts, _remaining, invalid} ->
        bad_flags =
          invalid
          |> Enum.map(fn {flag, _} -> flag end)
          |> Enum.join(", ")

        {:error, "unknown option(s): #{bad_flags}"}
    end
  end

  # =========================================================================
  # Config builder
  # =========================================================================
  #
  # Takes the parsed options and positional arguments and validates them
  # into a Config struct. Returns {:error, reason} if validation fails.

  defp build_config(opts, remaining) do
    # Extract the package name from positional args
    case remaining do
      [] ->
        {:error, "missing required argument: PACKAGE_NAME"}

      [pkg_name | _extra] ->
        with :ok <- validate_package_name(pkg_name),
             {:ok, pkg_type} <- validate_type(Keyword.get(opts, :type, "library")),
             {:ok, languages} <- validate_languages(Keyword.get(opts, :language, "all")),
             {:ok, direct_deps} <- validate_deps(Keyword.get(opts, :depends_on, "")) do
          case ScaffoldGenerator.find_repo_root() do
            {:ok, repo_root} ->
              config = %Config{
                package_name: pkg_name,
                pkg_type: pkg_type,
                languages: languages,
                direct_deps: direct_deps,
                layer: Keyword.get(opts, :layer, 0),
                description: Keyword.get(opts, :description, ""),
                dry_run: Keyword.get(opts, :dry_run, false),
                repo_root: repo_root
              }

              {:ok, config}

            {:error, reason} ->
              {:error, reason}
          end
        end
    end
  end

  # =========================================================================
  # Validation helpers
  # =========================================================================

  defp validate_package_name(name) do
    if ScaffoldGenerator.valid_kebab_case?(name) do
      :ok
    else
      {:error,
       "invalid package name #{inspect(name)} (must be kebab-case: lowercase, digits, hyphens)"}
    end
  end

  defp validate_type(pkg_type) when pkg_type in ["library", "program"] do
    {:ok, pkg_type}
  end

  defp validate_type(other) do
    {:error, "invalid type #{inspect(other)} (must be \"library\" or \"program\")"}
  end

  defp validate_languages("all") do
    {:ok, ScaffoldGenerator.valid_languages()}
  end

  defp validate_languages(lang_str) do
    languages =
      lang_str
      |> String.split(",")
      |> Enum.map(&String.trim/1)
      |> Enum.reject(&(&1 == ""))

    invalid =
      Enum.find(languages, fn lang ->
        lang not in ScaffoldGenerator.valid_languages()
      end)

    case invalid do
      nil ->
        {:ok, languages}

      bad_lang ->
        {:error,
         "unknown language #{inspect(bad_lang)} (valid: #{Enum.join(ScaffoldGenerator.valid_languages(), ", ")})"}
    end
  end

  defp validate_deps(""), do: {:ok, []}

  defp validate_deps(deps_str) do
    deps =
      deps_str
      |> String.split(",")
      |> Enum.map(&String.trim/1)
      |> Enum.reject(&(&1 == ""))

    invalid = Enum.find(deps, fn dep -> not ScaffoldGenerator.valid_kebab_case?(dep) end)

    case invalid do
      nil -> {:ok, deps}
      bad_dep -> {:error, "invalid dependency name #{inspect(bad_dep)} (must be kebab-case)"}
    end
  end

  # =========================================================================
  # Execution
  # =========================================================================
  #
  # Once we have a validated Config, we iterate over each requested language
  # and call the scaffold function. Messages are printed to stdout, errors
  # to stderr.

  defp execute({:help}) do
    IO.puts(help_text())
    0
  end

  defp execute({:version}) do
    IO.puts("scaffold-generator #{@version}")
    0
  end

  defp execute({:error, message}) do
    IO.puts(:stderr, "scaffold-generator: #{message}")
    1
  end

  defp execute({:ok, %Config{} = config}) do
    results =
      Enum.map(config.languages, fn lang ->
        case ScaffoldGenerator.scaffold(config, lang) do
          {:ok, messages} ->
            Enum.each(messages, &IO.puts/1)
            :ok

          {:error, reason} ->
            IO.puts(:stderr, "scaffold-generator [#{lang}]: #{reason}")
            :error
        end
      end)

    if Enum.any?(results, &(&1 == :error)), do: 1, else: 0
  end

  # =========================================================================
  # Help text
  # =========================================================================

  defp help_text do
    """
    scaffold-generator #{@version}

    Generate CI-ready package scaffolding for the coding-adventures monorepo.

    USAGE:
        scaffold-generator PACKAGE_NAME [OPTIONS]

    ARGUMENTS:
        PACKAGE_NAME    Package name in kebab-case (e.g., "logic-gates")

    OPTIONS:
        -t, --type TYPE         Package type: "library" or "program" (default: "library")
        -l, --language LANGS    Comma-separated languages or "all" (default: "all")
        -d, --depends-on DEPS   Comma-separated dependency names (kebab-case)
            --layer N           Layer number in the computing stack (default: 0)
            --description TEXT  Short description of the package
            --dry-run           Show what would be created without creating it
        -h, --help              Show this help message
        -v, --version           Show version

    EXAMPLES:
        scaffold-generator my-package
        scaffold-generator logic-gates -l python,go --description "Boolean logic gates"
        scaffold-generator cpu-core -d logic-gates,registers --layer 5 --dry-run
    """
    |> String.trim()
  end
end
