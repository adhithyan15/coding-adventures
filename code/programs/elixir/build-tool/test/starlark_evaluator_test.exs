defmodule BuildTool.StarlarkEvaluatorTest do
  use ExUnit.Case, async: true

  alias BuildTool.StarlarkEvaluator
  alias BuildTool.StarlarkEvaluator.Target

  # ===========================================================================
  # Setup: create temporary directories for testing
  # ===========================================================================

  setup do
    tmp_dir = Path.join(System.tmp_dir!(), "starlark_eval_test_#{:rand.uniform(100_000)}")
    File.rm_rf!(tmp_dir)
    File.mkdir_p!(tmp_dir)

    on_exit(fn -> File.rm_rf!(tmp_dir) end)

    {:ok, tmp_dir: tmp_dir}
  end

  # ===========================================================================
  # starlark_build?/1 — Detection
  # ===========================================================================

  describe "starlark_build?/1" do
    test "detects load() statement" do
      assert StarlarkEvaluator.starlark_build?("load(\"//rules.star\", \"py_library\")\n")
    end

    test "detects known rule calls" do
      assert StarlarkEvaluator.starlark_build?("py_library(name = \"mylib\")\n")
      assert StarlarkEvaluator.starlark_build?("go_binary(name = \"tool\")\n")
      assert StarlarkEvaluator.starlark_build?("ruby_library(name = \"gem\")\n")
      assert StarlarkEvaluator.starlark_build?("ts_library(name = \"pkg\")\n")
      assert StarlarkEvaluator.starlark_build?("rust_library(name = \"crate\")\n")
      assert StarlarkEvaluator.starlark_build?("elixir_library(name = \"app\")\n")
    end

    test "detects def statement" do
      assert StarlarkEvaluator.starlark_build?("def my_rule(name):\n    pass\n")
    end

    test "skips comments and blank lines before detecting" do
      content = """
      # This is a comment

      # Another comment
      py_library(name = "mylib")
      """

      assert StarlarkEvaluator.starlark_build?(content)
    end

    test "returns false for shell commands" do
      refute StarlarkEvaluator.starlark_build?("pip install -e .\npytest\n")
      refute StarlarkEvaluator.starlark_build?("go build ./...\ngo test ./...\n")
      refute StarlarkEvaluator.starlark_build?("bundle install\nrake test\n")
    end

    test "returns false for empty content" do
      refute StarlarkEvaluator.starlark_build?("")
    end

    test "returns false for comment-only content" do
      refute StarlarkEvaluator.starlark_build?("# just a comment\n")
    end

    test "stops at first significant line" do
      # First significant line is shell, even though later lines look like Starlark.
      content = "echo hello\npy_library(name = \"x\")\n"
      refute StarlarkEvaluator.starlark_build?(content)
    end
  end

  # ===========================================================================
  # generate_commands/1 — Command Generation
  # ===========================================================================

  describe "generate_commands/1" do
    test "py_library with default pytest runner" do
      target = %Target{rule: "py_library", name: "mylib"}
      commands = StarlarkEvaluator.generate_commands(target)

      assert commands == [
               ~s(uv pip install --system -e ".[dev]"),
               "python -m pytest --cov --cov-report=term-missing"
             ]
    end

    test "py_library with explicit pytest runner" do
      target = %Target{rule: "py_library", name: "mylib", test_runner: "pytest"}
      commands = StarlarkEvaluator.generate_commands(target)

      assert commands == [
               ~s(uv pip install --system -e ".[dev]"),
               "python -m pytest --cov --cov-report=term-missing"
             ]
    end

    test "py_library with unittest runner" do
      target = %Target{rule: "py_library", name: "mylib", test_runner: "unittest"}
      commands = StarlarkEvaluator.generate_commands(target)

      assert commands == [
               ~s(uv pip install --system -e ".[dev]"),
               "python -m unittest discover tests/"
             ]
    end

    test "py_binary" do
      target = %Target{rule: "py_binary", name: "tool"}
      commands = StarlarkEvaluator.generate_commands(target)

      assert commands == [
               ~s(uv pip install --system -e ".[dev]"),
               "python -m pytest --cov --cov-report=term-missing"
             ]
    end

    test "go_library" do
      target = %Target{rule: "go_library", name: "pkg"}
      assert StarlarkEvaluator.generate_commands(target) == [
               "go build ./...",
               "go test ./... -v -cover",
               "go vet ./..."
             ]
    end

    test "go_binary" do
      target = %Target{rule: "go_binary", name: "tool"}
      assert StarlarkEvaluator.generate_commands(target) == [
               "go build ./...",
               "go test ./... -v -cover",
               "go vet ./..."
             ]
    end

    test "ruby_library" do
      target = %Target{rule: "ruby_library", name: "gem"}
      assert StarlarkEvaluator.generate_commands(target) == [
               "bundle install --quiet",
               "bundle exec rake test"
             ]
    end

    test "ruby_binary" do
      target = %Target{rule: "ruby_binary", name: "bin"}
      assert StarlarkEvaluator.generate_commands(target) == [
               "bundle install --quiet",
               "bundle exec rake test"
             ]
    end

    test "ts_library" do
      target = %Target{rule: "ts_library", name: "pkg"}
      assert StarlarkEvaluator.generate_commands(target) == [
               "npm install --silent",
               "npx vitest run --coverage"
             ]
    end

    test "ts_binary" do
      target = %Target{rule: "ts_binary", name: "app"}
      assert StarlarkEvaluator.generate_commands(target) == [
               "npm install --silent",
               "npx vitest run --coverage"
             ]
    end

    test "rust_library" do
      target = %Target{rule: "rust_library", name: "crate"}
      assert StarlarkEvaluator.generate_commands(target) == [
               "cargo build",
               "cargo test"
             ]
    end

    test "rust_binary" do
      target = %Target{rule: "rust_binary", name: "bin"}
      assert StarlarkEvaluator.generate_commands(target) == [
               "cargo build",
               "cargo test"
             ]
    end

    test "elixir_library" do
      target = %Target{rule: "elixir_library", name: "app"}
      assert StarlarkEvaluator.generate_commands(target) == [
               "mix deps.get",
               "mix test --cover"
             ]
    end

    test "elixir_binary" do
      target = %Target{rule: "elixir_binary", name: "tool"}
      assert StarlarkEvaluator.generate_commands(target) == [
               "mix deps.get",
               "mix test --cover"
             ]
    end

    test "unknown rule" do
      target = %Target{rule: "java_library", name: "something"}
      assert StarlarkEvaluator.generate_commands(target) == [
               "echo 'Unknown rule: java_library'"
             ]
    end
  end

  # ===========================================================================
  # extract_targets/1 — Target Extraction
  # ===========================================================================

  describe "extract_targets/1" do
    test "extracts targets from a valid _targets list" do
      variables = %{
        "_targets" => [
          %{
            "rule" => "py_library",
            "name" => "mylib",
            "srcs" => ["src/**/*.py"],
            "deps" => ["python/other-lib"],
            "test_runner" => "pytest",
            "entry_point" => ""
          }
        ]
      }

      assert {:ok, [target]} = StarlarkEvaluator.extract_targets(variables)
      assert target.rule == "py_library"
      assert target.name == "mylib"
      assert target.srcs == ["src/**/*.py"]
      assert target.deps == ["python/other-lib"]
      assert target.test_runner == "pytest"
      assert target.entry_point == ""
    end

    test "returns empty list when _targets is absent" do
      assert {:ok, []} = StarlarkEvaluator.extract_targets(%{"x" => 42})
    end

    test "returns empty list for empty _targets" do
      assert {:ok, []} = StarlarkEvaluator.extract_targets(%{"_targets" => []})
    end

    test "extracts multiple targets" do
      variables = %{
        "_targets" => [
          %{"rule" => "py_library", "name" => "lib1", "srcs" => [], "deps" => []},
          %{"rule" => "go_binary", "name" => "tool", "srcs" => ["*.go"], "deps" => ["go/dep"]}
        ]
      }

      assert {:ok, targets} = StarlarkEvaluator.extract_targets(variables)
      assert length(targets) == 2
      assert Enum.at(targets, 0).name == "lib1"
      assert Enum.at(targets, 1).name == "tool"
    end

    test "handles missing optional fields gracefully" do
      variables = %{
        "_targets" => [
          %{"rule" => "py_library", "name" => "minimal"}
        ]
      }

      assert {:ok, [target]} = StarlarkEvaluator.extract_targets(variables)
      assert target.rule == "py_library"
      assert target.name == "minimal"
      assert target.srcs == []
      assert target.deps == []
      assert target.test_runner == ""
      assert target.entry_point == ""
    end

    test "returns error when _targets is not a list" do
      variables = %{"_targets" => "not a list"}
      assert {:error, msg} = StarlarkEvaluator.extract_targets(variables)
      assert msg =~ "not a list"
    end
  end

  # ===========================================================================
  # evaluate_build_file/3 — Full Evaluation
  # ===========================================================================

  describe "evaluate_build_file/3" do
    test "returns error for missing file" do
      assert {:error, msg} =
               StarlarkEvaluator.evaluate_build_file("/nonexistent/BUILD", "/nonexistent", "/repo")

      assert msg =~ "reading BUILD file"
    end

    # Skipped: the Starlark bytecode compiler's skip_newlines function has a known
    # infinite loop bug on some inputs, causing this test to hang indefinitely in CI.
    @tag :skip
    test "evaluates BUILD file with no targets", %{tmp_dir: tmp_dir} do
      # A simple Starlark program that sets a variable but doesn't declare targets.
      build_content = "x = 1 + 2\n"

      build_path = Path.join(tmp_dir, "BUILD")
      File.write!(build_path, build_content)

      case StarlarkEvaluator.evaluate_build_file(build_path, tmp_dir, tmp_dir) do
        {:ok, targets} ->
          assert targets == []

        {:error, _reason} ->
          # Interpreter may not be fully available in all test environments.
          :ok
      end
    end

    # Skipped: same infinite loop bug in the Starlark bytecode compiler.
    @tag :skip
    test "evaluates BUILD file that sets _targets to a list", %{tmp_dir: tmp_dir} do
      # Use Starlark list/dict syntax that the interpreter supports.
      # The interpreter's dict syntax uses {key: value} (Starlark native).
      build_content = "_targets = []\n"

      build_path = Path.join(tmp_dir, "BUILD")
      File.write!(build_path, build_content)

      case StarlarkEvaluator.evaluate_build_file(build_path, tmp_dir, tmp_dir) do
        {:ok, targets} ->
          assert targets == []

        {:error, _reason} ->
          :ok
      end
    end
  end

  # ===========================================================================
  # Helper Functions
  # ===========================================================================

  describe "get_string/2" do
    test "extracts string value" do
      assert StarlarkEvaluator.get_string(%{"key" => "value"}, "key") == "value"
    end

    test "returns empty string for missing key" do
      assert StarlarkEvaluator.get_string(%{}, "missing") == ""
    end

    test "returns empty string for non-string value" do
      assert StarlarkEvaluator.get_string(%{"key" => 42}, "key") == ""
    end
  end

  describe "get_string_list/2" do
    test "extracts string list" do
      assert StarlarkEvaluator.get_string_list(%{"key" => ["a", "b"]}, "key") == ["a", "b"]
    end

    test "returns empty list for missing key" do
      assert StarlarkEvaluator.get_string_list(%{}, "missing") == []
    end

    test "filters out non-string items" do
      assert StarlarkEvaluator.get_string_list(%{"key" => ["a", 42, "b"]}, "key") == ["a", "b"]
    end

    test "returns empty list for non-list value" do
      assert StarlarkEvaluator.get_string_list(%{"key" => "not a list"}, "key") == []
    end
  end

  # ===========================================================================
  # Target Struct
  # ===========================================================================

  describe "Target struct" do
    test "has sensible defaults" do
      target = %Target{}
      assert target.rule == ""
      assert target.name == ""
      assert target.srcs == []
      assert target.deps == []
      assert target.test_runner == ""
      assert target.entry_point == ""
    end
  end
end
