# frozen_string_literal: true

# test_scaffold_generator.rb -- Tests for the Ruby scaffold-generator
# ====================================================================
#
# === What These Tests Verify ===
#
# These tests exercise the core logic of the scaffold generator:
#   - Name normalization (to_snake_case, to_camel_case, to_joined_lower)
#   - Dependency reading for all 6 languages
#   - Transitive closure computation
#   - Topological sort ordering
#   - File generation for all 6 languages
#   - Critical TypeScript fields (main must be src/index.ts)
#   - Ruby require ordering (deps before own modules)
#
# === Why Unit Tests Instead of Integration Tests ===
#
# The scaffold generator writes files to disk. Rather than testing the
# full CLI flow (which would need a real repo structure), we test each
# piece of logic in isolation with tmpdir for file generation tests.

require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "json"
require "coding_adventures_scaffold_generator"

# Shortcut to the module under test for readability.
SG = CodingAdventures::ScaffoldGenerator

# ===========================================================================
# Test: Name Normalization
# ===========================================================================
#
# These tests verify the three name conversion functions. The input is
# always kebab-case (e.g., "my-package"), and each function converts to
# a different naming convention.

class TestNameNormalization < Minitest::Test
  # --- to_snake_case ---
  # Converts hyphens to underscores: "my-package" => "my_package"

  def test_to_snake_case_simple
    assert_equal "my_package", SG.to_snake_case("my-package")
  end

  def test_to_snake_case_single_word
    assert_equal "alu", SG.to_snake_case("alu")
  end

  def test_to_snake_case_multiple_hyphens
    assert_equal "my_cool_pkg", SG.to_snake_case("my-cool-pkg")
  end

  # --- to_camel_case ---
  # Capitalizes each segment and joins: "my-package" => "MyPackage"

  def test_to_camel_case_simple
    assert_equal "MyPackage", SG.to_camel_case("my-package")
  end

  def test_to_camel_case_single_word
    assert_equal "Alu", SG.to_camel_case("alu")
  end

  def test_to_camel_case_multiple_hyphens
    assert_equal "MyCoolPkg", SG.to_camel_case("my-cool-pkg")
  end

  # --- to_joined_lower ---
  # Removes hyphens: "my-package" => "mypackage" (Go convention)

  def test_to_joined_lower_simple
    assert_equal "mypackage", SG.to_joined_lower("my-package")
  end

  def test_to_joined_lower_single_word
    assert_equal "alu", SG.to_joined_lower("alu")
  end

  # --- dir_name ---
  # Ruby and Elixir use snake_case; others use kebab-case

  def test_dir_name_ruby_uses_snake_case
    assert_equal "my_package", SG.dir_name("my-package", "ruby")
  end

  def test_dir_name_elixir_uses_snake_case
    assert_equal "my_package", SG.dir_name("my-package", "elixir")
  end

  def test_dir_name_python_uses_kebab
    assert_equal "my-package", SG.dir_name("my-package", "python")
  end

  def test_dir_name_go_uses_kebab
    assert_equal "my-package", SG.dir_name("my-package", "go")
  end

  def test_dir_name_typescript_uses_kebab
    assert_equal "my-package", SG.dir_name("my-package", "typescript")
  end

  def test_dir_name_rust_uses_kebab
    assert_equal "my-package", SG.dir_name("my-package", "rust")
  end
end

# ===========================================================================
# Test: Dependency Reading
# ===========================================================================
#
# These tests create temporary metadata files for each language and verify
# that read_deps correctly extracts the dependency names.

