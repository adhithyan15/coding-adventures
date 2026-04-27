defmodule BuildTool.CIWorkflowTest do
  use ExUnit.Case, async: true

  alias BuildTool.CIWorkflow

  test "allows toolchain-scoped dotnet changes" do
    change =
      CIWorkflow.analyze_patch("""
      @@ -312,0 +313,6 @@
      +      - name: Set up .NET
      +        if: needs.detect.outputs.needs_dotnet == 'true'
      +        uses: actions/setup-dotnet@v4
      +        with:
      +          dotnet-version: '9.0.x'
      """)

    refute change.requires_full_rebuild
    assert CIWorkflow.sorted_toolchains(change.toolchains) == ["dotnet"]
  end

  test "allows shared jvm toolchain changes" do
    change =
      CIWorkflow.analyze_patch("""
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
      """)

    refute change.requires_full_rebuild
    assert CIWorkflow.sorted_toolchains(change.toolchains) == ["java", "kotlin"]
  end

  test "ignores comment-only changes" do
    change =
      CIWorkflow.analyze_patch("""
      @@ -316,2 +316,2 @@
      -          # .NET 8 is the current LTS release.
      +          # .NET 9 is the current LTS release.
      """)

    refute change.requires_full_rebuild
    assert MapSet.size(change.toolchains) == 0
  end

  test "requires a full rebuild for build command changes" do
    change =
      CIWorkflow.analyze_patch("""
      @@ -404,1 +404,1 @@
      -          $BT -root . -validate-build-files -language all
      +          $BT -root . -force -validate-build-files -language all
      """)

    assert change.requires_full_rebuild
  end

  test "requires a full rebuild for unknown workflow changes" do
    change =
      CIWorkflow.analyze_patch("""
      @@ -170,0 +171,1 @@
      +      timeout-minutes: 45
      """)

    assert change.requires_full_rebuild
  end
end
