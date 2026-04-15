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
