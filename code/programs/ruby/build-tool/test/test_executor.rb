# frozen_string_literal: true

# test_executor.rb -- Tests for parallel build execution
# ======================================================
#
# These tests verify single-package building, the full execute_builds
# pipeline with skip/force/dry-run logic, and dep-skipped propagation
# when a dependency fails.

require_relative "test_helper"

class TestExecutor < Minitest::Test
  include TestHelper

  # -- run_package_build tests -------------------------------------------------

  def test_run_package_build_success
    dir = create_temp_dir
    write_file(dir / "BUILD", "echo hello")

    pkg = BuildTool::Package.new(
      name: "test/pkg", path: dir,
      build_commands: ['echo "hello"'], language: "unknown"
    )

    result = BuildTool::Executor.run_package_build(pkg)
    assert_equal "built", result.status
    assert_equal 0, result.return_code
    assert_includes result.stdout, "hello"
    assert result.duration >= 0
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_run_package_build_failure
    dir = create_temp_dir
    write_file(dir / "BUILD", "exit 1")

    pkg = BuildTool::Package.new(
      name: "test/pkg", path: dir,
      build_commands: ["exit 1"], language: "unknown"
    )

    result = BuildTool::Executor.run_package_build(pkg)
    assert_equal "failed", result.status
    assert_equal 1, result.return_code
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_run_package_build_stops_on_first_failure
    dir = create_temp_dir

    pkg = BuildTool::Package.new(
      name: "test/pkg", path: dir,
      build_commands: ['echo "step1"', "exit 1", 'echo "step3"'],
      language: "unknown"
    )

    result = BuildTool::Executor.run_package_build(pkg)
    assert_equal "failed", result.status
    assert_includes result.stdout, "step1"
    refute_includes result.stdout, "step3"
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_run_package_build_multiple_commands
    dir = create_temp_dir

    pkg = BuildTool::Package.new(
      name: "test/pkg", path: dir,
      build_commands: ['echo "one"', 'echo "two"'],
      language: "unknown"
    )

    result = BuildTool::Executor.run_package_build(pkg)
    assert_equal "built", result.status
    assert_includes result.stdout, "one"
    assert_includes result.stdout, "two"
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- BuildResult Data.define test --------------------------------------------

  def test_build_result_defaults
    result = BuildTool::BuildResult.new(package_name: "test/pkg", status: "built")
    assert_equal 0.0, result.duration
    assert_equal "", result.stdout
    assert_equal "", result.stderr
    assert_equal 0, result.return_code
  end

  # -- execute_builds integration tests ----------------------------------------

  def test_execute_builds_skip_when_cached
    packages = BuildTool::Discovery.discover_packages(simple_fixture)
    graph = BuildTool::Resolver.resolve_dependencies(packages)

    pkg = packages.first
    pkg_hash = BuildTool::Hasher.hash_package(pkg)
    dep_hash = BuildTool::Hasher.hash_deps(pkg.name, graph, { pkg.name => pkg_hash })

    # Pre-populate cache so the package is up-to-date.
    cache = BuildTool::BuildCache.new
    cache.record(pkg.name, pkg_hash, dep_hash, "success")

    results = BuildTool::Executor.execute_builds(
      packages: packages, graph: graph, cache: cache,
      package_hashes: { pkg.name => pkg_hash },
      deps_hashes: { pkg.name => dep_hash }
    )

    assert_equal "skipped", results[pkg.name].status
  end

  def test_execute_builds_force_rebuilds_cached
    packages = BuildTool::Discovery.discover_packages(simple_fixture)
    graph = BuildTool::Resolver.resolve_dependencies(packages)

    pkg = packages.first
    pkg_hash = BuildTool::Hasher.hash_package(pkg)
    dep_hash = BuildTool::Hasher.hash_deps(pkg.name, graph, { pkg.name => pkg_hash })

    cache = BuildTool::BuildCache.new
    cache.record(pkg.name, pkg_hash, dep_hash, "success")

    results = BuildTool::Executor.execute_builds(
      packages: packages, graph: graph, cache: cache,
      package_hashes: { pkg.name => pkg_hash },
      deps_hashes: { pkg.name => dep_hash },
      force: true
    )

    assert_equal "built", results[pkg.name].status
  end

  def test_execute_builds_dry_run
    packages = BuildTool::Discovery.discover_packages(simple_fixture)
    graph = BuildTool::Resolver.resolve_dependencies(packages)
    cache = BuildTool::BuildCache.new

    pkg = packages.first
    pkg_hash = BuildTool::Hasher.hash_package(pkg)

    results = BuildTool::Executor.execute_builds(
      packages: packages, graph: graph, cache: cache,
      package_hashes: { pkg.name => pkg_hash },
      deps_hashes: { pkg.name => "fake" },
      dry_run: true
    )

    assert_equal "would-build", results[pkg.name].status
  end

  def test_execute_builds_dep_skipped_on_failure
    # Create two packages where pkg-a depends on pkg-b, and pkg-b fails.
    dir = create_temp_dir
    pkg_b_dir = dir / "python" / "pkg-b"
    write_file(pkg_b_dir / "BUILD", "exit 1")
    pkg_a_dir = dir / "python" / "pkg-a"
    write_file(pkg_a_dir / "BUILD", 'echo "building a"')

    pkg_b = BuildTool::Package.new(
      name: "python/pkg-b", path: pkg_b_dir,
      build_commands: ["exit 1"], language: "python"
    )
    pkg_a = BuildTool::Package.new(
      name: "python/pkg-a", path: pkg_a_dir,
      build_commands: ['echo "building a"'], language: "python"
    )

    graph = BuildTool::DirectedGraph.new
    graph.add_edge("python/pkg-b", "python/pkg-a")

    cache = BuildTool::BuildCache.new

    results = BuildTool::Executor.execute_builds(
      packages: [pkg_a, pkg_b], graph: graph, cache: cache,
      package_hashes: { "python/pkg-a" => "ha", "python/pkg-b" => "hb" },
      deps_hashes: { "python/pkg-a" => "da", "python/pkg-b" => "db" }
    )

    assert_equal "failed", results["python/pkg-b"].status
    assert_equal "dep-skipped", results["python/pkg-a"].status
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_execute_builds_diamond
    packages = BuildTool::Discovery.discover_packages(diamond_fixture)
    graph = BuildTool::Resolver.resolve_dependencies(packages)
    cache = BuildTool::BuildCache.new

    pkg_hashes = {}
    dep_hashes = {}
    packages.each do |pkg|
      pkg_hashes[pkg.name] = BuildTool::Hasher.hash_package(pkg)
      dep_hashes[pkg.name] = BuildTool::Hasher.hash_deps(pkg.name, graph, pkg_hashes)
    end

    results = BuildTool::Executor.execute_builds(
      packages: packages, graph: graph, cache: cache,
      package_hashes: pkg_hashes, deps_hashes: dep_hashes
    )

    # All 4 should build successfully.
    assert_equal 4, results.size
    results.each_value { |r| assert_equal "built", r.status }
  end
end
