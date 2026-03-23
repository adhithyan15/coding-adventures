defmodule BuildTool.PlanTest do
  use ExUnit.Case, async: true

  alias BuildTool.Plan
  alias BuildTool.Plan.PackageEntry

  # ===========================================================================
  # Setup
  # ===========================================================================

  setup do
    tmp_dir = Path.join(System.tmp_dir!(), "build_tool_plan_test_#{:rand.uniform(100_000)}")
    File.rm_rf!(tmp_dir)
    File.mkdir_p!(tmp_dir)

    on_exit(fn -> File.rm_rf!(tmp_dir) end)

    {:ok, tmp_dir: tmp_dir}
  end

  # ===========================================================================
  # Round-trip: write then read
  # ===========================================================================

  describe "write_plan/2 and read_plan/1 round-trip" do
    test "round-trips a simple plan", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "plan.json")

      plan = %Plan{
        diff_base: "origin/main",
        force: false,
        affected_packages: ["python/logic-gates", "go/directed-graph"],
        packages: [
          %PackageEntry{
            name: "python/logic-gates",
            rel_path: "code/packages/python/logic-gates",
            language: "python",
            build_commands: ["pip install -e .", "pytest"],
            is_starlark: false
          },
          %PackageEntry{
            name: "go/directed-graph",
            rel_path: "code/packages/go/directed-graph",
            language: "go",
            build_commands: ["go build ./...", "go test ./..."],
            is_starlark: false
          }
        ],
        dependency_edges: [["python/logic-gates", "python/arithmetic"]],
        languages_needed: %{"python" => true, "go" => true}
      }

      assert :ok = Plan.write_plan(plan, path)
      assert {:ok, loaded} = Plan.read_plan(path)

      assert loaded.schema_version == Plan.current_schema_version()
      assert loaded.diff_base == "origin/main"
      assert loaded.force == false
      assert loaded.affected_packages == ["python/logic-gates", "go/directed-graph"]
      assert length(loaded.packages) == 2
      assert loaded.dependency_edges == [["python/logic-gates", "python/arithmetic"]]
      assert loaded.languages_needed == %{"python" => true, "go" => true}
    end

    test "round-trips a plan with Starlark metadata", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "starlark-plan.json")

      plan = %Plan{
        diff_base: "origin/main",
        force: false,
        affected_packages: ["python/logic-gates"],
        packages: [
          %PackageEntry{
            name: "python/logic-gates",
            rel_path: "code/packages/python/logic-gates",
            language: "python",
            build_commands: ["pip install -e .", "pytest"],
            is_starlark: true,
            declared_srcs: ["src/**/*.py", "tests/**/*.py"],
            declared_deps: ["python/arithmetic"]
          }
        ],
        dependency_edges: [],
        languages_needed: %{"python" => true}
      }

      assert :ok = Plan.write_plan(plan, path)
      assert {:ok, loaded} = Plan.read_plan(path)

      pkg = hd(loaded.packages)
      assert pkg.is_starlark == true
      assert pkg.declared_srcs == ["src/**/*.py", "tests/**/*.py"]
      assert pkg.declared_deps == ["python/arithmetic"]
    end

    test "round-trips a force-mode plan with nil affected_packages", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "force-plan.json")

      plan = %Plan{
        diff_base: "origin/main",
        force: true,
        affected_packages: nil,
        packages: [],
        dependency_edges: [],
        languages_needed: %{}
      }

      assert :ok = Plan.write_plan(plan, path)
      assert {:ok, loaded} = Plan.read_plan(path)

      assert loaded.force == true
      assert loaded.affected_packages == nil
    end

    test "round-trips a plan with empty affected_packages", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "empty-plan.json")

      plan = %Plan{
        affected_packages: [],
        packages: [],
        dependency_edges: [],
        languages_needed: %{}
      }

      assert :ok = Plan.write_plan(plan, path)
      assert {:ok, loaded} = Plan.read_plan(path)

      assert loaded.affected_packages == []
    end

    test "omits declared_srcs and declared_deps when empty", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "omit-plan.json")

      plan = %Plan{
        affected_packages: [],
        packages: [
          %PackageEntry{
            name: "go/foo",
            rel_path: "code/packages/go/foo",
            language: "go",
            build_commands: ["go build"],
            is_starlark: false,
            declared_srcs: [],
            declared_deps: []
          }
        ],
        dependency_edges: [],
        languages_needed: %{}
      }

      assert :ok = Plan.write_plan(plan, path)

      # Read raw JSON to verify omitempty behavior.
      {:ok, raw} = File.read(path)
      {:ok, json} = Jason.decode(raw)
      pkg_json = hd(json["packages"])

      refute Map.has_key?(pkg_json, "declared_srcs")
      refute Map.has_key?(pkg_json, "declared_deps")
    end
  end

  # ===========================================================================
  # Schema version rejection
  # ===========================================================================

  describe "schema version handling" do
    test "rejects plans with a higher schema version", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "future-plan.json")

      # Write a plan with a future schema version directly.
      future_json = Jason.encode!(%{
        "schema_version" => 999,
        "diff_base" => "origin/main",
        "force" => false,
        "affected_packages" => [],
        "packages" => [],
        "dependency_edges" => [],
        "languages_needed" => %{}
      })

      File.write!(path, future_json)

      assert {:error, msg} = Plan.read_plan(path)
      assert msg =~ "unsupported build plan version 999"
      assert msg =~ "supports up to #{Plan.current_schema_version()}"
    end

    test "accepts plans with the current schema version", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "current-plan.json")

      json = Jason.encode!(%{
        "schema_version" => Plan.current_schema_version(),
        "diff_base" => "origin/main",
        "force" => false,
        "affected_packages" => [],
        "packages" => [],
        "dependency_edges" => [],
        "languages_needed" => %{}
      })

      File.write!(path, json)

      assert {:ok, _plan} = Plan.read_plan(path)
    end

    test "accepts plans with an older schema version", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "old-plan.json")

      json = Jason.encode!(%{
        "schema_version" => 0,
        "diff_base" => "origin/main",
        "force" => false,
        "affected_packages" => [],
        "packages" => [],
        "dependency_edges" => [],
        "languages_needed" => %{}
      })

      File.write!(path, json)

      assert {:ok, plan} = Plan.read_plan(path)
      assert plan.schema_version == 0
    end
  end

  # ===========================================================================
  # Error handling
  # ===========================================================================

  describe "error handling" do
    test "returns error for missing file" do
      assert {:error, _} = Plan.read_plan("/nonexistent/path/plan.json")
    end

    test "returns error for invalid JSON", %{tmp_dir: tmp_dir} do
      path = Path.join(tmp_dir, "bad.json")
      File.write!(path, "this is not json {{{")

      assert {:error, _} = Plan.read_plan(path)
    end

    test "write_plan returns error for unwritable path" do
      plan = %Plan{packages: [], dependency_edges: [], languages_needed: %{}}
      assert {:error, _} = Plan.write_plan(plan, "/nonexistent/dir/plan.json")
    end
  end

  # ===========================================================================
  # Schema version accessor
  # ===========================================================================

  describe "current_schema_version/0" do
    test "returns an integer" do
      assert is_integer(Plan.current_schema_version())
    end

    test "is positive" do
      assert Plan.current_schema_version() >= 1
    end
  end
end
