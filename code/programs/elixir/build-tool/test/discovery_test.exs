defmodule BuildTool.DiscoveryTest do
  use ExUnit.Case, async: true
  import ExUnit.CaptureIO

  alias BuildTool.CLI
  alias BuildTool.Discovery
  alias BuildTool.Discovery.DuplicatePackageIdentityError
  alias BuildTool.StarlarkEvaluator

  # ---------------------------------------------------------------------------
  # Setup: create temporary directories for testing
  # ---------------------------------------------------------------------------

  setup do
    # A random name plus exclusive mkdir refuses an existing tree. Never
    # overwrite or remove another user's path in a shared temporary directory.
    token = :crypto.strong_rand_bytes(16) |> Base.url_encode64(padding: false)

    tmp_dir =
      Path.join(
        System.tmp_dir!(),
        "build_tool_discovery_test_#{token}"
      )

    :ok = File.mkdir(tmp_dir)

    on_exit(fn -> File.rm_rf!(tmp_dir) end)

    {:ok, tmp_dir: tmp_dir}
  end

  # ---------------------------------------------------------------------------
  # read_lines/1
  # ---------------------------------------------------------------------------

  describe "read_lines/1" do
    test "reads non-blank, non-comment lines", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "BUILD")

      File.write!(path, """
      # This is a comment
      pip install -e .

      pytest
      # Another comment
      """)

      assert Discovery.read_lines(path) == ["pip install -e .", "pytest"]
    end

    test "returns empty list for missing file" do
      assert Discovery.read_lines("/nonexistent/file") == []
    end

    test "trims whitespace from lines", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "BUILD")
      File.write!(path, "  echo hello  \n  echo world  \n")
      assert Discovery.read_lines(path) == ["echo hello", "echo world"]
    end
  end

  # ---------------------------------------------------------------------------
  # infer_language/1
  # ---------------------------------------------------------------------------

  describe "infer_language/1" do
    test "infers python" do
      assert Discovery.infer_language("/repo/code/packages/python/logic-gates") == "python"
    end

    test "infers ruby" do
      assert Discovery.infer_language("/repo/code/packages/ruby/logic_gates") == "ruby"
    end

    test "infers go" do
      assert Discovery.infer_language("/repo/code/packages/go/directed-graph") == "go"
    end

    test "infers rust" do
      assert Discovery.infer_language("/repo/code/packages/rust/logic-gates") == "rust"
    end

    test "infers typescript" do
      assert Discovery.infer_language("/repo/code/programs/typescript/web-app") == "typescript"
    end

    test "infers elixir" do
      assert Discovery.infer_language("/repo/code/packages/elixir/progress-bar") == "elixir"
    end

    test "returns unknown for unrecognized path" do
      assert Discovery.infer_language("/some/random/path") == "unknown"
    end

    test "handles backslash paths (Windows)" do
      assert Discovery.infer_language("C:\\repo\\code\\packages\\python\\logic-gates") == "python"
    end
  end

  # ---------------------------------------------------------------------------
  # infer_package_name/2
  # ---------------------------------------------------------------------------

  describe "infer_package_name/2" do
    test "builds qualified name" do
      assert Discovery.infer_package_name("/repo/code/packages/python/logic-gates", "python") ==
               "python/logic-gates"
    end
  end

  # ---------------------------------------------------------------------------
  # get_build_file_for_platform/2
  # ---------------------------------------------------------------------------

  describe "get_build_file_for_platform/2" do
    test "returns BUILD_mac on darwin when it exists", %{tmp_dir: tmp_dir} do
      File.write!(Path.join(tmp_dir, "BUILD_mac"), "echo mac")
      File.write!(Path.join(tmp_dir, "BUILD"), "echo generic")

      result = Discovery.get_build_file_for_platform(tmp_dir, :darwin)
      assert result == Path.join(tmp_dir, "BUILD_mac")
    end

    test "returns BUILD_linux on linux when it exists", %{tmp_dir: tmp_dir} do
      File.write!(Path.join(tmp_dir, "BUILD_linux"), "echo linux")
      File.write!(Path.join(tmp_dir, "BUILD"), "echo generic")

      result = Discovery.get_build_file_for_platform(tmp_dir, :linux)
      assert result == Path.join(tmp_dir, "BUILD_linux")
    end

    test "falls back to BUILD when platform file doesn't exist", %{tmp_dir: tmp_dir} do
      File.write!(Path.join(tmp_dir, "BUILD"), "echo generic")

      result = Discovery.get_build_file_for_platform(tmp_dir, :darwin)
      assert result == Path.join(tmp_dir, "BUILD")
    end

    test "returns nil when no BUILD file exists", %{tmp_dir: tmp_dir} do
      assert Discovery.get_build_file_for_platform(tmp_dir, :darwin) == nil
    end

    # -- BUILD_windows tests --------------------------------------------------

    test "returns BUILD_windows on windows when it exists", %{tmp_dir: tmp_dir} do
      File.write!(Path.join(tmp_dir, "BUILD_windows"), "echo windows")
      File.write!(Path.join(tmp_dir, "BUILD"), "echo generic")

      result = Discovery.get_build_file_for_platform(tmp_dir, :windows)
      assert result == Path.join(tmp_dir, "BUILD_windows")
    end

    test "falls back to BUILD on windows when BUILD_windows missing", %{tmp_dir: tmp_dir} do
      File.write!(Path.join(tmp_dir, "BUILD"), "echo generic")

      result = Discovery.get_build_file_for_platform(tmp_dir, :windows)
      assert result == Path.join(tmp_dir, "BUILD")
    end

    test "BUILD_windows not used on darwin", %{tmp_dir: tmp_dir} do
      File.write!(Path.join(tmp_dir, "BUILD_windows"), "echo windows")
      File.write!(Path.join(tmp_dir, "BUILD"), "echo generic")

      result = Discovery.get_build_file_for_platform(tmp_dir, :darwin)
      assert result == Path.join(tmp_dir, "BUILD")
    end

    # -- BUILD_mac_and_linux tests --------------------------------------------

    test "returns BUILD_mac_and_linux on darwin", %{tmp_dir: tmp_dir} do
      File.write!(Path.join(tmp_dir, "BUILD_mac_and_linux"), "echo unix")
      File.write!(Path.join(tmp_dir, "BUILD"), "echo generic")

      result = Discovery.get_build_file_for_platform(tmp_dir, :darwin)
      assert result == Path.join(tmp_dir, "BUILD_mac_and_linux")
    end

    test "returns BUILD_mac_and_linux on linux", %{tmp_dir: tmp_dir} do
      File.write!(Path.join(tmp_dir, "BUILD_mac_and_linux"), "echo unix")
      File.write!(Path.join(tmp_dir, "BUILD"), "echo generic")

      result = Discovery.get_build_file_for_platform(tmp_dir, :linux)
      assert result == Path.join(tmp_dir, "BUILD_mac_and_linux")
    end

    test "BUILD_mac_and_linux not used on windows", %{tmp_dir: tmp_dir} do
      File.write!(Path.join(tmp_dir, "BUILD_mac_and_linux"), "echo unix")
      File.write!(Path.join(tmp_dir, "BUILD"), "echo generic")

      result = Discovery.get_build_file_for_platform(tmp_dir, :windows)
      assert result == Path.join(tmp_dir, "BUILD")
    end

    test "variant files alone never establish canonical package membership", %{tmp_dir: tmp_dir} do
      File.write!(Path.join(tmp_dir, "BUILD_windows"), "echo windows")
      File.write!(Path.join(tmp_dir, "BUILD_mac"), "echo mac")
      File.write!(Path.join(tmp_dir, "BUILD_linux"), "echo linux")
      File.write!(Path.join(tmp_dir, "BUILD_mac_and_linux"), "echo unix")

      for platform <- [:windows, :darwin, :linux] do
        assert Discovery.get_build_file_for_platform(tmp_dir, platform) == nil
      end
    end

    test "only exact-case canonical and variant basenames are recognized", %{tmp_dir: tmp_dir} do
      File.write!(Path.join(tmp_dir, "build"), "echo wrong canonical\n")
      File.write!(Path.join(tmp_dir, "build_windows"), "echo wrong windows\n")

      assert Discovery.get_build_file_for_platform(tmp_dir, :windows) == nil
      assert Discovery.discover_packages(tmp_dir, :windows) == []

      # A case-insensitive filesystem cannot hold `build` and `BUILD` together.
      File.rm!(Path.join(tmp_dir, "build"))
      File.write!(Path.join(tmp_dir, "BUILD"), "echo canonical\n")

      assert Discovery.get_build_file_for_platform(tmp_dir, :windows) ==
               Path.join(tmp_dir, "BUILD")

      assert Discovery.discover_packages(tmp_dir, :windows) |> hd() |> Map.fetch!(:build_commands) ==
               ["echo canonical"]
    end

    test "BUILD_mac overrides BUILD_mac_and_linux on darwin", %{tmp_dir: tmp_dir} do
      File.write!(Path.join(tmp_dir, "BUILD_mac"), "echo mac")
      File.write!(Path.join(tmp_dir, "BUILD_mac_and_linux"), "echo unix")
      File.write!(Path.join(tmp_dir, "BUILD"), "echo generic")

      result = Discovery.get_build_file_for_platform(tmp_dir, :darwin)
      assert result == Path.join(tmp_dir, "BUILD_mac")
    end
  end

  # ---------------------------------------------------------------------------
  # discover_packages/1
  # ---------------------------------------------------------------------------

  describe "discover_packages/1" do
    test "projects the checked Windows override through production discovery", %{
      tmp_dir: tmp_dir
    } do
      fixture = materialize_discovery_fixture!(tmp_dir, "discovery-windows-override.json")
      code_root = Path.join(tmp_dir, "code")

      packages = Discovery.discover_packages(code_root, :windows)

      assert project_discovery(packages, tmp_dir, :windows) ==
               fixture["expected"]["result"]["packages"]

      [package] = packages
      assert package.build_commands == ["python -m unittest discover tests"]
      assert package.build_content == "python -m unittest discover tests\n"
      refute StarlarkEvaluator.starlark_build?(Enum.join(package.build_commands, "\n"))

      # On the actual Windows runner, the unparameterized public entry points
      # must route BEAM's {:win32, _} result to the same override.
      if match?({:win32, _}, :os.type()) do
        assert Discovery.get_build_file(package.path) ==
                 Path.join(package.path, "BUILD_windows")

        assert project_discovery(Discovery.discover_packages(code_root), tmp_dir, :windows) ==
                 fixture["expected"]["result"]["packages"]

        plan_path = Path.join(tmp_dir, "windows-plan.json")

        output =
          capture_io(fn ->
            assert CLI.run(["--root", tmp_dir, "--force", "--emit-plan", plan_path]) == 0
          end)

        refute output =~ "Evaluated 1 Starlark BUILD"
        [planned] = plan_path |> File.read!() |> Jason.decode!() |> Map.fetch!("packages")
        assert planned["name"] == "python/demo"
        assert planned["build_commands"] == ["python -m unittest discover tests"]
        assert planned["is_starlark"] == false
      end
    end

    test "projects the checked variant-only non-package on every platform", %{
      tmp_dir: tmp_dir
    } do
      fixture =
        materialize_discovery_fixture!(tmp_dir, "discovery-variant-without-canonical.json")

      code_root = Path.join(tmp_dir, "code")

      for platform <- [:windows, :darwin, :linux] do
        packages = Discovery.discover_packages(code_root, platform)

        assert project_discovery(packages, tmp_dir, platform) ==
                 fixture["expected"]["result"]["packages"]
      end

      assert Discovery.discover_packages(code_root) == []
    end

    test "recurses through a variant-only directory to canonical child", %{tmp_dir: tmp_dir} do
      parent = Path.join([tmp_dir, "code", "packages", "python", "parent"])
      child = Path.join(parent, "child")
      File.mkdir_p!(child)
      File.write!(Path.join(parent, "BUILD_windows"), "echo variant\n")
      File.write!(Path.join(child, "BUILD"), "echo child\n")

      assert Enum.map(
               Discovery.discover_packages(Path.join(tmp_dir, "code"), :windows),
               & &1.name
             ) ==
               ["python/child"]
    end

    test "rejects checked duplicate identity before CLI resolution", %{tmp_dir: tmp_dir} do
      fixture_path =
        Path.expand(
          "../../../../specs/fixtures/build-tool-v1/cases/discovery-duplicate-identity.json",
          __DIR__
        )

      fixture = fixture_path |> File.read!() |> Jason.decode!()
      files = fixture["workspace"]["files"]
      assert length(files) == 2

      Enum.each(files, fn file ->
        parts = String.split(file["path"], "/")
        assert Enum.take(parts, 2) == ["code", "packages"]
        assert List.last(parts) == "BUILD"

        assert Enum.all?(
                 parts,
                 &(&1 not in ["", ".", ".."] and not String.contains?(&1, ["\\", ":"]))
               )

        destination = Path.join([tmp_dir | parts])
        File.mkdir_p!(Path.dirname(destination))
        {:ok, output} = File.open(destination, [:write, :exclusive, :binary])
        :ok = IO.binwrite(output, file["content_utf8"])
        :ok = File.close(output)
      end)

      error =
        assert_raise DuplicatePackageIdentityError, fn ->
          Discovery.discover_packages(Path.join(tmp_dir, "code"))
        end

      diagnostic = %{
        "code" => error.code,
        "severity" => "error",
        "path" => hd(error.paths),
        "package" => error.package,
        "details" => %{"paths" => error.paths}
      }

      # The test-local projection compares the native diagnostic to the
      # neutral case. It is not a registered conformance adapter.
      assert %{
               "schema_version" => 1,
               "case_id" => fixture["id"],
               "domain" => fixture["domain"],
               "outcome" => "error",
               "result" => %{},
               "diagnostics" => [diagnostic]
             } == fixture["expected"]

      refute Exception.message(error) =~ tmp_dir

      stderr =
        capture_io(:stderr, fn ->
          assert CLI.run(["--root", tmp_dir, "--force", "--dry-run"]) == 2
        end)

      assert stderr =~ "DUPLICATE_PACKAGE_IDENTITY"
      assert stderr =~ "unknown/demo"
      assert stderr =~ "code/packages/alpha/demo"
      assert stderr =~ "code/packages/beta/demo"
      refute stderr =~ tmp_dir
    end

    test "matches the complete neutral language registry", %{tmp_dir: tmp_dir} do
      fixture_path =
        Path.expand(
          "../../../../specs/fixtures/build-tool-v1/cases/discovery-language-registry.json",
          __DIR__
        )

      fixture = fixture_path |> File.read!() |> Jason.decode!()
      files = fixture["workspace"]["files"]
      expected = fixture["expected"]["result"]["packages"]

      # These cardinalities pin the independent upstream corpus: a future
      # fixture change must be reviewed, not silently normalized to our walk.
      assert length(files) == 29
      assert length(expected) == 25

      Enum.each(files, fn file ->
        parts = String.split(file["path"], "/")

        assert hd(parts) == "code"
        assert List.last(parts) == "BUILD"

        assert Enum.all?(
                 parts,
                 &(&1 not in ["", ".", ".."] and not String.contains?(&1, ["\\", ":"]))
               )

        destination = Path.join([tmp_dir | parts])
        File.mkdir_p!(Path.dirname(destination))
        File.write!(destination, file["content_utf8"])
      end)

      actual =
        Path.join(tmp_dir, "code")
        |> Discovery.discover_packages()
        |> Enum.map(fn package ->
          %{
            "name" => package.name,
            "language" => package.language,
            "is_starlark" => false,
            "rel_path" => package.path |> Path.relative_to(tmp_dir) |> String.replace("\\", "/"),
            "build_file" =>
              package.path
              |> Discovery.get_build_file()
              |> Path.relative_to(tmp_dir)
              |> String.replace("\\", "/")
          }
        end)
        |> Enum.sort_by(& &1["name"])

      assert actual == Enum.sort_by(expected, & &1["name"])
    end

    test "projects checked Dune discovery records through the production walk", %{
      tmp_dir: tmp_dir
    } do
      fixture_path =
        Path.expand(
          "../../../../specs/fixtures/build-tool-v1/cases/discovery-language-registry.json",
          __DIR__
        )

      fixture = fixture_path |> File.read!() |> Jason.decode!()

      ocaml_files =
        Enum.filter(fixture["workspace"]["files"], fn file ->
          String.starts_with?(file["path"], "code/packages/ocaml/")
        end)

      expected =
        fixture["expected"]["result"]["packages"]
        |> Enum.filter(&String.starts_with?(&1["build_file"], "code/packages/ocaml/"))
        |> Enum.map(&{&1["name"], &1["build_file"], &1["rel_path"]})
        |> Enum.sort()

      assert length(ocaml_files) == 4
      assert length(expected) == 3

      Enum.each(ocaml_files, fn file ->
        path = file["path"]
        parts = String.split(path, "/")

        assert Enum.take(parts, 3) == ["code", "packages", "ocaml"]

        assert Enum.all?(
                 parts,
                 &(&1 not in ["", ".", ".."] and not String.contains?(&1, ["\\", ":"]))
               )

        destination = Path.join([tmp_dir | parts])
        File.mkdir_p!(Path.dirname(destination))
        File.write!(destination, file["content_utf8"])
      end)

      actual =
        Path.join(tmp_dir, "code")
        |> Discovery.discover_packages()
        |> Enum.map(fn package ->
          {package.name,
           package.path
           |> Discovery.get_build_file()
           |> Path.relative_to(tmp_dir)
           |> String.replace("\\", "/"),
           package.path |> Path.relative_to(tmp_dir) |> String.replace("\\", "/")}
        end)
        |> Enum.sort()

      assert actual == expected

      assert Enum.map(actual, &elem(&1, 0)) == [
               "ocaml/case-source",
               "ocaml/demo-ocaml",
               "ocaml/near-source"
             ]
    end

    test "discovers packages with BUILD files", %{tmp_dir: tmp_dir} do
      # Create: tmp_dir/packages/python/logic-gates/BUILD
      pkg_dir = Path.join([tmp_dir, "packages", "python", "logic-gates"])
      File.mkdir_p!(pkg_dir)
      File.write!(Path.join(pkg_dir, "BUILD"), "pytest\n")

      packages = Discovery.discover_packages(tmp_dir)
      assert length(packages) == 1
      [pkg] = packages
      assert pkg.name == "python/logic-gates"
      assert pkg.language == "python"
      assert pkg.build_commands == ["pytest"]
      assert pkg.build_content == "pytest\n"
      assert pkg.path == pkg_dir
    end

    test "discovers multiple packages sorted by name", %{tmp_dir: tmp_dir} do
      for name <- ["alpha", "beta"] do
        dir = Path.join([tmp_dir, "packages", "python", name])
        File.mkdir_p!(dir)
        File.write!(Path.join(dir, "BUILD"), "echo #{name}\n")
      end

      packages = Discovery.discover_packages(tmp_dir)
      names = Enum.map(packages, & &1.name)
      assert names == ["python/alpha", "python/beta"]
    end

    test "skips directories in the skip list", %{tmp_dir: tmp_dir} do
      # Create a package inside node_modules — should be skipped.
      skip_dir = Path.join([tmp_dir, "node_modules", "python", "hidden"])
      File.mkdir_p!(skip_dir)
      File.write!(Path.join(skip_dir, "BUILD"), "echo hidden\n")

      packages = Discovery.discover_packages(tmp_dir)
      assert packages == []
    end

    test "does not recurse into package directories", %{tmp_dir: tmp_dir} do
      # Create a package with a sub-BUILD — the sub should NOT be discovered.
      parent = Path.join([tmp_dir, "packages", "python", "parent"])
      File.mkdir_p!(parent)
      File.write!(Path.join(parent, "BUILD"), "echo parent\n")

      child = Path.join(parent, "child")
      File.mkdir_p!(child)
      File.write!(Path.join(child, "BUILD"), "echo child\n")

      packages = Discovery.discover_packages(tmp_dir)
      assert length(packages) == 1
      assert hd(packages).name == "python/parent"
    end

    test "returns empty list when no packages found", %{tmp_dir: tmp_dir} do
      assert Discovery.discover_packages(tmp_dir) == []
    end
  end

  defp materialize_discovery_fixture!(tmp_dir, filename) do
    fixture_path =
      Path.expand("../../../../specs/fixtures/build-tool-v1/cases/#{filename}", __DIR__)

    fixture = fixture_path |> File.read!() |> Jason.decode!()
    assert fixture["domain"] == "discovery"
    assert length(fixture["workspace"]["files"]) == 2

    Enum.each(fixture["workspace"]["files"], fn file ->
      parts = String.split(file["path"], "/")
      assert Enum.take(parts, 2) == ["code", "packages"]
      assert List.last(parts) in ["BUILD", "BUILD_windows", "BUILD_mac"]

      assert Enum.all?(
               parts,
               &(&1 not in ["", ".", ".."] and not String.contains?(&1, ["\\", ":"]))
             )

      destination = Path.join([tmp_dir | parts])
      File.mkdir_p!(Path.dirname(destination))
      {:ok, output} = File.open(destination, [:write, :exclusive, :binary])
      :ok = IO.binwrite(output, file["content_utf8"])
      :ok = File.close(output)
    end)

    fixture
  end

  defp project_discovery(packages, tmp_dir, platform) do
    packages
    |> Enum.map(fn package ->
      %{
        "name" => package.name,
        "language" => package.language,
        "is_starlark" =>
          StarlarkEvaluator.starlark_build?(Enum.join(package.build_commands, "\n")),
        "rel_path" => package.path |> Path.relative_to(tmp_dir) |> String.replace("\\", "/"),
        "build_file" =>
          package.path
          |> Discovery.get_build_file_for_platform(platform)
          |> Path.relative_to(tmp_dir)
          |> String.replace("\\", "/")
      }
    end)
    |> Enum.sort_by(& &1["name"])
  end
end
