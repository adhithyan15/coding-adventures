# frozen_string_literal: true

# test_new_resolver_features.rb -- Tests for v0.3.0 resolver additions
# =====================================================================
#
# Tests for TypeScript, Rust, and Swift dependency parsers, the library-
# over-program priority logic in build_known_names, and the new
# affected_set parameter on execute_builds.

require_relative "test_helper"

class TestParseTypescriptDeps < Minitest::Test
  include TestHelper

  def test_parses_dependencies_block
    dir = create_temp_dir
    pkg_dir = dir / "typescript" / "pkg"
    write_file(pkg_dir / "package.json", <<~JSON)
      {
        "name": "@coding-adventures/pkg",
        "dependencies": {
          "@coding-adventures/logic-gates": "file:../logic-gates"
        }
      }
    JSON

    pkg = BuildTool::Package.new(
      name: "typescript/pkg", path: pkg_dir,
      build_commands: [], language: "typescript"
    )

    known = { "@coding-adventures/logic-gates" => "typescript/logic-gates" }
    deps = BuildTool::Resolver.parse_typescript_deps(pkg, known)
    assert_equal ["typescript/logic-gates"], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_parses_dev_dependencies_block
    dir = create_temp_dir
    pkg_dir = dir / "typescript" / "pkg"
    write_file(pkg_dir / "package.json", <<~JSON)
      {
        "devDependencies": {
          "@coding-adventures/arithmetic": "file:../arithmetic"
        }
      }
    JSON

    pkg = BuildTool::Package.new(
      name: "typescript/pkg", path: pkg_dir,
      build_commands: [], language: "typescript"
    )

    known = { "@coding-adventures/arithmetic" => "typescript/arithmetic" }
    deps = BuildTool::Resolver.parse_typescript_deps(pkg, known)
    assert_equal ["typescript/arithmetic"], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_no_package_json
    dir = create_temp_dir
    pkg = BuildTool::Package.new(
      name: "typescript/pkg", path: dir,
      build_commands: [], language: "typescript"
    )
    deps = BuildTool::Resolver.parse_typescript_deps(pkg, {})
    assert_equal [], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_skips_external_deps
    dir = create_temp_dir
    pkg_dir = dir / "typescript" / "pkg"
    write_file(pkg_dir / "package.json", <<~JSON)
      {
        "dependencies": {
          "react": "^18.0.0",
          "typescript": "^5.0.0"
        }
      }
    JSON

    pkg = BuildTool::Package.new(
      name: "typescript/pkg", path: pkg_dir,
      build_commands: [], language: "typescript"
    )
    deps = BuildTool::Resolver.parse_typescript_deps(pkg, {})
    assert_equal [], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_multiple_internal_deps
    dir = create_temp_dir
    pkg_dir = dir / "typescript" / "pkg"
    write_file(pkg_dir / "package.json", <<~JSON)
      {
        "dependencies": {
          "@coding-adventures/logic-gates": "file:../logic-gates",
          "@coding-adventures/arithmetic": "file:../arithmetic"
        }
      }
    JSON

    pkg = BuildTool::Package.new(
      name: "typescript/pkg", path: pkg_dir,
      build_commands: [], language: "typescript"
    )
    known = {
      "@coding-adventures/logic-gates" => "typescript/logic-gates",
      "@coding-adventures/arithmetic" => "typescript/arithmetic"
    }
    deps = BuildTool::Resolver.parse_typescript_deps(pkg, known)
    assert_includes deps, "typescript/logic-gates"
    assert_includes deps, "typescript/arithmetic"
  ensure
    FileUtils.rm_rf(dir)
  end
end

class TestParseRustDeps < Minitest::Test
  include TestHelper

  def test_parses_path_deps
    dir = create_temp_dir
    pkg_dir = dir / "rust" / "my-crate"
    write_file(pkg_dir / "Cargo.toml", <<~TOML)
      [package]
      name = "my-crate"

      [dependencies]
      logic-gates = { path = "../logic-gates" }
    TOML

    pkg = BuildTool::Package.new(
      name: "rust/my-crate", path: pkg_dir,
      build_commands: [], language: "rust"
    )
    known = { "logic-gates" => "rust/logic-gates" }
    deps = BuildTool::Resolver.parse_rust_deps(pkg, known)
    assert_equal ["rust/logic-gates"], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_no_cargo_toml
    dir = create_temp_dir
    pkg = BuildTool::Package.new(
      name: "rust/pkg", path: dir,
      build_commands: [], language: "rust"
    )
    deps = BuildTool::Resolver.parse_rust_deps(pkg, {})
    assert_equal [], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_skips_registry_deps
    dir = create_temp_dir
    pkg_dir = dir / "rust" / "my-crate"
    write_file(pkg_dir / "Cargo.toml", <<~TOML)
      [package]
      name = "my-crate"

      [dependencies]
      serde = "1.0"
    TOML

    pkg = BuildTool::Package.new(
      name: "rust/my-crate", path: pkg_dir,
      build_commands: [], language: "rust"
    )
    deps = BuildTool::Resolver.parse_rust_deps(pkg, {})
    assert_equal [], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_multiple_path_deps
    dir = create_temp_dir
    pkg_dir = dir / "rust" / "my-crate"
    write_file(pkg_dir / "Cargo.toml", <<~TOML)
      [package]
      name = "my-crate"

      [dependencies]
      logic-gates = { path = "../logic-gates" }
      arithmetic = { path = "../arithmetic" }
    TOML

    pkg = BuildTool::Package.new(
      name: "rust/my-crate", path: pkg_dir,
      build_commands: [], language: "rust"
    )
    known = {
      "logic-gates" => "rust/logic-gates",
      "arithmetic" => "rust/arithmetic"
    }
    deps = BuildTool::Resolver.parse_rust_deps(pkg, known)
    assert_includes deps, "rust/logic-gates"
    assert_includes deps, "rust/arithmetic"
  ensure
    FileUtils.rm_rf(dir)
  end
