# frozen_string_literal: true

# test_resolver.rb -- Tests for dependency resolution and DirectedGraph
# =====================================================================
#
# These tests cover:
# - The DirectedGraph data structure (nodes, edges, traversals, Kahn's)
# - Python dependency parsing from pyproject.toml
# - Ruby dependency parsing from .gemspec
# - Go dependency parsing from go.mod
# - The full resolve_dependencies pipeline

require_relative "test_helper"

class TestDirectedGraph < Minitest::Test
  # -- Basic graph operations --------------------------------------------------

  def test_add_node
    graph = BuildTool::DirectedGraph.new
    graph.add_node("a")
    assert graph.has_node?("a")
    refute graph.has_node?("b")
  end

  def test_add_edge
    graph = BuildTool::DirectedGraph.new
    graph.add_edge("a", "b")
    assert graph.has_node?("a")
    assert graph.has_node?("b")
    assert_includes graph.successors("a"), "b"
    assert_includes graph.predecessors("b"), "a"
  end

  def test_nodes
    graph = BuildTool::DirectedGraph.new
    graph.add_node("a")
    graph.add_node("b")
    assert_equal %w[a b].to_set, graph.nodes.to_set
  end

  def test_successors_empty
    graph = BuildTool::DirectedGraph.new
    graph.add_node("a")
    assert_equal [], graph.successors("a")
  end

  def test_predecessors_empty
    graph = BuildTool::DirectedGraph.new
    graph.add_node("a")
    assert_equal [], graph.predecessors("a")
  end

  # -- Transitive closure / dependents -----------------------------------------

  def test_transitive_closure_linear
    # a -> b -> c
    graph = BuildTool::DirectedGraph.new
    graph.add_edge("a", "b")
    graph.add_edge("b", "c")

    assert_equal Set["b", "c"], graph.transitive_closure("a")
    assert_equal Set["c"], graph.transitive_closure("b")
    assert_equal Set[], graph.transitive_closure("c")
  end

  def test_transitive_closure_nonexistent_node
    graph = BuildTool::DirectedGraph.new
    assert_equal Set[], graph.transitive_closure("nonexistent")
  end

  def test_transitive_dependents_linear
    # a -> b -> c  (edges: a points to b, b points to c)
    # transitive_dependents("c") should return {b, a} (everything that depends on c)
    graph = BuildTool::DirectedGraph.new
    graph.add_edge("a", "b")
    graph.add_edge("b", "c")

    assert_equal Set["a", "b"], graph.transitive_dependents("c")
    assert_equal Set["a"], graph.transitive_dependents("b")
    assert_equal Set[], graph.transitive_dependents("a")
  end

  def test_transitive_dependents_nonexistent_node
    graph = BuildTool::DirectedGraph.new
    assert_equal Set[], graph.transitive_dependents("nonexistent")
  end

  # -- independent_groups (Kahn's algorithm) -----------------------------------

  def test_independent_groups_linear
    # d -> b -> a  (d must be built first)
    graph = BuildTool::DirectedGraph.new
    graph.add_edge("d", "b")
    graph.add_edge("b", "a")

    groups = graph.independent_groups
    assert_equal [["d"], ["b"], ["a"]], groups
  end

  def test_independent_groups_diamond
    # Diamond: d -> b, d -> c, b -> a, c -> a
    graph = BuildTool::DirectedGraph.new
    graph.add_edge("d", "b")
    graph.add_edge("d", "c")
    graph.add_edge("b", "a")
    graph.add_edge("c", "a")

    groups = graph.independent_groups
    assert_equal 3, groups.size
    assert_equal ["d"], groups[0]
    assert_equal %w[b c], groups[1].sort
    assert_equal ["a"], groups[2]
  end

  def test_independent_groups_no_edges
    graph = BuildTool::DirectedGraph.new
    graph.add_node("a")
    graph.add_node("b")
    graph.add_node("c")

    groups = graph.independent_groups
    assert_equal 1, groups.size
    assert_equal %w[a b c], groups[0].sort
  end

  def test_independent_groups_cycle_raises
    graph = BuildTool::DirectedGraph.new
    graph.add_edge("a", "b")
    graph.add_edge("b", "a")

    assert_raises(RuntimeError) { graph.independent_groups }
  end
end