class TestDependencyReading < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("scaffold-test")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  # --- Python ---
  # Python BUILD files use: pip install -e ../dep-name

  def test_read_python_deps
    pkg_dir = File.join(@tmpdir, "my-pkg")
    FileUtils.mkdir_p(pkg_dir)
    File.write(File.join(pkg_dir, "BUILD"), <<~BUILD)
      pip install -e ../logic-gates -e ../arithmetic -e .[dev] --quiet
      python -m pytest tests/ -v
    BUILD
    deps = SG.read_python_deps(pkg_dir)
    assert_equal %w[logic-gates arithmetic], deps
  end

  def test_read_python_deps_no_build_file
    assert_equal [], SG.read_python_deps(File.join(@tmpdir, "nonexistent"))
  end

  # --- Go ---
  # Go go.mod uses: replace ... => ../dep

  def test_read_go_deps
    pkg_dir = File.join(@tmpdir, "my-pkg")
    FileUtils.mkdir_p(pkg_dir)
    File.write(File.join(pkg_dir, "go.mod"), <<~MOD)
      module github.com/example/my-pkg

      go 1.26

      require (
      \tgithub.com/example/logic-gates v0.0.0
      )

      replace (
      \tgithub.com/example/logic-gates => ../logic-gates
      \tgithub.com/example/arithmetic => ../arithmetic
      )
    MOD
    deps = SG.read_go_deps(pkg_dir)
    assert_equal %w[logic-gates arithmetic], deps
  end

  def test_read_go_deps_no_mod_file
    assert_equal [], SG.read_go_deps(File.join(@tmpdir, "nonexistent"))
  end

  # --- Ruby ---
  # Ruby Gemfile uses: gem "name", path: "../dep"

  def test_read_ruby_deps
    pkg_dir = File.join(@tmpdir, "my-pkg")
    FileUtils.mkdir_p(pkg_dir)
    File.write(File.join(pkg_dir, "Gemfile"), <<~GEM)
      source "https://rubygems.org"
      gemspec
      gem "coding_adventures_logic_gates", path: "../logic_gates"
      gem "coding_adventures_arithmetic", path: "../arithmetic"
    GEM
    deps = SG.read_ruby_deps(pkg_dir)
    assert_equal %w[logic-gates arithmetic], deps
  end

  def test_read_ruby_deps_no_gemfile
    assert_equal [], SG.read_ruby_deps(File.join(@tmpdir, "nonexistent"))
  end

  # --- TypeScript ---
  # TypeScript package.json uses: "file:../dep"

  def test_read_ts_deps
    pkg_dir = File.join(@tmpdir, "my-pkg")
    FileUtils.mkdir_p(pkg_dir)
    File.write(File.join(pkg_dir, "package.json"), JSON.generate({
      "name" => "@coding-adventures/my-pkg",
      "dependencies" => {
        "@coding-adventures/logic-gates" => "file:../logic-gates",
        "@coding-adventures/arithmetic" => "file:../arithmetic"
      }
    }))
    deps = SG.read_ts_deps(pkg_dir)
    assert_equal %w[logic-gates arithmetic], deps
  end

  def test_read_ts_deps_no_pkg_json
    assert_equal [], SG.read_ts_deps(File.join(@tmpdir, "nonexistent"))
  end

  def test_read_ts_deps_invalid_json
    pkg_dir = File.join(@tmpdir, "my-pkg")
    FileUtils.mkdir_p(pkg_dir)
    File.write(File.join(pkg_dir, "package.json"), "not valid json {{{")
    assert_equal [], SG.read_ts_deps(pkg_dir)
  end

  # --- Rust ---
  # Rust Cargo.toml uses: path = "../dep"

  def test_read_rust_deps
    pkg_dir = File.join(@tmpdir, "my-pkg")
    FileUtils.mkdir_p(pkg_dir)
    File.write(File.join(pkg_dir, "Cargo.toml"), <<~TOML)
      [package]
      name = "my-pkg"

      [dependencies]
      logic-gates = { path = "../logic-gates" }
      arithmetic = { path = "../arithmetic" }
    TOML
    deps = SG.read_rust_deps(pkg_dir)
    assert_equal %w[logic-gates arithmetic], deps
  end

  def test_read_rust_deps_no_cargo_file
    assert_equal [], SG.read_rust_deps(File.join(@tmpdir, "nonexistent"))
  end

  # --- Elixir ---
  # Elixir mix.exs uses: path: "../dep"

  def test_read_elixir_deps
    pkg_dir = File.join(@tmpdir, "my-pkg")
    FileUtils.mkdir_p(pkg_dir)
    File.write(File.join(pkg_dir, "mix.exs"), <<~EX)
      defmodule MyPkg.MixProject do
        defp deps do
          [
            {:coding_adventures_logic_gates, path: "../logic_gates"},
            {:coding_adventures_arithmetic, path: "../arithmetic"}
          ]
        end
      end
    EX
    deps = SG.read_elixir_deps(pkg_dir)
    assert_equal %w[logic-gates arithmetic], deps
  end

  def test_read_elixir_deps_no_mix_file
    assert_equal [], SG.read_elixir_deps(File.join(@tmpdir, "nonexistent"))
  end

  # --- Perl ---
  # Perl cpanfile uses: requires 'coding-adventures-<name>';

  def test_read_perl_deps
    pkg_dir = File.join(@tmpdir, "my-pkg")
    FileUtils.mkdir_p(pkg_dir)
    File.write(File.join(pkg_dir, "cpanfile"), <<~CPAN)
      requires 'coding-adventures-logic-gates';
      requires 'coding-adventures-arithmetic';
      on 'test' => sub {
          requires 'Test2::V0';
      };
    CPAN
    deps = SG.read_perl_deps(pkg_dir)
    assert_equal %w[logic-gates arithmetic], deps
  end

  def test_read_perl_deps_no_cpanfile
    assert_equal [], SG.read_perl_deps(File.join(@tmpdir, "nonexistent"))
  end

  # --- read_deps dispatcher ---
  # Verify the dispatcher correctly routes to language-specific readers

  def test_read_deps_dispatches_to_python
    pkg_dir = File.join(@tmpdir, "my-pkg")
    FileUtils.mkdir_p(pkg_dir)
    File.write(File.join(pkg_dir, "BUILD"), "python -m pip install -e ../some-dep -e .[dev] --quiet\n")
    deps = SG.read_deps(pkg_dir, "python")
    assert_equal %w[some-dep], deps
  end

  def test_read_deps_unknown_language
    assert_equal [], SG.read_deps(@tmpdir, "unknown")
  end