end

class TestParseSwiftDeps < Minitest::Test
  include TestHelper

  def test_parses_package_swift
    dir = create_temp_dir
    pkg_dir = dir / "swift" / "MyPackage"
    write_file(pkg_dir / "Package.swift", <<~SWIFT)
      let package = Package(
          name: "MyPackage",
          dependencies: [
              .package(path: "../logic-gates"),
          ]
      )
    SWIFT

    pkg = BuildTool::Package.new(
      name: "swift/MyPackage", path: pkg_dir,
      build_commands: [], language: "swift"
    )
    known = { "logic-gates" => "swift/logic-gates" }
    deps = BuildTool::Resolver.parse_swift_deps(pkg, known)
    assert_equal ["swift/logic-gates"], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_no_package_swift
    dir = create_temp_dir
    pkg = BuildTool::Package.new(
      name: "swift/pkg", path: dir,
      build_commands: [], language: "swift"
    )
    deps = BuildTool::Resolver.parse_swift_deps(pkg, {})
    assert_equal [], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_skips_url_deps
    dir = create_temp_dir
    pkg_dir = dir / "swift" / "MyPackage"
    write_file(pkg_dir / "Package.swift", <<~SWIFT)
      let package = Package(
          dependencies: [
              .package(url: "https://github.com/apple/swift-argument-parser", from: "1.0.0"),
          ]
      )
    SWIFT

    pkg = BuildTool::Package.new(
      name: "swift/pkg", path: pkg_dir,
      build_commands: [], language: "swift"
    )
    deps = BuildTool::Resolver.parse_swift_deps(pkg, {})
    assert_equal [], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_skips_path_traversal
    dir = create_temp_dir
    pkg_dir = dir / "swift" / "MyPackage"
    write_file(pkg_dir / "Package.swift", '        .package(path: "../../evil/path"),')

    pkg = BuildTool::Package.new(
      name: "swift/pkg", path: pkg_dir,
      build_commands: [], language: "swift"
    )
    known = { "evil/path" => "swift/evil" }
    deps = BuildTool::Resolver.parse_swift_deps(pkg, known)
    assert_equal [], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_multiple_deps
    dir = create_temp_dir
    pkg_dir = dir / "swift" / "MyPackage"
    write_file(pkg_dir / "Package.swift", <<~SWIFT)
      let package = Package(
          dependencies: [
              .package(path: "../logic-gates"),
              .package(path: "../arithmetic"),
          ]
      )
    SWIFT

    pkg = BuildTool::Package.new(
      name: "swift/pkg", path: pkg_dir,
      build_commands: [], language: "swift"
    )
    known = {
      "logic-gates" => "swift/logic-gates",
      "arithmetic" => "swift/arithmetic"
    }
    deps = BuildTool::Resolver.parse_swift_deps(pkg, known)
    assert_includes deps, "swift/logic-gates"
    assert_includes deps, "swift/arithmetic"
  ensure
    FileUtils.rm_rf(dir)
  end
end

