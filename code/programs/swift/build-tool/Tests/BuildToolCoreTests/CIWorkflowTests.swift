import BuildToolCore
import Testing

struct CIWorkflowTests {
    @Test
    func analyzePatchAllowsToolchainScopedDotnetChanges() {
        let change = CIWorkflow.analyzePatch(
            """
            @@ -312,0 +313,6 @@
            +      - name: Set up .NET
            +        if: needs.detect.outputs.needs_dotnet == 'true'
            +        uses: actions/setup-dotnet@v4
            +        with:
            +          dotnet-version: '9.0.x'
            """
        )

        #expect(change.requiresFullRebuild == false)
        #expect(CIWorkflow.sortedToolchains(change.toolchains) == ["dotnet"])
    }

    @Test
    func analyzePatchAllowsSharedJVMToolchainChanges() {
        let change = CIWorkflow.analyzePatch(
            """
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
            """
        )

        #expect(change.requiresFullRebuild == false)
        #expect(CIWorkflow.sortedToolchains(change.toolchains) == ["java", "kotlin"])
    }

    @Test
    func analyzePatchIgnoresCommentOnlyChanges() {
        let change = CIWorkflow.analyzePatch(
            """
            @@ -316,2 +316,2 @@
            -          # .NET 8 is the current LTS release.
            +          # .NET 9 is the current LTS release.
            """
        )

        #expect(change.requiresFullRebuild == false)
        #expect(CIWorkflow.sortedToolchains(change.toolchains).isEmpty)
    }

    @Test
    func analyzePatchRequiresFullRebuildForBuildCommandChanges() {
        let change = CIWorkflow.analyzePatch(
            """
            @@ -404,1 +404,1 @@
            -          $BT -root . -validate-build-files -language all
            +          $BT -root . -force -validate-build-files -language all
            """
        )

        #expect(change.requiresFullRebuild == true)
    }

    @Test
    func toolchainMappingSupportsSharedFamilies() {
        #expect(toolchainForPackageLanguage("wasm") == "rust")
        #expect(toolchainForPackageLanguage("csharp") == "dotnet")
        #expect(toolchainForPackageLanguage("fsharp") == "dotnet")
        #expect(toolchainForPackageLanguage("python") == "python")
    }
}
