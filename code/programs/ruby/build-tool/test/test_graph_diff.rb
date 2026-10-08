# frozen_string_literal: true

# The checked-in corpus is data, not output captured from this Ruby engine.
# Every case crosses the same production, process-free entry point that callers
# can use with already-materialized values. Pinning IDs makes a new neutral
# case a deliberate implementation task, not a silently ignored file.

require_relative "test_helper"
require_relative "../lib/build_tool/graph_diff"

class TestGraphDiff < Minitest::Test
  CORPUS = Pathname(__dir__).join("../../../..", "specs/fixtures/build-tool-v1").expand_path
  CASES = CORPUS.join("cases")
  GRAPH_IDS = %w[
    graph/canonical-edge-order graph/chain graph/cycle graph/diamond
    graph/empty graph/isolated graph/multiple-components
    graph/partial-cycle-no-output
  ].freeze
  DIFF_IDS = %w[
    diff-selection/exact-build-fronts diff-selection/forced-package
    diff-selection/known-unmatched-near-build diff-selection/match-work-at-limit
    diff-selection/match-work-over-limit diff-selection/package-prefix
    diff-selection/repository-boundary-reverse-index
    diff-selection/shared-input-multiconsumer
    diff-selection/strict-glob-character-classes
    diff-selection/transitive-package-change diff-selection/unknown-path-all
    diff-selection/unknown-path-error
  ].freeze

  def cases(prefix)
    paths = Dir.glob(CASES.join("#{prefix}-*.json").to_s).sort
    paths.map { |path| JSON.parse(File.read(path, encoding: "UTF-8")) }
  end

  def test_exact_graph_case_roster_and_results
    documents = cases("graph")
    assert_equal GRAPH_IDS.sort, documents.map { |document| document.fetch("id") }.sort
    documents.each do |document|
      options = document.fetch("input").fetch("options")
      input = BuildTool::GraphDiff::GraphInput.new(
        options.fetch("packages"), options.fetch("edges")
      )
      assert_case(document, BuildTool::GraphDiff.evaluate_graph(input))
    end
  end

  def test_exact_diff_case_roster_and_results
    documents = cases("diff-selection")
    assert_equal DIFF_IDS.sort, documents.map { |document| document.fetch("id") }.sort
    documents.each do |document|
      source = document.fetch("input")
      options = source.fetch("options")
      boundary = if options.key?("boundary_sha256")
        JSON.parse(CORPUS.join("repository-source-input-boundary.json").read)
      end
      input = BuildTool::GraphDiff::DiffInput.new(
        options.fetch("packages"), options.fetch("edges"),
        options.fetch("forced_packages"), options.fetch("unknown_path_policy"),
        source.fetch("changed_paths"), options.fetch("boundary_sha256", ""), boundary
      )
      assert_case(document, BuildTool::GraphDiff.evaluate_diff_selection(input))
    end
  end

  def test_pure_inputs_are_not_mutated
    options = JSON.parse(CASES.join("diff-selection-transitive.json").read).fetch("input")
    before = Marshal.dump(options)
    values = options.fetch("options")
    input = BuildTool::GraphDiff::DiffInput.new(
      values.fetch("packages"), values.fetch("edges"), values.fetch("forced_packages"),
      values.fetch("unknown_path_policy"), options.fetch("changed_paths"), "", nil
    )
    BuildTool::GraphDiff.evaluate_diff_selection(input)
    assert_equal before, Marshal.dump(options)
  end

  def test_graph_rejects_invalid_topology_without_partial_result
    graph = BuildTool::GraphDiff
    names = %w[code/a code/b]
    invalid_utf8 = "\xFF".b.force_encoding(Encoding::UTF_8)
    assert_contract("GRAPH_PACKAGE_INVALID") do
      graph.evaluate_graph(graph::GraphInput.new([invalid_utf8], []))
    end
    assert_contract("GRAPH_PACKAGE_DUPLICATE") do
      graph.evaluate_graph(graph::GraphInput.new([names.first, names.first], []))
    end
    assert_contract("GRAPH_EDGE_UNKNOWN") do
      graph.evaluate_graph(graph::GraphInput.new(names, [["code/missing", names.last]]))
    end
    assert_contract("GRAPH_EDGE_SELF") do
      graph.evaluate_graph(graph::GraphInput.new(names, [[names.first, names.first]]))
    end
    assert_contract("GRAPH_EDGE_DUPLICATE") do
      graph.evaluate_graph(graph::GraphInput.new(names, [[names.first, names.last]] * 2))
    end
    result = graph.evaluate_graph(graph::GraphInput.new(names, [[names.first, names.last], [names.last, names.first]]))
    assert_equal({}, result.result)
    assert_equal "GRAPH_CYCLE", result.error_code
  end

  def test_diff_rejects_invalid_materialized_values
    invalid_utf8 = "\xFF".b.force_encoding(Encoding::UTF_8)
    assert_contract("DIFF_PACKAGE_INVALID") { diff_input(packages: [nil]) }
    assert_contract("DIFF_PATH_INVALID") { diff_input(paths: [invalid_utf8]) }
    assert_contract("DIFF_PATH_INVALID") { diff_input(paths: ["code/CON.txt"]) }
    assert_contract("DIFF_PATH_INVALID") { diff_input(paths: ["code/a/"]) }
    assert_contract("DIFF_PATH_INVALID") { diff_input(root: "code/A/../b") }
    assert_contract("DIFF_PATH_INVALID") { diff_input(root: "code/a/") }
    assert_contract("DIFF_SOURCE_MODE_INVALID") { diff_input(mode: "anything") }
    assert_contract("DIFF_GLOB_INVALID") { diff_input(globs: ["**/[z-a]"]) }
    assert_contract("DIFF_GLOB_INVALID") { diff_input(globs: ["src/*.rb/"]) }
    assert_contract("DIFF_POLICY_INVALID") { diff_input(policy: "ignore") }
    assert_contract("DIFF_FORCED_PACKAGE_UNKNOWN") { diff_input(forced: ["code/other"]) }
    assert_contract("DIFF_BOUNDARY_DIGEST_MISMATCH") { diff_input(digest: "wrong") }
  end

  def test_match_limit_returns_no_partial_selection
    document = cases("diff-selection").find { |entry| entry.fetch("id") == "diff-selection/match-work-over-limit" }
    options = document.fetch("input").fetch("options")
    input = BuildTool::GraphDiff::DiffInput.new(
      options.fetch("packages"), options.fetch("edges"), options.fetch("forced_packages"),
      options.fetch("unknown_path_policy"), document.fetch("input").fetch("changed_paths"), "", nil
    )
    result = BuildTool::GraphDiff.evaluate_diff_selection(input)
    assert_equal({}, result.result)
    assert_equal "DIFF_MATCH_LIMIT_EXCEEDED", result.error_code
  end

  def test_boundary_digest_rejects_changed_content
    document = cases("diff-selection").find do |entry|
      entry.fetch("id") == "diff-selection/repository-boundary-reverse-index"
    end
    values = document.fetch("input").fetch("options")
    boundary = JSON.parse(CORPUS.join("repository-source-input-boundary.json").read)
    boundary.fetch("boundaries").first["reason"] = "tampered"
    input = BuildTool::GraphDiff::DiffInput.new(
      values.fetch("packages"), values.fetch("edges"), values.fetch("forced_packages"),
      values.fetch("unknown_path_policy"), document.fetch("input").fetch("changed_paths"),
      values.fetch("boundary_sha256"), boundary
    )
    assert_contract("DIFF_BOUNDARY_DIGEST_MISMATCH") do
      BuildTool::GraphDiff.evaluate_diff_selection(input)
    end
  end

  private

  def diff_input(packages: nil, root: "code/a", mode: "strict_globs", globs: [],
    paths: [], policy: "error", forced: [], digest: "")
    package = {"name" => "code/a", "rel_path" => root, "source_mode" => mode, "source_globs" => globs}
    input = BuildTool::GraphDiff::DiffInput.new(
      packages || [package], [], forced, policy, paths, digest, nil
    )
    BuildTool::GraphDiff.evaluate_diff_selection(input)
  end

  def assert_contract(code)
    error = assert_raises(BuildTool::GraphDiff::ContractError) { yield }
    assert_equal code, error.code
  end

  def assert_case(document, actual)
    expected = document.fetch("expected")
    if expected.fetch("outcome") == "ok"
      assert_nil actual.error_code, document.fetch("id")
      assert_equal expected.fetch("result"), actual.result, document.fetch("id")
    else
      assert_equal({}, actual.result, document.fetch("id"))
      assert_equal expected.fetch("diagnostics").fetch(0).fetch("code"), actual.error_code,
        document.fetch("id")
    end
  end
end
