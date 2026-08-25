# frozen_string_literal: true

require_relative "test_helper"

class TestValidator < Minitest::Test
  include TestHelper

  TRACKED_ARTIFACT_CASES = %w[
    validation-tracked-artifacts-clean.json
    validation-tracked-artifacts-forbidden.json
    validation-tracked-artifacts-aliases.json
    validation-tracked-artifacts-invalid.json
    validation-tracked-artifacts-unicode-boundaries.json
  ].freeze
  CONFORMANCE_CASES = Pathname(__dir__) / "../../../../specs/fixtures/build-tool-v1/cases"

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

  def test_validate_build_contracts_flags_windows_lua_sibling_drift
    Dir.mktmpdir("build_tool_validator") do |tmp|
      root = Pathname(tmp)
      package_path = root / "code/packages/lua/arm1_gatelevel"
      package_path.mkpath

      packages = [
        BuildTool::Package.new(
          name: "lua/arm1_gatelevel",
          path: package_path,
          build_commands: ["echo"],
          language: "lua"
        )
      ]

      write_file(package_path / "BUILD", <<~BUILD)
        (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
        (cd ../logic_gates && luarocks make --local coding-adventures-logic-gates-0.1.0-1.rockspec)
        (cd ../arithmetic && luarocks make --local coding-adventures-arithmetic-0.1.0-1.rockspec)
        (cd ../arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
        luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
      BUILD
      write_file(package_path / "BUILD_windows", <<~BUILD)
        (cd ..\\arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
        luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
      BUILD

      error = BuildTool::Validator.validate_build_contracts(root, packages)

      refute_nil error
      assert_includes error, "BUILD_windows is missing sibling installs present in BUILD"
      assert_includes error, "../logic_gates"
      assert_includes error, "../arithmetic"
      assert_includes error, "--deps-mode=none or --no-manifest"
    end
  end

  def test_validate_build_contracts_flags_perl_test2_bootstrap_without_notest
    Dir.mktmpdir("build_tool_validator") do |tmp|
      root = Pathname(tmp)
      package_path = root / "code/packages/perl/draw-instructions-svg"
      package_path.mkpath

      packages = [
        BuildTool::Package.new(
          name: "perl/draw-instructions-svg",
          path: package_path,
          build_commands: ["echo"],
          language: "perl"
        )
      ]

      write_file(package_path / "BUILD", <<~BUILD)
        cpanm --quiet Test2::V0
        prove -l -I../draw-instructions/lib -v t/
      BUILD

      error = BuildTool::Validator.validate_build_contracts(root, packages)

      refute_nil error
      assert_includes error, "Test2::V0 without --notest"
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

  TRACKED_ARTIFACT_CASES.each do |fixture_name|
    define_method("test_matches_shared_#{fixture_name.delete_suffix('.json').tr('-', '_')}_fixture") do
      fixture = JSON.parse((CONFORMANCE_CASES / fixture_name).read)
      snapshot = fixture.fetch("input").fetch("options").fetch("tracked_artifact_snapshot")

      actual = BuildTool::Validator.validate_tracked_artifact_snapshot(
        snapshot.fetch("entries"),
        unicode_version: snapshot.fetch("unicode_version")
      )

      assert_equal fixture.fetch("expected").fetch("diagnostics"), actual
    end
  end

  def test_tracked_artifact_rejects_unicode_version_drift_before_entries
    assert_equal "17.0.0", BuildTool::Validator::TRACKED_ARTIFACT_UNICODE_VERSION

    error = assert_raises(ArgumentError) do
      BuildTool::Validator.validate_tracked_artifact_snapshot(
        [{"ordinal" => 1, "path" => "/hostile", "entry_kind" => "regular"}],
        unicode_version: "15.1.0"
      )
    end

    assert_equal "tracked artifact Unicode version must be 17.0.0", error.message
  end

  def test_tracked_artifact_redacts_every_unsafe_path_class
    unsafe_paths = {
      "" => "EMPTY",
      "a" * 513 => "TOO_LONG",
      "code/packages/e\u0301/file.rb" => "NON_NFC",
      "/absolute/file.rb" => "ABSOLUTE",
      "C:\\repo\\file.rb" => "DRIVE_QUALIFIED",
      "code//file.rb" => "EMPTY_SEGMENT",
      "code/trailing/" => "EMPTY_SEGMENT",
      "code\\trailing\\" => "EMPTY_SEGMENT",
      "code/<unsafe>/file.rb" => "UNSAFE_CHARACTER",
      "code/../file.rb" => "DOT_SEGMENT",
      "code/trailing./file.rb" => "TRAILING_DOT_OR_SPACE",
      "code/CON.txt/file.rb" => "RESERVED_BASENAME"
    }

    unsafe_paths.each do |unsafe_path, expected_problem|
      diagnostics = BuildTool::Validator.validate_tracked_artifact_snapshot(
        [{"ordinal" => 7, "path" => unsafe_path, "entry_kind" => "regular"}]
      )

      assert_equal 1, diagnostics.length
      assert_equal "repository", diagnostics[0].fetch("path")
      assert_equal expected_problem, diagnostics[0].fetch("details").fetch("problem")
      refute_includes JSON.generate(diagnostics), unsafe_path unless unsafe_path.empty?
    end
  end

  def test_tracked_artifact_uses_lexical_separators_and_unicode_scalar_lengths
    assert_empty BuildTool::Validator.validate_tracked_artifact_snapshot(
      [{"ordinal" => 1, "path" => "code\\src\\file.rb", "entry_kind" => "regular"}]
    )
    assert_empty BuildTool::Validator.validate_tracked_artifact_snapshot(
      [{"ordinal" => 2, "path" => "😀" * 512, "entry_kind" => "regular"}]
    )

    diagnostics = BuildTool::Validator.validate_tracked_artifact_snapshot(
      [{"ordinal" => 3, "path" => "😀" * 513, "entry_kind" => "regular"}]
    )
    assert_equal "TOO_LONG", diagnostics[0].fetch("details").fetch("problem")
  end

  def test_tracked_artifact_uses_only_pinned_unicode_17_tables
    unicode = BuildTool::TrackedArtifactUnicode17
    todhri_source = [0x105D2, 0x0307].pack("U*")
    todhri_composed = [0x105C9].pack("U")
    assert_equal todhri_composed, unicode.nfc(todhri_source)

    diagnostics = BuildTool::Validator.validate_tracked_artifact_snapshot(
      [{"ordinal" => 1, "path" => todhri_source, "entry_kind" => "regular"}]
    )
    assert_equal "NON_NFC", diagnostics[0].fetch("details").fetch("problem")

    outlined = "NODE_MODULES".codepoints.map do |scalar|
      scalar == 0x5F ? scalar : 0x1CCD6 + scalar - 0x41
    end.pack("U*")
    assert_equal "node_modules", unicode.nfkc_casefold(outlined)
    assert_equal "TRACKED_ARTIFACT_FORBIDDEN",
                 BuildTool::Validator.validate_tracked_artifact_snapshot(
                   [{"ordinal" => 2, "path" => "code/#{outlined}/file.rb", "entry_kind" => "regular"}]
                 )[0].fetch("code")

    assert_equal "CONIN$", unicode.full_uppercase("conın$")
    assert_equal "RESERVED_BASENAME",
                 BuildTool::Validator.validate_tracked_artifact_snapshot(
                   [{"ordinal" => 3, "path" => "code/conın$.txt/file.rb", "entry_kind" => "regular"}]
                 )[0].fetch("details").fetch("problem")

    assert_equal "q\u0300", unicode.nfc("q\u0300")
    assert_empty BuildTool::Validator.validate_tracked_artifact_snapshot(
      [{"ordinal" => 4, "path" => "q\u0300/file.rb", "entry_kind" => "regular"}]
    )
  end

  def test_tracked_artifact_sorts_by_unicode_scalar_value
    private_use = [0xE000].pack("U")
    supplementary = [0x10000].pack("U")
    diagnostics = BuildTool::Validator.validate_tracked_artifact_snapshot(
      [
        {"ordinal" => 1, "path" => "#{supplementary}/node_modules/a", "entry_kind" => "regular"},
        {"ordinal" => 2, "path" => "#{private_use}/node_modules/b", "entry_kind" => "regular"}
      ]
    )

    assert_equal ["#{private_use}/node_modules/b", "#{supplementary}/node_modules/a"],
                 diagnostics.map { |diagnostic| diagnostic.fetch("path") }
  end

  def test_tracked_artifact_entry_kind_is_inert_metadata
    diagnostics = BuildTool::Validator.validate_tracked_artifact_snapshot(
      %w[regular symlink reparse].each_with_index.map do |entry_kind, index|
        {"ordinal" => index + 1, "path" => "node_modules/#{entry_kind}", "entry_kind" => entry_kind}
      end
    )

    assert_equal %w[regular symlink reparse],
                 diagnostics.map { |diagnostic| diagnostic.fetch("details").fetch("entry_kind") }
  end
end