end

# ===========================================================================
# Test: Transitive Closure
# ===========================================================================
#
# These tests create a small dependency graph in a tmpdir and verify that
# transitive_closure correctly discovers all indirect dependencies.

class TestTransitiveClosure < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("scaffold-closure")
    # Create: A depends on B, B depends on C, C has no deps
    # Using Python format for simplicity
    %w[A B C].each { |d| FileUtils.mkdir_p(File.join(@tmpdir, d)) }
    File.write(File.join(@tmpdir, "A", "BUILD"), "python -m pip install -e ../B -e .[dev] --quiet\n")
    File.write(File.join(@tmpdir, "B", "BUILD"), "python -m pip install -e ../C -e .[dev] --quiet\n")
    File.write(File.join(@tmpdir, "C", "BUILD"), "python -m pip install -e .[dev] --quiet\n")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_closure_finds_all_transitive_deps
    result = SG.transitive_closure(%w[A], "python", @tmpdir)
    assert_equal %w[A B C], result
  end

  def test_closure_with_no_deps
    result = SG.transitive_closure([], "python", @tmpdir)
    assert_equal [], result
  end

  def test_closure_with_leaf_dep
    result = SG.transitive_closure(%w[C], "python", @tmpdir)
    assert_equal %w[C], result
  end
end

# ===========================================================================
# Test: Topological Sort
# ===========================================================================
#
# Verify that topological_sort returns dependencies in leaf-first order,
# which is the correct install order for BUILD files.

class TestTopologicalSort < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("scaffold-topo")
    %w[A B C].each { |d| FileUtils.mkdir_p(File.join(@tmpdir, d)) }
    File.write(File.join(@tmpdir, "A", "BUILD"), "python -m pip install -e ../B -e .[dev] --quiet\n")
    File.write(File.join(@tmpdir, "B", "BUILD"), "python -m pip install -e ../C -e .[dev] --quiet\n")
    File.write(File.join(@tmpdir, "C", "BUILD"), "python -m pip install -e .[dev] --quiet\n")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_topological_order_is_leaf_first
    result = SG.topological_sort(%w[A B C], "python", @tmpdir)
    # C has no deps, so it comes first. B depends on C, A depends on B.
    assert_equal %w[C B A], result
  end

  def test_topological_sort_single_item
    result = SG.topological_sort(%w[C], "python", @tmpdir)
    assert_equal %w[C], result
  end

  def test_topological_sort_empty
    result = SG.topological_sort([], "python", @tmpdir)
    assert_equal [], result
  end
end

# ===========================================================================
# Test: File Generation - Python
# ===========================================================================

class TestGeneratePython < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("scaffold-python")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_generates_pyproject_toml
    SG.generate_python(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "pyproject.toml"))
    content = File.read(File.join(@tmpdir, "pyproject.toml"))
    assert_includes content, 'name = "coding-adventures-my-package"'
    assert_includes content, 'version = "0.1.0"'
  end

  def test_generates_init_py
    SG.generate_python(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "src", "my_package", "__init__.py"))
  end

  def test_generates_test_file
    SG.generate_python(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "tests", "test_my_package.py"))
  end

  def test_build_includes_dep_installs
    SG.generate_python(@tmpdir, "my-package", "A test", "", %w[logic-gates], %w[logic-gates])
    build = File.read(File.join(@tmpdir, "BUILD"))
    assert_includes build, "python -m pip install -e ../logic-gates -e .[dev] --quiet"
  end
