defmodule BuildTool.Resolver do
  @moduledoc """
  Reads package metadata files (pyproject.toml, .gemspec, go.mod,
  package.json, Cargo.toml, mix.exs, .rockspec) and extracts internal
  dependencies, building a directed graph.

  ## Why dependency resolution matters

  In a monorepo, packages often depend on each other. If package B depends
  on package A, we must build A before B. The resolver reads each package's
  metadata file to discover these relationships, then encodes them as edges
  in a directed graph.

  ## Dependency naming conventions

  Each language ecosystem uses a different naming convention for packages:

    - **Python**: pyproject.toml uses "coding-adventures-" prefix with hyphens.
      `"coding-adventures-logic-gates"` maps to `"python/logic-gates"`.

    - **Ruby**: .gemspec uses "coding_adventures_" prefix with underscores.
      `"coding_adventures_logic_gates"` maps to `"ruby/logic_gates"`.

    - **Go**: go.mod uses full module paths. We map based on the last path
      component: `"go/directed-graph"`.

    - **TypeScript**: package.json uses "@coding-adventures/" scoped npm names.
      `"@coding-adventures/logic-gates"` maps to `"typescript/logic-gates"`.

    - **Rust**: Cargo.toml uses the crate name (the directory basename).
      `"logic-gates"` maps to `"rust/logic-gates"`.

    - **Elixir**: mix.exs uses atom names with "coding_adventures_" prefix.
      `":coding_adventures_logic_gates"` maps to `"elixir/logic-gates"`.

    - **Lua**: .rockspec uses "coding-adventures-" prefix with hyphens.
      `"coding-adventures-logic-gates"` maps to `"lua/logic_gates"`.

  External dependencies (those not matching the monorepo prefix) are silently
  skipped — we only care about internal build ordering.

  ## The directed graph

  Edges go FROM dependency TO dependent: if B depends on A, the edge is
  A -> B. This convention means "A must be built before B", and
  `independent_groups/1` naturally produces the correct build order.
  """

  alias BuildTool.DirectedGraph

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Parses package metadata to discover dependencies and builds a directed graph.

  The graph contains all discovered packages as nodes. Edges represent build
  ordering: an edge from A to B means "A must be built before B" (because B
  depends on A). External dependencies — those not found among the discovered
  packages — are silently skipped.

  ## Example

      iex> packages = [
      ...>   %{name: "python/logic-gates", path: "/repo/code/packages/python/logic-gates", language: "python", build_commands: []},
      ...>   %{name: "python/arithmetic", path: "/repo/code/packages/python/arithmetic", language: "python", build_commands: []}
      ...> ]
      iex> graph = BuildTool.Resolver.resolve_dependencies(packages)
      iex> BuildTool.DirectedGraph.has_node?(graph, "python/logic-gates")
      true
  """
  def resolve_dependencies(packages) do
    # First, add all packages as nodes. Even packages with no dependencies
    # need to be in the graph so they appear in independent_groups().
    graph =
      Enum.reduce(packages, DirectedGraph.new(), fn pkg, g ->
        DirectedGraph.add_node(g, pkg.name)
      end)

    # Build the ecosystem-specific name mapping table.
    known_names_by_scope =
      packages
      |> Enum.map(&dependency_scope(&1.language))
      |> Enum.uniq()
      |> Map.new(fn scope -> {scope, build_known_names(packages, scope)} end)

    # Parse dependencies for each package and add edges.
    Enum.reduce(packages, graph, fn pkg, g ->
      deps = parse_deps(pkg, Map.fetch!(known_names_by_scope, dependency_scope(pkg.language)))

      Enum.reduce(deps, g, fn dep_name, g2 ->
        # Edge direction: dep -> pkg means "dep must be built before pkg".
        DirectedGraph.add_edge(g2, dep_name, pkg.name)
      end)
    end)
  end

  @doc """
  Builds a mapping from ecosystem-specific dependency names to internal
  package names.

  This mapping is the "Rosetta Stone" of the build system. Each language
  ecosystem uses its own naming convention, and this function converts
  between them.

  Exported for testing.
  """
  def build_known_names(packages, language \\ nil) do
    Enum.reduce(packages, %{}, fn pkg, acc ->
      if language != nil and not in_dependency_scope?(pkg.language, language) do
        acc
      else
      case pkg.language do
        "python" ->
          # Convert dir name to PyPI name: "logic-gates" -> "coding-adventures-logic-gates"
          pypi_name = "coding-adventures-" <> String.downcase(Path.basename(pkg.path))
          Map.put(acc, pypi_name, pkg.name)

        "ruby" ->
          # Convert dir name to gem name: "logic_gates" -> "coding_adventures_logic_gates"
          gem_name = "coding_adventures_" <> String.downcase(Path.basename(pkg.path))
          Map.put(acc, gem_name, pkg.name)

        "go" ->
          # For Go, read the module path from go.mod.
          go_mod = Path.join(pkg.path, "go.mod")

          case File.read(go_mod) do
            {:ok, data} ->
              data
              |> String.split("\n")
              |> Enum.find_value(acc, fn line ->
                if String.starts_with?(line, "module ") do
                  module_path =
                    line
                    |> String.trim_leading("module ")
                    |> String.trim()
                    |> String.downcase()

                  Map.put(acc, module_path, pkg.name)
                end
              end)

            {:error, _} ->
              acc
          end

        "typescript" ->
          # Convert dir name to npm scoped name: "logic-gates" -> "@coding-adventures/logic-gates"
          npm_name = "@coding-adventures/" <> String.downcase(Path.basename(pkg.path))
          Map.put(acc, npm_name, pkg.name)

        "rust" ->
          # Rust crate names use the directory name directly (kebab-case).
          crate_name = String.downcase(Path.basename(pkg.path))

          acc
          |> Map.put(crate_name, pkg.name)
          |> maybe_put(read_cargo_package_name(pkg.path), pkg.name)

        "wasm" ->
          crate_name = String.downcase(Path.basename(pkg.path))

          acc
          |> Map.put(crate_name, pkg.name)
          |> maybe_put(read_cargo_package_name(pkg.path), pkg.name)

        "elixir" ->
          # Elixir mix names: "logic-gates" -> "coding_adventures_logic_gates"
          app_name =
            "coding_adventures_" <>
              String.replace(String.downcase(Path.basename(pkg.path)), "-", "_")

          Map.put(acc, app_name, pkg.name)

        "lua" ->
          # Lua rockspec names: "logic_gates" -> "coding-adventures-logic-gates"
          # Directory names use underscores, but rockspec names use hyphens
          # with a "coding-adventures-" prefix, matching the Python convention.
          rockspec_name =
            "coding-adventures-" <>
              String.replace(String.downcase(Path.basename(pkg.path)), "_", "-")

          Map.put(acc, rockspec_name, pkg.name)

        "perl" ->
          # Perl CPAN dist names use hyphens: "logic-gates" -> "coding-adventures-logic-gates"
          # This matches the Python convention exactly.
          cpan_name = "coding-adventures-" <> String.downcase(Path.basename(pkg.path))
          Map.put(acc, cpan_name, pkg.name)

        "haskell" ->
          cabal_name = "coding-adventures-" <> String.downcase(Path.basename(pkg.path))
          Map.put(acc, cabal_name, pkg.name)

        "csharp" ->
          Map.put(acc, String.downcase(Path.basename(pkg.path)), pkg.name)

        "fsharp" ->
          Map.put(acc, String.downcase(Path.basename(pkg.path)), pkg.name)

        "dotnet" ->
          Map.put(acc, String.downcase(Path.basename(pkg.path)), pkg.name)

        _ ->
          acc
      end
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Language-specific parsers
  # ---------------------------------------------------------------------------
  #
  # Each parser reads a language's metadata file and extracts internal
  # dependencies. The approach is regex-based — we don't need a full parser
  # for TOML, JSON, or Ruby DSLs because we only need to extract a narrow
  # slice of information.

  defp parse_deps(pkg, known_names) do
    case pkg.language do
      "python" -> parse_python_deps(pkg, known_names)
      "ruby" -> parse_ruby_deps(pkg, known_names)
      "go" -> parse_go_deps(pkg, known_names)
      "typescript" -> parse_typescript_deps(pkg, known_names)
      "rust" -> parse_rust_deps(pkg, known_names)
      "wasm" -> parse_rust_deps(pkg, known_names)
      "elixir" -> parse_elixir_deps(pkg, known_names)
      "lua" -> parse_lua_deps(pkg, known_names)
      "perl" -> parse_perl_deps(pkg, known_names)
      "haskell" -> parse_haskell_deps(pkg, known_names)
      "csharp" -> parse_dotnet_deps(pkg, known_names)
      "fsharp" -> parse_dotnet_deps(pkg, known_names)
      "dotnet" -> parse_dotnet_deps(pkg, known_names)
      _ -> []
    end
  end

  # ---------------------------------------------------------------------------
  # Python: pyproject.toml
  # ---------------------------------------------------------------------------
  #
  # Python's pyproject.toml declares dependencies in:
  #
  #   dependencies = [
  #       "coding-adventures-logic-gates>=0.1.0",
  #       "coding-adventures-arithmetic",
  #   ]
  #
  # We scan for the dependencies array and extract quoted strings, stripping
  # version specifiers.

  defp parse_python_deps(pkg, known_names) do
    pyproject = Path.join(pkg.path, "pyproject.toml")

    case File.read(pyproject) do
      {:ok, data} ->
        data
        |> String.split("\n")
        |> parse_python_deps_lines(known_names, false, [])

      {:error, _} ->
        []
    end
  end

  defp parse_python_deps_lines([], _known, _in_deps, acc), do: Enum.reverse(acc)

  defp parse_python_deps_lines([line | rest], known, in_deps, acc) do
    trimmed = String.trim(line)

    cond do
      not in_deps ->
        if String.starts_with?(trimmed, "dependencies") and String.contains?(trimmed, "=") do
          [_, after_eq] = String.split(trimmed, "=", parts: 2)
          after_eq = String.trim(after_eq)

          if String.starts_with?(after_eq, "[") do
            if String.contains?(after_eq, "]") do
              # Single-line array
              new_deps = extract_quoted_deps(after_eq, known)
              Enum.reverse(new_deps ++ acc)
            else
              # Multi-line array starts
              new_deps = extract_quoted_deps(after_eq, known)
              parse_python_deps_lines(rest, known, true, new_deps ++ acc)
            end
          else
            parse_python_deps_lines(rest, known, false, acc)
          end
        else
          parse_python_deps_lines(rest, known, false, acc)
        end

      true ->
        if String.contains?(trimmed, "]") do
          new_deps = extract_quoted_deps(trimmed, known)
          Enum.reverse(new_deps ++ acc)
        else
          new_deps = extract_quoted_deps(trimmed, known)
          parse_python_deps_lines(rest, known, true, new_deps ++ acc)
        end
    end
  end

  # Extracts quoted dependency names from a line and maps them to internal
  # package names. Version specifiers (>=, <, etc.) are stripped.
  defp extract_quoted_deps(line, known_names) do
    ~r/["']([^"']+)["']/
    |> Regex.scan(line)
    |> Enum.flat_map(fn
      [_, dep_str] ->
        # Strip version specifiers: split on >=, <=, >, <, ==, !=, ~=, ;, spaces
        dep_name =
          dep_str
          |> String.split(~r/[>=<!~\s;]/, parts: 2)
          |> hd()
          |> String.trim()
          |> String.downcase()

        case Map.get(known_names, dep_name) do
          nil -> []
          pkg_name -> [pkg_name]
        end

      _ ->
        []
    end)
  end

  # ---------------------------------------------------------------------------
  # Ruby: .gemspec
  # ---------------------------------------------------------------------------
  #
  # Ruby gemspecs declare dependencies with:
  #
  #   spec.add_dependency "coding_adventures_logic_gates"
  #
  # We use a regex to find these lines.

  defp parse_ruby_deps(pkg, known_names) do
    # Find .gemspec files in the package directory
    case File.ls(pkg.path) do
      {:ok, entries} ->
        gemspec =
          Enum.find(entries, fn entry -> String.ends_with?(entry, ".gemspec") end)

        if gemspec do
          gemspec_path = Path.join(pkg.path, gemspec)

          case File.read(gemspec_path) do
            {:ok, data} ->
              ~r/spec\.add_dependency\s+"([^"]+)"/
              |> Regex.scan(data)
              |> Enum.flat_map(fn
                [_, gem_name] ->
                  gem_name = gem_name |> String.trim() |> String.downcase()

                  case Map.get(known_names, gem_name) do
                    nil -> []
                    pkg_name -> [pkg_name]
                  end

                _ ->
                  []
              end)

            {:error, _} ->
              []
          end
        else
          []
        end

      {:error, _} ->
        []
    end
  end

  # ---------------------------------------------------------------------------
  # Go: go.mod
  # ---------------------------------------------------------------------------
  #
  # Go modules declare dependencies in go.mod with:
  #
  #   require github.com/user/repo/pkg v1.0.0
  #
  # or in a block:
  #
  #   require (
  #       github.com/user/repo/pkg v1.0.0
  #   )

  defp parse_go_deps(pkg, known_names) do
    go_mod = Path.join(pkg.path, "go.mod")

    case File.read(go_mod) do
      {:ok, data} ->
        data
        |> String.split("\n")
        |> parse_go_mod_lines(known_names, false, [])

      {:error, _} ->
        []
    end
  end

  defp parse_go_mod_lines([], _known, _in_block, acc), do: Enum.reverse(acc)

  defp parse_go_mod_lines([line | rest], known, in_block, acc) do
    stripped = String.trim(line)

    cond do
      stripped == "require (" ->
        parse_go_mod_lines(rest, known, true, acc)

      stripped == ")" ->
        parse_go_mod_lines(rest, known, false, acc)

      in_block or String.starts_with?(stripped, "require ") ->
        clean =
          stripped
          |> String.trim_leading("require ")
          |> String.trim()

        parts = String.split(clean, ~r/\s+/, parts: 2)

        case parts do
          [module_path | _] ->
            module_path = String.downcase(module_path)

            case Map.get(known, module_path) do
              nil -> parse_go_mod_lines(rest, known, in_block, acc)
              pkg_name -> parse_go_mod_lines(rest, known, in_block, [pkg_name | acc])
            end

          _ ->
            parse_go_mod_lines(rest, known, in_block, acc)
        end

      true ->
        parse_go_mod_lines(rest, known, in_block, acc)
    end
  end

  # ---------------------------------------------------------------------------
  # TypeScript: package.json
  # ---------------------------------------------------------------------------
  #
  # TypeScript packages declare dependencies in package.json:
  #
  #   "dependencies": {
  #       "@coding-adventures/logic-gates": "file:../logic-gates"
  #   }

  defp parse_typescript_deps(pkg, known_names) do
    package_json = Path.join(pkg.path, "package.json")

    case File.read(package_json) do
      {:ok, data} ->
        data
        |> String.split("\n")
        |> parse_ts_lines(known_names, false, [])

      {:error, _} ->
        []
    end
  end

  defp parse_ts_lines([], _known, _in_deps, acc), do: Enum.reverse(acc)

  defp parse_ts_lines([line | rest], known, in_deps, acc) do
    trimmed = String.trim(line)

    cond do
      not in_deps ->
        if String.contains?(trimmed, "\"dependencies\"") and String.contains?(trimmed, "{") do
          parse_ts_lines(rest, known, true, acc)
        else
          parse_ts_lines(rest, known, false, acc)
        end

      String.contains?(trimmed, "}") ->
        parse_ts_lines(rest, known, false, acc)

      true ->
        new_deps =
          ~r/"(@coding-adventures\/[^"]+)"/
          |> Regex.scan(trimmed)
          |> Enum.flat_map(fn
            [_, dep_name] ->
              dep_name = String.downcase(dep_name)

              case Map.get(known, dep_name) do
                nil -> []
                pkg_name -> [pkg_name]
              end

            _ ->
              []
          end)

        parse_ts_lines(rest, known, true, new_deps ++ acc)
    end
  end

  # ---------------------------------------------------------------------------
  # Rust: Cargo.toml
  # ---------------------------------------------------------------------------
  #
  # Rust Cargo.toml declares workspace-local dependencies with path references:
  #
  #   [dependencies]
  #   logic-gates = { path = "../logic-gates" }

  defp parse_rust_deps(pkg, known_names) do
    cargo_toml = Path.join(pkg.path, "Cargo.toml")

    case File.read(cargo_toml) do
      {:ok, data} ->
        data
        |> String.split("\n")
        |> parse_rust_lines(known_names, false, [])

      {:error, _} ->
        []
    end
  end

  defp parse_dotnet_deps(pkg, known_names) do
    pkg.path
    |> project_files()
    |> Enum.flat_map(fn project_file ->
      case File.read(project_file) do
        {:ok, data} ->
          ~r/<ProjectReference\s+Include\s*=\s*"\.\.[\\\/]+([^\/\\"]+)[\\\/][^"]*"/
          |> Regex.scan(data)
          |> Enum.flat_map(fn
            [_, dep_dir] ->
              dep_dir = String.downcase(dep_dir)

              cond do
                String.contains?(dep_dir, "/") or String.contains?(dep_dir, "\\") or dep_dir == ".." ->
                  []

                true ->
                  case Map.get(known_names, dep_dir) do
                    nil -> []
                    pkg_name -> [pkg_name]
                  end
              end

            _ ->
              []
          end)

        {:error, _} ->
          []
      end
    end)
  end

  defp parse_rust_lines([], _known, _in_deps, acc), do: Enum.reverse(acc)

  defp parse_rust_lines([line | rest], known, in_deps, acc) do
    trimmed = String.trim(line)

    cond do
      String.starts_with?(trimmed, "[") ->
        parse_rust_lines(rest, known, trimmed == "[dependencies]", acc)

      not in_deps ->
        parse_rust_lines(rest, known, false, acc)

      String.contains?(trimmed, "path") and String.contains?(trimmed, "=") ->
        [crate_name | _] = String.split(trimmed, "=", parts: 2)
        crate_name = crate_name |> String.trim() |> String.downcase()

        case Map.get(known, crate_name) do
          nil -> parse_rust_lines(rest, known, true, acc)
          pkg_name -> parse_rust_lines(rest, known, true, [pkg_name | acc])
        end

      true ->
        parse_rust_lines(rest, known, true, acc)
    end
  end

  # ---------------------------------------------------------------------------
  # Elixir: mix.exs
  # ---------------------------------------------------------------------------
  #
  # Elixir mix.exs declares internal path dependencies usually like:
  #
  #   {:coding_adventures_logic_gates, path: "../logic-gates"}
  #
  # We use a regex to capture the atom name starting with `coding_adventures_`.

  defp parse_elixir_deps(pkg, known_names) do
    mix_exs = Path.join(pkg.path, "mix.exs")

    case File.read(mix_exs) do
      {:ok, data} ->
        ~r/\{:(coding_adventures_[a-z0-9_]+)/
        |> Regex.scan(data)
        |> Enum.flat_map(fn
          [_, app_name] ->
            app_name = String.downcase(app_name)

            case Map.get(known_names, app_name) do
              nil -> []
              pkg_name -> [pkg_name]
            end

          _ ->
            []
        end)

      {:error, _} ->
        []
    end
  end

  # ---------------------------------------------------------------------------
  # Lua: .rockspec
  # ---------------------------------------------------------------------------
  #
  # Lua packages use LuaRocks as their package manager. Each package has a
  # .rockspec file that declares dependencies in a Lua table:
  #
  #   dependencies = {
  #       "lua >= 5.4",
  #       "coding-adventures-logic-gates >= 0.1.0",
  #   }
  #
  # We scan for quoted strings inside the `dependencies = { ... }` block,
  # strip version specifiers, and look them up in known_names. External
  # dependencies (like "lua" itself) are silently skipped because they
  # won't appear in known_names.

  defp parse_lua_deps(pkg, known_names) do
    # Find .rockspec files in the package directory.
    case File.ls(pkg.path) do
      {:ok, entries} ->
        rockspec =
          Enum.find(entries, fn entry -> String.ends_with?(entry, ".rockspec") end)

        if rockspec do
          rockspec_path = Path.join(pkg.path, rockspec)

          case File.read(rockspec_path) do
            {:ok, data} ->
              data
              |> String.split("\n")
              |> parse_lua_deps_lines(known_names, false, [])

            {:error, _} ->
              []
          end
        else
          []
        end

      {:error, _} ->
        []
    end
  end

  defp parse_lua_deps_lines([], _known, _in_deps, acc), do: Enum.reverse(acc)

  defp parse_lua_deps_lines([line | rest], known, in_deps, acc) do
    trimmed = String.trim(line)

    cond do
      not in_deps ->
        # Look for the opening of `dependencies = {`
        if String.starts_with?(trimmed, "dependencies") and String.contains?(trimmed, "=") and
             String.contains?(trimmed, "{") do
          if String.contains?(trimmed, "}") do
            # Single-line dependencies block — extract and finish.
            new_deps = extract_lua_deps(trimmed, known)
            Enum.reverse(new_deps ++ acc)
          else
            # Multi-line block starts — extract any deps on this line and continue.
            new_deps = extract_lua_deps(trimmed, known)
            parse_lua_deps_lines(rest, known, true, new_deps ++ acc)
          end
        else
          parse_lua_deps_lines(rest, known, false, acc)
        end

      # Closing brace ends the dependencies block.
      String.contains?(trimmed, "}") ->
        new_deps = extract_lua_deps(trimmed, known)
        Enum.reverse(new_deps ++ acc)

      true ->
        new_deps = extract_lua_deps(trimmed, known)
        parse_lua_deps_lines(rest, known, true, new_deps ++ acc)
    end
  end

  # Extracts quoted dependency names from a Lua rockspec line and maps them
  # to internal package names. Version specifiers (>=, ==, etc.) are stripped.
  # ---------------------------------------------------------------------------
  # Perl: cpanfile
  # ---------------------------------------------------------------------------
  #
  # A cpanfile declares dependencies with one `requires` per line:
  #
  #     requires 'coding-adventures-logic-gates';
  #     requires 'coding-adventures-bitset', '>= 0.01';
  #
  # We scan for lines matching `requires 'coding-adventures-...'` and map
  # them to internal package names. External deps are silently skipped.

  defp parse_perl_deps(pkg, known_names) do
    cpanfile = Path.join(pkg.path, "cpanfile")

    case File.read(cpanfile) do
      {:ok, data} ->
        pattern = ~r/requires\s+['"]coding-adventures-([^'"]+)['"]/

        data
        |> String.split("\n")
        |> Enum.reject(fn line ->
          trimmed = String.trim(line)
          trimmed == "" or String.starts_with?(trimmed, "#")
        end)
        |> Enum.flat_map(fn line ->
          case Regex.run(pattern, line) do
            [_, dep_kebab] ->
              dep_name = "coding-adventures-" <> String.downcase(dep_kebab)

              case Map.get(known_names, dep_name) do
                nil -> []
                pkg_name -> [pkg_name]
              end

            _ ->
              []
          end
        end)

      {:error, _} ->
        []
    end
  end

  defp parse_haskell_deps(pkg, known_names) do
    # Check .cabal files
    case File.ls(pkg.path) do
      {:ok, entries} ->
        cabal = Enum.find(entries, fn entry -> String.ends_with?(entry, ".cabal") end)
        if cabal do
          cabal_path = Path.join(pkg.path, cabal)
          case File.read(cabal_path) do
            {:ok, data} ->
              pattern = ~r/coding-adventures-[a-zA-Z0-9-]+/
              data
              |> String.split("\n")
              |> Enum.flat_map(fn line ->
                Regex.scan(pattern, line)
                |> Enum.flat_map(fn
                  [dep_name] ->
                    case Map.get(known_names, String.downcase(dep_name)) do
                      nil -> []
                      pkg_name when pkg_name == pkg.name -> []
                      pkg_name -> [pkg_name]
                    end
                  _ -> []
                end)
              end)
            {:error, _} -> []
          end
        else
          []
        end
      {:error, _} -> []
    end
  end

  defp dependency_scope(language) do
    case language do
      lang when lang in ["csharp", "fsharp", "dotnet"] -> "dotnet"
      "wasm" -> "wasm"
      _ -> language
    end
  end

  defp in_dependency_scope?(package_language, scope) do
    case scope do
      "dotnet" -> package_language in ["csharp", "fsharp", "dotnet"]
      "wasm" -> package_language in ["wasm", "rust"]
      _ -> package_language == scope
    end
  end

  defp read_cargo_package_name(path) do
    cargo_toml = Path.join(path, "Cargo.toml")

    case File.read(cargo_toml) do
      {:ok, data} ->
        case Regex.run(~r/^\s*name\s*=\s*"([^"]+)"/m, data) do
          [_, name] -> String.downcase(String.trim(name))
          _ -> nil
        end

      {:error, _} ->
        nil
    end
  end

  defp project_files(path) do
    case File.ls(path) do
      {:ok, entries} ->
        entries
        |> Enum.sort()
        |> Enum.filter(fn entry ->
          String.ends_with?(entry, ".csproj") or String.ends_with?(entry, ".fsproj")
        end)
        |> Enum.map(&Path.join(path, &1))

      {:error, _} ->
        []
    end
  end

  defp maybe_put(map, nil, _value), do: map
  defp maybe_put(map, key, value), do: Map.put(map, key, value)

  defp extract_lua_deps(line, known_names) do
    ~r/"([^"]+)"/
    |> Regex.scan(line)
    |> Enum.flat_map(fn
      [_, dep_str] ->
        # Strip version specifiers: split on >=, <=, >, <, ==, ~= or whitespace.
        dep_name =
          dep_str
          |> String.split(~r/[>=<!~\s]/, parts: 2)
          |> hd()
          |> String.trim()
          |> String.downcase()

        case Map.get(known_names, dep_name) do
          nil -> []
          pkg_name -> [pkg_name]
        end

      _ ->
        []
    end)
  end
end
