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

  def test_validate_build_contracts_flags_lua_isolated_build_violations
    Dir.mktmpdir("build_tool_validator") do |tmp|
      root = Pathname(tmp)
      package_path = root / "code/packages/lua/problem_pkg"
      package_path.mkpath

      packages = [
        BuildTool::Package.new(
          name: "lua/problem_pkg",
          path: package_path,
          build_commands: ["echo"],
          language: "lua"
        )
      ]

      write_file(package_path / "BUILD", <<~BUILD)
        luarocks remove --force coding-adventures-branch-predictor 2>/dev/null || true
        (cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
        (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
        luarocks make --local coding-adventures-problem-pkg-0.1.0-1.rockspec
      BUILD

      error = BuildTool::Validator.validate_build_contracts(root, packages)

      refute_nil error
      assert_includes error, "coding-adventures-branch-predictor"
      assert_includes error, "state_machine before directed_graph"
    end
  end

  def test_validate_build_contracts_flags_guarded_lua_install_without_deps_mode
    Dir.mktmpdir("build_tool_validator") do |tmp|
      root = Pathname(tmp)
      package_path = root / "code/packages/lua/guarded_pkg"
      package_path.mkpath

      packages = [
        BuildTool::Package.new(
          name: "lua/guarded_pkg",
          path: package_path,
          build_commands: ["echo"],
          language: "lua"
        )
      ]

      write_file(package_path / "BUILD", <<~BUILD)
        luarocks show coding-adventures-transistors >/dev/null 2>&1 || (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
        luarocks make --local coding-adventures-guarded-pkg-0.1.0-1.rockspec
      BUILD

      error = BuildTool::Validator.validate_build_contracts(root, packages)

      refute_nil error
      assert_includes error, "--deps-mode=none or --no-manifest"
    end
  end

  def test_validate_build_contracts_allows_safe_lua_isolated_builds
    Dir.mktmpdir("build_tool_validator") do |tmp|
      root = Pathname(tmp)
      package_path = root / "code/packages/lua/safe_pkg"
      package_path.mkpath

      packages = [
        BuildTool::Package.new(
          name: "lua/safe_pkg",
          path: package_path,
          build_commands: ["echo"],
          language: "lua"
        )
      ]

      write_file(package_path / "BUILD", <<~BUILD)
        luarocks remove --force coding-adventures-safe-pkg 2>/dev/null || true
        luarocks show coding-adventures-directed-graph >/dev/null 2>&1 || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
        luarocks show coding-adventures-state-machine >/dev/null 2>&1 || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
        luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
      BUILD
      write_file(package_path / "BUILD_windows", <<~BUILD)
        luarocks show coding-adventures-directed-graph 1>nul 2>nul || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
        luarocks show coding-adventures-state-machine 1>nul 2>nul || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
        luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
      BUILD

      assert_nil BuildTool::Validator.validate_build_contracts(root, packages)
    end
  end
end
