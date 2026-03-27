# =========================================================================
# Tests for CodingAdventures.ScaffoldGenerator
# =========================================================================
#
# These tests verify the core algorithms of the scaffold generator:
#
#   1. Name normalization (snake_case, CamelCase, joinedlower)
#   2. Dependency reading from metadata files
#   3. Transitive closure (BFS)
#   4. Topological sort (Kahn's algorithm)
#   5. File generation for all 6 languages
#   6. CLI argument parsing
#
# Most tests use temporary directories to avoid polluting the real repo.
# =========================================================================

defmodule CodingAdventures.ScaffoldGeneratorTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.ScaffoldGenerator
  alias CodingAdventures.ScaffoldGenerator.Config

  # =========================================================================
  # Name normalization tests
  # =========================================================================
  #
  # These are pure functions, so they're easy to test exhaustively.
  # We test the three naming conventions used across the six languages:
  #
  #   | Input          | snake_case     | CamelCase      | joinedlower    |
  #   |----------------|----------------|----------------|----------------|
  #   | my-package     | my_package     | MyPackage      | mypackage      |
  #   | logic-gates    | logic_gates    | LogicGates     | logicgates     |
  #   | a              | a              | A              | a              |

  describe "to_snake_case/1" do
    test "converts hyphens to underscores" do
      assert ScaffoldGenerator.to_snake_case("my-package") == "my_package"
    end

    test "handles single word" do
      assert ScaffoldGenerator.to_snake_case("package") == "package"
    end

    test "handles multiple hyphens" do
      assert ScaffoldGenerator.to_snake_case("a-b-c-d") == "a_b_c_d"
    end
  end

  describe "to_camel_case/1" do
    test "capitalizes each segment" do
      assert ScaffoldGenerator.to_camel_case("my-package") == "MyPackage"
    end

    test "handles single word" do
      assert ScaffoldGenerator.to_camel_case("package") == "Package"
    end

    test "handles multiple segments" do
      assert ScaffoldGenerator.to_camel_case("logic-gates-advanced") == "LogicGatesAdvanced"
    end

    test "handles single character segments" do
      assert ScaffoldGenerator.to_camel_case("a-b") == "AB"
    end
  end

  describe "to_joined_lower/1" do
    test "removes hyphens" do
      assert ScaffoldGenerator.to_joined_lower("my-package") == "mypackage"
    end

    test "handles single word" do
      assert ScaffoldGenerator.to_joined_lower("package") == "package"
    end

    test "handles multiple hyphens" do
      assert ScaffoldGenerator.to_joined_lower("a-b-c") == "abc"
    end
  end

  describe "dir_name/2" do
    test "ruby uses snake_case" do
      assert ScaffoldGenerator.dir_name("my-package", "ruby") == "my_package"
    end

    test "elixir uses snake_case" do
      assert ScaffoldGenerator.dir_name("my-package", "elixir") == "my_package"
    end

    test "python uses kebab-case" do
      assert ScaffoldGenerator.dir_name("my-package", "python") == "my-package"
    end

    test "go uses kebab-case" do
      assert ScaffoldGenerator.dir_name("my-package", "go") == "my-package"
    end

    test "typescript uses kebab-case" do
      assert ScaffoldGenerator.dir_name("my-package", "typescript") == "my-package"
    end

    test "rust uses kebab-case" do
      assert ScaffoldGenerator.dir_name("my-package", "rust") == "my-package"
    end
  end

  # =========================================================================
  # Validation tests
  # =========================================================================

  describe "valid_kebab_case?/1" do
    test "accepts simple name" do
      assert ScaffoldGenerator.valid_kebab_case?("package")
    end

    test "accepts hyphenated name" do
      assert ScaffoldGenerator.valid_kebab_case?("my-package")
    end

    test "accepts numbers" do
      assert ScaffoldGenerator.valid_kebab_case?("package2")
    end

    test "rejects uppercase" do
      refute ScaffoldGenerator.valid_kebab_case?("MyPackage")
    end

    test "rejects underscores" do
      refute ScaffoldGenerator.valid_kebab_case?("my_package")
    end

    test "rejects leading hyphen" do
      refute ScaffoldGenerator.valid_kebab_case?("-package")
    end

    test "rejects trailing hyphen" do
      refute ScaffoldGenerator.valid_kebab_case?("package-")
    end

    test "rejects empty string" do
      refute ScaffoldGenerator.valid_kebab_case?("")
    end

    test "rejects double hyphens" do
      refute ScaffoldGenerator.valid_kebab_case?("my--package")
    end
  end

  # =========================================================================
  # Dependency reading tests
  # =========================================================================
  #
  # We create temporary directories with realistic metadata files and
  # verify that each language's dependency reader extracts the correct
  # dep names in kebab-case.

  describe "read_deps/2 - Python" do
    test "reads -e ../ entries from BUILD file" do
      tmp = create_tmp_dir()
      build_content = """
      pip install -e ../logic-gates -e ../registers -e .[dev] --quiet
      python -m pytest tests/ -v
      """
      File.write!(Path.join(tmp, "BUILD"), build_content)

      assert {:ok, deps} = ScaffoldGenerator.read_deps(tmp, "python")
      assert "logic-gates" in deps
      assert "registers" in deps
      assert length(deps) == 2
    end

    test "returns empty list when no BUILD file" do
      tmp = create_tmp_dir()
      assert {:ok, []} = ScaffoldGenerator.read_deps(tmp, "python")
    end
  end

  describe "read_deps/2 - Go" do
    test "reads replace directives from go.mod" do
      tmp = create_tmp_dir()
      go_mod = """
      module github.com/example/pkg

      go 1.26

      require (
      \tgithub.com/example/logic-gates v0.0.0
      )

      replace (
      \tgithub.com/example/logic-gates => ../logic-gates
      \tgithub.com/example/registers => ../registers
      )
      """
      File.write!(Path.join(tmp, "go.mod"), go_mod)

      assert {:ok, deps} = ScaffoldGenerator.read_deps(tmp, "go")
      assert "logic-gates" in deps
      assert "registers" in deps
      assert length(deps) == 2
    end

    test "returns empty list when no go.mod" do
      tmp = create_tmp_dir()
      assert {:ok, []} = ScaffoldGenerator.read_deps(tmp, "go")
    end
  end

  describe "read_deps/2 - Ruby" do
    test "reads path entries from Gemfile" do
      tmp = create_tmp_dir()
      gemfile = """
      source "https://rubygems.org"
      gemspec

      gem "coding_adventures_logic_gates", path: "../logic_gates"
      gem "coding_adventures_registers", path: "../registers"
      """
      File.write!(Path.join(tmp, "Gemfile"), gemfile)

      assert {:ok, deps} = ScaffoldGenerator.read_deps(tmp, "ruby")
      assert "logic-gates" in deps
      assert "registers" in deps
      assert length(deps) == 2
    end

    test "returns empty list when no Gemfile" do
      tmp = create_tmp_dir()
      assert {:ok, []} = ScaffoldGenerator.read_deps(tmp, "ruby")
    end
  end

  describe "read_deps/2 - TypeScript" do
    test "reads file:../ entries from package.json" do
      tmp = create_tmp_dir()
      pkg_json = """
      {
        "name": "@coding-adventures/test",
        "dependencies": {
          "@coding-adventures/logic-gates": "file:../logic-gates",
          "@coding-adventures/registers": "file:../registers"
        }
      }
      """
      File.write!(Path.join(tmp, "package.json"), pkg_json)

      assert {:ok, deps} = ScaffoldGenerator.read_deps(tmp, "typescript")
      assert "logic-gates" in deps
      assert "registers" in deps
      assert length(deps) == 2
    end

    test "returns empty list when no package.json" do
      tmp = create_tmp_dir()
      assert {:ok, []} = ScaffoldGenerator.read_deps(tmp, "typescript")
    end
  end

  describe "read_deps/2 - Rust" do
    test "reads path entries from Cargo.toml" do
      tmp = create_tmp_dir()
      cargo = """
      [package]
      name = "test"

      [dependencies]
      logic-gates = { path = "../logic-gates" }
      registers = { path = "../registers" }
      """
      File.write!(Path.join(tmp, "Cargo.toml"), cargo)

      assert {:ok, deps} = ScaffoldGenerator.read_deps(tmp, "rust")
      assert "logic-gates" in deps
      assert "registers" in deps
      assert length(deps) == 2
    end

    test "returns empty list when no Cargo.toml" do
      tmp = create_tmp_dir()
      assert {:ok, []} = ScaffoldGenerator.read_deps(tmp, "rust")
    end
  end

  describe "read_deps/2 - Elixir" do
    test "reads path entries from mix.exs" do
      tmp = create_tmp_dir()
      mix_exs = """
      defmodule Test.MixProject do
        defp deps do
          [
            {:coding_adventures_logic_gates, path: "../logic_gates"},
            {:coding_adventures_registers, path: "../registers"}
          ]
        end
      end
      """
      File.write!(Path.join(tmp, "mix.exs"), mix_exs)

      assert {:ok, deps} = ScaffoldGenerator.read_deps(tmp, "elixir")
      assert "logic-gates" in deps
      assert "registers" in deps
      assert length(deps) == 2
    end

    test "returns empty list when no mix.exs" do
      tmp = create_tmp_dir()
      assert {:ok, []} = ScaffoldGenerator.read_deps(tmp, "elixir")
    end

    test "reads Perl deps from cpanfile" do
      tmp = create_tmp_dir()
      File.write!(Path.join(tmp, "cpanfile"), """
      requires 'coding-adventures-logic-gates';
      requires 'coding-adventures-arithmetic';
      on 'test' => sub {
          requires 'Test2::V0';
      };
      """)

      assert {:ok, deps} = ScaffoldGenerator.read_deps(tmp, "perl")
      assert "logic-gates" in deps
      assert "arithmetic" in deps
      assert length(deps) == 2
    end

    test "returns empty list when no cpanfile" do
      tmp = create_tmp_dir()
      assert {:ok, []} = ScaffoldGenerator.read_deps(tmp, "perl")
    end
  end

  # =========================================================================
  # Transitive closure tests
  # =========================================================================
  #
  # We create a small dependency graph on disk:
  #   A depends on B
  #   B depends on C
  #   C depends on nothing
  #
  # Starting from [A], the transitive closure should be {A, B, C}.

  describe "transitive_closure/3" do
    test "finds all transitive dependencies" do
      tmp = create_tmp_dir()

      # Create package A that depends on B
      pkg_a = Path.join(tmp, "a")
      File.mkdir_p!(pkg_a)
      File.write!(Path.join(pkg_a, "BUILD"), "pip install -e ../b -e .[dev] --quiet\n")

      # Create package B that depends on C
      pkg_b = Path.join(tmp, "b")
      File.mkdir_p!(pkg_b)
      File.write!(Path.join(pkg_b, "BUILD"), "pip install -e ../c -e .[dev] --quiet\n")

      # Create package C with no deps
      pkg_c = Path.join(tmp, "c")
      File.mkdir_p!(pkg_c)
      File.write!(Path.join(pkg_c, "BUILD"), "pip install -e .[dev] --quiet\n")

      assert {:ok, all_deps} = ScaffoldGenerator.transitive_closure(["a"], "python", tmp)
      assert "a" in all_deps
      assert "b" in all_deps
      assert "c" in all_deps
      assert length(all_deps) == 3
    end

    test "returns empty list for no dependencies" do
      tmp = create_tmp_dir()
      assert {:ok, []} = ScaffoldGenerator.transitive_closure([], "python", tmp)
    end

    test "handles diamond dependencies without duplicates" do
      tmp = create_tmp_dir()

      # A -> B, A -> C, B -> D, C -> D
      File.mkdir_p!(Path.join(tmp, "a"))
      File.write!(Path.join([tmp, "a", "BUILD"]), "pip install -e ../b -e ../c -e .[dev] --quiet\n")

      File.mkdir_p!(Path.join(tmp, "b"))
      File.write!(Path.join([tmp, "b", "BUILD"]), "pip install -e ../d -e .[dev] --quiet\n")

      File.mkdir_p!(Path.join(tmp, "c"))
      File.write!(Path.join([tmp, "c", "BUILD"]), "pip install -e ../d -e .[dev] --quiet\n")

      File.mkdir_p!(Path.join(tmp, "d"))
      File.write!(Path.join([tmp, "d", "BUILD"]), "echo done\n")

      assert {:ok, all_deps} = ScaffoldGenerator.transitive_closure(["a"], "python", tmp)
      assert length(all_deps) == 4
      assert Enum.sort(all_deps) == ["a", "b", "c", "d"]
    end
  end

  # =========================================================================
  # Topological sort tests
  # =========================================================================

  describe "topological_sort/3" do
    test "returns leaf-first order" do
      tmp = create_tmp_dir()

      # A depends on B, B depends on C
      File.mkdir_p!(Path.join(tmp, "a"))
      File.write!(Path.join([tmp, "a", "BUILD"]), "pip install -e ../b -e .[dev] --quiet\n")

      File.mkdir_p!(Path.join(tmp, "b"))
      File.write!(Path.join([tmp, "b", "BUILD"]), "pip install -e ../c -e .[dev] --quiet\n")

      File.mkdir_p!(Path.join(tmp, "c"))
      File.write!(Path.join([tmp, "c", "BUILD"]), "echo done\n")

      assert {:ok, sorted} = ScaffoldGenerator.topological_sort(["a", "b", "c"], "python", tmp)
      # C should come before B, B should come before A
      assert_before(sorted, "c", "b")
      assert_before(sorted, "b", "a")
    end

    test "handles independent packages" do
      tmp = create_tmp_dir()

      File.mkdir_p!(Path.join(tmp, "x"))
      File.write!(Path.join([tmp, "x", "BUILD"]), "echo done\n")

      File.mkdir_p!(Path.join(tmp, "y"))
      File.write!(Path.join([tmp, "y", "BUILD"]), "echo done\n")

      assert {:ok, sorted} = ScaffoldGenerator.topological_sort(["x", "y"], "python", tmp)
      assert length(sorted) == 2
      assert "x" in sorted
      assert "y" in sorted
    end

    test "handles single package" do
      tmp = create_tmp_dir()

      File.mkdir_p!(Path.join(tmp, "solo"))
      File.write!(Path.join([tmp, "solo", "BUILD"]), "echo done\n")

      assert {:ok, ["solo"]} = ScaffoldGenerator.topological_sort(["solo"], "python", tmp)
    end
  end

  # =========================================================================
  # File generation tests
  # =========================================================================
  #
  # These tests verify that the scaffold function creates the expected
  # files for each language.

  describe "scaffold/2 - file generation" do
    test "generates Python package structure" do
      {tmp, config} = setup_scaffold_test("test-pkg", "python")

      assert {:ok, _messages} = ScaffoldGenerator.scaffold(config, "python")

      target = Path.join(tmp, "test-pkg")
      assert File.exists?(Path.join(target, "pyproject.toml"))
      assert File.exists?(Path.join(target, "BUILD"))
      assert File.exists?(Path.join(target, "README.md"))
      assert File.exists?(Path.join(target, "CHANGELOG.md"))
      assert File.exists?(Path.join([target, "src", "test_pkg", "__init__.py"]))
      assert File.exists?(Path.join([target, "tests", "test_test_pkg.py"]))
    end

    test "generates Go package structure" do
      {tmp, config} = setup_scaffold_test("test-pkg", "go")

      assert {:ok, _messages} = ScaffoldGenerator.scaffold(config, "go")

      target = Path.join(tmp, "test-pkg")
      assert File.exists?(Path.join(target, "go.mod"))
      assert File.exists?(Path.join(target, "test_pkg.go"))
      assert File.exists?(Path.join(target, "test_pkg_test.go"))
      assert File.exists?(Path.join(target, "BUILD"))
      assert File.exists?(Path.join(target, "README.md"))
    end

    test "generates Ruby package structure" do
      {tmp, config} = setup_scaffold_test("test-pkg", "ruby")

      assert {:ok, _messages} = ScaffoldGenerator.scaffold(config, "ruby")

      target = Path.join(tmp, "test_pkg")
      assert File.exists?(Path.join(target, "coding_adventures_test_pkg.gemspec"))
      assert File.exists?(Path.join(target, "Gemfile"))
      assert File.exists?(Path.join(target, "Rakefile"))
      assert File.exists?(Path.join(target, "BUILD"))
      assert File.exists?(Path.join(target, "README.md"))
      assert File.exists?(Path.join([target, "lib", "coding_adventures_test_pkg.rb"]))
      assert File.exists?(Path.join([target, "lib", "coding_adventures", "test_pkg", "version.rb"]))
    end

    test "generates TypeScript package structure" do
      {tmp, config} = setup_scaffold_test("test-pkg", "typescript")

      assert {:ok, _messages} = ScaffoldGenerator.scaffold(config, "typescript")

      target = Path.join(tmp, "test-pkg")
      assert File.exists?(Path.join(target, "package.json"))
      assert File.exists?(Path.join(target, "tsconfig.json"))
      assert File.exists?(Path.join(target, "vitest.config.ts"))
      assert File.exists?(Path.join(target, "BUILD"))
      assert File.exists?(Path.join([target, "src", "index.ts"]))
      assert File.exists?(Path.join([target, "tests", "test-pkg.test.ts"]))
    end

    test "generates Rust package structure" do
      {tmp, config} = setup_scaffold_test("test-pkg", "rust")

      assert {:ok, _messages} = ScaffoldGenerator.scaffold(config, "rust")

      target = Path.join(tmp, "test-pkg")
      assert File.exists?(Path.join(target, "Cargo.toml"))
      assert File.exists?(Path.join([target, "src", "lib.rs"]))
      assert File.exists?(Path.join(target, "BUILD"))
      assert File.exists?(Path.join(target, "README.md"))
    end

    test "generates Elixir package structure" do
      {tmp, config} = setup_scaffold_test("test-pkg", "elixir")

      assert {:ok, _messages} = ScaffoldGenerator.scaffold(config, "elixir")

      target = Path.join(tmp, "test_pkg")
      assert File.exists?(Path.join(target, "mix.exs"))
      assert File.exists?(Path.join([target, "lib", "coding_adventures", "test_pkg.ex"]))
      assert File.exists?(Path.join([target, "test", "test_pkg_test.exs"]))
      assert File.exists?(Path.join([target, "test", "test_helper.exs"]))
      assert File.exists?(Path.join(target, "BUILD"))
      assert File.exists?(Path.join(target, "README.md"))
    end

    test "generates Perl package structure" do
      {tmp, config} = setup_scaffold_test("test-pkg", "perl")

      assert {:ok, _messages} = ScaffoldGenerator.scaffold(config, "perl")

      target = Path.join(tmp, "test-pkg")
      assert File.exists?(Path.join(target, "Makefile.PL"))
      assert File.exists?(Path.join(target, "cpanfile"))
      assert File.exists?(Path.join([target, "lib", "CodingAdventures", "TestPkg.pm"]))
      assert File.exists?(Path.join([target, "t", "00-load.t"]))
      assert File.exists?(Path.join([target, "t", "01-basic.t"]))
      assert File.exists?(Path.join(target, "BUILD"))
      assert File.exists?(Path.join(target, "README.md"))
    end

    test "dry run does not create files" do
      {tmp, config} = setup_scaffold_test("test-pkg", "python")
      dry_config = %{config | dry_run: true}

      assert {:ok, messages} = ScaffoldGenerator.scaffold(dry_config, "python")
      assert Enum.any?(messages, &String.contains?(&1, "[dry-run]"))
      refute File.exists?(Path.join(tmp, "test-pkg"))
    end

    test "errors when target directory already exists" do
      {tmp, config} = setup_scaffold_test("test-pkg", "python")
      File.mkdir_p!(Path.join(tmp, "test-pkg"))

      assert {:error, msg} = ScaffoldGenerator.scaffold(config, "python")
      assert String.contains?(msg, "already exists")
    end

    test "Python BUILD includes transitive deps in order" do
      {tmp, config} = setup_scaffold_test_with_deps("my-app", "python")

      assert {:ok, _messages} = ScaffoldGenerator.scaffold(config, "python")

      target = Path.join(tmp, "my-app")
      build_content = File.read!(Path.join(target, "BUILD"))
      assert String.contains?(build_content, "-e ../dep-b")
      assert String.contains?(build_content, "-e ../dep-a")
    end

    test "generated Elixir mix.exs contains correct module name" do
      {tmp, config} = setup_scaffold_test("test-pkg", "elixir")

      assert {:ok, _} = ScaffoldGenerator.scaffold(config, "elixir")

      target = Path.join(tmp, "test_pkg")
      mix_content = File.read!(Path.join(target, "mix.exs"))
      assert String.contains?(mix_content, "CodingAdventures.TestPkg.MixProject")
      assert String.contains?(mix_content, "app: :coding_adventures_test_pkg")
    end

    test "generated Elixir BUILD stays portable when package has deps" do
      {tmp, config} = setup_scaffold_test("my-app", "elixir")

      dep_b_dir = Path.join(tmp, "dep_b")
      File.mkdir_p!(dep_b_dir)

      File.write!(
        Path.join(dep_b_dir, "mix.exs"),
        """
        defmodule CodingAdventures.DepB.MixProject do
          use Mix.Project

          def project do
            [
              app: :coding_adventures_dep_b,
              version: "0.1.0",
              elixir: "~> 1.14",
              deps: []
            ]
          end
        end
        """
      )

      dep_a_dir = Path.join(tmp, "dep_a")
      File.mkdir_p!(dep_a_dir)

      File.write!(
        Path.join(dep_a_dir, "mix.exs"),
        """
        defmodule CodingAdventures.DepA.MixProject do
          use Mix.Project

          def project do
            [
              app: :coding_adventures_dep_a,
              version: "0.1.0",
              elixir: "~> 1.14",
              deps: deps()
            ]
          end

          defp deps do
            [
              {:coding_adventures_dep_b, path: "../dep_b"}
            ]
          end
        end
        """
      )

      config = %{config | direct_deps: ["dep-a"], layer: 3, description: "An app with deps"}

      assert {:ok, _messages} = ScaffoldGenerator.scaffold(config, "elixir")

      target = Path.join(tmp, "my_app")
      build_content = File.read!(Path.join(target, "BUILD"))

      assert build_content == "mix deps.get --quiet && mix test --cover\n"
      refute String.contains?(build_content, "cd ../")
      refute String.contains?(build_content, "\\")
    end

    test "generated Go go.mod contains correct module path" do
      {tmp, config} = setup_scaffold_test("test-pkg", "go")

      assert {:ok, _} = ScaffoldGenerator.scaffold(config, "go")

      target = Path.join(tmp, "test-pkg")
      mod_content = File.read!(Path.join(target, "go.mod"))
      assert String.contains?(mod_content, "github.com/adhithyan15/coding-adventures/code/packages/go/test-pkg")
    end
  end

  # =========================================================================
  # CLI argument parsing tests
  # =========================================================================

  describe "CLI.parse_args/1" do
    alias CodingAdventures.ScaffoldGenerator.CLI

    test "parses help flag" do
      assert {:help} = CLI.parse_args(["--help"])
      assert {:help} = CLI.parse_args(["-h"])
    end

    test "parses version flag" do
      assert {:version} = CLI.parse_args(["--version"])
      assert {:version} = CLI.parse_args(["-v"])
    end

    test "errors on missing package name" do
      assert {:error, msg} = CLI.parse_args([])
      assert String.contains?(msg, "missing required argument")
    end

    test "errors on invalid package name" do
      assert {:error, msg} = CLI.parse_args(["Invalid_Name"])
      assert String.contains?(msg, "invalid package name")
    end

    test "errors on unknown flag" do
      assert {:error, msg} = CLI.parse_args(["pkg", "--unknown"])
      assert String.contains?(msg, "unknown option")
    end

    test "errors on invalid type" do
      assert {:error, msg} = CLI.parse_args(["pkg", "--type", "invalid"])
      assert String.contains?(msg, "invalid type")
    end

    test "errors on invalid language" do
      assert {:error, msg} = CLI.parse_args(["pkg", "--language", "java"])
      assert String.contains?(msg, "unknown language")
    end

    test "errors on invalid dependency name" do
      assert {:error, msg} = CLI.parse_args(["pkg", "--depends-on", "Invalid_Dep"])
      assert String.contains?(msg, "invalid dependency name")
    end

    test "parses valid config with all options" do
      result =
        CLI.parse_args([
          "my-package",
          "--type", "library",
          "--language", "python,go",
          "--depends-on", "dep-a,dep-b",
          "--layer", "5",
          "--description", "A test package",
          "--dry-run"
        ])

      assert {:ok, %Config{} = config} = result
      assert config.package_name == "my-package"
      assert config.pkg_type == "library"
      assert config.languages == ["python", "go"]
      assert config.direct_deps == ["dep-a", "dep-b"]
      assert config.layer == 5
      assert config.description == "A test package"
      assert config.dry_run == true
    end

    test "uses defaults for omitted options" do
      {:ok, config} = CLI.parse_args(["my-package"])

      assert config.pkg_type == "library"
      assert config.languages == ScaffoldGenerator.valid_languages()
      assert config.direct_deps == []
      assert config.layer == 0
      assert config.description == ""
      assert config.dry_run == false
    end
  end

  # =========================================================================
  # Dedent helper tests
  # =========================================================================

  describe "dedent/1" do
    test "removes common leading whitespace" do
      input = "    line one\n    line two\n    line three\n"
      assert ScaffoldGenerator.dedent(input) == "line one\nline two\nline three\n"
    end

    test "preserves relative indentation" do
      input = "    line one\n      indented\n    line three\n"
      assert ScaffoldGenerator.dedent(input) == "line one\n  indented\nline three\n"
    end

    test "handles empty lines" do
      input = "    line one\n\n    line three\n"
      assert ScaffoldGenerator.dedent(input) == "line one\n\nline three\n"
    end

    test "handles no indentation" do
      input = "line one\nline two\n"
      assert ScaffoldGenerator.dedent(input) == "line one\nline two\n"
    end
  end

  # =========================================================================
  # Test helpers
  # =========================================================================

  # Creates a temporary directory for test isolation.
  defp create_tmp_dir do
    tmp = Path.join(System.tmp_dir!(), "scaffold_test_#{:rand.uniform(1_000_000)}")
    File.mkdir_p!(tmp)

    # Register cleanup
    on_exit(fn -> File.rm_rf!(tmp) end)

    tmp
  end

  # Sets up a scaffold test environment for a given language.
  # Returns {base_dir, config} where base_dir is the language-specific dir.
  defp setup_scaffold_test(pkg_name, lang) do
    tmp = create_tmp_dir()

    # Create a fake repo root with the expected structure
    repo_root = Path.join(tmp, "repo")
    lang_dir = Path.join([repo_root, "code", "packages", lang])
    File.mkdir_p!(lang_dir)

    # Also create .git so find_repo_root works... but we override repo_root directly
    File.mkdir_p!(Path.join(repo_root, ".git"))

    config = %Config{
      package_name: pkg_name,
      pkg_type: "library",
      languages: [lang],
      direct_deps: [],
      layer: 0,
      description: "A test package",
      dry_run: false,
      repo_root: repo_root
    }

    {lang_dir, config}
  end

  # Sets up a scaffold test with dependencies already on disk.
  # Creates dep-a (depends on dep-b) and dep-b (no deps).
  defp setup_scaffold_test_with_deps(pkg_name, lang) do
    tmp = create_tmp_dir()

    repo_root = Path.join(tmp, "repo")
    lang_dir = Path.join([repo_root, "code", "packages", lang])
    File.mkdir_p!(lang_dir)
    File.mkdir_p!(Path.join(repo_root, ".git"))

    # Create dep-b (leaf - no dependencies)
    dep_b_dir = Path.join(lang_dir, "dep-b")
    File.mkdir_p!(dep_b_dir)
    File.write!(Path.join(dep_b_dir, "BUILD"), "pip install -e .[dev] --quiet\n")

    # Create dep-a (depends on dep-b)
    dep_a_dir = Path.join(lang_dir, "dep-a")
    File.mkdir_p!(dep_a_dir)
    File.write!(Path.join(dep_a_dir, "BUILD"), "pip install -e ../dep-b -e .[dev] --quiet\n")

    config = %Config{
      package_name: pkg_name,
      pkg_type: "library",
      languages: [lang],
      direct_deps: ["dep-a"],
      layer: 3,
      description: "An app with deps",
      dry_run: false,
      repo_root: repo_root
    }

    {lang_dir, config}
  end

  # Asserts that item_a appears before item_b in the list.
  defp assert_before(list, item_a, item_b) do
    idx_a = Enum.find_index(list, &(&1 == item_a))
    idx_b = Enum.find_index(list, &(&1 == item_b))
    assert idx_a != nil, "#{item_a} not found in #{inspect(list)}"
    assert idx_b != nil, "#{item_b} not found in #{inspect(list)}"
    assert idx_a < idx_b, "Expected #{item_a} before #{item_b}, got #{inspect(list)}"
  end
end