end

# ===========================================================================
# Test: File Generation - Go
# ===========================================================================

class TestGenerateGo < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("scaffold-go")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_generates_go_mod
    SG.generate_go(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "go.mod"))
    content = File.read(File.join(@tmpdir, "go.mod"))
    assert_includes content, "module github.com/adhithyan15/coding-adventures/code/packages/go/my-package"
  end

  def test_generates_source_file
    SG.generate_go(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "my_package.go"))
    content = File.read(File.join(@tmpdir, "my_package.go"))
    assert_includes content, "package mypackage"
  end

  def test_generates_test_file
    SG.generate_go(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "my_package_test.go"))
  end

  def test_go_mod_with_deps
    SG.generate_go(@tmpdir, "my-package", "A test", "", %w[logic-gates], %w[logic-gates])
    content = File.read(File.join(@tmpdir, "go.mod"))
    assert_includes content, "require ("
    assert_includes content, "replace ("
    assert_includes content, "=> ../logic-gates"
  end
end

# ===========================================================================
# Test: File Generation - Ruby
# ===========================================================================

class TestGenerateRuby < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("scaffold-ruby")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_generates_gemspec
    SG.generate_ruby(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "coding_adventures_my_package.gemspec"))
    content = File.read(File.join(@tmpdir, "coding_adventures_my_package.gemspec"))
    assert_includes content, 'spec.name          = "coding_adventures_my_package"'
    assert_includes content, "CodingAdventures::MyPackage::VERSION"
  end

  def test_generates_gemfile
    SG.generate_ruby(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "Gemfile"))
  end

  def test_generates_entry_point
    SG.generate_ruby(@tmpdir, "my-package", "A test package", "", [], [])
    entry_path = File.join(@tmpdir, "lib", "coding_adventures_my_package.rb")
    assert File.exist?(entry_path)
    content = File.read(entry_path)
    assert_includes content, "module CodingAdventures"
    assert_includes content, "module MyPackage"
  end

  def test_generates_version_file
    SG.generate_ruby(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "lib", "coding_adventures", "my_package", "version.rb"))
  end

  def test_generates_test_file
    SG.generate_ruby(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "test", "test_my_package.rb"))
  end

  # --- CRITICAL: Ruby require ordering ---
  # Dependencies must be required BEFORE own modules. This is a recurring
  # CI failure documented in lessons.md.

  def test_ruby_require_ordering_deps_before_own_modules
    SG.generate_ruby(@tmpdir, "my-package", "A test", "", %w[logic-gates], %w[logic-gates])
    entry = File.read(File.join(@tmpdir, "lib", "coding_adventures_my_package.rb"))
    # The require for the dependency must appear BEFORE require_relative
    dep_idx = entry.index('require "coding_adventures_logic_gates"')
    rel_idx = entry.index('require_relative "coding_adventures/my_package/version"')
    refute_nil dep_idx, "should require dependency"
    refute_nil rel_idx, "should require_relative own version"
    assert dep_idx < rel_idx, "dependency require must come BEFORE require_relative"
  end

  def test_ruby_gemfile_includes_transitive_deps
    SG.generate_ruby(@tmpdir, "my-package", "A test", "", %w[logic-gates], %w[arithmetic logic-gates])
    gemfile = File.read(File.join(@tmpdir, "Gemfile"))
    assert_includes gemfile, 'gem "coding_adventures_arithmetic", path: "../arithmetic"'
    assert_includes gemfile, 'gem "coding_adventures_logic_gates", path: "../logic_gates"'
  end
end

# ===========================================================================
# Test: File Generation - TypeScript
# ===========================================================================

