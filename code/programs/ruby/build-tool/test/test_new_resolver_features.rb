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

# TestCargoLinkDependencyNames pins each line rule of the Cargo contract in
# code/specs/build-tool-conformance.md, one manifest shape per case. The table
# is the Go reference tool's TestCargoLinkDependencyNames, case for case, so a
# divergence between fronts shows up as the same named case failing.
class TestCargoLinkDependencyNames < Minitest::Test
  include TestHelper

  CASES = [
    [
      "top-level inline",
      "[dependencies]\n" \
      "a = { path = \"../a\" }\n",
      ["a"]
    ],
    [
      "build-dependencies",
      "[build-dependencies]\n" \
      "a = { path = \"../a\" }\n",
      ["a"]
    ],
    [
      "legacy build_dependencies",
      "[build_dependencies]\n" \
      "a = { path = \"../a\" }\n",
      ["a"]
    ],
    [
      "quoted cfg target",
      "[target.'cfg(any(target_os = \"linux\", windows))'.dependencies]\n" \
      "a = { path = \"../a\" }\n",
      ["a"]
    ],
    [
      "bare triple target build",
      "[target.x86_64-pc-windows-msvc.build-dependencies]\n" \
      "a = { path = \"../a\" }\n",
      ["a"]
    ],
    [
      "legacy target build",
      "[target.'cfg(unix)'.build_dependencies]\n" \
      "a = { path = \"../a\" }\n",
      ["a"]
    ],
    [
      "spaced header segments",
      "[ target . 'cfg(unix)' . dependencies ]\n" \
      "a = { path = \"../a\" }\n",
      ["a"]
    ],
    [
      "quoted table segment",
      "[\"dependencies\"]\n" \
      "a = { path = \"../a\" }\n",
      ["a"]
    ],
    [
      "sub-table",
      "[dependencies.a]\n" \
      "version = \"1\"\n" \
      "path = \"../a\"\n",
      ["a"]
    ],
    [
      "sub-table rename before path",
      "[build-dependencies.x]\n" \
      "package = \"A\"\n" \
      "path = '../a'\n",
      ["a"]
    ],
    [
      "target sub-table, dotted quoted triple",
      "[target.\"thumbv8m.main-none-eabihf\".dependencies.a]\n" \
      "path = \"../a\"\n",
      ["a"]
    ],
    [
      "quoted sub-table key",
      "[dependencies.\"a\"]\n" \
      "path = \"../a\"\n",
      ["a"]
    ],
    [
      "sub-table without path",
      "[dependencies.a]\n" \
      "version = \"1\"\n",
      []
    ],
    [
      "sub-table first path wins, ended by header",
      "[dependencies.a]\n" \
      "path = \"../a\"\n" \
      "path = \"../b\"\n" \
      "[package]\n" \
      "name = \"z\"\n",
      ["a"]
    ],
    [
      "sub-table non-string path",
      "[dependencies.a]\n" \
      "path = 1\n",
      []
    ],
    [
      "sub-table empty path",
      "[dependencies.a]\n" \
      "path = \"\"\n",
      []
    ],
    [
      "sub-table capitalised Path key",
      "[dependencies.a]\n" \
      "Path = \"../a\"\n",
      []
    ],
    [
      "rename",
      "[dependencies]\n" \
      "x = { package = \"Logic-Gates\", path = \"../g\" }\n",
      ["logic-gates"]
    ],
    [
      "capitalised Package is not a rename",
      "[dependencies]\n" \
      "x = { Package = \"y\", path = \"../g\" }\n",
      ["x"]
    ],
    [
      "capitalised Path is not a path",
      "[dependencies]\n" \
      "x = { Path = \"../g\" }\n",
      []
    ],
    [
      "version plus path with trailing comment",
      "[dependencies]\n" \
      "a = { version = \"1\", path = \"../a\" } # b = { path = \"../b\" }\n",
      ["a"]
    ],
    [
      "hash inside a string is not a comment",
      "[dependencies]\n" \
      "a = { path = \"../a#1\" }\n",
      ["a"]
    ],
    [
      "quoted entry key",
      "[dependencies]\n" \
      "\"a\" = { path = \"../a\" }\n",
      ["a"]
    ],
    [
      "registry entry with path feature",
      "[dependencies]\n" \
      "a = { version = \"1\", features = [\"path\", \"x\"] }\n" \
      "b = \"1\"\n",
      []
    ],
    [
      "nested table path is not the entry's",
      "[dependencies]\n" \
      "a = { x = { path = \"../a\" } }\n",
      []
    ],
    [
      "features array commas do not split fields",
      "[dependencies]\n" \
      "a = { features = [\"p\", \"q\"], path = \"../a\" }\n",
      ["a"]
    ],
    [
      "empty path",
      "[dependencies]\n" \
      "a = { path = \"\" }\n",
      []
    ],
    [
      "array continued onto later lines",
      "[dependencies]\n" \
      "a = { path = \"../a\", features = [\n" \
      "    \"x\", # [not-a-header]\n" \
      "] }\n" \
      "b = { path = \"../b\" }\n",
      ["a", "b"]
    ],
    [
      "continuation lines are not headers",
      "[dependencies]\n" \
      "a = { path = \"../a\", features = [\n" \
      "[\"x\"]\n" \
      "] }\n" \
      "b = { path = \"../b\" }\n",
      ["a", "b"]
    ],
    [
      "braces continued onto later lines",
      "[dependencies]\n" \
      "a = { version = \"1\",\n" \
      "  path = \"../a\" }\n",
      ["a"]
    ],
    [
      "file ends inside the value",
      "[dependencies]\n" \
      "a = { path = \"../a\", features = [\n" \
      "  \"x\",\n",
      []
    ],
    [
      "continued value must end with a brace",
      "[dependencies]\n" \
      "a = { path = \"../a\", features = [\n" \
      "] } x\n",
      []
    ],
    [
      "dotted key is out of scope",
      "[dependencies]\n" \
      "a.path = \"../a\"\n",
      []
    ],
    [
      "commented-out entry",
      "[dependencies]\n" \
      "# a = { path = \"../a\" }\n",
      []
    ],
    [
      "dev-dependencies",
      "[dev-dependencies]\n" \
      "a = { path = \"../a\" }\n" \
      "[dev_dependencies]\n" \
      "b = { path = \"../b\" }\n",
      []
    ],
    [
      "target dev-dependencies",
      "[target.'cfg(unix)'.dev-dependencies]\n" \
      "a = { path = \"../a\" }\n",
      []
    ],
    [
      "dev-dependencies sub-table",
      "[dev-dependencies.a]\n" \
      "path = \"../a\"\n",
      []
    ],
    [
      "patch, replace, workspace",
      "[patch.crates-io]\n" \
      "a = { path = \"../a\" }\n" \
      "[workspace.dependencies]\n" \
      "b = { path = \"../b\" }\n" \
      "[replace]\n" \
      "\"c:1.0.0\" = { path = \"../c\" }\n",
      []
    ],
    [
      "capitalised table name",
      "[Dependencies]\n" \
      "a = { path = \"../a\" }\n",
      []
    ],
    [
      "features sub-table of a dependency",
      "[dependencies.a.features]\n" \
      "path = \"../a\"\n",
      []
    ],
    [
      "target table itself",
      "[target.'cfg(unix)']\n" \
      "a = { path = \"../a\" }\n",
      []
    ],
    [
      "array of tables ends the table",
      "[dependencies]\n" \
      "[[bin]]\n" \
      "a = { path = \"../a\" }\n",
      []
    ],
    [
      "malformed header ends the table",
      "[dependencies]\n" \
      "[a b]\n" \
      "c = { path = \"../c\" }\n",
      []
    ],
    [
      "unterminated quoted header ends the table",
      "[dependencies]\n" \
      "[target.'cfg(unix).dependencies]\n" \
      "c = { path = \"../c\" }\n",
      []
    ],
    [
      "CRLF and BOM",
      "\ufeff[dependencies]\r\n" \
      "a = { path = \"../a\" }\r\n",
      ["a"]
    ],
    [
      "union across tables keeps duplicates for the caller",
      "[dependencies]\n" \
      "a = { path = \"../a\" }\n" \
      "[build-dependencies]\n" \
      "a = { path = \"../a\" }\n",
      ["a", "a"]
    ],
    [
      "quoted inline path key",
      "[dependencies]\n" \
      "a = { \"path\" = \"../a\" }\n",
      ["a"]
    ],
    [
      "literal-quoted inline path key",
      "[dependencies]\n" \
      "a = { 'path' = \"../a\" }\n",
      ["a"]
    ],
    [
      "quoted inline package key renames",
      "[dependencies]\n" \
      "x = { \"package\" = \"a\", path = \"../a\" }\n",
      ["a"]
    ],
    [
      "quoted capitalised inline Path key",
      "[dependencies]\n" \
      "a = { \"Path\" = \"../a\" }\n",
      []
    ],
    [
      "escaped inline path key is compared verbatim",
      "[dependencies]\n" \
      "a = { \"pa\\u0074h\" = \"../a\" }\n",
      []
    ],
    [
      "dotted inline field key",
      "[dependencies]\n" \
      "a = { path.x = \"../a\" }\n",
      []
    ],
    [
      "quoted sub-table path key",
      "[dependencies.a]\n" \
      "\"path\" = \"../a\"\n",
      ["a"]
    ],
    [
      "quoted sub-table package key",
      "[dependencies.x]\n" \
      "'package' = \"a\"\n" \
      "path = \"../a\"\n",
      ["a"]
    ],
    [
      "quoted capitalised sub-table Path key",
      "[dependencies.a]\n" \
      "\"Path\" = \"../a\"\n",
      []
    ],
    [
      "open string continues across the join",
      "[dependencies]\n" \
      "a = { path = \"../a\", x = \"q [\n" \
      "\" }\n",
      ["a"]
    ],
    [
      "line-final backslash escapes the joining space",
      "[dependencies]\n" \
      "a = { x = \"q\\\n" \
      "\", path = \"../a\" }\n",
      ["a"]
    ]
  ].freeze

  CASES.each_with_index do |(name, manifest, expected), index|
    define_method("test_cargo_shape_%02d_%s" % [index, name.gsub(/[^a-z0-9]+/i, "_")]) do
      assert_equal expected, BuildTool::Resolver.cargo_link_dependency_names(manifest), name
    end
  end

  # The table is the Go reference's, row for row: a row added there and not
  # here (or the reverse) must fail loudly.
  def test_shape_table_matches_the_go_reference_row_count
    assert_equal 57, CASES.length
  end

  # Rescanning the whole joined value for every appended line is quadratic:
  # a 20,000-line continuation (about 500 KB) took tens of seconds. Carrying
  # the scanner state makes it one pass; the bound is generous so a loaded CI
  # machine cannot flake it.
  def test_continuation_join_is_linear
    manifest = "[dependencies]\na = { path = \"../a\", features = [\n" +
      ("  \"feature-name-padding\",\n" * 20_000) + "] }\n"

    started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    names = BuildTool::Resolver.cargo_link_dependency_names(manifest)
    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started

    assert_equal ["a"], names
    assert_operator elapsed, :<, 10, "a 20,000-line continuation took #{elapsed}s"
  end

  # `sub(/[ \t]+\z/, "")` retries from every byte of an interior run of
  # spaces: quadratic in a long run between two other characters.
  def test_trim_is_linear_in_an_interior_run_of_spaces
    line = "a#{" " * 100_000}b"

    started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    trimmed = BuildTool::Resolver.trim_spaces_and_tabs(" \t#{line}\t ")
    names = BuildTool::Resolver.cargo_link_dependency_names("[dependencies]\n#{line}\n")
    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started

    assert_equal line, trimmed
    assert_equal [], names
    assert_operator elapsed, :<, 5, "a 100 KB run of spaces took #{elapsed}s"
  end

  # `^\s*name` runs `\s*` across every following blank line from each line
  # start (Ruby's `^` is always multi-line): quadratic in a run of them.
  def test_package_name_reader_is_linear_in_blank_lines
    dir = create_temp_dir
    pkg = BuildTool::Package.new(name: "rust/late", path: dir, build_commands: [], language: "rust")
    write_file(dir / "Cargo.toml", ("\n" * 100_000) + "name =\n\"unterminated\n")

    started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    name = BuildTool::Resolver.read_cargo_package_name(pkg)
    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started

    assert_nil name
    assert_operator elapsed, :<, 5, "100,000 blank lines took #{elapsed}s"

    write_file(dir / "Cargo.toml", ("\n" * 100_000) + "[package]\nname = \"Late-Crate\"\n")
    assert_equal "late-crate", BuildTool::Resolver.read_cargo_package_name(pkg)
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_package_name_does_not_span_lines
    dir = create_temp_dir
    write_file(dir / "Cargo.toml", "[package]\nname =\n\"split\"\nname = \"app\"\n")
    pkg = BuildTool::Package.new(name: "rust/app", path: dir, build_commands: [], language: "rust")

    assert_equal "app", BuildTool::Resolver.read_cargo_package_name(pkg)
  ensure
    FileUtils.rm_rf(dir)
  end

  # Step 5: a candidate resolving to the crate itself -- by its directory
  # alias, by its [package] name through a renamed sub-table, or from a
  # target table -- is DEPENDENCY_SELF_EDGE, neither dropped nor drawn.
  SELF_EDGE_ENTRIES = {
    "directory alias" => "[dependencies]\napp-dir = { path = \".\" }\n",
    "cargo name via a renamed sub-table" => "[build-dependencies.self]\npackage = \"app-crate\"\npath = \".\"\n",
    "target table" => "[target.'cfg(unix)'.dependencies]\napp-crate = { path = \".\" }\n"
  }.freeze

  SELF_EDGE_ENTRIES.each do |label, entry|
    define_method("test_self_edge_is_a_diagnostic_#{label.gsub(/[^a-z0-9]+/i, "_")}") do
      dir = create_temp_dir
      # The checkout itself sits under code/packages/checkout/: the diagnostic
      # must cut at the LAST code/packages pair and never print the checkout.
      app = dir / "code" / "packages" / "checkout" / "code" / "packages" / "rust" / "app-dir"
      write_file(app / "Cargo.toml", "[package]\nname = \"app-crate\"\n\n#{entry}")
      packages = [BuildTool::Package.new(name: "rust/app-dir", path: app, build_commands: [], language: "rust")]

      error = assert_raises(BuildTool::DependencySelfEdgeError) do
        BuildTool::Resolver.resolve_dependencies(packages)
      end

      assert_equal "DEPENDENCY_SELF_EDGE", error.code
      assert_equal "rust/app-dir", error.package
      assert_equal "code/packages/rust/app-dir/Cargo.toml", error.manifest
      assert_equal "rust/app-dir", error.dependency
      assert_equal "DEPENDENCY_SELF_EDGE: package=rust/app-dir " \
        "manifest=code/packages/rust/app-dir/Cargo.toml dependency=rust/app-dir", error.message
      refute_includes error.message, dir.to_s
    ensure
      FileUtils.rm_rf(dir)
    end
  end

  def test_duplicate_links_collapse_and_unknown_names_drop
    dir = create_temp_dir
    write_file(dir / "app" / "Cargo.toml", <<~TOML)
      [package]
      name = "app"

      [dependencies]
      lib = { path = "../lib" }
      serde = { path = "../vendored-serde" }

      [build-dependencies]
      lib = { path = "../lib" }

      [target.'cfg(windows)'.dependencies.lib]
      path = "../lib"
    TOML
    write_file(dir / "lib" / "Cargo.toml", "[package]\nname = \"lib\"\n")
    packages = %w[app lib].map do |name|
      BuildTool::Package.new(name: "rust/#{name}", path: dir / name, build_commands: [], language: "rust")
    end

    deps = BuildTool::Resolver.parse_rust_deps(packages.first, BuildTool::Resolver.build_known_names(packages))

    assert_equal ["rust/lib"], deps
  ensure
    FileUtils.rm_rf(dir)
  end

  def test_manifest_is_read_as_bytes
    dir = create_temp_dir
    File.binwrite(dir / "Cargo.toml", "\xEF\xBB\xBF[dependencies]\r\nb = { path = \"../\xC3\xA9\" }\r\n".b)
    pkg = BuildTool::Package.new(name: "rust/a", path: dir, build_commands: [], language: "rust")

    assert_equal ["rust/b"], BuildTool::Resolver.parse_rust_deps(pkg, {"b" => "rust/b"})
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
