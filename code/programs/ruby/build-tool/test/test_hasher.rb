# frozen_string_literal: true

# test_hasher.rb -- Tests for SHA256 file hashing
# ================================================
#
# These tests verify source file collection, individual file hashing,
# package hashing, and dependency hashing.

require_relative "test_helper"

class TestHasher < Minitest::Test
  include TestHelper

  # -- collect_source_files tests ----------------------------------------------

  def test_collect_source_files_python
    # Python packages should collect .py, .toml, .cfg files and BUILD.
    dir = create_temp_dir
    pkg_dir = dir / "python" / "mypkg"
    write_file(pkg_dir / "BUILD", "echo build")
    write_file(pkg_dir / "src" / "main.py", "print('hi')")
    write_file(pkg_dir / "pyproject.toml", "[project]")
    write_file(pkg_dir / "README.md", "ignore me") # not a source file

    pkg = BuildTool::Package.new(
      name: "python/mypkg", path: pkg_dir,
      build_commands: ["echo build"], language: "python"
    )

    files = BuildTool::Hasher.collect_source_files(pkg)
    basenames = files.map { |f| f.relative_path_from(pkg_dir).to_s }

    assert_includes basenames, "BUILD"
    assert_includes basenames, "src/main.py"
    assert_includes basenames, "pyproject.toml"
    refute_includes basenames, "README.md"
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_collect_source_files_ruby
    dir = create_temp_dir
    pkg_dir = dir / "ruby" / "mypkg"
    write_file(pkg_dir / "BUILD", "echo build")
    write_file(pkg_dir / "lib" / "main.rb", "puts 'hi'")
    write_file(pkg_dir / "Gemfile", "source 'https://rubygems.org'")
    write_file(pkg_dir / "Rakefile", "task :test")
    write_file(pkg_dir / "README.md", "ignore me")

    pkg = BuildTool::Package.new(
      name: "ruby/mypkg", path: pkg_dir,
      build_commands: ["echo build"], language: "ruby"
    )

    files = BuildTool::Hasher.collect_source_files(pkg)
    basenames = files.map { |f| f.relative_path_from(pkg_dir).to_s }

    assert_includes basenames, "BUILD"
    assert_includes basenames, "lib/main.rb"
    assert_includes basenames, "Gemfile"
    assert_includes basenames, "Rakefile"
    refute_includes basenames, "README.md"
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_collect_source_files_sorted
    dir = create_temp_dir
    pkg_dir = dir / "python" / "mypkg"
    write_file(pkg_dir / "BUILD", "echo build")
    write_file(pkg_dir / "src" / "z_module.py", "")
    write_file(pkg_dir / "src" / "a_module.py", "")

    pkg = BuildTool::Package.new(
      name: "python/mypkg", path: pkg_dir,
      build_commands: [], language: "python"
    )

    files = BuildTool::Hasher.collect_source_files(pkg)
    relative = files.map { |f| f.relative_path_from(pkg_dir).to_s }
    assert_equal relative.sort, relative
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- hash_file tests ---------------------------------------------------------

  def test_hash_file_deterministic
    dir = create_temp_dir
    file = dir / "test.txt"
    write_file(file, "hello world")

    hash1 = BuildTool::Hasher.hash_file(file)
    hash2 = BuildTool::Hasher.hash_file(file)
    assert_equal hash1, hash2
    assert_equal 64, hash1.length # SHA256 hex digest is 64 chars
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_hash_file_changes_with_content
    dir = create_temp_dir
    file = dir / "test.txt"

    write_file(file, "content v1")
    hash1 = BuildTool::Hasher.hash_file(file)

    write_file(file, "content v2")
    hash2 = BuildTool::Hasher.hash_file(file)

    refute_equal hash1, hash2
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- hash_package tests ------------------------------------------------------

  def test_hash_package_deterministic
    packages = BuildTool::Discovery.discover_packages(simple_fixture)
    pkg = packages.first

    hash1 = BuildTool::Hasher.hash_package(pkg)
    hash2 = BuildTool::Hasher.hash_package(pkg)
    assert_equal hash1, hash2
  end

  def test_hash_package_changes_when_file_modified
    dir = create_temp_dir
    pkg_dir = dir / "mypkg"
    write_file(pkg_dir / "BUILD", "echo build")
    write_file(pkg_dir / "src" / "main.py", "v1")

    pkg = BuildTool::Package.new(
      name: "python/mypkg", path: pkg_dir,
      build_commands: ["echo build"], language: "python"
    )

    hash1 = BuildTool::Hasher.hash_package(pkg)
    write_file(pkg_dir / "src" / "main.py", "v2")
    hash2 = BuildTool::Hasher.hash_package(pkg)

    refute_equal hash1, hash2
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_hash_package_empty_returns_hash
    dir = create_temp_dir
    pkg = BuildTool::Package.new(
      name: "unknown/empty", path: dir,
      build_commands: [], language: "unknown"
    )

    hash = BuildTool::Hasher.hash_package(pkg)
    assert_equal 64, hash.length
  ensure
    FileUtils.rm_rf(dir)
  end

  # -- hash_deps tests ---------------------------------------------------------

  def test_hash_deps_no_deps
    graph = BuildTool::DirectedGraph.new
    graph.add_node("pkg-a")

    hash = BuildTool::Hasher.hash_deps("pkg-a", graph, {})
    assert_equal 64, hash.length
  end

  def test_hash_deps_with_deps
    # Edge: dep -> dependent (dep must build first)
    graph = BuildTool::DirectedGraph.new
    graph.add_edge("pkg-b", "pkg-a")

    hashes = { "pkg-a" => "aaaa", "pkg-b" => "bbbb" }
    hash = BuildTool::Hasher.hash_deps("pkg-a", graph, hashes)
    assert_equal 64, hash.length
  end

  def test_hash_deps_changes_when_dep_changes
    graph = BuildTool::DirectedGraph.new
    graph.add_edge("pkg-b", "pkg-a")

    hash1 = BuildTool::Hasher.hash_deps("pkg-a", graph, { "pkg-b" => "v1" })
    hash2 = BuildTool::Hasher.hash_deps("pkg-a", graph, { "pkg-b" => "v2" })
    refute_equal hash1, hash2
  end

  def test_hash_deps_nonexistent_node
    graph = BuildTool::DirectedGraph.new
    hash = BuildTool::Hasher.hash_deps("nonexistent", graph, {})
    assert_equal 64, hash.length
  end
end
