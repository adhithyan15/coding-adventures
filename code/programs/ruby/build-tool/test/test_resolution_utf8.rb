# frozen_string_literal: true

# test_resolution_utf8.rb -- Shared rockspec UTF-8 conformance
# ============================================================

require_relative "test_helper"

require "open3"
require "rbconfig"

class TestResolutionUtf8 < Minitest::Test
  include TestHelper

  REPO_ROOT = Pathname(__dir__).join("..", "..", "..", "..", "..").expand_path
  CASES_DIR = REPO_ROOT / "code" / "specs" / "fixtures" / "build-tool-v1" / "cases"
  BUILD_TOOL = REPO_ROOT / "code" / "programs" / "ruby" / "build-tool" / "build.rb"
  INVALID_SEQUENCES = {
    illegal_lead: "\xFF".b,
    unexpected_continuation: "\x80".b,
    truncated_sequence: "\xE2\x82".b,
    overlong_encoding: "\xC0\xAF".b,
    surrogate: "\xED\xA0\x80".b
  }.freeze

  def test_shared_valid_fixture_has_exact_edges
    workspace, fixture = materialize_case("resolution-lua-utf8.json")
    packages = BuildTool::Discovery.discover_packages(workspace / "code")

    graph = BuildTool::Resolver.resolve_dependencies(packages)

    assert_equal fixture.dig("expected", "result", "edges"), graph_edges(graph)
    assert_equal %w[lua/other lua/pkg], graph.nodes.sort
  ensure
    FileUtils.rm_rf(workspace)
  end

  def test_shared_invalid_fixture_raises_typed_stable_error
    workspace, = materialize_case("resolution-lua-invalid-utf8.json")
    packages = BuildTool::Discovery.discover_packages(workspace / "code")

    error = assert_raises(BuildTool::MetadataEncodingError) do
      BuildTool::Resolver.resolve_dependencies(packages)
    end

    assert_equal "METADATA_INVALID_UTF8", error.code
    assert_equal "lua/pkg", error.package
    assert_equal "code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec", error.manifest
    assert_equal "UTF-8", error.encoding
    assert_equal expected_diagnostic, error.message
    refute_includes error.message, workspace.to_s
  ensure
    FileUtils.rm_rf(workspace)
  end

  def test_literal_replacement_character_is_valid_utf8
    workspace = create_temp_dir
    package = write_lua_package(
      workspace,
      "pkg",
      <<~ROCKSPEC
        package = "coding-adventures-pkg"
        version = "0.1.0-1"
        description = { summary = "Literal replacement character: �" }
        dependencies = { "lua >= 5.4" }
      ROCKSPEC
    )

    assert_equal [], BuildTool::Resolver.parse_lua_deps(package, {})
  ensure
    FileUtils.rm_rf(workspace)
  end

  def test_rejects_representative_malformed_utf8_classes
    INVALID_SEQUENCES.each do |name, sequence|
      workspace = create_temp_dir
      bytes = <<~ROCKSPEC.b + sequence + "\n".b
        package = "coding-adventures-pkg"
        version = "0.1.0-1"
        dependencies = { "lua >= 5.4" }
        -- malformed #{name}:
      ROCKSPEC
      package = write_lua_package(workspace, "pkg", bytes, binary: true)

      error = assert_raises(BuildTool::MetadataEncodingError, name.to_s) do
        BuildTool::Resolver.parse_lua_deps(package, {})
      end
      assert_equal expected_diagnostic, error.message, name.to_s
    ensure
      FileUtils.rm_rf(workspace)
    end
  end

  def test_real_cli_returns_exit_two_with_stable_stderr
    workspace, = materialize_case("resolution-lua-invalid-utf8.json")

    stdout, stderr, status = Open3.capture3(
      RbConfig.ruby,
      BUILD_TOOL.to_s,
      "--root", workspace.to_s,
      "--language", "lua",
      "--force",
      "--dry-run"
    )

    assert_equal 2, status.exitstatus
    assert_includes stderr.lines.map(&:strip), expected_diagnostic
    refute_includes stderr, workspace.to_s
    refute_includes stdout, workspace.to_s
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
      content = if entry.key?("content_utf8")
                  entry.fetch("content_utf8").encode(Encoding::UTF_8)
                else
                  entry.fetch("content_base64").unpack1("m0")
                end
      File.binwrite(path, content)
    end
    [workspace, fixture]
  end

  def write_lua_package(workspace, name, content, binary: false)
    package_dir = workspace / "code" / "packages" / "lua" / name
    package_dir.mkpath
    File.write(package_dir / "BUILD", "echo building #{name}\n")
    manifest = package_dir / "coding-adventures-#{name}-0.1.0-1.rockspec"
    binary ? File.binwrite(manifest, content) : File.write(manifest, content, encoding: "UTF-8")
    BuildTool::Package.new(
      name: "lua/#{name}",
      path: package_dir,
      build_commands: ["echo building #{name}"],
      language: "lua"
    )
  end

  def graph_edges(graph)
    graph.nodes.flat_map do |from|
      graph.successors(from).map { |to| [from, to] }
    end.sort
  end

  def expected_diagnostic
    "METADATA_INVALID_UTF8: package=lua/pkg " \
      "manifest=code/packages/lua/pkg/coding-adventures-pkg-0.1.0-1.rockspec " \
      "encoding=UTF-8"
  end
end