class TestResolver < Minitest::Test
  include TestHelper

  # -- Python dependency parsing -----------------------------------------------

  def test_parse_python_deps_with_known_deps
    dir = create_temp_dir
    pkg_dir = dir / "python" / "pkg-a"
    write_file(pkg_dir / "pyproject.toml", <<~TOML)
      [project]
      name = "coding-adventures-pkg-a"
      version = "0.1.0"
      dependencies = ["coding-adventures-pkg-b>=0.1", "coding-adventures-pkg-c"]
    TOML
    write_file(pkg_dir / "BUILD", "echo build")

    pkg = BuildTool::Package.new(
      name: "python/pkg-a", path: pkg_dir,
      build_commands: ["echo build"], language: "python"
    )

    known = {
      "coding-adventures-pkg-b" => "python/pkg-b",
      "coding-adventures-pkg-c" => "python/pkg-c"
    }

    deps = BuildTool::Resolver.parse_python_deps(pkg, known)
    assert_equal %w[python/pkg-b python/pkg-c], deps.sort
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_parse_python_deps_skips_external
    dir = create_temp_dir
    pkg_dir = dir / "python" / "pkg-a"
    write_file(pkg_dir / "pyproject.toml", <<~TOML)
      [project]
      name = "coding-adventures-pkg-a"
      dependencies = ["requests>=2.0", "coding-adventures-pkg-b"]
    TOML

    pkg = BuildTool::Package.new(
      name: "python/pkg-a", path: pkg_dir,
      build_commands: [], language: "python"
    )

    known = { "coding-adventures-pkg-b" => "python/pkg-b" }
    deps = BuildTool::Resolver.parse_python_deps(pkg, known)
    assert_equal ["python/pkg-b"], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_parse_python_deps_missing_pyproject
    dir = create_temp_dir
    pkg = BuildTool::Package.new(
      name: "python/pkg-a", path: dir,
      build_commands: [], language: "python"
    )
    deps = BuildTool::Resolver.parse_python_deps(pkg, {})
    assert_equal [], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- Ruby dependency parsing -------------------------------------------------

  def test_parse_ruby_deps
    dir = create_temp_dir
    pkg_dir = dir / "ruby" / "pkg_a"
    write_file(pkg_dir / "pkg_a.gemspec", <<~GEMSPEC)
      Gem::Specification.new do |spec|
        spec.name = "coding_adventures_pkg_a"
        spec.add_dependency "coding_adventures_pkg_b"
        spec.add_dependency "some_external_gem"
      end
    GEMSPEC

    pkg = BuildTool::Package.new(
      name: "ruby/pkg_a", path: pkg_dir,
      build_commands: [], language: "ruby"
    )

    known = { "coding_adventures_pkg_b" => "ruby/pkg_b" }
    deps = BuildTool::Resolver.parse_ruby_deps(pkg, known)
    assert_equal ["ruby/pkg_b"], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_parse_ruby_deps_no_gemspec
    dir = create_temp_dir
    pkg = BuildTool::Package.new(
      name: "ruby/pkg_a", path: dir,
      build_commands: [], language: "ruby"
    )
    deps = BuildTool::Resolver.parse_ruby_deps(pkg, {})
    assert_equal [], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- Go dependency parsing ---------------------------------------------------

  def test_parse_go_deps_single_require
    dir = create_temp_dir
    pkg_dir = dir / "go" / "mymod"
    write_file(pkg_dir / "go.mod", <<~GOMOD)
      module github.com/user/mymod

      go 1.21

      require github.com/user/dep v1.0.0
    GOMOD

    pkg = BuildTool::Package.new(
      name: "go/mymod", path: pkg_dir,
      build_commands: [], language: "go"
    )

    known = { "github.com/user/dep" => "go/dep" }
    deps = BuildTool::Resolver.parse_go_deps(pkg, known)
    assert_equal ["go/dep"], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_parse_go_deps_require_block
    dir = create_temp_dir
    pkg_dir = dir / "go" / "mymod"
    write_file(pkg_dir / "go.mod", <<~GOMOD)
      module github.com/user/mymod

      go 1.21

      require (
      \tgithub.com/user/dep-a v1.0.0
      \tgithub.com/user/dep-b v2.0.0
      )
    GOMOD

    pkg = BuildTool::Package.new(
      name: "go/mymod", path: pkg_dir,
      build_commands: [], language: "go"
    )

    known = {
      "github.com/user/dep-a" => "go/dep-a",
      "github.com/user/dep-b" => "go/dep-b"
    }
    deps = BuildTool::Resolver.parse_go_deps(pkg, known)
    assert_equal %w[go/dep-a go/dep-b], deps.sort
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_parse_go_deps_missing_gomod
    dir = create_temp_dir
    pkg = BuildTool::Package.new(
      name: "go/pkg", path: dir,
      build_commands: [], language: "go"
    )
    deps = BuildTool::Resolver.parse_go_deps(pkg, {})
    assert_equal [], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- build_known_names -------------------------------------------------------

  def test_build_known_names_python
    dir = Pathname("/repo/code/packages/python/logic-gates")
    pkg = BuildTool::Package.new(
      name: "python/logic-gates", path: dir,
      build_commands: [], language: "python"
    )
    known = BuildTool::Resolver.build_known_names([pkg])
    assert_equal({ "coding-adventures-logic-gates" => "python/logic-gates" }, known)
  end

  def test_build_known_names_ruby
    dir = Pathname("/repo/code/packages/ruby/logic_gates")
    pkg = BuildTool::Package.new(
      name: "ruby/logic_gates", path: dir,
      build_commands: [], language: "ruby"
    )
    known = BuildTool::Resolver.build_known_names([pkg])
    assert_equal({ "coding_adventures_logic_gates" => "ruby/logic_gates" }, known)
  end

  # -- resolve_dependencies integration ----------------------------------------

  def test_resolve_dependencies_diamond
    packages = BuildTool::Discovery.discover_packages(diamond_fixture)
    graph = BuildTool::Resolver.resolve_dependencies(packages)

    # pkg-d should be built first (no deps), then b and c, then a.
    groups = graph.independent_groups
    assert_equal 3, groups.size
    assert_equal ["python/pkg-d"], groups[0]
    assert_equal %w[python/pkg-b python/pkg-c], groups[1].sort
    assert_equal ["python/pkg-a"], groups[2]
  end

  def test_resolve_dependencies_no_packages
    graph = BuildTool::Resolver.resolve_dependencies([])
    assert_equal [], graph.nodes
  end
end
