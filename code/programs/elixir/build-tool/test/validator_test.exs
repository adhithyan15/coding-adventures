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
end
