# frozen_string_literal: true

# test_hasher.rb -- Tests for SHA256 file hashing
# ================================================
#
# These tests verify source file collection, individual file hashing,
# package hashing, and dependency hashing.

require_relative "test_helper"

class TestHasher < Minitest::Test
  include TestHelper

  SOURCE_COLLECTION_FIXTURES = %w[
    source-collection-extension.json
    source-collection-declared.json
  ].freeze
  SOURCE_COLLECTION_CASES = Pathname(__dir__).expand_path
    .join("../../../../specs/fixtures/build-tool-v1/cases")
  HASHING_FIXTURE = "hashing-cache-missing.json"

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

  def test_generated_directory_registry_matches_both_neutral_fixtures
    SOURCE_COLLECTION_FIXTURES.each do |filename|
      fixture = read_source_collection_fixture(filename)
      excluded = fixture.dig("input", "options", "candidates")
        .filter_map do |candidate|
          next unless candidate.fetch("path").start_with?("excluded-")

          candidate.fetch("path").split("/").fetch(1)
        end
        .sort

      assert_equal BuildTool::Hasher::GENERATED_DIRECTORY_COMPONENTS.sort, excluded
    end
  end

  def test_extension_collection_projects_neutral_exact_pruning_fixtures
    assert_projected_source_collection_fixtures(glob: false)
  end

  def test_declared_source_collection_projects_neutral_exact_pruning_fixtures
    assert_projected_source_collection_fixtures(glob: true)
  end

  def test_collectors_consume_complete_native_ocaml_fixtures
    SOURCE_COLLECTION_FIXTURES.each do |filename|
      fixture = read_source_collection_fixture(filename)
      dir = create_temp_dir
      materialize_complete_fixture(dir, fixture)
      options = fixture.dig("input", "options")
      pkg = BuildTool::Package.new(
        name: "ocaml/demo", path: dir,
        build_commands: ["echo build"], language: "ocaml",
        declared_srcs: options.fetch("declared_srcs")
      )

      actual = BuildTool::Hasher.collect_source_files(pkg).map do |path|
        {
          "path" => portable_relative(path, dir),
          "digest" => BuildTool::Hasher.hash_file(path)
        }
      end

      assert_equal fixture.dig("expected", "result", "files"), actual, filename
    ensure
      FileUtils.rm_rf(dir) if dir
    end
  end

  def test_ocaml_declared_manifests_and_exact_build_fronts_do_not_widen
    dir = create_temp_dir
    BuildTool::Hasher::BUILD_FILENAMES.each { |name| write_file(dir / name, name) }
    write_file(dir / "BUILD_custom", "lookalike")
    write_file(dir / "demo.opam", "root")
    write_file(dir / "nested" / "dependency.opam", "nested")
    write_file(dir / "dune", "root metadata")
    write_file(dir / ".ocamlformat", "profile=default")
    write_file(dir / "src" / "main.ml", "let x = 1")
    pkg = BuildTool::Package.new(
      name: "ocaml/demo", path: dir,
      build_commands: [], language: "ocaml", declared_srcs: ["src/**/*.ml"]
    )

    actual = BuildTool::Hasher.collect_source_files(pkg).map { |path| portable_relative(path, dir) }
    expected = [".ocamlformat", *BuildTool::Hasher::BUILD_FILENAMES, "demo.opam", "dune", "src/main.ml"].sort
    assert_equal expected, actual
    refute_includes actual, "BUILD_custom"
    refute_includes actual, "nested/dependency.opam"
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_ocaml_extension_mode_includes_nested_opam_sources
    dir = create_temp_dir
    write_file(dir / "demo.opam", "root")
    write_file(dir / "nested" / "dependency.opam", "nested")
    pkg = BuildTool::Package.new(
      name: "ocaml/demo", path: dir,
      build_commands: [], language: "ocaml"
    )

    actual = BuildTool::Hasher.collect_source_files(pkg).map { |path| portable_relative(path, dir) }
    assert_equal ["demo.opam", "nested/dependency.opam"], actual
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_declared_ocaml_glob_can_explicitly_include_nested_opam_sources
    dir = create_temp_dir
    write_file(dir / "demo.opam", "root")
    write_file(dir / "nested" / "dependency.opam", "nested")
    pkg = BuildTool::Package.new(
      name: "ocaml/demo", path: dir,
      build_commands: [], language: "ocaml", declared_srcs: ["nested/**/*.opam"]
    )

    actual = BuildTool::Hasher.collect_source_files(pkg).map { |path| portable_relative(path, dir) }
    assert_equal ["demo.opam", "nested/dependency.opam"], actual
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_portable_sort_uses_utf8_bytes
    dir = create_temp_dir
    write_file(dir / "\u{e000}.ml", "bmp")
    write_file(dir / "\u{10000}.ml", "astral")
    pkg = BuildTool::Package.new(
      name: "ocaml/demo", path: dir,
      build_commands: [], language: "ocaml"
    )

    assert_equal ["\u{e000}.ml", "\u{10000}.ml"],
      BuildTool::Hasher.collect_source_files(pkg).map { |path| portable_relative(path, dir) }
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_posix_backslash_filename_is_rejected_in_both_source_modes
    skip "Windows uses backslash as its native separator" if File::ALT_SEPARATOR

    dir = create_temp_dir
    write_file(dir / "a\\b.rb", "root")
    write_file(dir / "a" / "b.rb", "nested")
    extension_pkg = BuildTool::Package.new(
      name: "ruby/demo", path: dir,
      build_commands: [], language: "ruby"
    )
    declared_pkg = BuildTool::Package.new(
      name: "ruby/demo", path: dir,
      build_commands: [], language: "ruby", declared_srcs: ["a/**/*.rb"]
    )

    assert_raises(ArgumentError) { BuildTool::Hasher.collect_source_files(extension_pkg) }
    assert_raises(ArgumentError) { BuildTool::Hasher.collect_source_files(declared_pkg) }
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_discovery_only_specs_directory_remains_source_eligible
    dir = create_temp_dir
    pkg_dir = dir / "ruby" / "mypkg"
    write_file(pkg_dir / "specs" / "contract.rb", "CONTRACT = true")
    pkg = BuildTool::Package.new(
      name: "ruby/mypkg", path: pkg_dir,
      build_commands: ["echo build"], language: "ruby"
    )

    [
      BuildTool::Hasher.collect_source_files_extension(pkg),
      BuildTool::Hasher.collect_source_files_glob(pkg, ["**/*.rb"])
    ].each do |files|
      assert_equal ["specs/contract.rb"], files.map { |path| portable_relative(path, pkg_dir) }
    end
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_collectors_do_not_follow_directory_symlinks
    dir = create_temp_dir
    outside = create_temp_dir
    pkg_dir = dir / "ruby" / "mypkg"
    write_file(pkg_dir / "BUILD", "echo build")
    write_file(pkg_dir / "source.rb", "SOURCE = true")
    write_file(outside / "external.rb", "EXTERNAL = true")

    begin
      File.symlink(outside, pkg_dir / "linked")
    rescue Errno::EACCES, Errno::EPERM, NotImplementedError
      skip "this host does not permit a directory symlink fixture"
    end

    pkg = BuildTool::Package.new(
      name: "ruby/mypkg", path: pkg_dir,
      build_commands: ["echo build"], language: "ruby"
    )

    [
      BuildTool::Hasher.collect_source_files_extension(pkg),
      BuildTool::Hasher.collect_source_files_glob(pkg, ["**/*.rb"])
    ].each do |files|
      relative = files.map { |path| portable_relative(path, pkg_dir) }
      assert_equal ["BUILD", "source.rb"], relative
    end
  ensure
    FileUtils.rm_rf(dir) if dir
    FileUtils.rm_rf(outside) if outside
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

  def test_hash_file_rejects_a_link
    dir = create_temp_dir
    outside = create_temp_dir
    target = outside / "source.rb"
    write_file(target, "SOURCE = true")
    link = dir / "source.rb"
    begin
      File.symlink(target, link)
    rescue Errno::EACCES, Errno::EPERM, NotImplementedError
      skip "this host does not permit a file symlink fixture"
    end

    assert_raises(IOError) { BuildTool::Hasher.hash_file(link) }
  ensure
    FileUtils.rm_rf(dir) if dir
    FileUtils.rm_rf(outside) if outside
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

  def test_hash_package_changes_when_identical_bytes_are_renamed
    dir = create_temp_dir
    write_file(dir / "source.rb", "same bytes")
    pkg = BuildTool::Package.new(
      name: "ruby/demo", path: dir,
      build_commands: [], language: "ruby"
    )
    original = BuildTool::Hasher.hash_package(pkg)
    FileUtils.mkdir_p(dir / "nested")
    FileUtils.mv(dir / "source.rb", dir / "nested" / "renamed.rb")

    refute_equal original, BuildTool::Hasher.hash_package(pkg)
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_hash_package_matches_language_neutral_hashing_v1_oracle
    fixture = read_source_collection_fixture(HASHING_FIXTURE)
    dir = create_temp_dir
    repository = dir / "repository"
    fixture.dig("workspace", "files").each do |entry|
      write_file(repository / entry.fetch("path"), entry.fetch("content_utf8"))
    end
    package_root = repository / "code" / "packages" / "python" / "demo"
    pkg = BuildTool::Package.new(
      name: fixture.dig("input", "options", "package"), path: package_root,
      build_commands: [], language: "python", declared_srcs: ["src/data.bin"]
    )

    assert_equal fixture.dig("expected", "result", "package_digest"),
      BuildTool::Hasher.hash_package(pkg)
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_hash_package_frames_utf8_paths_raw_bytes_and_boundaries
    dir = create_temp_dir
    raw = "\0\xff\r\n".b + ("x" * 8192).b
    write_binary_file(dir / "caf\u00e9.rb", raw)
    write_binary_file(dir / "empty.rb", "".b)
    pkg = BuildTool::Package.new(
      name: "ruby/demo", path: dir,
      build_commands: [], language: "ruby"
    )

    expected = expected_framed_hash(
      "code/packages/ruby/demo/caf\u00e9.rb" => raw,
      "code/packages/ruby/demo/empty.rb" => "".b
    )
    assert_equal expected, BuildTool::Hasher.hash_package(pkg)
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_repository_anchor_distinguishes_packages_from_programs
    dir = create_temp_dir
    package_root = dir / "code" / "packages" / "ruby" / "demo"
    program_root = dir / "code" / "programs" / "ruby" / "demo"
    write_file(package_root / "source.rb", "same")
    write_file(program_root / "source.rb", "same")
    package = BuildTool::Package.new(
      name: "ruby/demo", path: package_root,
      build_commands: [], language: "ruby"
    )
    program = BuildTool::Package.new(
      name: "ruby/programs/demo", path: program_root,
      build_commands: [], language: "ruby"
    )

    refute_equal BuildTool::Hasher.hash_package(package), BuildTool::Hasher.hash_package(program)
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_hash_package_rejects_nonportable_identity_fallbacks
    dir = create_temp_dir
    write_file(dir / "source.rb", "source")

    ["ruby/../demo", "ruby\\demo/bad"].each do |name|
      pkg = BuildTool::Package.new(
        name: name, path: dir,
        build_commands: [], language: "ruby"
      )

      assert_raises(ArgumentError) { BuildTool::Hasher.hash_package(pkg) }
    end
  ensure
    FileUtils.rm_rf(dir) if dir
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

    hashes = {"pkg-a" => "aaaa", "pkg-b" => "bbbb"}
    hash = BuildTool::Hasher.hash_deps("pkg-a", graph, hashes)
    assert_equal 64, hash.length
  end

  def test_hash_deps_changes_when_dep_changes
    graph = BuildTool::DirectedGraph.new
    graph.add_edge("pkg-b", "pkg-a")

    hash1 = BuildTool::Hasher.hash_deps("pkg-a", graph, {"pkg-b" => "v1"})
    hash2 = BuildTool::Hasher.hash_deps("pkg-a", graph, {"pkg-b" => "v2"})
    refute_equal hash1, hash2
  end

  def test_hash_deps_nonexistent_node
    graph = BuildTool::DirectedGraph.new
    hash = BuildTool::Hasher.hash_deps("nonexistent", graph, {})
    assert_equal 64, hash.length
  end

  # -- collect_source_files_glob tests -----------------------------------------
  # These tests verify the glob-based file collection used for Starlark
  # packages with declared_srcs.

  # GlobPackage is a test double that adds declared_srcs to Package.
  GlobPackage = Struct.new(:name, :path, :build_commands, :language,
    :declared_srcs)

  def test_collect_source_files_glob_matches_declared
    dir = create_temp_dir
    pkg_dir = dir / "python" / "mypkg"
    write_file(pkg_dir / "BUILD", "py_library(name='mypkg')")
    write_file(pkg_dir / "src" / "main.py", "print('hi')")
    write_file(pkg_dir / "tests" / "test_main.py", "import pytest")
    write_file(pkg_dir / "README.md", "ignore me")
    write_file(pkg_dir / "CHANGELOG.md", "also ignore")

    pkg = GlobPackage.new(
      name: "python/mypkg", path: pkg_dir,
      build_commands: [], language: "python",
      declared_srcs: ["src/**/*.py", "tests/**/*.py"]
    )

    files = BuildTool::Hasher.collect_source_files(pkg)
    basenames = files.map { |f| f.relative_path_from(pkg_dir).to_s }

    # BUILD is always included.
    assert_includes basenames, "BUILD"
    # Declared srcs are included.
    assert_includes basenames, "src/main.py"
    assert_includes basenames, "tests/test_main.py"
    # Non-declared files are excluded.
    refute_includes basenames, "README.md"
    refute_includes basenames, "CHANGELOG.md"
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_collect_source_files_glob_build_prefix_included
    dir = create_temp_dir
    pkg_dir = dir / "python" / "mypkg"
    write_file(pkg_dir / "BUILD", "py_library()")
    write_file(pkg_dir / "BUILD_mac", "# mac specific")
    write_file(pkg_dir / "src" / "main.py", "")

    pkg = GlobPackage.new(
      name: "python/mypkg", path: pkg_dir,
      build_commands: [], language: "python",
      declared_srcs: ["src/**/*.py"]
    )

    files = BuildTool::Hasher.collect_source_files(pkg)
    basenames = files.map { |f| f.relative_path_from(pkg_dir).to_s }

    assert_includes basenames, "BUILD"
    assert_includes basenames, "BUILD_mac"
    assert_includes basenames, "src/main.py"
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_collect_source_files_glob_sorted
    dir = create_temp_dir
    pkg_dir = dir / "python" / "mypkg"
    write_file(pkg_dir / "BUILD", "py_library()")
    write_file(pkg_dir / "src" / "z_module.py", "")
    write_file(pkg_dir / "src" / "a_module.py", "")

    pkg = GlobPackage.new(
      name: "python/mypkg", path: pkg_dir,
      build_commands: [], language: "python",
      declared_srcs: ["src/**/*.py"]
    )

    files = BuildTool::Hasher.collect_source_files(pkg)
    relative = files.map { |f| f.relative_path_from(pkg_dir).to_s }
    assert_equal relative.sort, relative
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_collect_source_files_falls_back_without_declared_srcs
    # A package without declared_srcs should use extension-based filtering.
    dir = create_temp_dir
    pkg_dir = dir / "python" / "mypkg"
    write_file(pkg_dir / "BUILD", "echo build")
    write_file(pkg_dir / "src" / "main.py", "print('hi')")
    write_file(pkg_dir / "README.md", "ignore me")

    pkg = BuildTool::Package.new(
      name: "python/mypkg", path: pkg_dir,
      build_commands: ["echo build"], language: "python"
    )

    files = BuildTool::Hasher.collect_source_files(pkg)
    basenames = files.map { |f| f.relative_path_from(pkg_dir).to_s }

    assert_includes basenames, "BUILD"
    assert_includes basenames, "src/main.py"
    refute_includes basenames, "README.md"
  ensure
    FileUtils.rm_rf(dir)
  end

  private

  def read_source_collection_fixture(filename)
    JSON.parse((SOURCE_COLLECTION_CASES / filename).read)
  end

  def portable_relative(path, root)
    path.relative_path_from(root).to_s.tr("\\", "/")
  end

  def project_fixture_path(path)
    path.sub(/\.mli?\z/, ".rb")
  end

  def materialize_projected_fixture(root, fixture)
    fixture.dig("input", "options", "candidates").each do |candidate|
      path = candidate.fetch("path")
      next unless candidate.fetch("kind") == "file"
      next unless path.match?(%r{\A(?:excluded-\d+|case/|near/)})

      write_file(root / project_fixture_path(path), [candidate.fetch("content_hex")].pack("H*"))
    end
  end

  def materialize_complete_fixture(root, fixture)
    inert_roots = fixture.dig("input", "options", "candidates")
      .reject { |candidate| candidate.fetch("kind") == "file" }
      .map { |candidate| candidate.fetch("path") }
    fixture.dig("input", "options", "candidates").each do |candidate|
      next unless candidate.fetch("kind") == "file"
      next if inert_roots.any? { |path| candidate.fetch("path").start_with?("#{path}/") }

      write_binary_file(root / candidate.fetch("path"), [candidate.fetch("content_hex")].pack("H*"))
    end
  end

  def write_binary_file(path, content)
    path.dirname.mkpath
    path.binwrite(content)
  end

  def expected_framed_hash(files)
    digest = Digest::SHA256.new
    files.sort_by { |path, _content| path.encode(Encoding::UTF_8).b }.each do |path, content|
      path_bytes = path.encode(Encoding::UTF_8).b
      body = content.b
      digest.update([path_bytes.bytesize].pack("Q>"))
      digest.update(path_bytes)
      digest.update([body.bytesize].pack("Q>"))
      digest.update(body)
    end
    digest.hexdigest
  end

  def projected_expected_paths(fixture)
    fixture.dig("expected", "result", "files")
      .map { |entry| project_fixture_path(entry.fetch("path")) }
      .select { |path| path.match?(%r{\A(?:case/|near/)}) }
      .sort
  end

  def assert_projected_source_collection_fixtures(glob:)
    SOURCE_COLLECTION_FIXTURES.each do |filename|
      dir = create_temp_dir
      materialize_projected_fixture(dir, read_source_collection_fixture(filename))
      pkg = BuildTool::Package.new(
        name: "ruby/test-pkg", path: dir,
        build_commands: ["echo build"], language: "ruby"
      )
      files = if glob
        BuildTool::Hasher.collect_source_files_glob(pkg, ["**/*.rb"])
      else
        BuildTool::Hasher.collect_source_files_extension(pkg)
      end

      assert_equal projected_expected_paths(read_source_collection_fixture(filename)),
        files.map { |path| portable_relative(path, dir) }
    ensure
      FileUtils.rm_rf(dir) if dir
    end
  end
end
