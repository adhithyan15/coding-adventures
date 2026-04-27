defmodule BuildTool.StarlarkEvaluator do
  @moduledoc """
  Starlark BUILD file evaluator for the Elixir build tool.

  ## Chapter 1: Why Starlark BUILD Files?

  Traditional BUILD files in this monorepo are shell scripts — each line is a
  command executed sequentially. This works but has limitations:

    - **No change detection metadata**: the build tool guesses which files
      matter based on file extensions, not explicit declarations.
    - **No dependency declarations**: deps are parsed from language-specific
      config files (pyproject.toml, go.mod, etc.) with heuristic matching.
    - **No validation**: a typo in a BUILD file only surfaces at build time.

  Starlark BUILD files solve all three. They're real programs that declare
  targets with explicit srcs, deps, and build metadata. The build tool
  evaluates them using the Starlark interpreter package and extracts the
  declared targets.

  ## Chapter 2: How Evaluation Works

  The evaluation pipeline mirrors the Go implementation at
  `code/programs/go/build-tool/internal/starlark/evaluator.go`:

    1. Read the BUILD file contents.
    2. Create a file resolver rooted at the repo root (for `load()` statements).
    3. Execute the BUILD file through the interpreter pipeline:
       `source → tokens → AST → bytecode → execution`
    4. Extract the `_targets` list from the result's variables.
    5. Convert each target map to a `Target` struct.

  ## Chapter 3: Detecting Starlark vs Shell BUILD Files

  We use a simple heuristic: scan the first non-comment, non-blank line.
  If it starts with `load(`, `def `, or matches a known rule call pattern
  (like `py_library(`), it's Starlark. Otherwise it's shell.

  This approach is deliberately conservative — if there's any doubt, we
  treat the file as shell, which is the legacy default.

  ## Chapter 4: Generating Shell Commands from Targets

  Once we have targets, we convert each one into shell commands that the
  executor can run. Each rule type maps to a standard set of commands:

    | Rule             | Commands                                         |
    |------------------|--------------------------------------------------|
    | `py_library`     | `uv pip install --system -e ".[dev]"` + pytest   |
    | `go_library`     | `go build` + `go test` + `go vet`                |
    | `ruby_library`   | `bundle install` + `rake test`                   |
    | `ts_library`     | `npm install` + `vitest`                         |
    | `rust_library`   | `cargo build` + `cargo test`                     |
    | `elixir_library` | `mix deps.get` + `mix test`                      |

  This table is identical to the Go implementation's `GenerateCommands`.
  """

  alias CodingAdventures.StarlarkInterpreter

  # Schema version for the _ctx build context dict.
  @ctx_schema_version 1

  # OS normalization: :os.type() -> runtime.GOOS equivalents.
  #
  # Elixir's :os.type() returns {family, name} tuples:
  #   {:unix, :darwin}  -> "darwin"
  #   {:unix, :linux}   -> "linux"
  #   {:win32, :nt}     -> "windows"
  @os_map %{
    darwin: "darwin",
    linux: "linux",
    nt: "windows"
  }

  # Characters that trigger quoting in shell strings.
  @shell_meta MapSet.new(String.graphemes(" \t\"'$`\\|&;()<>!#*?[]{}"))

  # ===========================================================================
  # Target Struct
  # ===========================================================================
  #
  # Each call to py_library(), go_library(), etc. in a Starlark BUILD file
  # produces one Target. The struct holds the declared metadata so the build
  # tool can generate commands and detect changes.

  defmodule Target do
    @moduledoc """
    A single build target declared in a Starlark BUILD file.

    ## Fields

      - `rule` — Rule type: `"py_library"`, `"go_binary"`, etc.
      - `name` — Target name: `"starlark-vm"`, `"build-tool"`, etc.
      - `srcs` — Declared source file patterns for change detection.
      - `deps` — Dependencies as `"language/package-name"` strings.
      - `test_runner` — Test framework: `"pytest"`, `"vitest"`, `"minitest"`, etc.
      - `entry_point` — Binary entry point: `"main.py"`, `"src/index.ts"`, etc.
    """
    defstruct rule: "",
              name: "",
              srcs: [],
              deps: [],
              test_runner: "",
              entry_point: "",
              commands: []
  end

  # ===========================================================================
  # Starlark Detection
  # ===========================================================================

  @known_rules [
    "py_library(", "py_binary(",
    "go_library(", "go_binary(",
    "ruby_library(", "ruby_binary(",
    "ts_library(", "ts_binary(",
    "rust_library(", "rust_binary(",
    "elixir_library(", "elixir_binary("
  ]

  @doc """
  Detect whether a BUILD file contains Starlark code (as opposed to shell).

  We scan the first non-comment, non-blank line for Starlark-specific patterns:

    - `load("...")` statements — importing rule definitions
    - `def ` — function definitions
    - Known rule calls — `py_library(`, `go_binary(`, etc.

  If none of these appear on the first significant line, we treat it as shell.

  ## Examples

      iex> BuildTool.StarlarkEvaluator.starlark_build?("load(\\"//rules.star\\", \\"py_library\\")\\n")
      true

      iex> BuildTool.StarlarkEvaluator.starlark_build?("python -m pip install -e .\\npytest\\n")
      false

      iex> BuildTool.StarlarkEvaluator.starlark_build?("# comment\\npy_library(name = \\"x\\")\\n")
      true

      iex> BuildTool.StarlarkEvaluator.starlark_build?("")
      false
  """
  def starlark_build?(content) when is_binary(content) do
    content
    |> String.split("\n")
    |> Enum.reduce_while(false, fn line, _acc ->
      trimmed = String.trim(line)

      cond do
        # Skip blank lines and comments — they tell us nothing about format.
        trimmed == "" or String.starts_with?(trimmed, "#") ->
          {:cont, false}

        # Starlark indicator: load() statement.
        String.starts_with?(trimmed, "load(") ->
          {:halt, true}

        # Starlark indicator: function definition.
        String.starts_with?(trimmed, "def ") ->
          {:halt, true}

        # Starlark indicator: known rule call.
        Enum.any?(@known_rules, fn rule -> String.starts_with?(trimmed, rule) end) ->
          {:halt, true}

        # First significant line doesn't match any Starlark pattern — it's shell.
        true ->
          {:halt, false}
      end
    end)
  end

  # ===========================================================================
  # BUILD File Evaluation
  # ===========================================================================

  @doc """
  Evaluate a Starlark BUILD file and extract its declared targets.

  This function:

    1. Reads the BUILD file from disk
    2. Creates a file resolver that resolves `load()` paths relative to the repo root
    3. Runs the Starlark interpreter on the source
    4. Extracts the `_targets` list from the result's variables
    5. Converts each target map to a `%Target{}` struct

  ## Parameters

    - `build_file_path` — Absolute path to the BUILD file
    - `pkg_dir` — Absolute path to the package directory (for future glob support)
    - `repo_root` — Absolute path to the repo root (for resolving `load()` paths)

  ## Returns

    - `{:ok, [%Target{}]}` on success
    - `{:error, reason}` on failure

  ## Example

      {:ok, targets} = BuildTool.StarlarkEvaluator.evaluate_build_file(
        "/repo/code/packages/python/mylib/BUILD",
        "/repo/code/packages/python/mylib",
        "/repo"
      )
      hd(targets).name  #=> "mylib"
  """
  def evaluate_build_file(build_file_path, _pkg_dir, repo_root) do
    case File.read(build_file_path) do
      {:error, reason} ->
        {:error, "reading BUILD file #{build_file_path}: #{inspect(reason)}"}

      {:ok, content} ->
        # Ensure source ends with newline (parser requirement).
        source =
          if String.ends_with?(content, "\n") do
            content
          else
            content <> "\n"
          end

        # -----------------------------------------------------------------------
        # File Resolver
        # -----------------------------------------------------------------------
        #
        # The file resolver is a function that maps load() labels to file
        # contents. Labels in our BUILD files are paths relative to the repo
        # root, like "code/packages/starlark/library-rules/python_library.star".
        #
        # The resolver joins the label with the repo root and reads the file.
        file_resolver = fn label ->
          full_path = Path.join(repo_root, label)

          case File.read(full_path) do
            {:ok, data} -> data
            {:error, err} -> raise "load(#{inspect(label)}): #{inspect(err)}"
          end
        end

        # -----------------------------------------------------------------------
        # Build context dict (_ctx)
        # -----------------------------------------------------------------------
        #
        # The _ctx dict is injected into every Starlark scope so that BUILD
        # files and .star rules can branch on platform, architecture, etc.
        # See spec 15 for the full schema.
        {_family, os_name} = :os.type()
        normalized_os = Map.get(@os_map, os_name, Atom.to_string(os_name))

        ctx_dict = %{
          "version" => @ctx_schema_version,
          "os" => normalized_os,
          "arch" => to_string(:erlang.system_info(:system_architecture)) |> normalize_arch(),
          "cpu_count" => System.schedulers_online(),
          "ci" => System.get_env("CI", "") != "",
          "repo_root" => repo_root
        }

        # -----------------------------------------------------------------------
        # Execute through the interpreter pipeline
        # -----------------------------------------------------------------------
        try do
          result = StarlarkInterpreter.interpret(source,
            file_resolver: file_resolver,
            globals: %{"_ctx" => ctx_dict}
          )

          # Extract _targets from the result's variables.
          case extract_targets(result.variables) do
            {:ok, targets} -> {:ok, targets}
            {:error, reason} -> {:error, "extracting targets from #{build_file_path}: #{reason}"}
          end
        rescue
          err ->
            {:error, "evaluating BUILD file #{build_file_path}: #{Exception.message(err)}"}
        end
    end
  end

  # ===========================================================================
  # Target Extraction
  # ===========================================================================
  #
  # The Starlark BUILD file is expected to populate a `_targets` variable —
  # a list of dicts, each with keys like "rule", "name", "srcs", "deps",
  # "test_runner", and "entry_point".
  #
  # The rule definition functions (py_library, go_binary, etc.) are defined in
  # .star files loaded via load(). They append target dicts to _targets.

  @doc false
  def extract_targets(variables) when is_map(variables) do
    case Map.fetch(variables, "_targets") do
      :error ->
        # No _targets variable — the BUILD file didn't declare any targets.
        # This is valid (e.g., a BUILD file that only defines helper functions).
        {:ok, []}

      {:ok, raw_targets} when is_list(raw_targets) ->
        targets =
          raw_targets
          |> Enum.with_index()
          |> Enum.map(fn {raw, idx} ->
            if is_map(raw) do
              %Target{
                rule: get_string(raw, "rule"),
                name: get_string(raw, "name"),
                srcs: get_string_list(raw, "srcs"),
                deps: get_string_list(raw, "deps"),
                test_runner: get_string(raw, "test_runner"),
                entry_point: get_string(raw, "entry_point"),
                commands: get_dict_list(raw, "commands")
              }
            else
              raise "expected _targets[#{idx}] to be a map, got: #{inspect(raw)}"
            end
          end)

        {:ok, targets}

      {:ok, other} ->
        {:error, "_targets is not a list (got #{inspect(other)})"}
    end
  end

  # ===========================================================================
  # Command Generation
  # ===========================================================================

  @doc """
  Convert a `%Target{}` into shell commands that the executor can run.

  This bridges Starlark declarations to actual build/test commands. Each rule
  type maps to a standard set of commands — the same mapping used by the Go
  implementation's `GenerateCommands` function.

  ## Examples

      iex> target = %BuildTool.StarlarkEvaluator.Target{rule: "py_library", name: "mylib"}
      iex> BuildTool.StarlarkEvaluator.generate_commands(target)
      ["uv pip install --system -e \\".[dev]\\"", "python -m pytest --cov --cov-report=term-missing"]

      iex> target = %BuildTool.StarlarkEvaluator.Target{rule: "go_library", name: "mylib"}
      iex> BuildTool.StarlarkEvaluator.generate_commands(target)
      ["go build ./...", "go test ./... -v -cover", "go vet ./..."]
  """
  def generate_commands(%Target{rule: rule, test_runner: test_runner}) do
    case rule do
      "py_library" ->
        runner =
          if test_runner == "" or test_runner == nil do
            "pytest"
          else
            test_runner
          end

        install_cmd = ~s(uv pip install --system -e ".[dev]")

        test_cmd =
          if runner == "pytest" do
            "python -m pytest --cov --cov-report=term-missing"
          else
            "python -m unittest discover tests/"
          end

        [install_cmd, test_cmd]

      "py_binary" ->
        [
          ~s(uv pip install --system -e ".[dev]"),
          "python -m pytest --cov --cov-report=term-missing"
        ]

      rule when rule in ["go_library", "go_binary"] ->
        [
          "go build ./...",
          "go test ./... -v -cover",
          "go vet ./..."
        ]

      rule when rule in ["ruby_library", "ruby_binary"] ->
        [
          "bundle install --quiet",
          "bundle exec rake test"
        ]

      rule when rule in ["ts_library", "ts_binary"] ->
        [
          "npm install --silent",
          "npx vitest run --coverage"
        ]

      rule when rule in ["rust_library", "rust_binary"] ->
        [
          "cargo build",
          "cargo test"
        ]

      rule when rule in ["elixir_library", "elixir_binary"] ->
        [
          "mix deps.get",
          "mix test --cover"
        ]

      unknown ->
        ["echo 'Unknown rule: #{unknown}'"]
    end
  end

  # ===========================================================================
  # Helper Functions
  # ===========================================================================
  #
  # These safely extract typed values from maps. They mirror the Go
  # implementation's getString and getStringList helper functions.
  # Returning defaults instead of raising keeps the evaluator robust
  # against malformed BUILD files.

  @doc false
  def get_string(map, key) when is_map(map) do
    case Map.get(map, key) do
      val when is_binary(val) -> val
      _ -> ""
    end
  end

  @doc false
  def get_string_list(map, key) when is_map(map) do
    case Map.get(map, key) do
      val when is_list(val) ->
        Enum.filter(val, &is_binary/1)

      _ ->
        []
    end
  end

  @doc false
  def get_dict_list(map, key) when is_map(map) do
    case Map.get(map, key) do
      val when is_list(val) ->
        Enum.filter(val, &is_map/1)

      _ ->
        []
    end
  end

  # ===========================================================================
  # Architecture Normalization
  # ===========================================================================
  #
  # Erlang's :erlang.system_info(:system_architecture) returns strings like
  # "aarch64-apple-darwin24.3.0" or "x86_64-pc-linux-gnu". We extract just
  # the CPU architecture and normalize to Go's GOARCH convention.

  @doc false
  def normalize_arch(arch_string) do
    cond do
      String.starts_with?(arch_string, "aarch64") -> "arm64"
      String.starts_with?(arch_string, "arm64") -> "arm64"
      String.starts_with?(arch_string, "x86_64") -> "amd64"
      String.starts_with?(arch_string, "i386") -> "386"
      String.starts_with?(arch_string, "i686") -> "386"
      true -> arch_string |> String.split("-") |> hd()
    end
  end

  # ===========================================================================
  # Command Rendering
  # ===========================================================================
  #
  # Convert structured command dicts from cmd.star into shell-safe strings.
  # Each command dict has the form:
  #   %{"program" => "cargo", "args" => ["build", "--release"]}
  #
  # The renderer joins program + args, quoting any argument that contains
  # shell metacharacters.

  @doc """
  Render a single command dict to a shell-safe string.

  ## Examples

      iex> BuildTool.StarlarkEvaluator.render_command(%{"program" => "cargo", "args" => ["build"]})
      "cargo build"

      iex> BuildTool.StarlarkEvaluator.render_command(%{"program" => "uv", "args" => ["pip", "install", "--system", "-e", ".[dev]"]})
      "uv pip install --system -e \\".[dev]\\""
  """
  def render_command(%{"program" => program} = cmd) when is_binary(program) and program != "" do
    args = Map.get(cmd, "args", [])

    parts =
      [quote_arg(program) | Enum.map(args, fn arg -> quote_arg(to_string(arg)) end)]

    Enum.join(parts, " ")
  end

  @doc """
  Render a list of command dicts to shell-safe strings, skipping non-maps.
  """
  def render_commands(cmds) when is_list(cmds) do
    cmds
    |> Enum.filter(&is_map/1)
    |> Enum.filter(fn cmd -> is_binary(Map.get(cmd, "program")) end)
    |> Enum.map(&render_command/1)
  end

  @doc false
  def quote_arg(""), do: "\"\""

  def quote_arg(arg) when is_binary(arg) do
    if needs_quoting?(arg) do
      escaped =
        arg
        |> String.replace("\\", "\\\\")
        |> String.replace("\"", "\\\"")

      "\"#{escaped}\""
    else
      arg
    end
  end

  defp needs_quoting?(arg) do
    arg
    |> String.graphemes()
    |> Enum.any?(fn c -> MapSet.member?(@shell_meta, c) end)
  end
end
