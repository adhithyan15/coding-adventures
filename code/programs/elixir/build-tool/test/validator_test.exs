defmodule BuildTool.ValidatorTest do
  use ExUnit.Case, async: true

  alias BuildTool.Validator

  setup do
    tmp_dir = Path.join(System.tmp_dir!(), "build_tool_validator_test_#{:rand.uniform(100_000)}")
    File.rm_rf!(tmp_dir)
    File.mkdir_p!(tmp_dir)

    on_exit(fn -> File.rm_rf!(tmp_dir) end)

    {:ok, tmp_dir: tmp_dir}
  end

  test "fails without normalized outputs", %{tmp_dir: tmp_dir} do
    File.mkdir_p!(Path.join(tmp_dir, ".github/workflows"))

    File.write!(Path.join(tmp_dir, ".github/workflows/ci.yml"), """
    jobs:
      detect:
        outputs:
          needs_python: ${{ steps.detect.outputs.needs_python }}
          needs_elixir: ${{ steps.detect.outputs.needs_elixir }}
      build:
        steps:
          - name: Full build on main merge
            run: ./build-tool -root . -force -validate-build-files -language all
    """)

    packages = [
      %{language: "elixir"},
      %{language: "python"}
    ]

    error = Validator.validate_ci_full_build_toolchains(tmp_dir, packages)

    assert error =~ ".github/workflows/ci.yml"
    assert error =~ "elixir"
    assert error =~ "python"
  end

  test "allows normalized outputs", %{tmp_dir: tmp_dir} do
    File.mkdir_p!(Path.join(tmp_dir, ".github/workflows"))

    File.write!(Path.join(tmp_dir, ".github/workflows/ci.yml"), """
    jobs:
      detect:
        outputs:
          needs_python: ${{ steps.toolchains.outputs.needs_python }}
          needs_elixir: ${{ steps.toolchains.outputs.needs_elixir }}
        steps:
          - name: Normalize toolchain requirements
            id: toolchains
            run: |
              printf '%s\\n' \\
                'needs_python=true' \\
                'needs_elixir=true' >> "$GITHUB_OUTPUT"
      build:
        steps:
          - name: Full build on main merge
            run: ./build-tool -root . -force -validate-build-files -language all
    """)

    packages = [
      %{language: "elixir"},
      %{language: "python"}
    ]

    assert Validator.validate_ci_full_build_toolchains(tmp_dir, packages) == nil
  end

  test "validate_build_contracts flags Lua isolated-build violations", %{tmp_dir: tmp_dir} do
    pkg_path = Path.join(tmp_dir, "code/packages/lua/problem_pkg")
    File.mkdir_p!(pkg_path)

    File.write!(Path.join(pkg_path, "BUILD"), """
    luarocks remove --force coding-adventures-branch-predictor 2>/dev/null || true
    (cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
    (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
    luarocks make --local coding-adventures-problem-pkg-0.1.0-1.rockspec
    """)

    packages = [
      %{language: "lua", path: pkg_path}
    ]

    error = Validator.validate_build_contracts(tmp_dir, packages)

    assert error =~ "coding-adventures-branch-predictor"
    assert error =~ "state_machine before directed_graph"
  end

  test "validate_build_contracts flags guarded Lua installs without deps mode", %{
    tmp_dir: tmp_dir
  } do
    pkg_path = Path.join(tmp_dir, "code/packages/lua/guarded_pkg")
    File.mkdir_p!(pkg_path)

    File.write!(Path.join(pkg_path, "BUILD"), """
    luarocks show coding-adventures-transistors >/dev/null 2>&1 || (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
    luarocks make --local coding-adventures-guarded-pkg-0.1.0-1.rockspec
    """)

    packages = [
      %{language: "lua", path: pkg_path}
    ]

    error = Validator.validate_build_contracts(tmp_dir, packages)

    assert error =~ "--deps-mode=none or --no-manifest"
  end

  test "validate_build_contracts allows safe Lua isolated-build patterns", %{tmp_dir: tmp_dir} do
    pkg_path = Path.join(tmp_dir, "code/packages/lua/safe_pkg")
    File.mkdir_p!(pkg_path)

    File.write!(Path.join(pkg_path, "BUILD"), """
    luarocks remove --force coding-adventures-safe-pkg 2>/dev/null || true
    luarocks show coding-adventures-directed-graph >/dev/null 2>&1 || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
    luarocks show coding-adventures-state-machine >/dev/null 2>&1 || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
    luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
    """)

    File.write!(Path.join(pkg_path, "BUILD_windows"), """
    luarocks show coding-adventures-directed-graph 1>nul 2>nul || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
    luarocks show coding-adventures-state-machine 1>nul 2>nul || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
    luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
    """)

    packages = [
      %{language: "lua", path: pkg_path}
    ]

    assert Validator.validate_build_contracts(tmp_dir, packages) == nil
  end
end
