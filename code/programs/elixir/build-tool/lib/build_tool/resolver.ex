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

    - **Rust**: Cargo.toml path dependencies name the crate by its directory
      basename or its declared `[package]` name. `"logic-gates"` maps to
      `"rust/logic-gates"`. WASM wrappers are known only by their declared
      name.

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
          # A WASM wrapper is known only by its declared Cargo name
          # ("graph-wasm"). Its bare directory name ("graph") belongs to the
          # Rust crate it wraps, and claiming it would turn the wrapper's own
          # `graph = { path = "../../rust/graph" }` into a self-edge.
          maybe_put(acc, read_cargo_package_name(pkg.path), pkg.name)

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
  # ## Which tables are build-graph edges
  #
  # A graph edge means "rebuild and re-test the dependent when this changes",
  # so it must cover everything Cargo compiles INTO the crate:
  #
  #   table                                  edge?  why
  #   -------------------------------------  -----  -----------------------------
  #   [dependencies]                         yes    linked into the library
  #   [build-dependencies]                   yes    compiled into build.rs, which
  #                                                 generates or configures it
  #   [target.<cfg>.dependencies]            yes    linked on that target; CI
  #   [target.<cfg>.build-dependencies]      yes    builds on several hosts, so
  #                                                 the union over every <cfg>
  #   [dev-dependencies] (every form)        no     tests only; Cargo allows dev
  #                                                 cycles, a graph must not
  #   [patch], [replace], [workspace], ...   no     not a dependency at all
  #
  # `build_dependencies` is Cargo's legacy spelling of `build-dependencies`.
  #
  # ## Which entry shapes
  #
  # Both TOML spellings of a path dependency are read:
  #
  #   [dependencies]
  #   logic-gates = { path = "../logic-gates" }          <- inline table
  #   gates = { package = "logic-gates", path = "..." }  <- renamed: "logic-gates"
  #
  #   [dependencies.logic-gates]                         <- sub-table
  #   path = "../logic-gates"
  #
  # The crate name (the `package` rename if present, else the key) is then
  # looked up in the known names, and each internal package is returned once.
  # The line procedure is the cross-implementation contract in the Rust
  # paragraph of code/specs/build-tool-conformance.md; every build-tool front
  # implements the same one (ported here from the Go reference,
  # cargoLinkDependencyNames), and the shared fixture
  # resolution-rust-field-aware pins it.
  #
  # ## Bytes, not graphemes
  #
  # An Elixir string is a binary. Every helper below walks it BYTE by byte
  # (`:binary.at/2`, `binary_part/3`), never with the grapheme-aware `String`
  # functions: "\r\n" is one grapheme, and the contract is defined on LF and
  # on ASCII delimiters. In UTF-8 an ASCII byte is never part of a multi-byte
  # character, so byte offsets are exact and every slice is whole characters.

  defp parse_rust_deps(pkg, known_names) do
    cargo_toml = Path.join(pkg.path, "Cargo.toml")

    case File.read(cargo_toml) do
      {:ok, data} ->
        data
        |> cargo_link_dependency_names()
        |> Enum.flat_map(fn crate_name ->
          case Map.get(known_names, crate_name) do
            nil -> []
            pkg_name -> [pkg_name]
          end
        end)
        # A crate may sit in several link tables at once (say in
        # `[dependencies]` and `[build-dependencies]`): one edge.
        |> Enum.uniq()

      {:error, _} ->
        []
    end
  end

  # The table names whose path entries link into, or build, the crate. They
  # are compared exactly: TOML keys are case-sensitive, so `[Dependencies]`
  # is some other table.
  @cargo_link_tables ["dependencies", "build-dependencies", "build_dependencies"]

  @doc """
  Returns the ASCII-lowercased crate names of every path dependency in a
  Cargo manifest's link tables, in manifest order, duplicates included (the
  caller collapses them).

  It is one pass over the lines with a three-state machine. The current
  table is one of:

    * `:ignored` — not a link table: its lines are skipped;
    * `:inline` — `[D]` or `[target.C.D]`: one `name = { ... }` entry per line;
    * `{:sub, n}` — `[D.N]` or `[target.C.D.N]`: the keys of the one entry `N`.

  Every header line — even a malformed one, or an array of tables — ends the
  current table, and so does the end of the file; a sub-table's entry is
  complete only then, because its `path` and `package` keys may come in any
  order.

  One inline entry may cover several lines. TOML 1.0 lets an ARRAY inside an
  inline table span lines, and the repository writes exactly that:

      coding_adventures_vault_pm_cli = { path = "../vault-pm-cli", features = [
          "crash-injection",
      ] }

  So while the entry's value still has an unclosed `{` or `[`, the next lines
  are joined onto it with one space (and are not looked at as headers or
  entries). If the file ends first, the entry contributes nothing.

  Exported for testing.

  ## Example

      iex> BuildTool.Resolver.cargo_link_dependency_names(
      ...>   "[dependencies]\\ngates = { package = \\"Logic-Gates\\", path = \\"../g\\" }\\n"
      ...> )
      ["logic-gates"]
  """
  def cargo_link_dependency_names(text) do
    # Step 1 of the contract: ignore a leading byte-order mark, split at LF.
    text =
      case text do
        <<0xEF, 0xBB, 0xBF, rest::binary>> -> rest
        _ -> text
      end

    text
    |> :binary.split("\n", [:global])
    |> walk_cargo_lines(:ignored, nil, nil, [])
  end

  defp walk_cargo_lines([], table, sub_path, sub_package, acc) do
    table |> finish_cargo_table(sub_path, sub_package, acc) |> Enum.reverse()
  end

  defp walk_cargo_lines([raw | rest], table, sub_path, sub_package, acc) do
    line = clean_cargo_line(raw)

    cond do
      cargo_header?(line) ->
        acc = finish_cargo_table(table, sub_path, sub_package, acc)
        walk_cargo_lines(rest, classify_cargo_link_header(line), nil, nil, acc)

      true ->
        case toml_index_outside_strings(line, ?=) do
          nil -> walk_cargo_lines(rest, table, sub_path, sub_package, acc)
          equals -> cargo_entry(table, line, equals, rest, sub_path, sub_package, acc)
        end
    end
  end

  # A line `key = value` under the current table.
  defp cargo_entry(:ignored, _line, _equals, rest, sub_path, sub_package, acc) do
    walk_cargo_lines(rest, :ignored, sub_path, sub_package, acc)
  end

  defp cargo_entry(:inline, line, equals, rest, sub_path, sub_package, acc) do
    key = binary_part(line, 0, equals)
    value = toml_trim_blank(toml_tail(line, equals + 1))

    case join_cargo_continuation(value, rest) do
      {:ok, value, rest} ->
        acc =
          case cargo_path_dependency_name(key, value) do
            nil -> acc
            name -> [name | acc]
          end

        walk_cargo_lines(rest, :inline, sub_path, sub_package, acc)

      :eof ->
        # The file ended inside the value: no entry.
        walk_cargo_lines([], :inline, sub_path, sub_package, acc)
    end
  end

  defp cargo_entry({:sub, _} = table, line, equals, rest, sub_path, sub_package, acc) do
    # Only the two bare, lowercase keys matter; the first occurrence of each
    # wins. `version`, `features`, `optional` ... are ignored.
    key = toml_trim_blank(binary_part(line, 0, equals))

    case {key, toml_single_string(toml_tail(line, equals + 1))} do
      {"path", {:ok, value}} when sub_path == nil ->
        walk_cargo_lines(rest, table, value, sub_package, acc)

      {"package", {:ok, value}} when sub_package == nil ->
        walk_cargo_lines(rest, table, sub_path, value, acc)

      _ ->
        walk_cargo_lines(rest, table, sub_path, sub_package, acc)
    end
  end

  # While the value opens with `{` and still has an unclosed `{` or `[`, the
  # next line is cleaned and joined onto it with one space.
  defp join_cargo_continuation(value, rest) do
    if String.starts_with?(value, "{") and toml_open_bracket_depth(value) > 0 do
      case rest do
        [] -> :eof
        [next | rest] -> join_cargo_continuation(value <> " " <> clean_cargo_line(next), rest)
      end
    else
      {:ok, value, rest}
    end
  end

  # A sub-table is a candidate only once it ends, when `path` was set; it is
  # named by its `package` rename, else by its key `N`.
  defp finish_cargo_table({:sub, key}, sub_path, sub_package, acc) when sub_path != nil do
    [String.downcase(sub_package || key, :ascii) | acc]
  end

  defp finish_cargo_table(_table, _sub_path, _sub_package, acc), do: acc

  # Step 1: drop ONE trailing CR, strip the comment, trim spaces and tabs.
  defp clean_cargo_line(raw) do
    raw =
      if String.ends_with?(raw, "\r"),
        do: binary_part(raw, 0, byte_size(raw) - 1),
        else: raw

    raw |> toml_strip_line_comment() |> toml_trim_blank()
  end

  defp cargo_header?(line), do: String.starts_with?(line, "[") and String.ends_with?(line, "]")

  # Decides how to read the lines under a header line (already cleaned,
  # starting with `[` and ending with `]`).
  #
  # The header is split into dotted-key segments at every `.` OUTSIDE a
  # string, and each segment is decoded to its value. That is what makes a
  # cfg header one segment however much punctuation it quotes:
  #
  #   [target.'cfg(any(target_os = "linux", windows))'.dependencies]
  #    target  C (one quoted segment)                    D
  #
  #   [target."thumbv8m.main-none-eabihf".dependencies.pi]  -> sub-table "pi"
  #   [target.x86_64-unknown-linux-gnu.build-dependencies]  -> bare triple
  #   ["dependencies"]                                      -> same as [dependencies]
  #
  # Segment lists that are not exactly one of [D], [D.N], [target.C.D] or
  # [target.C.D.N] — and malformed headers — are ignored tables.
  defp classify_cargo_link_header("[[" <> _), do: :ignored

  defp classify_cargo_link_header(line) do
    segments =
      line
      |> binary_part(1, byte_size(line) - 2)
      |> toml_split_outside_strings(?., false)
      |> Enum.map(&toml_key_segment/1)

    if Enum.member?(segments, :error) do
      :ignored
    else
      segments
      |> Enum.map(fn {:ok, segment} -> segment end)
      |> drop_cargo_target()
      |> case do
        [table] when table in @cargo_link_tables -> :inline
        [table, key] when table in @cargo_link_tables -> {:sub, key}
        _ -> :ignored
      end
    end
  end

  # Drops `target` and the cfg, whatever it is, from [target.C.…].
  defp drop_cargo_target(["target", _cfg, _ | _] = segments), do: Enum.drop(segments, 2)
  defp drop_cargo_target(segments), do: segments

  # Returns the crate a completed inline-table entry `key = value` names when
  # it is a path dependency — the only kind that can point at a repository
  # package. The key is the crate name unless the inline table renames it
  # with `package = "..."`:
  #
  #   logic-gates = { path = "../logic-gates" }                    -> logic-gates
  #   gates = { package = "logic-gates", path = "../logic-gates" } -> logic-gates
  #   serde = "1"                                                  -> not a path dep
  #   x = { version = "1", features = ["path"] }                   -> not a path dep
  #   foo.path = "../foo"                                          -> dotted key: no
  #
  # Fields are split at commas outside strings AND outside nested `[...]` or
  # `{...}`, so a `features` array cannot leak a fake field and a nested
  # table's `path` is not the entry's. Field keys must be exactly `path` and
  # `package` (Cargo ignores `Package`, so it must not count as a rename).
  defp cargo_path_dependency_name(key, value) do
    value = toml_trim_blank(value)

    with {:ok, crate_name} <- toml_key_segment(key),
         true <-
           byte_size(value) >= 2 and String.starts_with?(value, "{") and
             String.ends_with?(value, "}") do
      {path, package_name} =
        value
        |> binary_part(1, byte_size(value) - 2)
        |> toml_split_outside_strings(?,, true)
        |> Enum.reduce({nil, nil}, &collect_cargo_field/2)

      if path, do: String.downcase(package_name || crate_name, :ascii), else: nil
    else
      _ -> nil
    end
  end

  defp collect_cargo_field(field, {path, package_name} = found) do
    case toml_index_outside_strings(field, ?=) do
      nil ->
        found

      equals ->
        key = toml_trim_blank(binary_part(field, 0, equals))

        case {key, toml_single_string(toml_tail(field, equals + 1))} do
          {"path", {:ok, value}} when path == nil -> {value, package_name}
          {"package", {:ok, value}} when package_name == nil -> {path, value}
          _ -> found
        end
    end
  end

  # -- TOML lexical helpers (bytes) -------------------------------------------

  # The bytes of `s` from `index` to its end.
  defp toml_tail(s, index), do: binary_part(s, index, byte_size(s) - index)

  # Trims spaces and tabs only — the contract's notion of blank.
  defp toml_trim_blank(s), do: s |> trim_leading_blank() |> trim_trailing_blank()

  defp trim_leading_blank(<<c, rest::binary>>) when c in [?\s, ?\t], do: trim_leading_blank(rest)
  defp trim_leading_blank(s), do: s

  defp trim_trailing_blank(s) do
    size = byte_size(s)

    if size > 0 and :binary.last(s) in [?\s, ?\t],
      do: trim_trailing_blank(binary_part(s, 0, size - 1)),
      else: s
  end

  # Returns the index just past the string that opens at byte `start` (a `"`
  # or `'`), or nil if it is not closed on this line. In a basic "..." string
  # a backslash makes the next byte part of the string, so `\"` does not end
  # it; a literal '...' string has no escapes.
  defp toml_string_end(s, start), do: toml_string_end(s, :binary.at(s, start), start + 1)

  defp toml_string_end(s, _quote, index) when index >= byte_size(s), do: nil

  defp toml_string_end(s, quote, index) do
    case :binary.at(s, index) do
      ?\\ when quote == ?" -> toml_string_end(s, quote, index + 2)
      ^quote -> index + 1
      _ -> toml_string_end(s, quote, index + 1)
    end
  end

  # Removes a TOML comment: everything from the first `#` outside a string.
  # An unclosed string runs to the end of the line.
  defp toml_strip_line_comment(s), do: strip_comment(s, 0)

  defp strip_comment(s, index) when index >= byte_size(s), do: s

  defp strip_comment(s, index) do
    case :binary.at(s, index) do
      c when c in [?", ?'] ->
        case toml_string_end(s, index) do
          nil -> s
          stop -> strip_comment(s, stop)
        end

      ?# ->
        binary_part(s, 0, index)

      _ ->
        strip_comment(s, index + 1)
    end
  end

  # Returns the index of the first `target` byte outside every string, or nil
  # (also when a string is left unclosed before it).
  defp toml_index_outside_strings(s, target), do: index_outside(s, target, 0)

  defp index_outside(s, _target, index) when index >= byte_size(s), do: nil

  defp index_outside(s, target, index) do
    case :binary.at(s, index) do
      c when c in [?", ?'] ->
        case toml_string_end(s, index) do
          nil -> nil
          stop -> index_outside(s, target, stop)
        end

      ^target ->
        index

      _ ->
        index_outside(s, target, index + 1)
    end
  end

  # Counts the `{` and `[` outside strings that are not closed again within
  # `s`. Zero (or less) means the value is complete; an unclosed string stops
  # the count where it opens.
  defp toml_open_bracket_depth(s), do: bracket_depth(s, 0, 0)

  defp bracket_depth(s, index, depth) when index >= byte_size(s), do: depth

  defp bracket_depth(s, index, depth) do
    case :binary.at(s, index) do
      c when c in [?", ?'] ->
        case toml_string_end(s, index) do
          nil -> depth
          stop -> bracket_depth(s, stop, depth)
        end

      c when c in [?{, ?[] ->
        bracket_depth(s, index + 1, depth + 1)

      c when c in [?}, ?]] ->
        bracket_depth(s, index + 1, depth - 1)

      _ ->
        bracket_depth(s, index + 1, depth)
    end
  end

  # Splits `s` at every `separator` byte outside strings. With `nested` set,
  # a separator inside `[...]` or `{...}` does not split either. An unclosed
  # string makes the rest of `s` the last part.
  defp toml_split_outside_strings(s, separator, nested) do
    split_outside(s, separator, nested, 0, 0, 0, [])
  end

  defp split_outside(s, _separator, _nested, start, _depth, index, parts)
       when index >= byte_size(s) do
    Enum.reverse([toml_tail(s, start) | parts])
  end

  defp split_outside(s, separator, nested, start, depth, index, parts) do
    c = :binary.at(s, index)

    cond do
      c in [?", ?'] ->
        case toml_string_end(s, index) do
          nil -> Enum.reverse([toml_tail(s, start) | parts])
          stop -> split_outside(s, separator, nested, start, depth, stop, parts)
        end

      nested and c in [?[, ?{] ->
        split_outside(s, separator, nested, start, depth + 1, index + 1, parts)

      nested and c in [?], ?}] ->
        split_outside(s, separator, nested, start, depth - 1, index + 1, parts)

      c == separator and depth == 0 ->
        part = binary_part(s, start, index - start)
        split_outside(s, separator, nested, index + 1, depth, index + 1, [part | parts])

      true ->
        split_outside(s, separator, nested, start, depth, index + 1, parts)
    end
  end

  # Decodes one key or header segment: a bare key (ASCII letters, digits,
  # `_`, `-`), or exactly one complete string, whose value is the text
  # between its quotes taken verbatim (escapes are not decoded). Anything
  # else — empty, dotted, or text around a string — is not a key.
  defp toml_key_segment(raw) do
    segment = toml_trim_blank(raw)

    cond do
      segment == "" ->
        :error

      :binary.first(segment) in [?", ?'] ->
        if toml_string_end(segment, 0) == byte_size(segment),
          do: {:ok, binary_part(segment, 1, byte_size(segment) - 2)},
          else: :error

      toml_bare_key?(segment) ->
        {:ok, segment}

      true ->
        :error
    end
  end

  defp toml_bare_key?(<<>>), do: true

  defp toml_bare_key?(<<c, rest::binary>>)
       when c in ?a..?z or c in ?A..?Z or c in ?0..?9 or c in [?_, ?-],
       do: toml_bare_key?(rest)

  defp toml_bare_key?(_), do: false

  # Returns `{:ok, content}` for a value that is exactly one complete,
  # non-empty string (after trimming), as `path` and `package` require.
  # `"1"` is one; `"a" "b"`, `1`, `""` and `["x"]` are not.
  defp toml_single_string(raw) do
    value = toml_trim_blank(raw)

    with true <- value != "" and :binary.first(value) in [?", ?'],
         true <- toml_string_end(value, 0) == byte_size(value),
         content when content != "" <- binary_part(value, 1, byte_size(value) - 2) do
      {:ok, content}
    else
      _ -> :error
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