class TestBuildKnownNamesNewLanguages < Minitest::Test
  include TestHelper

  def test_typescript_scoped_name
    pkg = BuildTool::Package.new(
      name: "typescript/logic-gates",
      path: Pathname("/fake/packages/typescript/logic-gates"),
      build_commands: [], language: "typescript"
    )
    known = BuildTool::Resolver.build_known_names([pkg])
    assert_equal "typescript/logic-gates", known["@coding-adventures/logic-gates"]
  end

  def test_rust_crate_name
    pkg = BuildTool::Package.new(
      name: "rust/logic-gates",
      path: Pathname("/fake/packages/rust/logic-gates"),
      build_commands: [], language: "rust"
    )
    known = BuildTool::Resolver.build_known_names([pkg])
    assert_equal "rust/logic-gates", known["logic-gates"]
  end

  def test_swift_dir_name
    pkg = BuildTool::Package.new(
      name: "swift/logic-gates",
      path: Pathname("/fake/packages/swift/logic-gates"),
      build_commands: [], language: "swift"
    )
    known = BuildTool::Resolver.build_known_names([pkg])
    assert_equal "swift/logic-gates", known["logic-gates"]
  end

  def test_elixir_app_name
    pkg = BuildTool::Package.new(
      name: "elixir/logic_gates",
      path: Pathname("/fake/packages/elixir/logic_gates"),
      build_commands: [], language: "elixir"
    )
    known = BuildTool::Resolver.build_known_names([pkg])
    assert_equal "elixir/logic_gates", known["coding_adventures_logic_gates"]
  end

  def test_library_wins_over_program
    # Library package (under packages/) should overwrite a program (under programs/)
    prog_pkg = BuildTool::Package.new(
      name: "python/my-tool",
      path: Pathname("/fake/programs/python/my-lib"),
      build_commands: [], language: "python"
    )
    lib_pkg = BuildTool::Package.new(
      name: "python/my-lib",
      path: Pathname("/fake/packages/python/my-lib"),
      build_commands: [], language: "python"
    )
    known = BuildTool::Resolver.build_known_names([prog_pkg, lib_pkg])
    assert_equal "python/my-lib", known["coding-adventures-my-lib"]
  end

  def test_first_program_stays_without_library
    prog1 = BuildTool::Package.new(
      name: "python/tool-a",
      path: Pathname("/fake/programs/python/my-tool"),
      build_commands: [], language: "python"
    )
    prog2 = BuildTool::Package.new(
      name: "python/tool-b",
      path: Pathname("/fake/programs/python/my-tool"),
      build_commands: [], language: "python"
    )
    known = BuildTool::Resolver.build_known_names([prog1, prog2])
    assert_equal "python/tool-a", known["coding-adventures-my-tool"]
  end
end

class TestExecutorAffectedSet < Minitest::Test
  include TestHelper

  def test_skips_packages_outside_affected_set
    # Create a package that is NOT in the affected set — it should be skipped.
    dir = create_temp_dir
    pkg_dir = dir / "python" / "pkg-a"
    write_file(pkg_dir / "BUILD", 'echo "should not run"')

    pkg = BuildTool::Package.new(
      name: "python/pkg-a", path: pkg_dir,
      build_commands: ['echo "should not run"'], language: "python"
    )

    graph = BuildTool::DirectedGraph.new
    graph.add_node("python/pkg-a")
    cache = BuildTool::BuildCache.new

    # affected_set does NOT include python/pkg-a
    results = BuildTool::Executor.execute_builds(
      packages: [pkg], graph: graph, cache: cache,
      package_hashes: { "python/pkg-a" => "abc" },
      deps_hashes: { "python/pkg-a" => "def" },
      affected_set: {}
    )

    assert_equal "skipped", results["python/pkg-a"].status
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_builds_packages_inside_affected_set
    dir = create_temp_dir
    pkg_dir = dir / "python" / "pkg-a"
    write_file(pkg_dir / "BUILD", 'echo "running"')

    pkg = BuildTool::Package.new(
      name: "python/pkg-a", path: pkg_dir,
      build_commands: ['echo "running"'], language: "python"
    )

    graph = BuildTool::DirectedGraph.new
    graph.add_node("python/pkg-a")
    cache = BuildTool::BuildCache.new

    results = BuildTool::Executor.execute_builds(
      packages: [pkg], graph: graph, cache: cache,
      package_hashes: { "python/pkg-a" => "abc" },
      deps_hashes: { "python/pkg-a" => "def" },
      affected_set: { "python/pkg-a" => true }
    )

    assert_equal "built", results["python/pkg-a"].status
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_nil_affected_set_builds_all
    # When affected_set is nil (no git-diff mode), everything should run normally.
    dir = create_temp_dir
    pkg_dir = dir / "python" / "pkg-a"
    write_file(pkg_dir / "BUILD", 'echo "running"')

    pkg = BuildTool::Package.new(
      name: "python/pkg-a", path: pkg_dir,
      build_commands: ['echo "running"'], language: "python"
    )

    graph = BuildTool::DirectedGraph.new
    graph.add_node("python/pkg-a")
    cache = BuildTool::BuildCache.new

    results = BuildTool::Executor.execute_builds(
      packages: [pkg], graph: graph, cache: cache,
      package_hashes: { "python/pkg-a" => "abc" },
      deps_hashes: { "python/pkg-a" => "def" }
      # affected_set: nil by default
    )

    assert_equal "built", results["python/pkg-a"].status
  ensure
    FileUtils.rm_rf(dir)
  end
end