class TestGenerateTypeScript < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("scaffold-ts")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_generates_package_json
    SG.generate_typescript(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "package.json"))
  end

  # --- CRITICAL: TypeScript main field ---
  # The "main" field MUST be "src/index.ts" (not "dist/index.js") so that
  # Vitest can resolve file: dependencies without a compile step.
  # This is a recurring CI failure documented in lessons.md.

  def test_typescript_main_is_src_index_ts
    SG.generate_typescript(@tmpdir, "my-package", "A test", "", [], [])
    pkg = JSON.parse(File.read(File.join(@tmpdir, "package.json")))
    assert_equal "src/index.ts", pkg["main"],
      "CRITICAL: main must be src/index.ts, not dist/index.js (see lessons.md)"
  end

  def test_typescript_type_is_module
    SG.generate_typescript(@tmpdir, "my-package", "A test", "", [], [])
    pkg = JSON.parse(File.read(File.join(@tmpdir, "package.json")))
    assert_equal "module", pkg["type"]
  end

  def test_typescript_has_vitest_coverage_v8
    SG.generate_typescript(@tmpdir, "my-package", "A test", "", [], [])
    pkg = JSON.parse(File.read(File.join(@tmpdir, "package.json")))
    dev_deps = pkg["devDependencies"]
    assert dev_deps.key?("@vitest/coverage-v8"),
      "CRITICAL: @vitest/coverage-v8 must be in devDependencies (see lessons.md)"
  end

  def test_typescript_deps_use_file_protocol
    SG.generate_typescript(@tmpdir, "my-package", "A test", "", %w[logic-gates], %w[logic-gates])
    pkg = JSON.parse(File.read(File.join(@tmpdir, "package.json")))
    assert_equal "file:../logic-gates", pkg["dependencies"]["@coding-adventures/logic-gates"]
  end

  def test_generates_tsconfig
    SG.generate_typescript(@tmpdir, "my-package", "A test", "", [], [])
    assert File.exist?(File.join(@tmpdir, "tsconfig.json"))
  end

  def test_generates_vitest_config
    SG.generate_typescript(@tmpdir, "my-package", "A test", "", [], [])
    assert File.exist?(File.join(@tmpdir, "vitest.config.ts"))
  end

  def test_build_has_npm_ci
    SG.generate_typescript(@tmpdir, "my-package", "A test", "", %w[logic-gates],
                           %w[arithmetic logic-gates])
    build = File.read(File.join(@tmpdir, "BUILD"))
    assert_includes build, "npm ci --quiet"
    assert_includes build, "npx vitest run --coverage"
  end
end

# ===========================================================================
# Test: File Generation - Rust
# ===========================================================================

class TestGenerateRust < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("scaffold-rust")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_generates_cargo_toml
    SG.generate_rust(@tmpdir, "my-package", "A test package", "", [])
    assert File.exist?(File.join(@tmpdir, "Cargo.toml"))
    content = File.read(File.join(@tmpdir, "Cargo.toml"))
    assert_includes content, 'name = "my-package"'
  end

  def test_generates_lib_rs
    SG.generate_rust(@tmpdir, "my-package", "A test package", "", [])
    assert File.exist?(File.join(@tmpdir, "src", "lib.rs"))
  end

  def test_cargo_with_deps
    SG.generate_rust(@tmpdir, "my-package", "A test", "", %w[logic-gates])
    content = File.read(File.join(@tmpdir, "Cargo.toml"))
    assert_includes content, 'logic-gates = { path = "../logic-gates" }'
  end
end

# ===========================================================================
# Test: File Generation - Elixir
# ===========================================================================

class TestGenerateElixir < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("scaffold-elixir")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_generates_mix_exs
    SG.generate_elixir(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "mix.exs"))
    content = File.read(File.join(@tmpdir, "mix.exs"))
    assert_includes content, "CodingAdventures.MyPackage.MixProject"
    assert_includes content, ":coding_adventures_my_package"
  end

  def test_generates_lib_file
    SG.generate_elixir(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "lib", "coding_adventures", "my_package.ex"))
  end

  def test_generates_test_files
    SG.generate_elixir(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "test", "my_package_test.exs"))
    assert File.exist?(File.join(@tmpdir, "test", "test_helper.exs"))
  end

  def test_mix_with_deps
    SG.generate_elixir(@tmpdir, "my-package", "A test", "", %w[logic-gates], %w[logic-gates])
    content = File.read(File.join(@tmpdir, "mix.exs"))
    assert_includes content, ':coding_adventures_logic_gates, path: "../logic_gates"'
  end
end

# ===========================================================================
# Test: File Generation - Perl
# ===========================================================================

