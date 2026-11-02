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
  REGISTRY_SOURCE_COLLECTION_FIXTURES = %w[
    source-collection-extension.json
    source-collection-declared.json
    source-collection-registry-roles.json
    source-collection-engram-wasm-exact-inputs.json
    source-collection-typescript-blog-exact-inputs.json
    source-collection-typescript-landing-page-exact-inputs.json
    source-collection-typescript-site-foreign-package.json
  ].freeze
  SOURCE_COLLECTION_CASES = Pathname(__dir__).expand_path
    .join("../../../../specs/fixtures/build-tool-v1/cases")
  HASHING_FIXTURE = "hashing-cache-missing.json"
  DEPENDENCY_HASHING_FIXTURE_IDS = %w[
    hashing-cache/dependency-change-after
    hashing-cache/dependency-order-before
    hashing-cache/failed-prior-record
  ].freeze

  class CountingPredecessorChain
    attr_reader :calls

    def initialize(length)
      @length = length
      @calls = 0
    end

    def has_node?(_name)
      true
    end

    def each_predecessor(name)
      @calls += 1
      index = (name == "ruby/app") ? 0 : Integer(name.delete_prefix("ruby/Node"), 10) + 1
      yield "ruby/Node#{index}" if index < @length
    end
  end

  class CountingPredecessorStar
    attr_reader :yields

    def initialize(width)
      @width = width
      @yields = 0
    end

    def has_node?(_name)
      true
    end

    def each_predecessor(name)
      return unless name == "ruby/app"

      @width.times do |index|
        @yields += 1
        yield "ruby/Node#{index}"
      end
    end
  end

  def test_production_registry_equals_checked_neutral_registry
    checked = SOURCE_COLLECTION_CASES.parent / "language-source-input-registry.json"
    packaged = Pathname(__dir__).expand_path / "../lib/build_tool/language_source_input_registry.json"
    assert_equal checked.binread, packaged.binread
    assert_equal JSON.parse(checked.read), BuildTool::Hasher.source_input_registry
    assert_equal "5201a045ea3e2086fd9be316f2692743ca329f1d84f1c0983a0da47e96b3f621",
      BuildTool::Hasher.source_input_registry_digest
  end

  def test_collectors_consume_all_package_local_registry_cases
    REGISTRY_SOURCE_COLLECTION_FIXTURES.each do |filename|
      fixture = read_source_collection_fixture(filename)
      options = fixture.dig("input", "options")
      assert_equal BuildTool::Hasher.source_input_registry_digest, options.fetch("registry_sha256")
      temp = create_temp_dir
      package_root = temp / options.fetch("package_root")
      materialize_complete_fixture(package_root, fixture)
      language = options.fetch("language")
      package_name = if options.fetch("package_root").start_with?("code/sites/")
        "unknown/#{package_root.basename}"
      else
        "#{language}/#{package_root.basename}"
      end
      pkg = BuildTool::Package.new(
        name: package_name, path: package_root,
        build_commands: [], language: language,
        declared_srcs: options.fetch("declared_srcs")
      )
      actual = BuildTool::Hasher.collect_source_files(pkg).map do |path|
        {"path" => portable_relative(path, package_root), "digest" => BuildTool::Hasher.hash_file(path)}
      end
      assert_equal fixture.dig("expected", "result", "files"), actual, filename
    ensure
      FileUtils.rm_rf(temp) if temp
    end
  end

  def test_unknown_language_fails_before_walking_package
    missing = create_temp_dir / "missing"
    pkg = BuildTool::Package.new(name: "unknown/missing", path: missing,
      build_commands: [], language: "unknown")
    assert_raises(ArgumentError) { BuildTool::Hasher.collect_source_files(pkg) }
  ensure
    FileUtils.rm_rf(missing.parent) if missing
  end

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
    expected = [".ocamlformat", *BuildTool::Hasher::BUILD_FILENAMES, "demo.opam", "src/main.ml"].sort
    assert_equal expected, actual
    refute_includes actual, "BUILD_custom"
    refute_includes actual, "nested/dependency.opam"
  ensure
    FileUtils.rm_rf(dir) if dir
  end

  def test_ocaml_extension_mode_keeps_variable_opam_manifests_root_only
    dir = create_temp_dir
    write_file(dir / "demo.opam", "root")
    write_file(dir / "nested" / "dependency.opam", "nested")
    pkg = BuildTool::Package.new(
      name: "ocaml/demo", path: dir,
      build_commands: [], language: "ocaml"
    )

    actual = BuildTool::Hasher.collect_source_files(pkg).map { |path| portable_relative(path, dir) }
    assert_equal ["demo.opam"], actual
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
      name: "ruby/empty", path: dir,
      build_commands: [], language: "ruby"
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
    graph.add_edge("ruby/pkg-b", "ruby/pkg-a")

    hashes = {"ruby/pkg-b" => "bb" * 32}
    hash = BuildTool::Hasher.hash_deps("ruby/pkg-a", graph, hashes)
    assert_equal expected_dependency_hash(hashes), hash
  end

  def test_hash_deps_changes_when_dep_changes
    graph = BuildTool::DirectedGraph.new
    graph.add_edge("ruby/pkg-b", "ruby/pkg-a")

    hash1 = BuildTool::Hasher.hash_deps("ruby/pkg-a", graph, {"ruby/pkg-b" => "11" * 32})
    hash2 = BuildTool::Hasher.hash_deps("ruby/pkg-a", graph, {"ruby/pkg-b" => "22" * 32})
    refute_equal hash1, hash2
  end

  def test_hash_deps_nonexistent_node
    graph = BuildTool::DirectedGraph.new
    hash = BuildTool::Hasher.hash_deps("nonexistent", graph, {})
    assert_equal 64, hash.length
  end

  def test_dependency_hashing_consumes_exact_neutral_adversarial_roster
    fixtures = dependency_hashing_fixtures
    assert_equal DEPENDENCY_HASHING_FIXTURE_IDS,
      fixtures.map { |fixture| fixture.fetch("id") }.sort
    snapshots_by_combined_digest = fixtures.to_h do |fixture|
      result = fixture.dig("expected", "result")
      [result.fetch("combined_digest"),
        [result.fetch("package_digest"), result.fetch("dependencies_digest")]]
    end

    fixtures.each do |fixture|
      options = fixture.dig("input", "options")
      expected = fixture.dig("expected", "result")
      dependency_hashes = options.fetch("dependency_digests").to_h do |entry|
        [entry.fetch("package"), entry.fetch("digest")]
      end
      graph = BuildTool::DirectedGraph.new
      graph.add_node(options.fetch("package"))
      options.fetch("dependency_digests").reverse_each do |entry|
        graph.add_edge(entry.fetch("package"), options.fetch("package"))
      end
      options.fetch("dependents").each do |dependent|
        graph.add_edge(options.fetch("package"), dependent)
      end

      dependencies_digest = BuildTool::Hasher.hash_deps(
        options.fetch("package"), graph, dependency_hashes
      )
      assert_equal expected.fetch("dependencies_digest"), dependencies_digest, fixture.fetch("id")
      assert_equal expected_dependency_hash(dependency_hashes), dependencies_digest,
        fixture.fetch("id")
      assert_equal expected.fetch("combined_digest"),
        BuildTool::Hasher.combine_hashes(expected.fetch("package_digest"), dependencies_digest),
        fixture.fetch("id")

      prior = options.fetch("prior_cache")
      assert_equal "record", prior.fetch("state"), fixture.fetch("id")
      prior_package_digest, prior_dependencies_digest =
        snapshots_by_combined_digest.fetch(prior.fetch("combined_digest"))
      cache = BuildTool::BuildCache.new
      cache.record(
        options.fetch("package"), prior_package_digest, prior_dependencies_digest,
        prior.fetch("status")
      )
      cache_miss = cache.needs_build?(
        options.fetch("package"), expected.fetch("package_digest"), dependencies_digest
      )
      assert_equal expected.fetch("cache_status"), cache_miss ? "miss" : "hit", fixture.fetch("id")
      invalidated = if cache_miss
        [options.fetch("package"), *graph.transitive_closure(options.fetch("package"))].sort
      else
        []
      end
      assert_equal expected.fetch("invalidated_packages"), invalidated, fixture.fetch("id")
      assert_equal [], fixture.dig("expected", "diagnostics"), fixture.fetch("id")
    end
  end

  def test_dependency_hash_accepts_case_preserving_package_identity
    graph = BuildTool::DirectedGraph.new
    graph.add_edge("swift/Code128", "ruby/app")
    hashes = {"swift/Code128" => "11" * 32}

    assert_equal expected_dependency_hash(hashes), BuildTool::Hasher.hash_deps("ruby/app", graph, hashes)
  end

  def test_dependency_hash_frames_identity_and_raw_digest_boundaries
    first = BuildTool::DirectedGraph.new
    first.add_edge("ruby/a", "ruby/app")
    first.add_edge("ruby/bc", "ruby/app")
    second = BuildTool::DirectedGraph.new
    second.add_edge("ruby/ab", "ruby/app")
    second.add_edge("ruby/c", "ruby/app")
    first_hashes = {"ruby/a" => "11" * 32, "ruby/bc" => "22" * 32}
    second_hashes = {"ruby/ab" => "11" * 32, "ruby/c" => "22" * 32}

    refute_equal BuildTool::Hasher.hash_deps("ruby/app", first, first_hashes),
      BuildTool::Hasher.hash_deps("ruby/app", second, second_hashes)
    assert_equal first_hashes, {"ruby/a" => "11" * 32, "ruby/bc" => "22" * 32}
  end

  def test_dependency_hash_rejects_missing_or_noncanonical_digest_without_echoing_it
    graph = BuildTool::DirectedGraph.new
    graph.add_edge("ruby/base", "ruby/app")

    error = assert_raises(ArgumentError) do
      BuildTool::Hasher.hash_deps("ruby/app", graph, {})
    end
    assert_equal "missing SHA-256 dependency digest", error.message

    ["AB" * 32, "ab" * 31, "gg" * 32, "ab" * 32 + "00"].each do |invalid|
      error = assert_raises(ArgumentError) do
        BuildTool::Hasher.hash_deps("ruby/app", graph, {"ruby/base" => invalid})
      end
      assert_equal "invalid SHA-256 dependency digest", error.message
      refute_includes error.message, invalid
    end
  end

  def test_dependency_hash_rejects_noncanonical_or_excessive_identities
    invalid = BuildTool::DirectedGraph.new
    invalid.add_edge("ruby/../escape", "ruby/app")
    error = assert_raises(ArgumentError) do
      BuildTool::Hasher.hash_deps("ruby/app", invalid, {"ruby/../escape" => "11" * 32})
    end
    assert_equal "invalid package identity", error.message

    oversized = BuildTool::DirectedGraph.new
    hashes = {}
    4_097.times do |index|
      name = "ruby/item#{index}"
      oversized.add_edge(name, "ruby/app")
      hashes[name] = "11" * 32
    end
    error = assert_raises(ArgumentError) do
      BuildTool::Hasher.hash_deps("ruby/app", oversized, hashes)
    end
    assert_equal "dependency identity limit exceeded", error.message
  end

  def test_dependency_hash_stops_reverse_traversal_at_identity_limit
    graph = CountingPredecessorChain.new(1_000_000)

    error = assert_raises(ArgumentError) do
      BuildTool::Hasher.hash_deps("ruby/app", graph, {})
    end
    assert_equal "dependency identity limit exceeded", error.message
    assert_operator graph.calls, :<=, BuildTool::Hasher::MAX_HASHING_IDENTITIES + 1
  end

  def test_dependency_hash_stops_wide_predecessor_enumeration_at_first_excess_identity
    graph = CountingPredecessorStar.new(1_000_000)

    error = assert_raises(ArgumentError) do
      BuildTool::Hasher.hash_deps("ruby/app", graph, {})
    end
    assert_equal "dependency identity limit exceeded", error.message
    assert_equal BuildTool::Hasher::MAX_HASHING_IDENTITIES + 1, graph.yields
  end

  def test_combined_hash_uses_decoded_digests_and_rejects_invalid_inputs
    package_digest = "11" * 32
    dependencies_digest = "bb" * 32
    expected = Digest::SHA256.hexdigest([package_digest, dependencies_digest].pack("H64H64"))
    assert_equal expected, BuildTool::Hasher.combine_hashes(package_digest, dependencies_digest)

    {"package" => "bad", "dependency" => "AA" * 32}.each do |role, invalid|
      error = assert_raises(ArgumentError) do
        BuildTool::Hasher.combine_hashes(
          (role == "package") ? invalid : package_digest,
          (role == "dependency") ? invalid : dependencies_digest
        )
      end
      assert_equal "invalid SHA-256 #{role} digest", error.message
    end
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

  def test_invalid_later_glob_fails_before_filesystem_enumeration
    missing = Pathname(Dir.tmpdir) / "build_tool_missing_glob_preflight_#{Process.pid}"
    FileUtils.rm_rf(missing)
    pkg = GlobPackage.new(
      name: "python/missing", path: missing,
      build_commands: [], language: "python",
      declared_srcs: ["**/*.py", "[z-a].py"]
    )

    error = assert_raises(BuildTool::GlobMatch::InvalidPatternError) do
      BuildTool::Hasher.collect_source_files_glob(pkg, pkg.declared_srcs)
    end
    assert_equal "ambiguous or descending character class in glob pattern", error.message
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

  def expected_dependency_hash(dependency_hashes)
    digest = Digest::SHA256.new
    dependency_hashes.sort_by { |name, _digest| name.b }.each do |name, raw_digest|
      name_bytes = name.b
      digest.update([name_bytes.bytesize].pack("Q>"))
      digest.update(name_bytes)
      digest_bytes = [raw_digest].pack("H64")
      digest.update([digest_bytes.bytesize].pack("Q>"))
      digest.update(digest_bytes)
    end
    digest.hexdigest
  end

  def dependency_hashing_fixtures
    SOURCE_COLLECTION_CASES.glob("hashing-cache-*.json").filter_map do |path|
      fixture = JSON.parse(path.read)
      fixture if DEPENDENCY_HASHING_FIXTURE_IDS.include?(fixture.fetch("id"))
    end
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
