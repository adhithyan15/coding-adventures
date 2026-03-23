# frozen_string_literal: true

# test_plan.rb -- Tests for build plan serialization/deserialization
# ==================================================================
#
# These tests verify round-trip serialization, version rejection,
# missing file handling, and nil vs empty affected_packages semantics.

require_relative "test_helper"

class TestPlan < Minitest::Test
  include TestHelper

  # -- Round-trip tests --------------------------------------------------------

  def test_round_trip_basic
    # A plan written and then read should produce identical data.
    dir = create_temp_dir
    path = dir / "plan.json"

    original = BuildTool::Plan::BuildPlan.new(
      diff_base: "origin/main",
      force: false,
      affected_packages: %w[python/foo ruby/bar],
      packages: [
        BuildTool::Plan::PackageEntry.new(
          name: "python/foo",
          rel_path: "code/packages/python/foo",
          language: "python",
          build_commands: ["uv pip install -e .", "pytest"]
        ),
        BuildTool::Plan::PackageEntry.new(
          name: "ruby/bar",
          rel_path: "code/packages/ruby/bar",
          language: "ruby",
          build_commands: ["bundle install", "rake test"]
        )
      ],
      dependency_edges: [%w[python/foo ruby/bar]],
      languages_needed: { "python" => true, "ruby" => true }
    )

    BuildTool::Plan.write_plan(original, path)
    loaded = BuildTool::Plan.read_plan(path)

    assert_equal original.diff_base, loaded.diff_base
    assert_equal original.force, loaded.force
    assert_equal original.affected_packages, loaded.affected_packages
    assert_equal original.packages.size, loaded.packages.size
    assert_equal original.dependency_edges, loaded.dependency_edges
    assert_equal original.languages_needed, loaded.languages_needed

    # Verify package contents.
    assert_equal "python/foo", loaded.packages[0].name
    assert_equal "code/packages/python/foo", loaded.packages[0].rel_path
    assert_equal "python", loaded.packages[0].language
    assert_equal ["uv pip install -e .", "pytest"], loaded.packages[0].build_commands
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_round_trip_with_starlark_metadata
    # Starlark packages include declared_srcs and declared_deps.
    dir = create_temp_dir
    path = dir / "plan.json"

    original = BuildTool::Plan::BuildPlan.new(
      packages: [
        BuildTool::Plan::PackageEntry.new(
          name: "python/foo",
          rel_path: "code/packages/python/foo",
          language: "python",
          build_commands: ["pytest"],
          is_starlark: true,
          declared_srcs: ["src/**/*.py", "tests/**/*.py"],
          declared_deps: ["python/bar"]
        )
      ]
    )

    BuildTool::Plan.write_plan(original, path)
    loaded = BuildTool::Plan.read_plan(path)

    pkg = loaded.packages[0]
    assert_equal true, pkg.is_starlark
    assert_equal ["src/**/*.py", "tests/**/*.py"], pkg.declared_srcs
    assert_equal ["python/bar"], pkg.declared_deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_round_trip_empty_plan
    # An empty plan (no packages, no edges) should round-trip cleanly.
    dir = create_temp_dir
    path = dir / "plan.json"

    original = BuildTool::Plan::BuildPlan.new
    BuildTool::Plan.write_plan(original, path)
    loaded = BuildTool::Plan.read_plan(path)

    assert_equal "", loaded.diff_base
    assert_equal false, loaded.force
    assert_nil loaded.affected_packages
    assert_equal [], loaded.packages
    assert_equal [], loaded.dependency_edges
    assert_equal({}, loaded.languages_needed)
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- Nil vs empty affected_packages ------------------------------------------

  def test_nil_affected_packages_round_trip
    # nil means "rebuild all" -- must survive round-trip as nil, not [].
    dir = create_temp_dir
    path = dir / "plan.json"

    original = BuildTool::Plan::BuildPlan.new(affected_packages: nil)
    BuildTool::Plan.write_plan(original, path)
    loaded = BuildTool::Plan.read_plan(path)

    assert_nil loaded.affected_packages
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_empty_affected_packages_round_trip
    # [] means "nothing changed" -- must survive round-trip as [].
    dir = create_temp_dir
    path = dir / "plan.json"

    original = BuildTool::Plan::BuildPlan.new(affected_packages: [])
    BuildTool::Plan.write_plan(original, path)
    loaded = BuildTool::Plan.read_plan(path)

    assert_equal [], loaded.affected_packages
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- Schema version tests ----------------------------------------------------

  def test_current_version_accepted
    dir = create_temp_dir
    path = dir / "plan.json"

    plan = BuildTool::Plan::BuildPlan.new(schema_version: 1)
    BuildTool::Plan.write_plan(plan, path)
    loaded = BuildTool::Plan.read_plan(path)

    assert_equal BuildTool::Plan::CURRENT_SCHEMA_VERSION, loaded.schema_version
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_future_version_rejected
    # A plan with a schema_version higher than CURRENT should be rejected.
    dir = create_temp_dir
    path = dir / "plan.json"

    # Write a plan manually with a future version.
    future_plan = { "schema_version" => 999, "packages" => [] }
    path.write(JSON.generate(future_plan))

    error = assert_raises(RuntimeError) do
      BuildTool::Plan.read_plan(path)
    end

    assert_match(/unsupported build plan version 999/, error.message)
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_older_version_accepted
    # A plan with schema_version 0 (older) should be accepted.
    dir = create_temp_dir
    path = dir / "plan.json"

    old_plan = { "schema_version" => 0, "packages" => [] }
    path.write(JSON.generate(old_plan))

    loaded = BuildTool::Plan.read_plan(path)
    assert_equal 0, loaded.schema_version
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- Error handling ----------------------------------------------------------

  def test_missing_file_raises
    error = assert_raises(RuntimeError) do
      BuildTool::Plan.read_plan("/nonexistent/path/plan.json")
    end

    assert_match(/not found/, error.message)
  end

  def test_malformed_json_raises
    dir = create_temp_dir
    path = dir / "plan.json"
    path.write("not valid json {{{")

    assert_raises(RuntimeError) do
      BuildTool::Plan.read_plan(path)
    end
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- PackageEntry defaults ---------------------------------------------------

  def test_package_entry_defaults
    pkg = BuildTool::Plan::PackageEntry.new(
      name: "go/foo",
      rel_path: "code/packages/go/foo",
      language: "go",
      build_commands: ["go test"]
    )

    assert_equal false, pkg.is_starlark
    assert_equal [], pkg.declared_srcs
    assert_equal [], pkg.declared_deps
  end

  def test_build_plan_defaults
    plan = BuildTool::Plan::BuildPlan.new

    assert_equal BuildTool::Plan::CURRENT_SCHEMA_VERSION, plan.schema_version
    assert_equal "", plan.diff_base
    assert_equal false, plan.force
    assert_nil plan.affected_packages
    assert_equal [], plan.packages
    assert_equal [], plan.dependency_edges
    assert_equal({}, plan.languages_needed)
  end

  # -- Write always stamps current version ------------------------------------

  def test_write_stamps_current_version
    dir = create_temp_dir
    path = dir / "plan.json"

    # Even if schema_version is set to something else, write stamps current.
    plan = BuildTool::Plan::BuildPlan.new(schema_version: 42)
    BuildTool::Plan.write_plan(plan, path)

    raw = JSON.parse(path.read)
    assert_equal BuildTool::Plan::CURRENT_SCHEMA_VERSION, raw["schema_version"]
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- Path normalization ------------------------------------------------------

  def test_backslash_paths_normalized
    # Windows-style backslash paths should be normalized to forward slashes.
    dir = create_temp_dir
    path = dir / "plan.json"

    plan = BuildTool::Plan::BuildPlan.new(
      packages: [
        BuildTool::Plan::PackageEntry.new(
          name: "python/foo",
          rel_path: "code\\packages\\python\\foo",
          language: "python",
          build_commands: []
        )
      ]
    )

    BuildTool::Plan.write_plan(plan, path)
    raw = JSON.parse(path.read)

    assert_equal "code/packages/python/foo", raw["packages"][0]["rel_path"]
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- Omitempty for declared_srcs/deps ----------------------------------------

  def test_empty_srcs_deps_omitted_in_json
    dir = create_temp_dir
    path = dir / "plan.json"

    plan = BuildTool::Plan::BuildPlan.new(
      packages: [
        BuildTool::Plan::PackageEntry.new(
          name: "go/bar",
          rel_path: "code/packages/go/bar",
          language: "go",
          build_commands: ["go test"]
        )
      ]
    )

    BuildTool::Plan.write_plan(plan, path)
    raw = JSON.parse(path.read)

    # Empty declared_srcs and declared_deps should not appear in JSON.
    refute raw["packages"][0].key?("declared_srcs")
    refute raw["packages"][0].key?("declared_deps")
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_nonempty_srcs_included_in_json
    dir = create_temp_dir
    path = dir / "plan.json"

    plan = BuildTool::Plan::BuildPlan.new(
      packages: [
        BuildTool::Plan::PackageEntry.new(
          name: "python/foo",
          rel_path: "code/packages/python/foo",
          language: "python",
          build_commands: [],
          declared_srcs: ["src/**/*.py"]
        )
      ]
    )

    BuildTool::Plan.write_plan(plan, path)
    raw = JSON.parse(path.read)

    assert_equal ["src/**/*.py"], raw["packages"][0]["declared_srcs"]
  ensure
    FileUtils.rm_rf(dir)
  end
end