class TestGeneratePerl < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("scaffold-perl")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_generates_makefile_pl
    SG.generate_perl(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "Makefile.PL"))
    content = File.read(File.join(@tmpdir, "Makefile.PL"))
    assert_includes content, "CodingAdventures::MyPackage"
  end

  def test_generates_cpanfile
    SG.generate_perl(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "cpanfile"))
  end

  def test_generates_module_file
    SG.generate_perl(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "lib", "CodingAdventures", "MyPackage.pm"))
  end

  def test_generates_test_files
    SG.generate_perl(@tmpdir, "my-package", "A test package", "", [], [])
    assert File.exist?(File.join(@tmpdir, "t", "00-load.t"))
    assert File.exist?(File.join(@tmpdir, "t", "01-basic.t"))
  end

  def test_makefile_pl_has_dep
    SG.generate_perl(@tmpdir, "my-package", "A test", "", %w[logic-gates], %w[logic-gates])
    content = File.read(File.join(@tmpdir, "Makefile.PL"))
    assert_includes content, "CodingAdventures::LogicGates"
  end

  def test_cpanfile_has_dep
    SG.generate_perl(@tmpdir, "my-package", "A test", "", %w[logic-gates], %w[logic-gates])
    content = File.read(File.join(@tmpdir, "cpanfile"))
    assert_includes content, "coding-adventures-logic-gates"
  end

  def test_build_chain_installs
    SG.generate_perl(@tmpdir, "my-package", "A test", "", %w[logic-gates], %w[logic-gates])
    build = File.read(File.join(@tmpdir, "BUILD"))
    assert_includes build, "../logic-gates"
    assert_includes build, "prove -l -v t/"
  end

  def test_build_no_deps
    SG.generate_perl(@tmpdir, "my-package", "A test", "", [], [])
    build = File.read(File.join(@tmpdir, "BUILD"))
    refute_includes build, "cd ../"
    assert_includes build, "prove -l -v t/"
  end
end

# ===========================================================================
# Test: Common Files
# ===========================================================================

class TestCommonFiles < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("scaffold-common")
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_generates_readme
    SG.generate_common_files(@tmpdir, "my-package", "A test package", "python", 0, [])
    readme = File.read(File.join(@tmpdir, "README.md"))
    assert_includes readme, "# my-package"
    assert_includes readme, "A test package"
  end

  def test_readme_includes_layer_info
    SG.generate_common_files(@tmpdir, "my-package", "A test", "python", 5, [])
    readme = File.read(File.join(@tmpdir, "README.md"))
    assert_includes readme, "Layer 5"
  end

  def test_readme_no_layer_when_zero
    SG.generate_common_files(@tmpdir, "my-package", "A test", "python", 0, [])
    readme = File.read(File.join(@tmpdir, "README.md"))
    refute_includes readme, "Layer 0"
  end

  def test_readme_includes_deps
    SG.generate_common_files(@tmpdir, "my-package", "A test", "python", 0, %w[logic-gates])
    readme = File.read(File.join(@tmpdir, "README.md"))
    assert_includes readme, "- logic-gates"
  end

  def test_generates_changelog
    SG.generate_common_files(@tmpdir, "my-package", "A test", "python", 0, [])
    changelog = File.read(File.join(@tmpdir, "CHANGELOG.md"))
    assert_includes changelog, "# Changelog"
    assert_includes changelog, "scaffold-generator"
    assert_includes changelog, Date.today.iso8601
  end
end

# ===========================================================================
# Test: Version
# ===========================================================================

class TestVersion < Minitest::Test
  def test_version_exists
    refute_nil SG::VERSION
    assert_equal "1.0.0", SG::VERSION
  end
end

# ===========================================================================
# Test: KEBAB_RE validation
# ===========================================================================

class TestKebabRegex < Minitest::Test
  def test_valid_simple_name
    assert SG::KEBAB_RE.match?("my-package")
  end

  def test_valid_single_word
    assert SG::KEBAB_RE.match?("alu")
  end

  def test_valid_with_numbers
    assert SG::KEBAB_RE.match?("logic-gates2")
  end

  def test_invalid_uppercase
    refute SG::KEBAB_RE.match?("MyPackage")
  end

  def test_invalid_underscores
    refute SG::KEBAB_RE.match?("my_package")
  end

  def test_invalid_leading_hyphen
    refute SG::KEBAB_RE.match?("-bad")
  end

  def test_invalid_trailing_hyphen
    refute SG::KEBAB_RE.match?("bad-")
  end

  def test_invalid_double_hyphen
    refute SG::KEBAB_RE.match?("bad--name")
  end
