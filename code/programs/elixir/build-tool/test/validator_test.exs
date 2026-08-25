defmodule BuildTool.ValidatorTest do
  use ExUnit.Case, async: true

  alias BuildTool.Validator

  @conformance_cases Path.expand(
                       "../../../../specs/fixtures/build-tool-v1/cases",
                       __DIR__
                     )
  @tracked_artifact_cases [
    "validation-tracked-artifacts-clean.json",
    "validation-tracked-artifacts-forbidden.json",
    "validation-tracked-artifacts-aliases.json",
    "validation-tracked-artifacts-invalid.json",
    "validation-tracked-artifacts-unicode-boundaries.json"
  ]

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

  test "validate_build_contracts flags Windows Lua sibling drift", %{tmp_dir: tmp_dir} do
    pkg_path = Path.join(tmp_dir, "code/packages/lua/arm1_gatelevel")
    File.mkdir_p!(pkg_path)

    File.write!(Path.join(pkg_path, "BUILD"), """
    (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
    (cd ../logic_gates && luarocks make --local coding-adventures-logic-gates-0.1.0-1.rockspec)
    (cd ../arithmetic && luarocks make --local coding-adventures-arithmetic-0.1.0-1.rockspec)
    (cd ../arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
    luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
    """)

    File.write!(Path.join(pkg_path, "BUILD_windows"), """
    (cd ..\\arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
    luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
    """)

    packages = [
      %{language: "lua", path: pkg_path}
    ]

    error = Validator.validate_build_contracts(tmp_dir, packages)

    assert error =~ "BUILD_windows is missing sibling installs present in BUILD"
    assert error =~ "../logic_gates"
    assert error =~ "../arithmetic"
    assert error =~ "--deps-mode=none or --no-manifest"
  end

  test "validate_build_contracts flags Perl Test2 bootstrap without --notest", %{
    tmp_dir: tmp_dir
  } do
    pkg_path = Path.join(tmp_dir, "code/packages/perl/draw-instructions-svg")
    File.mkdir_p!(pkg_path)

    File.write!(Path.join(pkg_path, "BUILD"), """
    cpanm --quiet Test2::V0
    prove -l -I../draw-instructions/lib -v t/
    """)

    packages = [
      %{language: "perl", path: pkg_path}
    ]

    error = Validator.validate_build_contracts(tmp_dir, packages)

    assert error =~ "Test2::V0 without --notest"
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

  for fixture_name <- @tracked_artifact_cases do
    test "matches shared #{fixture_name} fixture" do
      fixture =
        @conformance_cases
        |> Path.join(unquote(fixture_name))
        |> File.read!()
        |> Jason.decode!()

      snapshot = get_in(fixture, ["input", "options", "tracked_artifact_snapshot"])

      actual =
        Validator.validate_tracked_artifact_snapshot(
          snapshot["entries"],
          snapshot["unicode_version"]
        )

      assert actual == get_in(fixture, ["expected", "diagnostics"])
    end
  end

  test "tracked artifact rejects Unicode version drift before entries" do
    assert Validator.tracked_artifact_unicode_version() == "17.0.0"

    assert_raise ArgumentError,
                 "tracked artifact Unicode version must be 17.0.0",
                 fn ->
                   Validator.validate_tracked_artifact_snapshot(
                     [%{}],
                     "15.1.0"
                   )
                 end
  end

  test "tracked artifact applies invalid-path precedence before redaction" do
    invalid_paths = [
      {String.duplicate("a", 512) <> "e\u0301", "TOO_LONG"},
      {"/e\u0301", "NON_NFC"},
      {"/../file.ex", "ABSOLUTE"},
      {"code//../file.ex", "EMPTY_SEGMENT"},
      {"code/../<unsafe>/file.ex", "UNSAFE_CHARACTER"},
      {"code/CON./file.ex", "TRAILING_DOT_OR_SPACE"}
    ]

    for {path, expected_problem} <- invalid_paths do
      [diagnostic] =
        Validator.validate_tracked_artifact_snapshot([
          %{"ordinal" => 1, "path" => path, "entry_kind" => "regular"}
        ])

      assert diagnostic["path"] == "repository"
      assert diagnostic["details"]["problem"] == expected_problem
    end
  end

  test "tracked artifact redacts every unsafe path class" do
    unsafe_paths = [
      {"", "EMPTY"},
      {String.duplicate("a", 513), "TOO_LONG"},
      {"code/packages/e\u0301/file.ex", "NON_NFC"},
      {"/absolute/file.ex", "ABSOLUTE"},
      {"C:\\repo\\file.ex", "DRIVE_QUALIFIED"},
      {"code//file.ex", "EMPTY_SEGMENT"},
      {"code/trailing/", "EMPTY_SEGMENT"},
      {"code\\trailing\\", "EMPTY_SEGMENT"},
      {"code/<unsafe>/file.ex", "UNSAFE_CHARACTER"},
      {"code/../file.ex", "DOT_SEGMENT"},
      {"code/trailing./file.ex", "TRAILING_DOT_OR_SPACE"},
      {"code/CON.txt/file.ex", "RESERVED_BASENAME"}
    ]

    for {unsafe_path, expected_problem} <- unsafe_paths do
      diagnostics =
        Validator.validate_tracked_artifact_snapshot([
          %{"ordinal" => 7, "path" => unsafe_path, "entry_kind" => "regular"}
        ])

      assert diagnostics == [
               %{
                 "code" => "TRACKED_ARTIFACT_PATH_INVALID",
                 "severity" => "error",
                 "path" => "repository",
                 "details" => %{
                   "ordinal" => 7,
                   "entry_kind" => "regular",
                   "problem" => expected_problem
                 }
               }
             ]
    end
  end

  test "tracked artifact uses lexical separators and Unicode scalar lengths" do
    assert Validator.validate_tracked_artifact_snapshot([
             %{"ordinal" => 1, "path" => "code\\src\\file.ex", "entry_kind" => "regular"}
           ]) == []

    assert Validator.validate_tracked_artifact_snapshot([
             %{
               "ordinal" => 2,
               "path" => String.duplicate("😀", 512),
               "entry_kind" => "regular"
             }
           ]) == []

    diagnostics =
      Validator.validate_tracked_artifact_snapshot([
        %{
          "ordinal" => 3,
          "path" => String.duplicate("😀", 513),
          "entry_kind" => "regular"
        }
      ])

    assert get_in(hd(diagnostics), ["details", "problem"]) == "TOO_LONG"
  end

  test "tracked artifact uses only pinned Unicode 17 tables" do
    unicode = BuildTool.TrackedArtifactUnicode17
    todhri_source = <<0x105D2::utf8, 0x0307::utf8>>
    todhri_composed = <<0x105C9::utf8>>
    assert unicode.nfc(todhri_source) == todhri_composed

    diagnostics =
      Validator.validate_tracked_artifact_snapshot([
        %{"ordinal" => 1, "path" => todhri_source, "entry_kind" => "regular"}
      ])

    assert get_in(hd(diagnostics), ["details", "problem"]) == "NON_NFC"

    outlined =
      "NODE_MODULES"
      |> String.to_charlist()
      |> Enum.map(fn scalar -> if scalar == ?_, do: scalar, else: 0x1CCD6 + scalar - ?A end)
      |> List.to_string()

    assert unicode.nfkc_casefold(outlined) == "node_modules"

    assert hd(
             Validator.validate_tracked_artifact_snapshot([
               %{
                 "ordinal" => 2,
                 "path" => "code/#{outlined}/file.ex",
                 "entry_kind" => "regular"
               }
             ])
           )["code"] == "TRACKED_ARTIFACT_FORBIDDEN"

    assert unicode.full_uppercase("conın$") == "CONIN$"

    assert get_in(
             hd(
               Validator.validate_tracked_artifact_snapshot([
                 %{
                   "ordinal" => 3,
                   "path" => "code/conın$.txt/file.ex",
                   "entry_kind" => "regular"
                 }
               ])
             ),
             ["details", "problem"]
           ) == "RESERVED_BASENAME"

    assert unicode.nfc("q\u0300") == "q\u0300"

    assert Validator.validate_tracked_artifact_snapshot([
             %{"ordinal" => 4, "path" => "q\u0300/file.ex", "entry_kind" => "regular"}
           ]) == []
  end

  test "tracked artifact sorts by Unicode scalar value" do
    private_use = <<0xE000::utf8>>
    supplementary = <<0x10000::utf8>>

    diagnostics =
      Validator.validate_tracked_artifact_snapshot([
        %{
          "ordinal" => 1,
          "path" => "#{supplementary}/node_modules/a",
          "entry_kind" => "regular"
        },
        %{
          "ordinal" => 2,
          "path" => "#{private_use}/node_modules/b",
          "entry_kind" => "regular"
        }
      ])

    assert Enum.map(diagnostics, & &1["path"]) == [
             "#{private_use}/node_modules/b",
             "#{supplementary}/node_modules/a"
           ]
  end

  test "tracked artifact uses canonical detail text as the final sort key" do
    diagnostics =
      Validator.validate_tracked_artifact_snapshot([
        %{"ordinal" => 2, "path" => "node_modules/a", "entry_kind" => "regular"},
        %{"ordinal" => 10, "path" => "node_modules/a", "entry_kind" => "regular"}
      ])

    assert Enum.map(diagnostics, &get_in(&1, ["details", "ordinal"])) == [10, 2]
  end

  test "tracked artifact entry kind is inert metadata" do
    diagnostics =
      ["regular", "symlink", "reparse"]
      |> Enum.with_index(1)
      |> Enum.map(fn {entry_kind, ordinal} ->
        %{
          "ordinal" => ordinal,
          "path" => "node_modules/#{<<96 + ordinal>>}",
          "entry_kind" => entry_kind
        }
      end)
      |> Validator.validate_tracked_artifact_snapshot()

    assert Enum.map(diagnostics, &get_in(&1, ["details", "entry_kind"])) == [
             "regular",
             "symlink",
             "reparse"
           ]
  end
end
