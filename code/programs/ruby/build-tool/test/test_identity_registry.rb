# frozen_string_literal: true

# test_identity_registry.rb -- Shared canonical identity conformance
# ==================================================================

require_relative "test_helper"

require "open3"
require "rbconfig"

class TestIdentityRegistry < Minitest::Test
  include TestHelper

  REPO_ROOT = Pathname(__dir__).join("..", "..", "..", "..", "..").expand_path
  CASES_DIR = REPO_ROOT / "code" / "specs" / "fixtures" / "build-tool-v1" / "cases"
  BUILD_TOOL = REPO_ROOT / "code" / "programs" / "ruby" / "build-tool" / "build.rb"

  def test_shared_language_registry_fixture
    workspace, fixture = materialize_case("discovery-language-registry.json")

    packages = BuildTool::Discovery.discover_packages(workspace / "code")
    actual = packages.map do |package|
      {
        "name" => package.name,
        "language" => package.language,
        "rel_path" => package.path.relative_path_from(workspace).to_s.tr("\\", "/")
      }
    end
    expected = fixture.dig("expected", "result", "packages").map do |package|
      package.slice("name", "language", "rel_path")
    end

    assert_equal expected, actual
  ensure
    FileUtils.rm_rf(workspace)
  end

  def test_shared_duplicate_identity_fixture_fails_closed
    workspace, fixture = materialize_case("discovery-duplicate-identity.json")

    error = assert_raises(BuildTool::DuplicatePackageIdentityError) do
      BuildTool::Discovery.discover_packages(workspace / "code")
    end
    diagnostic = fixture.dig("expected", "diagnostics", 0)

    assert_equal diagnostic.fetch("code"), error.code
    assert_equal diagnostic.fetch("package"), error.package
    assert_equal diagnostic.dig("details", "paths"), error.paths
    assert_equal diagnostic.fetch("path"), error.paths.first
    assert_equal "#{error.code}: package=#{error.package} paths=#{error.paths.join(',')}", error.message
    refute_includes error.message, workspace.to_s
  ensure
    FileUtils.rm_rf(workspace)
  end

  def test_shared_elixir_program_package_resolution_fixture
    workspace, fixture = materialize_case("resolution-elixir-program-package.json")
    packages = BuildTool::Discovery.discover_packages(workspace / "code")

    graph = BuildTool::Resolver.resolve_dependencies(packages)

    assert_equal fixture.dig("expected", "result", "edges"), graph_edges(graph)
    assert_equal %w[elixir/grammar_tools elixir/programs/grammar_tools], graph.nodes.sort
  ensure
    FileUtils.rm_rf(workspace)
  end

  def test_shared_legacy_build_dependency_fixture
    workspace, fixture = materialize_case("resolution-build-deps-comment.json")
    packages = BuildTool::Discovery.discover_packages(workspace / "code")

    graph = BuildTool::Resolver.resolve_dependencies(packages)

    assert_equal fixture.dig("expected", "result", "edges"), graph_edges(graph)
  ensure
    FileUtils.rm_rf(workspace)
  end

  def test_real_cli_returns_exit_two_for_duplicate_identity
    workspace, fixture = materialize_case("discovery-duplicate-identity.json")
    diagnostic = fixture.dig("expected", "diagnostics", 0)
    expected = "#{diagnostic.fetch('code')}: package=#{diagnostic.fetch('package')} " \
               "paths=#{diagnostic.dig('details', 'paths').join(',')}\n"

    stdout, stderr, status = Open3.capture3(
      RbConfig.ruby,
      BUILD_TOOL.to_s,
      "--root", workspace.to_s,
      "--force",
      "--dry-run"
    )

    assert_equal 2, status.exitstatus
    assert_equal expected, stderr
    assert_empty stdout
    refute_includes stderr, workspace.to_s
  ensure
    FileUtils.rm_rf(workspace)
  end

  private

  def materialize_case(filename)
    fixture = JSON.parse((CASES_DIR / filename).read(encoding: "UTF-8"))
    workspace = create_temp_dir
    fixture.fetch("workspace").fetch("files").each do |entry|
      path = workspace / entry.fetch("path")
      path.dirname.mkpath
      File.binwrite(path, entry.fetch("content_utf8").encode(Encoding::UTF_8))
    end
    [workspace, fixture]
  end

  def graph_edges(graph)
    graph.nodes.sort.flat_map do |from|
      graph.successors(from).sort.map { |to| [from, to] }
    end
  end
end
