# frozen_string_literal: true

require_relative "test_helper"

class TestCIWorkflow < Minitest::Test
  def test_analyze_patch_allows_toolchain_scoped_dotnet_changes
    change = BuildTool::CIWorkflow.analyze_patch(<<~PATCH)
      @@ -312,0 +313,6 @@
      +      - name: Set up .NET
      +        if: needs.detect.outputs.needs_dotnet == 'true'
      +        uses: actions/setup-dotnet@v4
      +        with:
      +          dotnet-version: '9.0.x'
    PATCH

    refute change.requires_full_rebuild
    assert_equal ["dotnet"], BuildTool::CIWorkflow.sorted_toolchains(change.toolchains)
  end

  def test_analyze_patch_allows_shared_jvm_toolchain_changes
    change = BuildTool::CIWorkflow.analyze_patch(<<~PATCH)
      @@ -314,0 +315,17 @@
      +      - name: Set up JDK 21
      +        if: needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true'
      +        uses: actions/setup-java@v4
      +        with:
      +          distribution: 'temurin'
      +          java-version: '21'
      +      - name: Set up Gradle
      +        if: needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true'
      +        uses: gradle/actions/setup-gradle@v4
      +      - name: Disable long-lived Gradle services on Windows CI
      +        if: (needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true') && runner.os == 'Windows'
      +        shell: bash
      +        run: |
      +          {
      +            echo 'GRADLE_OPTS=-Dorg.gradle.daemon=false -Dorg.gradle.vfs.watch=false'
      +          } >> "$GITHUB_ENV"
    PATCH

    refute change.requires_full_rebuild
    assert_equal ["java", "kotlin"], BuildTool::CIWorkflow.sorted_toolchains(change.toolchains)
  end

  def test_analyze_patch_ignores_comment_only_changes
    change = BuildTool::CIWorkflow.analyze_patch(<<~PATCH)
      @@ -316,2 +316,2 @@
      -          # .NET 8 is the current LTS release.
      +          # .NET 9 is the current LTS release.
    PATCH

    refute change.requires_full_rebuild
    assert_empty change.toolchains.to_a
  end

  def test_analyze_patch_requires_full_rebuild_for_build_command_changes
    change = BuildTool::CIWorkflow.analyze_patch(<<~PATCH)
      @@ -404,1 +404,1 @@
      -          $BT -root . -validate-build-files -language all
      +          $BT -root . -force -validate-build-files -language all
    PATCH

    assert change.requires_full_rebuild
  end

  def test_compute_languages_needed_includes_safe_ci_toolchains
    packages = [
      BuildTool::Package.new(
        name: "python/logic-gates",
        path: Pathname("/repo/code/packages/python/logic-gates"),
        build_commands: [],
        language: "python"
      )
    ]

    needed = BuildTool::CIWorkflow.compute_languages_needed(
      packages,
      { "python/logic-gates" => true },
      false,
      Set.new(["dotnet"])
    )

    assert needed["go"]
    assert needed["python"]
    assert needed["dotnet"]
    refute needed["rust"]
  end

  def test_compute_languages_needed_enables_everything_for_force
    needed = BuildTool::CIWorkflow.compute_languages_needed([], nil, true)

    BuildTool::CIWorkflow::ALL_TOOLCHAINS.each do |toolchain|
      assert needed[toolchain], "expected #{toolchain} to be enabled"
    end
  end

  def test_toolchain_for_package_language_maps_non_native_languages
    assert_equal "rust", BuildTool::CIWorkflow.toolchain_for_package_language("wasm")
    assert_equal "dotnet", BuildTool::CIWorkflow.toolchain_for_package_language("csharp")
    assert_equal "python", BuildTool::CIWorkflow.toolchain_for_package_language("python")
  end
end
