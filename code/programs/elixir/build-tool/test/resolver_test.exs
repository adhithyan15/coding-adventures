defmodule BuildTool.ResolverTest do
  use ExUnit.Case, async: true

  alias BuildTool.DirectedGraph
  alias BuildTool.Resolver

  doctest BuildTool.Resolver

  # ---------------------------------------------------------------------------
  # Setup: a fresh temporary directory per test
  # ---------------------------------------------------------------------------

  setup do
    # A random name plus exclusive mkdir refuses an existing tree. Never
    # overwrite or remove another user's path in a shared temporary directory.
    token = :crypto.strong_rand_bytes(16) |> Base.url_encode64(padding: false)
    tmp_dir = Path.join(System.tmp_dir!(), "build_tool_resolver_test_#{token}")
    :ok = File.mkdir(tmp_dir)
    on_exit(fn -> File.rm_rf!(tmp_dir) end)
    {:ok, tmp_dir: tmp_dir}
  end

  defp write_cargo!(root, relative_dir, contents) do
    dir = Path.join(root, relative_dir)
    File.mkdir_p!(dir)
    File.write!(Path.join(dir, "Cargo.toml"), contents)
    dir
  end

  defp package(name, path, language) do
    %{name: name, path: path, language: language, build_commands: []}
  end

  # ---------------------------------------------------------------------------
  # The Cargo line contract, one manifest shape per case
  # ---------------------------------------------------------------------------
  #
  # This table is the Go reference's TestCargoLinkDependencyNames, case for
  # case, so the two readers are held to the same shapes. Each rule it pins
  # is a step of the Rust paragraph of code/specs/build-tool-conformance.md.

  @cargo_link_cases [
    {"top-level inline", "[dependencies]\na = { path = \"../a\" }\n", ["a"]},
    {"build-dependencies", "[build-dependencies]\na = { path = \"../a\" }\n", ["a"]},
    {"legacy build_dependencies", "[build_dependencies]\na = { path = \"../a\" }\n", ["a"]},
    {"quoted cfg target",
     "[target.'cfg(any(target_os = \"linux\", windows))'.dependencies]\na = { path = \"../a\" }\n",
     ["a"]},
    {"bare triple target build",
     "[target.x86_64-pc-windows-msvc.build-dependencies]\na = { path = \"../a\" }\n", ["a"]},
    {"legacy target build", "[target.'cfg(unix)'.build_dependencies]\na = { path = \"../a\" }\n",
     ["a"]},
    {"spaced header segments",
     "[ target . 'cfg(unix)' . dependencies ]\na = { path = \"../a\" }\n", ["a"]},
    {"quoted table segment", "[\"dependencies\"]\na = { path = \"../a\" }\n", ["a"]},
    {"sub-table", "[dependencies.a]\nversion = \"1\"\npath = \"../a\"\n", ["a"]},
    {"sub-table rename before path", "[build-dependencies.x]\npackage = \"A\"\npath = '../a'\n",
     ["a"]},
    {"target sub-table, dotted quoted triple",
     "[target.\"thumbv8m.main-none-eabihf\".dependencies.a]\npath = \"../a\"\n", ["a"]},
    {"quoted sub-table key", "[dependencies.\"a\"]\npath = \"../a\"\n", ["a"]},
    {"sub-table without path", "[dependencies.a]\nversion = \"1\"\n", []},
    {"sub-table first path wins, ended by header",
     "[dependencies.a]\npath = \"../a\"\npath = \"../b\"\n[package]\nname = \"z\"\n", ["a"]},
    {"sub-table non-string path", "[dependencies.a]\npath = 1\n", []},
    {"sub-table empty path", "[dependencies.a]\npath = \"\"\n", []},
    {"sub-table capitalised Path key", "[dependencies.a]\nPath = \"../a\"\n", []},
    {"rename", "[dependencies]\nx = { package = \"Logic-Gates\", path = \"../g\" }\n",
     ["logic-gates"]},
    {"capitalised Package is not a rename",
     "[dependencies]\nx = { Package = \"y\", path = \"../g\" }\n", ["x"]},
    {"capitalised Path is not a path", "[dependencies]\nx = { Path = \"../g\" }\n", []},
    {"version plus path with trailing comment",
     "[dependencies]\na = { version = \"1\", path = \"../a\" } # b = { path = \"../b\" }\n",
     ["a"]},
    {"hash inside a string is not a comment", "[dependencies]\na = { path = \"../a#1\" }\n",
     ["a"]},
    {"quoted entry key", "[dependencies]\n\"a\" = { path = \"../a\" }\n", ["a"]},
    {"registry entry with path feature",
     "[dependencies]\na = { version = \"1\", features = [\"path\", \"x\"] }\nb = \"1\"\n", []},
    {"nested table path is not the entry's", "[dependencies]\na = { x = { path = \"../a\" } }\n",
     []},
    {"features array commas do not split fields",
     "[dependencies]\na = { features = [\"p\", \"q\"], path = \"../a\" }\n", ["a"]},
    {"empty path", "[dependencies]\na = { path = \"\" }\n", []},
    {"array continued onto later lines",
     "[dependencies]\na = { path = \"../a\", features = [\n    \"x\", # [not-a-header]\n] }\nb = { path = \"../b\" }\n",
     ["a", "b"]},
    {"continuation lines are not headers",
     "[dependencies]\na = { path = \"../a\", features = [\n[\"x\"]\n] }\nb = { path = \"../b\" }\n",
     ["a", "b"]},
    {"braces continued onto later lines",
     "[dependencies]\na = { version = \"1\",\n  path = \"../a\" }\n", ["a"]},
    {"file ends inside the value",
     "[dependencies]\na = { path = \"../a\", features = [\n  \"x\",\n", []},
    {"continued value must end with a brace",
     "[dependencies]\na = { path = \"../a\", features = [\n] } x\n", []},
    {"dotted key is out of scope", "[dependencies]\na.path = \"../a\"\n", []},
    {"commented-out entry", "[dependencies]\n# a = { path = \"../a\" }\n", []},
    {"dev-dependencies",
     "[dev-dependencies]\na = { path = \"../a\" }\n[dev_dependencies]\nb = { path = \"../b\" }\n",
     []},
    {"target dev-dependencies",
     "[target.'cfg(unix)'.dev-dependencies]\na = { path = \"../a\" }\n", []},
    {"dev-dependencies sub-table", "[dev-dependencies.a]\npath = \"../a\"\n", []},
    {"patch, replace, workspace",
     "[patch.crates-io]\na = { path = \"../a\" }\n[workspace.dependencies]\nb = { path = \"../b\" }\n[replace]\n\"c:1.0.0\" = { path = \"../c\" }\n",
     []},
    {"capitalised table name", "[Dependencies]\na = { path = \"../a\" }\n", []},
    {"features sub-table of a dependency", "[dependencies.a.features]\npath = \"../a\"\n", []},
    {"target table itself", "[target.'cfg(unix)']\na = { path = \"../a\" }\n", []},
    {"array of tables ends the table", "[dependencies]\n[[bin]]\na = { path = \"../a\" }\n", []},
    {"malformed header ends the table", "[dependencies]\n[a b]\nc = { path = \"../c\" }\n", []},
    {"unterminated quoted header ends the table",
     "[dependencies]\n[target.'cfg(unix).dependencies]\nc = { path = \"../c\" }\n", []},
    {"CRLF and BOM", "\uFEFF[dependencies]\r\na = { path = \"../a\" }\r\n", ["a"]},
    {"union across tables keeps duplicates for the caller",
     "[dependencies]\na = { path = \"../a\" }\n[build-dependencies]\na = { path = \"../a\" }\n",
     ["a", "a"]}
  ]

  test "the Cargo shape table matches the Go reference's 46 cases" do
    assert length(@cargo_link_cases) == 46
  end

  for {name, manifest, want} <- @cargo_link_cases do
    test "cargo link dependency names: #{name}" do
      assert Resolver.cargo_link_dependency_names(unquote(manifest)) == unquote(want)
    end
  end

  # ---------------------------------------------------------------------------
  # Resolution through the graph
  # ---------------------------------------------------------------------------

  test "a crate named by three link tables yields one edge; unknown paths drop", %{
    tmp_dir: tmp_dir
  } do
    app =
      write_cargo!(
        tmp_dir,
        "rust/app",
        "[package]\nname = \"app\"\n\n[dependencies]\nlib = { path = \"../lib\" }\n" <>
          "serde = { path = \"../vendored-serde\" }\n\n[build-dependencies]\n" <>
          "lib = { path = \"../lib\" }\n\n[target.'cfg(windows)'.dependencies.lib]\n" <>
          "path = \"../lib\"\n"
      )

    lib = write_cargo!(tmp_dir, "rust/lib", "[package]\nname = \"lib\"\n")

    graph =
      Resolver.resolve_dependencies([
        package("rust/app", app, "rust"),
        package("rust/lib", lib, "rust")
      ])

    assert DirectedGraph.has_edge?(graph, "rust/lib", "rust/app")
    assert graph.reverse["rust/app"] == MapSet.new(["rust/lib"])
  end

  test "build, target and renamed sub-table entries are edges; dev and patch are not", %{
    tmp_dir: tmp_dir
  } do
    app =
      write_cargo!(
        tmp_dir,
        "rust/app",
        "[package]\nname = \"app\"\n\n[build-dependencies]\nbuilder = { path = \"../builder\" }\n\n" <>
          "[target.\"cfg(unix)\".dependencies.unix_alias]\npackage = \"Unix-Glue\"\npath = \"../unix-glue\"\n\n" <>
          "[dev-dependencies]\ntester = { path = \"../tester\" }\n\n" <>
          "[patch.crates-io]\npatched = { path = \"../patched\" }\n"
      )

    packages =
      [package("rust/app", app, "rust")] ++
        for crate <- ["builder", "unix-glue", "tester", "patched"] do
          dir = write_cargo!(tmp_dir, "rust/" <> crate, "[package]\nname = \"#{crate}\"\n")
          package("rust/" <> crate, dir, "rust")
        end

    graph = Resolver.resolve_dependencies(packages)
    assert graph.reverse["rust/app"] == MapSet.new(["rust/builder", "rust/unix-glue"])
  end

  test "a missing Cargo.toml contributes no edges", %{tmp_dir: tmp_dir} do
    dir = Path.join(tmp_dir, "rust/empty")
    File.mkdir_p!(dir)
    graph = Resolver.resolve_dependencies([package("rust/empty", dir, "rust")])
    assert DirectedGraph.has_node?(graph, "rust/empty")
    assert graph.reverse["rust/empty"] in [nil, MapSet.new()]
  end

  test "a Rust crate is known by its directory and its declared name", %{tmp_dir: tmp_dir} do
    dir =
      write_cargo!(
        tmp_dir,
        "rust/logic-gates",
        "[package]\nname = \"Coding-Adventures-Logic-Gates\"\n"
      )

    known = Resolver.build_known_names([package("rust/logic-gates", dir, "rust")], "rust")
    assert known["logic-gates"] == "rust/logic-gates"
    assert known["coding-adventures-logic-gates"] == "rust/logic-gates"
  end

  # A WASM wrapper claiming its bare directory name ("graph") would turn its
  # own `graph = { path = "../../rust/graph" }` into a self-edge, which the
  # graph rejects outright.
  test "a WASM wrapper is known only by its declared name, so it depends on its crate", %{
    tmp_dir: tmp_dir
  } do
    rust = write_cargo!(tmp_dir, "rust/graph", "[package]\nname = \"coding-adventures-graph\"\n")

    wasm =
      write_cargo!(
        tmp_dir,
        "wasm/graph",
        "[package]\nname = \"graph-wasm\"\n\n[dependencies]\ngraph = { path = \"../../rust/graph\" }\n"
      )

    packages = [package("rust/graph", rust, "rust"), package("wasm/graph", wasm, "wasm")]
    known = Resolver.build_known_names(packages, "wasm")
    assert known["graph"] == "rust/graph"
    assert known["graph-wasm"] == "wasm/graph"

    graph = Resolver.resolve_dependencies(packages)
    assert DirectedGraph.has_edge?(graph, "rust/graph", "wasm/graph")
    refute DirectedGraph.has_edge?(graph, "wasm/graph", "wasm/graph")
  end
end
