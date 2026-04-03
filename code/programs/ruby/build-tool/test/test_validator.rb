# frozen_string_literal: true

require_relative "test_helper"

class TestValidator < Minitest::Test
  include TestHelper

  def test_fails_without_normalized_outputs
    Dir.mktmpdir("build_tool_validator") do |tmp|
      root = Pathname(tmp)
      packages = [
        BuildTool::Package.new(
          name: "elixir/actor",
          path: root / "code/packages/elixir/actor",
          build_commands: ["echo"],
          language: "elixir"
        ),
        BuildTool::Package.new(
          name: "python/actor",
          path: root / "code/packages/python/actor",
          build_commands: ["echo"],
          language: "python"
        )
      ]

      write_file(root / ".github/workflows/ci.yml", <<~YAML)
        jobs:
          detect:
            outputs:
              needs_python: ${{ steps.detect.outputs.needs_python }}
              needs_elixir: ${{ steps.detect.outputs.needs_elixir }}
          build:
            steps:
              - name: Full build on main merge
                run: ./build-tool -root . -force -validate-build-files -language all
      YAML

      error = BuildTool::Validator.validate_ci_full_build_toolchains(root, packages)

      refute_nil error
      assert_includes error, ".github/workflows/ci.yml"
      assert_includes error, "python"
      assert_includes error, "elixir"
    end
  end

  def test_allows_normalized_outputs
    Dir.mktmpdir("build_tool_validator") do |tmp|
      root = Pathname(tmp)
      packages = [
        BuildTool::Package.new(
          name: "elixir/actor",
          path: root / "code/packages/elixir/actor",
          build_commands: ["echo"],
          language: "elixir"
        ),
        BuildTool::Package.new(
          name: "python/actor",
          path: root / "code/packages/python/actor",
          build_commands: ["echo"],
          language: "python"
        )
      ]

      write_file(root / ".github/workflows/ci.yml", <<~YAML)
        jobs:
          detect:
            outputs:
              needs_python: ${{ steps.toolchains.outputs.needs_python }}
              needs_elixir: ${{ steps.toolchains.outputs.needs_elixir }}
            steps:
              - name: Normalize toolchain requirements
                id: toolchains
                run: |
                  printf '%s\n' \
                    'needs_python=true' \
                    'needs_elixir=true' >> "$GITHUB_OUTPUT"
          build:
            steps:
              - name: Full build on main merge
                run: ./build-tool -root . -force -validate-build-files -language all
      YAML

      assert_nil BuildTool::Validator.validate_ci_full_build_toolchains(root, packages)
    end
  end
end
