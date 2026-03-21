defmodule BuildTool.DiscoveryTest do
  use ExUnit.Case, async: true

  alias BuildTool.Discovery

  # ---------------------------------------------------------------------------
  # Setup: create temporary directories for testing
  # ---------------------------------------------------------------------------

  setup do
    # Create a temporary directory structure that mimics the monorepo layout.
    tmp_dir = Path.join(System.tmp_dir!(), "build_tool_discovery_test_#{:rand.uniform(100_000)}")
    File.rm_rf!(tmp_dir)
    File.mkdir_p!(tmp_dir)

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
  end

  # ---------------------------------------------------------------------------
  # discover_packages/1
  # ---------------------------------------------------------------------------

  describe "discover_packages/1" do
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
end
