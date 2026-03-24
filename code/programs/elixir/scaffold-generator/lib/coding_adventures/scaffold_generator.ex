# =========================================================================
# CodingAdventures.ScaffoldGenerator
# =========================================================================
#
# This module generates correctly-structured, CI-ready package directories
# for the coding-adventures monorepo. It supports all six languages:
# Python, Go, Ruby, TypeScript, Rust, and Elixir.
#
# # Why this tool exists
#
# The lessons.md file documents 12+ recurring categories of CI failures
# caused by agents hand-crafting packages inconsistently:
#
#   - Missing BUILD files
#   - TypeScript "main" pointing to dist/ instead of src/
#   - Missing transitive dependency installs in BUILD files
#   - Ruby require ordering (deps before own modules)
#   - Rust workspace Cargo.toml not updated
#   - Missing README.md or CHANGELOG.md
#
# This tool eliminates those failures. Run it, get a package that compiles,
# lints, and passes tests. Then fill in the business logic.
#
# # Architecture
#
# The scaffolder has three phases:
#
#   1. Parse CLI arguments (handled by CLI module with OptionParser)
#   2. Resolve dependencies (transitive closure + topological sort)
#   3. Generate files per language (templates for each of 6 languages)
#
# =========================================================================

defmodule CodingAdventures.ScaffoldGenerator do
  @moduledoc """
  Generates CI-ready package scaffolding for the coding-adventures monorepo.

  This is the Elixir port of the Go scaffold-generator. It supports all six
  languages (Python, Go, Ruby, TypeScript, Rust, Elixir) and handles:

  - Name normalization (kebab-case to snake_case, CamelCase, joinedlower)
  - Dependency reading from existing packages' metadata files
  - Transitive closure via BFS
  - Topological sort via Kahn's algorithm (leaf-first install order)
  - File generation with correct BUILD files, README, CHANGELOG
  """

  # =========================================================================
  # Constants
  # =========================================================================

  @valid_languages ~w(python go ruby typescript rust elixir)

  @kebab_case_re ~r/^[a-z][a-z0-9]*(-[a-z0-9]+)*$/

  # =========================================================================
  # Name normalization
  # =========================================================================
  #
  # The input package name is always kebab-case (e.g., "my-package"). Each
  # language has different naming conventions. These functions convert between
  # them.
  #
  # Examples:
  #
  #   to_snake_case("my-package")   => "my_package"
  #   to_camel_case("my-package")   => "MyPackage"
  #   to_joined_lower("my-package") => "mypackage"

  @doc """
  Converts a kebab-case name to snake_case.

  ## Examples

      iex> CodingAdventures.ScaffoldGenerator.to_snake_case("my-package")
      "my_package"

      iex> CodingAdventures.ScaffoldGenerator.to_snake_case("logic-gates")
      "logic_gates"
  """
  @spec to_snake_case(String.t()) :: String.t()
  def to_snake_case(kebab) when is_binary(kebab) do
    String.replace(kebab, "-", "_")
  end

  @doc """
  Converts a kebab-case name to CamelCase (PascalCase).

  Each segment separated by hyphens gets its first letter capitalized,
  then all segments are joined together.

  ## Examples

      iex> CodingAdventures.ScaffoldGenerator.to_camel_case("my-package")
      "MyPackage"

      iex> CodingAdventures.ScaffoldGenerator.to_camel_case("logic-gates")
      "LogicGates"
  """
  @spec to_camel_case(String.t()) :: String.t()
  def to_camel_case(kebab) when is_binary(kebab) do
    kebab
    |> String.split("-")
    |> Enum.map(fn
      "" -> ""
      segment -> String.capitalize(segment)
    end)
    |> Enum.join()
  end

  @doc """
  Converts a kebab-case name to joinedlower (Go package convention).

  Simply removes all hyphens.

  ## Examples

      iex> CodingAdventures.ScaffoldGenerator.to_joined_lower("my-package")
      "mypackage"
  """
  @spec to_joined_lower(String.t()) :: String.t()
  def to_joined_lower(kebab) when is_binary(kebab) do
    String.replace(kebab, "-", "")
  end

  @doc """
  Returns the directory name for a package in a given language.

  Ruby and Elixir use snake_case directories; all other languages
  use kebab-case (the name as-is).

  ## Examples

      iex> CodingAdventures.ScaffoldGenerator.dir_name("my-package", "ruby")
      "my_package"

      iex> CodingAdventures.ScaffoldGenerator.dir_name("my-package", "python")
      "my-package"
  """
  @spec dir_name(String.t(), String.t()) :: String.t()
  def dir_name(kebab, lang) when lang in ["ruby", "elixir"] do
    to_snake_case(kebab)
  end

  def dir_name(kebab, _lang), do: kebab

  # =========================================================================
  # Validation
  # =========================================================================

  @doc "Returns the list of valid language names."
  @spec valid_languages() :: [String.t()]
  def valid_languages, do: @valid_languages

  @doc "Returns the compiled regex for validating kebab-case names."
  @spec kebab_case_regex() :: Regex.t()
  def kebab_case_regex, do: @kebab_case_re

  @doc "Returns true if the given string is a valid kebab-case package name."
  @spec valid_kebab_case?(String.t()) :: boolean()
  def valid_kebab_case?(name) when is_binary(name) do
    Regex.match?(@kebab_case_re, name)
  end

  # =========================================================================
  # Dependency resolution
  # =========================================================================
  #
  # The scaffold generator reads existing packages' metadata to discover
  # their dependencies, then computes the transitive closure and
  # topological sort. This is the most critical feature -- missing
  # transitive deps in BUILD files is the #1 CI failure category.

  @doc """
  Reads the direct local dependencies of a package by parsing its
  metadata files. Returns dependency names in kebab-case.

  Each language stores dependencies differently:
  - Python: BUILD file `-e ../dep` entries
  - Go: go.mod `replace` directives with `=> ../dep`
  - Ruby: Gemfile `path: "../dep"` entries
  - TypeScript: package.json `"file:../"` dependency values
  - Rust: Cargo.toml `path = "../dep"` entries
  - Elixir: mix.exs `path: "../dep"` entries
  """
  @spec read_deps(String.t(), String.t()) :: {:ok, [String.t()]} | {:error, String.t()}
  def read_deps(pkg_dir, lang) do
    result =
      case lang do
        "python" -> read_python_deps(pkg_dir)
        "go" -> read_go_deps(pkg_dir)
        "ruby" -> read_ruby_deps(pkg_dir)
        "typescript" -> read_typescript_deps(pkg_dir)
        "rust" -> read_rust_deps(pkg_dir)
        "elixir" -> read_elixir_deps(pkg_dir)
        other -> {:error, "unknown language: #{other}"}
      end

    result
  end

  # -- Python: reads BUILD file for `-e ../` entries -----------------------

  defp read_python_deps(pkg_dir) do
    build_path = Path.join(pkg_dir, "BUILD")

    case File.read(build_path) do
      {:ok, content} ->
        deps =
          content
          |> String.split("\n")
          |> Enum.flat_map(fn line ->
            # Find ALL -e ../ entries on each line (new format puts them all on one line)
            Regex.scan(~r/-e\s+"?\.\.\/([a-z0-9][a-z0-9._-]*)"?/, line)
            |> Enum.flat_map(fn
              [_, dep] when dep != "." -> [dep]
              _ -> []
            end)
          end)

        {:ok, deps}

      {:error, _} ->
        {:ok, []}
    end
  end

  # -- Go: reads go.mod replace directives for ../dep paths ---------------

  defp read_go_deps(pkg_dir) do
    mod_path = Path.join(pkg_dir, "go.mod")

    case File.read(mod_path) do
      {:ok, content} ->
        deps =
          content
          |> String.split("\n")
          |> Enum.flat_map(fn line ->
            trimmed = String.trim(line)

            if String.contains?(trimmed, "=> ../") do
              case Regex.run(~r/=>\s+\.\.\/(\S+)/, trimmed) do
                [_, dep] -> [dep]
                _ -> []
              end
            else
              []
            end
          end)

        {:ok, deps}

      {:error, _} ->
        {:ok, []}
    end
  end

  # -- Ruby: reads Gemfile for path dependency entries --------------------

  defp read_ruby_deps(pkg_dir) do
    gemfile_path = Path.join(pkg_dir, "Gemfile")

    case File.read(gemfile_path) do
      {:ok, content} ->
        deps =
          content
          |> String.split("\n")
          |> Enum.flat_map(fn line ->
            if String.contains?(line, "path:") and String.contains?(line, "\"../") do
              case Regex.run(~r/"\.\.\/([^"]+)"/, line) do
                [_, dep] ->
                  # Convert snake_case dir back to kebab-case
                  [String.replace(dep, "_", "-")]

                _ ->
                  []
              end
            else
              []
            end
          end)

        {:ok, deps}

      {:error, _} ->
        {:ok, []}
    end
  end

  # -- TypeScript: reads package.json dependencies with "file:../" --------
  #
  # Since we don't have Jason (JSON parser) as a dependency, we use regex
  # to extract "file:../<dep-name>" values from package.json. This is
  # sufficient because the pattern is simple and consistent.

  defp read_typescript_deps(pkg_dir) do
    pkg_json_path = Path.join(pkg_dir, "package.json")

    case File.read(pkg_json_path) do
      {:ok, content} ->
        deps =
          Regex.scan(~r/"file:\.\.\/([^"]+)"/, content)
          |> Enum.map(fn [_, dep] -> dep end)

        {:ok, deps}

      {:error, _} ->
        {:ok, []}
    end
  end

  # -- Rust: reads Cargo.toml for path = "../dep" entries -----------------

  defp read_rust_deps(pkg_dir) do
    cargo_path = Path.join(pkg_dir, "Cargo.toml")

    case File.read(cargo_path) do
      {:ok, content} ->
        deps =
          content
          |> String.split("\n")
          |> Enum.flat_map(fn line ->
            if String.contains?(line, "path = \"../") do
              case Regex.run(~r/path\s*=\s*"\.\.\/([^"]+)"/, line) do
                [_, dep] -> [dep]
                _ -> []
              end
            else
              []
            end
          end)

        {:ok, deps}

      {:error, _} ->
        {:ok, []}
    end
  end

  # -- Elixir: reads mix.exs for path: "../dep" entries -------------------

  defp read_elixir_deps(pkg_dir) do
    mix_path = Path.join(pkg_dir, "mix.exs")

    case File.read(mix_path) do
      {:ok, content} ->
        deps =
          content
          |> String.split("\n")
          |> Enum.flat_map(fn line ->
            if String.contains?(line, "path: \"../") do
              case Regex.run(~r/path:\s*"\.\.\/([^"]+)"/, line) do
                [_, dep] ->
                  # Convert snake_case dir back to kebab-case
                  [String.replace(dep, "_", "-")]

                _ ->
                  []
              end
            else
              []
            end
          end)

        {:ok, deps}

      {:error, _} ->
        {:ok, []}
    end
  end

  # =========================================================================
  # Transitive closure (BFS)
  # =========================================================================
  #
  # Starting from the direct dependencies, we do a breadth-first search
  # to discover ALL transitive dependencies. Each dependency's own
  # dependencies are read from disk.
  #
  # Example: if A depends on B, and B depends on C, then A's transitive
  # closure is {B, C}.

  @doc """
  Computes all transitive dependencies starting from the given direct
  dependencies. Returns the full set sorted alphabetically (not including
  the package itself).

  Uses BFS: we start with the direct deps in a queue, and for each one
  we read its own deps from disk, adding any unseen ones to the queue.
  """
  @spec transitive_closure([String.t()], String.t(), String.t()) ::
          {:ok, [String.t()]} | {:error, String.t()}
  def transitive_closure(direct_deps, lang, base_dir) do
    bfs(direct_deps, lang, base_dir, MapSet.new())
  end

  defp bfs([], _lang, _base_dir, visited) do
    {:ok, visited |> MapSet.to_list() |> Enum.sort()}
  end

  defp bfs([dep | remaining], lang, base_dir, visited) do
    if MapSet.member?(visited, dep) do
      bfs(remaining, lang, base_dir, visited)
    else
      new_visited = MapSet.put(visited, dep)
      dep_dir = Path.join(base_dir, dir_name(dep, lang))

      case read_deps(dep_dir, lang) do
        {:ok, dep_deps} ->
          # Add newly discovered deps to the queue
          new_queue =
            dep_deps
            |> Enum.reject(&MapSet.member?(new_visited, &1))
            |> Kernel.++(remaining)

          bfs(new_queue, lang, base_dir, new_visited)

        {:error, reason} ->
          {:error, "reading deps of #{dep}: #{reason}"}
      end
    end
  end

  # =========================================================================
  # Topological sort (Kahn's algorithm)
  # =========================================================================
  #
  # We need dependencies in leaf-first order: packages that have no
  # dependencies of their own come first. This is the install order
  # needed for BUILD files.
  #
  # Kahn's algorithm:
  #   1. Build a graph: for each dep, find its own deps (within the set)
  #   2. Compute in-degree: how many deps each node has (within the set)
  #   3. Start with nodes that have in-degree 0 (leaves)
  #   4. Remove a leaf, decrease in-degree of nodes that depend on it
  #   5. Repeat until all nodes are placed
  #
  # If the result has fewer nodes than the input, there's a cycle.

  @doc """
  Returns dependencies in leaf-first order (dependencies that have no
  dependencies of their own come first). This is the install order needed
  for BUILD files.

  Uses Kahn's algorithm for topological sorting.
  """
  @spec topological_sort([String.t()], String.t(), String.t()) ::
          {:ok, [String.t()]} | {:error, String.t()}
  def topological_sort(all_deps, lang, base_dir) do
    dep_set = MapSet.new(all_deps)

    # Build adjacency: for each dep, find which deps it depends on (within the set)
    graph =
      all_deps
      |> Enum.reduce(%{}, fn dep, acc ->
        dep_dir = Path.join(base_dir, dir_name(dep, lang))

        dep_deps =
          case read_deps(dep_dir, lang) do
            {:ok, deps} -> Enum.filter(deps, &MapSet.member?(dep_set, &1))
            {:error, _} -> []
          end

        Map.put(acc, dep, dep_deps)
      end)

    # Compute in-degree: count how many dependencies each node has within the set
    in_degree =
      all_deps
      |> Enum.reduce(%{}, fn dep, acc ->
        count = length(Map.get(graph, dep, []))
        Map.put(acc, dep, count)
      end)

    # Start with nodes that have 0 in-degree (leaves -- they depend on nothing in the set)
    initial_queue =
      all_deps
      |> Enum.filter(fn dep -> Map.get(in_degree, dep, 0) == 0 end)
      |> Enum.sort()

    kahns_loop(initial_queue, graph, in_degree, all_deps, [])
  end

  defp kahns_loop([], _graph, _in_degree, all_deps, result_acc) do
    sorted = Enum.reverse(result_acc)

    if length(sorted) != length(all_deps) do
      {:error,
       "circular dependency detected: resolved #{length(sorted)} of #{length(all_deps)} deps"}
    else
      {:ok, sorted}
    end
  end

  defp kahns_loop([node | rest_queue], graph, in_degree, all_deps, result_acc) do
    # Find nodes that depend on this node and decrease their in-degree
    {updated_in_degree, newly_ready} =
      all_deps
      |> Enum.reduce({in_degree, []}, fn dep, {deg_acc, ready_acc} ->
        dep_deps = Map.get(graph, dep, [])

        if Enum.member?(dep_deps, node) do
          new_deg = Map.get(deg_acc, dep, 0) - 1
          updated_deg = Map.put(deg_acc, dep, new_deg)

          if new_deg == 0 do
            {updated_deg, [dep | ready_acc]}
          else
            {updated_deg, ready_acc}
          end
        else
          {deg_acc, ready_acc}
        end
      end)

    # Merge newly ready nodes into queue, keeping sorted order for determinism
    merged_queue = (rest_queue ++ newly_ready) |> Enum.sort()

    kahns_loop(merged_queue, graph, updated_in_degree, all_deps, [node | result_acc])
  end

  # =========================================================================
  # Configuration struct
  # =========================================================================

  defmodule Config do
    @moduledoc """
    Holds the parsed and validated configuration for scaffolding.
    """
    defstruct [
      :package_name,
      :pkg_type,
      :languages,
      :direct_deps,
      :layer,
      :description,
      :dry_run,
      :repo_root
    ]

    @type t :: %__MODULE__{
            package_name: String.t(),
            pkg_type: String.t(),
            languages: [String.t()],
            direct_deps: [String.t()],
            layer: non_neg_integer(),
            description: String.t(),
            dry_run: boolean(),
            repo_root: String.t()
          }
  end

  # =========================================================================
  # Find repo root
  # =========================================================================

  @doc """
  Walks up from the current directory to find the git root.
  Returns `{:ok, path}` or `{:error, reason}`.
  """
  @spec find_repo_root() :: {:ok, String.t()} | {:error, String.t()}
  def find_repo_root do
    find_repo_root(File.cwd!())
  end

  defp find_repo_root("/"), do: {:error, "not inside a git repository"}

  defp find_repo_root(current_dir) do
    if File.dir?(Path.join(current_dir, ".git")) do
      {:ok, current_dir}
    else
      find_repo_root(Path.dirname(current_dir))
    end
  end

  # =========================================================================
  # Main scaffold entry point
  # =========================================================================

  @doc """
  Scaffolds a package for a single language. Returns `:ok` or `{:error, reason}`.
  Output messages are collected in a list of strings.
  """
  @spec scaffold(Config.t(), String.t()) :: {:ok, [String.t()]} | {:error, String.t()}
  def scaffold(%Config{} = cfg, lang) do
    # Determine base directory
    base_category = if cfg.pkg_type == "library", do: "packages", else: "programs"
    base_dir = Path.join([cfg.repo_root, "code", base_category, lang])
    d_name = dir_name(cfg.package_name, lang)
    target_dir = Path.join(base_dir, d_name)

    with :ok <- check_not_exists(target_dir),
         :ok <- validate_deps_exist(cfg.direct_deps, lang, base_dir),
         {:ok, all_deps} <- transitive_closure(cfg.direct_deps, lang, base_dir),
         {:ok, ordered_deps} <- topological_sort(all_deps, lang, base_dir) do
      layer_ctx =
        if cfg.layer > 0, do: "Layer #{cfg.layer} in the computing stack.", else: ""

      if cfg.dry_run do
        messages = [
          "[dry-run] Would create #{lang} package at: #{target_dir}",
          "  Direct deps: #{inspect(cfg.direct_deps)}",
          "  All transitive deps: #{inspect(all_deps)}",
          "  Install order: #{inspect(ordered_deps)}"
        ]

        {:ok, messages}
      else
        File.mkdir_p!(target_dir)

        # Generate language-specific files
        generate_language_files(
          lang,
          target_dir,
          cfg.package_name,
          cfg.description,
          layer_ctx,
          cfg.direct_deps,
          all_deps,
          ordered_deps
        )

        # Generate common files (README, CHANGELOG)
        generate_common_files(
          target_dir,
          cfg.package_name,
          cfg.description,
          cfg.layer,
          cfg.direct_deps
        )

        messages = ["Created #{lang} package at: #{target_dir}"]

        post_messages =
          case lang do
            "rust" ->
              case update_rust_workspace(cfg.repo_root, cfg.package_name) do
                :ok ->
                  [
                    "  Updated code/packages/rust/Cargo.toml workspace members",
                    "  Run: cargo build --workspace (to verify)"
                  ]

                {:error, reason} ->
                  [
                    "  WARNING: Could not update Rust workspace: #{reason}",
                    "  You must manually add \"#{cfg.package_name}\" to code/packages/rust/Cargo.toml members"
                  ]
              end

            "typescript" ->
              ["  Run: cd #{target_dir} && npm install (to generate package-lock.json)"]

            "go" ->
              [
                "  Run: cd #{target_dir} && go mod tidy",
                "  After other packages depend on this, run go mod tidy in those too"
              ]

            _ ->
              []
          end

        {:ok, messages ++ post_messages}
      end
    end
  end

  defp check_not_exists(target_dir) do
    if File.exists?(target_dir) do
      {:error, "directory already exists: #{target_dir}"}
    else
      :ok
    end
  end

  defp validate_deps_exist(direct_deps, lang, base_dir) do
    missing =
      Enum.find(direct_deps, fn dep ->
        dep_dir = Path.join(base_dir, dir_name(dep, lang))
        not File.exists?(dep_dir)
      end)

    case missing do
      nil -> :ok
      dep -> {:error, "dependency #{inspect(dep)} not found for #{lang} at #{Path.join(base_dir, dir_name(dep, lang))}"}
    end
  end

  # =========================================================================
  # Language file dispatcher
  # =========================================================================

  defp generate_language_files(
         lang,
         target_dir,
         pkg_name,
         description,
         layer_ctx,
         direct_deps,
         all_deps,
         ordered_deps
       ) do
    case lang do
      "python" ->
        generate_python(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps)

      "go" ->
        generate_go(target_dir, pkg_name, description, layer_ctx, direct_deps, all_deps)

      "ruby" ->
        generate_ruby(target_dir, pkg_name, description, layer_ctx, direct_deps, all_deps)

      "typescript" ->
        generate_typescript(
          target_dir,
          pkg_name,
          description,
          layer_ctx,
          direct_deps,
          ordered_deps
        )

      "rust" ->
        generate_rust(target_dir, pkg_name, description, layer_ctx, direct_deps)

      "elixir" ->
        generate_elixir(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps)
    end
  end

  # =========================================================================
  # File generation -- Python
  # =========================================================================

  defp generate_python(target_dir, pkg_name, description, layer_ctx, _direct_deps, ordered_deps) do
    snake = to_snake_case(pkg_name)

    pyproject = """
    [build-system]
    requires = ["hatchling"]
    build-backend = "hatchling.build"

    [project]
    name = "coding-adventures-#{pkg_name}"
    version = "0.1.0"
    description = "#{description}"
    requires-python = ">=3.12"
    license = "MIT"
    authors = [{ name = "Adhithya Rajasekaran" }]
    readme = "README.md"

    [project.optional-dependencies]
    dev = ["pytest>=8.0", "pytest-cov>=5.0", "ruff>=0.4", "mypy>=1.10"]

    [tool.hatch.build.targets.wheel]
    packages = ["src/#{snake}"]

    [tool.ruff]
    target-version = "py312"
    line-length = 88

    [tool.ruff.lint]
    select = ["E", "W", "F", "I", "UP", "B", "SIM", "ANN"]

    [tool.pytest.ini_options]
    testpaths = ["tests"]
    addopts = "--cov=#{snake} --cov-report=term-missing --cov-fail-under=80"

    [tool.coverage.run]
    source = ["src/#{snake}"]

    [tool.coverage.report]
    fail_under = 80
    show_missing = true
    """

    init_py = """
    \"\"\"#{pkg_name} -- #{description}

    This package is part of the coding-adventures monorepo, a ground-up
    implementation of the computing stack from transistors to operating systems.
    #{layer_ctx}\"\"\"

    __version__ = "0.1.0"
    """

    test_py = """
    \"\"\"Tests for #{pkg_name}.\"\"\"

    from #{snake} import __version__


    class TestVersion:
        \"\"\"Verify the package is importable and has a version.\"\"\"

        def test_version_exists(self) -> None:
            assert __version__ == "0.1.0"
    """

    install_parts =
      ["python -m pip install"] ++
        Enum.map(ordered_deps, fn dep -> "-e ../#{dep}" end) ++
        ["-e .[dev]", "--quiet"]

    build_lines = [
      Enum.join(install_parts, " "),
      "python -m pytest tests/ -v"
    ]

    build_content = Enum.join(build_lines, "\n") <> "\n"

    # Create directories and write files
    src_dir = Path.join([target_dir, "src", snake])
    test_dir = Path.join(target_dir, "tests")
    File.mkdir_p!(src_dir)
    File.mkdir_p!(test_dir)

    write_dedented(Path.join(target_dir, "pyproject.toml"), pyproject)
    write_dedented(Path.join(src_dir, "__init__.py"), init_py)
    File.write!(Path.join(test_dir, "__init__.py"), "")
    write_dedented(Path.join(test_dir, "test_#{snake}.py"), test_py)
    File.write!(Path.join(target_dir, "BUILD"), build_content)
  end

  # =========================================================================
  # File generation -- Go
  # =========================================================================

  defp generate_go(target_dir, pkg_name, description, layer_ctx, direct_deps, all_transitive_deps) do
    go_pkg = to_joined_lower(pkg_name)
    snake = to_snake_case(pkg_name)

    go_mod_parts = [
      "module github.com/adhithyan15/coding-adventures/code/packages/go/#{pkg_name}\n",
      "go 1.26\n"
    ]

    go_mod_deps =
      if length(direct_deps) > 0 do
        require_lines =
          direct_deps
          |> Enum.map(fn dep ->
            "\tgithub.com/adhithyan15/coding-adventures/code/packages/go/#{dep} v0.0.0"
          end)
          |> Enum.join("\n")

        replace_lines =
          all_transitive_deps
          |> Enum.map(fn dep ->
            "\tgithub.com/adhithyan15/coding-adventures/code/packages/go/#{dep} => ../#{dep}"
          end)
          |> Enum.join("\n")

        "\nrequire (\n#{require_lines}\n)\n\nreplace (\n#{replace_lines}\n)\n"
      else
        ""
      end

    go_mod = Enum.join(go_mod_parts, "\n") <> go_mod_deps

    src_file = """
    // Package #{go_pkg} provides #{description}.
    //
    // This package is part of the coding-adventures monorepo, a ground-up
    // implementation of the computing stack from transistors to operating systems.
    // #{layer_ctx}
    package #{go_pkg}
    """

    test_file = """
    package #{go_pkg}

    import "testing"

    func TestPackageLoads(t *testing.T) {
    \tt.Log("#{pkg_name} package loaded successfully")
    }
    """

    build_content = "go test ./... -v -cover\n"

    File.write!(Path.join(target_dir, "go.mod"), go_mod)
    write_dedented(Path.join(target_dir, "#{snake}.go"), src_file)
    write_dedented(Path.join(target_dir, "#{snake}_test.go"), test_file)
    File.write!(Path.join(target_dir, "BUILD"), build_content)
  end

  # =========================================================================
  # File generation -- Ruby
  # =========================================================================

  defp generate_ruby(target_dir, pkg_name, description, _layer_ctx, direct_deps, all_transitive_deps) do
    snake = to_snake_case(pkg_name)
    camel = to_camel_case(pkg_name)

    dep_specs =
      direct_deps
      |> Enum.map(fn dep ->
        dep_snake = to_snake_case(dep)
        "  spec.add_dependency \"coding_adventures_#{dep_snake}\", \"~> 0.1\""
      end)
      |> Enum.join("\n")

    dep_specs_section = if dep_specs != "", do: dep_specs <> "\n", else: ""

    gemspec = """
    # frozen_string_literal: true

    require_relative "lib/coding_adventures/#{snake}/version"

    Gem::Specification.new do |spec|
      spec.name          = "coding_adventures_#{snake}"
      spec.version       = CodingAdventures::#{camel}::VERSION
      spec.authors       = ["Adhithya Rajasekaran"]
      spec.summary       = "#{description}"
      spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
      spec.license       = "MIT"
      spec.required_ruby_version = ">= 3.3.0"

      spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
      spec.require_paths = ["lib"]

      spec.metadata = {
        "source_code_uri"        => "https://github.com/adhithyan15/coding-adventures",
        "rubygems_mfa_required"  => "true"
      }

    #{dep_specs_section}  spec.add_development_dependency "minitest", "~> 5.0"
      spec.add_development_dependency "rake", "~> 13.0"
    end
    """

    gemfile_deps =
      if length(all_transitive_deps) > 0 do
        dep_lines =
          all_transitive_deps
          |> Enum.map(fn dep ->
            dep_snake = to_snake_case(dep)
            "gem \"coding_adventures_#{dep_snake}\", path: \"../#{dep_snake}\""
          end)
          |> Enum.join("\n")

        "\n# All transitive path dependencies must be listed here.\n# Bundler needs to know where to find each gem locally.\n#{dep_lines}\n"
      else
        ""
      end

    gemfile =
      "# frozen_string_literal: true\n\nsource \"https://rubygems.org\"\ngemspec\n#{gemfile_deps}"

    rakefile = """
    # frozen_string_literal: true

    require "rake/testtask"

    Rake::TestTask.new(:test) do |t|
      t.libs << "test"
      t.libs << "lib"
      t.test_files = FileList["test/**/test_*.rb"]
    end

    task default: :test
    """

    dep_requires =
      if length(direct_deps) > 0 do
        header =
          "# IMPORTANT: Require dependencies FIRST, before own modules.\n# Ruby loads files in require order. If our modules reference\n# constants from dependencies, those gems must be loaded first.\n"

        requires =
          direct_deps
          |> Enum.map(fn dep -> "require \"coding_adventures_#{to_snake_case(dep)}\"" end)
          |> Enum.join("\n")

        header <> requires <> "\n\n"
      else
        ""
      end

    entry_point =
      "# frozen_string_literal: true\n\n#{dep_requires}require_relative \"coding_adventures/#{snake}/version\"\n\nmodule CodingAdventures\n  # #{description}\n  module #{camel}\n  end\nend\n"

    version_rb = """
    # frozen_string_literal: true

    module CodingAdventures
      module #{camel}
        VERSION = "0.1.0"
      end
    end
    """

    test_rb = """
    # frozen_string_literal: true

    require "minitest/autorun"
    require "coding_adventures_#{snake}"

    class Test#{camel} < Minitest::Test
      def test_version_exists
        refute_nil CodingAdventures::#{camel}::VERSION
      end
    end
    """

    build_content = "bundle install --quiet\nbundle exec rake test\n"

    # Create directories and write files
    lib_dir = Path.join([target_dir, "lib", "coding_adventures", snake])
    test_dir = Path.join(target_dir, "test")
    File.mkdir_p!(lib_dir)
    File.mkdir_p!(test_dir)

    write_dedented(
      Path.join(target_dir, "coding_adventures_#{snake}.gemspec"),
      gemspec
    )

    File.write!(Path.join(target_dir, "Gemfile"), gemfile)
    write_dedented(Path.join(target_dir, "Rakefile"), rakefile)

    File.write!(
      Path.join(target_dir, "lib/coding_adventures_#{snake}.rb"),
      entry_point
    )

    write_dedented(
      Path.join(target_dir, "lib/coding_adventures/#{snake}/version.rb"),
      version_rb
    )

    write_dedented(Path.join(test_dir, "test_#{snake}.rb"), test_rb)
    File.write!(Path.join(target_dir, "BUILD"), build_content)
  end

  # =========================================================================
  # File generation -- TypeScript
  # =========================================================================

  defp generate_typescript(
         target_dir,
         pkg_name,
         description,
         layer_ctx,
         direct_deps,
         ordered_deps
       ) do
    deps_json =
      if length(direct_deps) > 0 do
        entries =
          direct_deps
          |> Enum.map(fn dep ->
            "    \"@coding-adventures/#{dep}\": \"file:../#{dep}\""
          end)
          |> Enum.join(",\n")

        entries
      else
        ""
      end

    package_json = """
    {
      "name": "@coding-adventures/#{pkg_name}",
      "version": "0.1.0",
      "description": "#{description}",
      "type": "module",
      "main": "src/index.ts",
      "scripts": {
        "build": "tsc",
        "test": "vitest run",
        "test:coverage": "vitest run --coverage"
      },
      "author": "Adhithya Rajasekaran",
      "license": "MIT",
      "dependencies": {
    #{deps_json}
      },
      "devDependencies": {
        "typescript": "^5.0.0",
        "vitest": "^3.0.0",
        "@vitest/coverage-v8": "^3.0.0"
      }
    }
    """

    tsconfig = """
    {
      "compilerOptions": {
        "target": "ES2022",
        "module": "ESNext",
        "moduleResolution": "bundler",
        "strict": true,
        "esModuleInterop": true,
        "skipLibCheck": true,
        "outDir": "dist",
        "rootDir": "src",
        "declaration": true
      },
      "include": ["src"]
    }
    """

    vitest_config = """
    import { defineConfig } from "vitest/config";

    export default defineConfig({
      test: {
        coverage: {
          provider: "v8",
          thresholds: {
            lines: 80,
          },
        },
      },
    });
    """

    index_ts = """
    /**
     * @coding-adventures/#{pkg_name}
     *
     * #{description}
     *
     * This package is part of the coding-adventures monorepo, a ground-up
     * implementation of the computing stack from transistors to operating systems.
     * #{layer_ctx}
     */

    export const VERSION = "0.1.0";
    """

    test_ts = """
    import { describe, it, expect } from "vitest";
    import { VERSION } from "../src/index.js";

    describe("#{pkg_name}", () => {
      it("has a version", () => {
        expect(VERSION).toBe("0.1.0");
      });
    });
    """

    build_content = "npm ci --quiet\nnpx vitest run --coverage\n"

    # Create directories and write files
    src_dir = Path.join(target_dir, "src")
    tests_dir = Path.join(target_dir, "tests")
    File.mkdir_p!(src_dir)
    File.mkdir_p!(tests_dir)

    write_dedented(Path.join(target_dir, "package.json"), package_json)
    write_dedented(Path.join(target_dir, "tsconfig.json"), tsconfig)
    write_dedented(Path.join(target_dir, "vitest.config.ts"), vitest_config)
    write_dedented(Path.join(src_dir, "index.ts"), index_ts)
    write_dedented(Path.join(tests_dir, "#{pkg_name}.test.ts"), test_ts)
    File.write!(Path.join(target_dir, "BUILD"), build_content)
  end

  # =========================================================================
  # File generation -- Rust
  # =========================================================================

  defp generate_rust(target_dir, pkg_name, description, layer_ctx, direct_deps) do
    dep_lines =
      direct_deps
      |> Enum.map(fn dep -> "#{dep} = { path = \"../#{dep}\" }" end)
      |> Enum.join("\n")

    dep_section = if dep_lines != "", do: dep_lines <> "\n", else: ""

    cargo_toml = """
    [package]
    name = "#{pkg_name}"
    version = "0.1.0"
    edition = "2021"
    description = "#{description}"

    [dependencies]
    #{dep_section}\
    """

    lib_rs = """
    //! # #{pkg_name}
    //!
    //! #{description}
    //!
    //! This crate is part of the coding-adventures monorepo, a ground-up
    //! implementation of the computing stack from transistors to operating systems.
    //! #{layer_ctx}

    #[cfg(test)]
    mod tests {
        #[test]
        fn it_loads() {
            assert!(true, "#{pkg_name} crate loaded successfully");
        }
    }
    """

    build_content = "cargo test -p #{pkg_name} -- --nocapture\n"

    # Create directories and write files
    src_dir = Path.join(target_dir, "src")
    File.mkdir_p!(src_dir)

    write_dedented(Path.join(target_dir, "Cargo.toml"), cargo_toml)
    write_dedented(Path.join(src_dir, "lib.rs"), lib_rs)
    File.write!(Path.join(target_dir, "BUILD"), build_content)
  end

  # =========================================================================
  # File generation -- Elixir
  # =========================================================================

  defp generate_elixir(target_dir, pkg_name, description, layer_ctx, direct_deps, ordered_deps) do
    snake = to_snake_case(pkg_name)
    camel = to_camel_case(pkg_name)

    dep_entries =
      direct_deps
      |> Enum.map(fn dep ->
        dep_snake = to_snake_case(dep)
        "      {:coding_adventures_#{dep_snake}, path: \"../#{dep_snake}\"}"
      end)
      |> Enum.join(",\n")

    dep_section = if dep_entries != "", do: dep_entries <> "\n", else: ""

    mix_exs = """
    defmodule CodingAdventures.#{camel}.MixProject do
      use Mix.Project

      def project do
        [
          app: :coding_adventures_#{snake},
          version: "0.1.0",
          elixir: "~> 1.14",
          start_permanent: Mix.env() == :prod,
          deps: deps(),
          test_coverage: [
            summary: [threshold: 80]
          ]
        ]
      end

      def application do
        [
          extra_applications: [:logger]
        ]
      end

      defp deps do
        [
    #{dep_section}    ]
      end
    end
    """

    lib_ex = """
    defmodule CodingAdventures.#{camel} do
      @moduledoc \"\"\"
      #{description}

      This module is part of the coding-adventures monorepo, a ground-up
      implementation of the computing stack from transistors to operating systems.
      #{layer_ctx}
      \"\"\"
    end
    """

    test_exs = """
    defmodule CodingAdventures.#{camel}Test do
      use ExUnit.Case

      test "module loads" do
        assert Code.ensure_loaded?(CodingAdventures.#{camel})
      end
    end
    """

    test_helper = "ExUnit.start()\n"

    build_content =
      if length(ordered_deps) > 0 do
        parts =
          Enum.map(ordered_deps, fn dep ->
            dep_snake = to_snake_case(dep)
            "cd ../#{dep_snake} && mix deps.get --quiet && mix compile --quiet"
          end) ++
            ["cd ../#{snake} && mix deps.get --quiet && mix test --cover"]

        Enum.join(parts, " && \\\n") <> "\n"
      else
        "mix deps.get --quiet && mix test --cover\n"
      end

    # Create directories and write files
    lib_dir = Path.join([target_dir, "lib", "coding_adventures"])
    test_dir = Path.join(target_dir, "test")
    File.mkdir_p!(lib_dir)
    File.mkdir_p!(test_dir)

    write_dedented(Path.join(target_dir, "mix.exs"), mix_exs)
    write_dedented(Path.join(lib_dir, "#{snake}.ex"), lib_ex)
    write_dedented(Path.join(test_dir, "#{snake}_test.exs"), test_exs)
    File.write!(Path.join(test_dir, "test_helper.exs"), test_helper)
    File.write!(Path.join(target_dir, "BUILD"), build_content)
  end

  # =========================================================================
  # Common files (README, CHANGELOG)
  # =========================================================================

  @doc """
  Generates README.md and CHANGELOG.md in the target directory.
  """
  def generate_common_files(target_dir, pkg_name, description, layer, direct_deps) do
    today = Date.utc_today() |> Date.to_iso8601()

    changelog = """
    # Changelog

    All notable changes to this package will be documented in this file.

    ## [0.1.0] - #{today}

    ### Added

    - Initial package scaffolding generated by scaffold-generator
    """

    layer_section =
      if layer > 0 do
        "\n## Layer #{layer}\n\nThis package is part of Layer #{layer} of the coding-adventures computing stack.\n"
      else
        ""
      end

    deps_section =
      if length(direct_deps) > 0 do
        dep_lines = Enum.map(direct_deps, fn dep -> "- #{dep}" end) |> Enum.join("\n")
        "\n## Dependencies\n\n#{dep_lines}\n"
      else
        ""
      end

    readme =
      "# #{pkg_name}\n\n#{description}\n#{layer_section}#{deps_section}\n## Development\n\n```bash\n# Run tests\nbash BUILD\n```\n"

    write_dedented(Path.join(target_dir, "CHANGELOG.md"), changelog)
    File.write!(Path.join(target_dir, "README.md"), readme)
  end

  # =========================================================================
  # Rust workspace integration
  # =========================================================================

  @doc """
  Adds a new crate to the workspace Cargo.toml members list.
  """
  def update_rust_workspace(repo_root, pkg_name) do
    workspace_path = Path.join([repo_root, "code", "packages", "rust", "Cargo.toml"])

    case File.read(workspace_path) do
      {:ok, content} ->
        if String.contains?(content, "\"#{pkg_name}\"") do
          :ok
        else
          case String.split(content, "members = [", parts: 2) do
            [before, remainder] ->
              case String.split(remainder, "]", parts: 2) do
                [members_body, closing] ->
                  new_content =
                    before <>
                      "members = [" <>
                      members_body <> "  \"#{pkg_name}\",\n]" <> closing

                  File.write!(workspace_path, new_content)
                  :ok

                _ ->
                  {:error, "cannot find closing ] for members array"}
              end

            _ ->
              {:error, "cannot find members = [ in workspace Cargo.toml"}
          end
        end

      {:error, reason} ->
        {:error, "cannot read workspace Cargo.toml: #{inspect(reason)}"}
    end
  end

  # =========================================================================
  # Helpers
  # =========================================================================

  # Writes content after stripping the common leading whitespace (heredoc indent).
  # This lets us write template strings with nice indentation in the source code
  # without that indentation ending up in the generated files.
  defp write_dedented(path, content) do
    dedented = dedent(content)
    File.write!(path, dedented)
  end

  @doc """
  Removes common leading whitespace from a multi-line string.
  This is used to de-indent heredoc-style template strings so that
  generated files don't have unwanted leading spaces.
  """
  @spec dedent(String.t()) :: String.t()
  def dedent(text) do
    lines = String.split(text, "\n")

    # Find minimum indentation of non-empty lines
    min_indent =
      lines
      |> Enum.reject(fn line -> String.trim(line) == "" end)
      |> Enum.map(fn line ->
        String.length(line) - String.length(String.trim_leading(line))
      end)
      |> Enum.min(fn -> 0 end)

    if min_indent == 0 do
      text
    else
      lines
      |> Enum.map(fn line ->
        if String.trim(line) == "" do
          ""
        else
          String.slice(line, min_indent..-1//1)
        end
      end)
      |> Enum.join("\n")
    end
  end
end