end

# ===========================================================================
# Test: Scaffold One (integration)
# ===========================================================================
#
# This tests the scaffold_one orchestration with a mock repo structure.

class TestScaffoldOne < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("scaffold-integration")
    # Create a minimal repo structure
    @repo_root = @tmpdir
    FileUtils.mkdir_p(File.join(@repo_root, ".git"))
    FileUtils.mkdir_p(File.join(@repo_root, "code", "packages", "python"))
    FileUtils.mkdir_p(File.join(@repo_root, "code", "packages", "go"))
    FileUtils.mkdir_p(File.join(@repo_root, "code", "packages", "ruby"))
    FileUtils.mkdir_p(File.join(@repo_root, "code", "packages", "typescript"))
    FileUtils.mkdir_p(File.join(@repo_root, "code", "packages", "rust"))
    FileUtils.mkdir_p(File.join(@repo_root, "code", "packages", "elixir"))
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_scaffold_python_creates_directory
    output = StringIO.new
    SG.scaffold_one("test-pkg", "library", "python", [], 0, "A test", false, @repo_root,
                     output: output)
    target = File.join(@repo_root, "code", "packages", "python", "test-pkg")
    assert Dir.exist?(target)
    assert File.exist?(File.join(target, "pyproject.toml"))
    assert File.exist?(File.join(target, "README.md"))
    assert File.exist?(File.join(target, "CHANGELOG.md"))
  end

  def test_scaffold_raises_if_dir_exists
    target = File.join(@repo_root, "code", "packages", "python", "test-pkg")
    FileUtils.mkdir_p(target)
    assert_raises(RuntimeError) do
      SG.scaffold_one("test-pkg", "library", "python", [], 0, "A test", false, @repo_root)
    end
  end

  def test_scaffold_raises_if_dep_missing
    assert_raises(RuntimeError) do
      SG.scaffold_one("test-pkg", "library", "python", %w[nonexistent-dep], 0, "A test",
                       false, @repo_root)
    end
  end

  def test_dry_run_does_not_create_files
    output = StringIO.new
    SG.scaffold_one("test-pkg", "library", "python", [], 0, "A test", true, @repo_root,
                     output: output)
    target = File.join(@repo_root, "code", "packages", "python", "test-pkg")
    refute Dir.exist?(target)
    assert_includes output.string, "[dry-run]"
  end

  def test_scaffold_program_uses_programs_dir
    output = StringIO.new
    FileUtils.mkdir_p(File.join(@repo_root, "code", "programs", "python"))
    SG.scaffold_one("test-prog", "program", "python", [], 0, "A test", false, @repo_root,
                     output: output)
    target = File.join(@repo_root, "code", "programs", "python", "test-prog")
    assert Dir.exist?(target)
  end

  def test_scaffold_ruby_uses_snake_case_dir
    output = StringIO.new
    SG.scaffold_one("my-package", "library", "ruby", [], 0, "A test", false, @repo_root,
                     output: output)
    target = File.join(@repo_root, "code", "packages", "ruby", "my_package")
    assert Dir.exist?(target)
  end
end

# ===========================================================================
# Test: Rust Workspace Update
# ===========================================================================

class TestRustWorkspaceUpdate < Minitest::Test
  def setup
    @tmpdir = Dir.mktmpdir("scaffold-rust-ws")
    @cargo_dir = File.join(@tmpdir, "code", "packages", "rust")
    FileUtils.mkdir_p(@cargo_dir)
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_adds_crate_to_workspace
    File.write(File.join(@cargo_dir, "Cargo.toml"), <<~TOML)
      [workspace]
      members = [
        "existing-crate",
      ]
    TOML
    result = SG.update_rust_workspace(@tmpdir, "new-crate")
    assert result
    content = File.read(File.join(@cargo_dir, "Cargo.toml"))
    assert_includes content, '"new-crate"'
  end

  def test_skips_if_already_present
    File.write(File.join(@cargo_dir, "Cargo.toml"), <<~TOML)
      [workspace]
      members = [
        "existing-crate",
      ]
    TOML
    result = SG.update_rust_workspace(@tmpdir, "existing-crate")
    assert result
  end

  def test_returns_false_if_no_cargo_toml
    result = SG.update_rust_workspace("/nonexistent", "crate")
    refute result
  end
end
