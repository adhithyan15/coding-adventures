module BuildToolFSharp.Tests.BuildToolTests

open System
open System.Collections.Generic
open System.IO
open System.Text.Json
open BuildToolFSharp.Program
open CodingAdventures.BuildTool.CSharp
open Xunit

let private repositoryRoot =
    let rec tryFindRoot (directory: DirectoryInfo) =
        if isNull directory then
            None
        else
            let marker =
                Path.Combine(
                    directory.FullName,
                    "code",
                    "specs",
                    "fixtures",
                    "build-tool-v1",
                    "pure-domains.schema.json"
                )

            if File.Exists(marker) then
                Some directory.FullName
            else
                tryFindRoot directory.Parent

    [ DirectoryInfo(__SOURCE_DIRECTORY__)
      DirectoryInfo(Directory.GetCurrentDirectory())
      DirectoryInfo(AppContext.BaseDirectory) ]
    |> List.tryPick tryFindRoot
    |> Option.defaultWith (fun () ->
        raise (DirectoryNotFoundException("Could not locate the coding-adventures repository root.")))

let private tempRoot () =
    let path = Path.Combine(Path.GetTempPath(), $"build-tool-fsharp-{Guid.NewGuid():N}")
    Directory.CreateDirectory(path) |> ignore
    path

let private writeFile (root: string) (relativePath: string) (content: string) =
    let fullPath =
        Path.Combine(root, relativePath.Replace('/', Path.DirectorySeparatorChar))

    Directory.CreateDirectory(Path.GetDirectoryName(fullPath)) |> ignore
    File.WriteAllText(fullPath, content)

let private stringsFrom (element: JsonElement) =
    element.EnumerateArray()
    |> Seq.map (fun value -> value.GetString())
    |> Seq.toArray

[<Fact>]
let ``help exits successfully`` () =
    let exitCode = main [| "--help" |]

    Assert.Equal(0, exitCode)

[<Fact>]
let ``force emit-plan writes a schema versioned plan`` () =
    let root = tempRoot ()

    try
        writeFile root "code/packages/fsharp/md5/BUILD" "dotnet --version\n"
        writeFile root "code/packages/fsharp/md5/CodingAdventures.Md5.fsproj" "<Project />\n"

        let exitCode =
            main [| "--root"; root; "--force"; "--emit-plan"; "--plan-file"; "build-plan.json" |]

        Assert.Equal(0, exitCode)

        let planPath = Path.Combine(root, "build-plan.json")
        Assert.True(File.Exists(planPath))

        use document = JsonDocument.Parse(File.ReadAllText(planPath))
        Assert.Equal(PlanFile.CurrentSchemaVersion, document.RootElement.GetProperty("schema_version").GetInt32())
    finally
        if Directory.Exists(root) then
            Directory.Delete(root, true)

[<Fact>]
let ``toolchain detection matches every shared conformance fixture through F sharp`` () =
    let fixtureDirectory =
        Path.Combine(repositoryRoot, "code", "specs", "fixtures", "build-tool-v1", "cases")

    let fixturePaths =
        Directory.GetFiles(fixtureDirectory, "toolchain-detection-*.json")
        |> Array.sortWith (fun left right -> StringComparer.Ordinal.Compare(left, right))

    Assert.Equal(11, fixturePaths.Length)

    for fixturePath in fixturePaths do
        use fixture = JsonDocument.Parse(File.ReadAllText(fixturePath))
        let options = fixture.RootElement.GetProperty("input").GetProperty("options")

        let packages =
            options.GetProperty("packages").EnumerateArray()
            |> Seq.map (fun package ->
                let buildFiles = Dictionary<string, string>(StringComparer.Ordinal)

                for buildFile in package.GetProperty("build_files").EnumerateObject() do
                    buildFiles.Add(buildFile.Name, buildFile.Value.GetString())

                ToolchainPackageSnapshot(
                    package.GetProperty("name").GetString(),
                    package.GetProperty("language").GetString(),
                    buildFiles
                ))
            |> Seq.toArray

        let scheduledElement = options.GetProperty("scheduled_packages")

        let scheduledPackages: IReadOnlyList<string> =
            if scheduledElement.ValueKind = JsonValueKind.Null then
                null
            else
                stringsFrom scheduledElement

        let forcedToolchains: IReadOnlyList<string> =
            stringsFrom (options.GetProperty("forced_toolchains"))

        let actual =
            evaluateToolchainSnapshot
                (options.GetProperty("platform").GetString())
                (options.GetProperty("force_full").GetBoolean())
                (packages :> IReadOnlyList<ToolchainPackageSnapshot>)
                scheduledPackages
                forcedToolchains

        let expected = fixture.RootElement.GetProperty("expected")
        let expectedOutcome = expected.GetProperty("outcome").GetString()
        Assert.Equal(expectedOutcome, actual.Outcome)

        if expectedOutcome = "ok" then
            let expectedToolchains = expected.GetProperty("result").GetProperty("toolchains")
            Assert.Equal(expectedToolchains.EnumerateObject() |> Seq.length, actual.Toolchains.Count)

            for expectedToolchain in expectedToolchains.EnumerateObject() do
                let mutable actualNeeded = false
                Assert.True(actual.Toolchains.TryGetValue(expectedToolchain.Name, &actualNeeded))
                Assert.Equal(expectedToolchain.Value.GetBoolean(), actualNeeded)

            Assert.Empty(actual.Diagnostics)
        else
            Assert.Empty(actual.Toolchains)
            Assert.Empty(expected.GetProperty("result").EnumerateObject())
            let expectedDiagnostic = expected.GetProperty("diagnostics").EnumerateArray() |> Seq.exactlyOne
            let actualDiagnostic = Assert.Single(actual.Diagnostics)
            Assert.Equal(expectedDiagnostic.GetProperty("code").GetString(), actualDiagnostic.Code)
            Assert.Equal(expectedDiagnostic.GetProperty("severity").GetString(), actualDiagnostic.Severity)

            let expectedPackageElement = expectedDiagnostic.GetProperty("package")

            let expectedPackage =
                if expectedPackageElement.ValueKind = JsonValueKind.Null then
                    null
                else
                    expectedPackageElement.GetString()

            Assert.Equal<string>(expectedPackage, actualDiagnostic.Package)

[<Theory>]
[<InlineData("validation-orphan-crates-clean.json")>]
[<InlineData("validation-orphan-crates-unlisted.json")>]
[<InlineData("validation-orphan-exemptions-invalid.json")>]
[<InlineData("validation-orphan-exemptions-stale.json")>]
let ``orphan crate validation matches shared conformance fixtures`` (fixtureName: string) =
    let fixturePath =
        Path.Combine(repositoryRoot, "code", "specs", "fixtures", "build-tool-v1", "cases", fixtureName)

    use fixture = JsonDocument.Parse(File.ReadAllText(fixturePath))

    let snapshot =
        fixture.RootElement.GetProperty("input").GetProperty("options").GetProperty("orphan_snapshot")

    let directories =
        snapshot.GetProperty("directories").EnumerateArray()
        |> Seq.map (fun path -> path.GetString())
        |> Seq.toArray

    let manifests =
        snapshot.GetProperty("manifests").EnumerateArray()
        |> Seq.map (fun manifest ->
            OrphanManifest(manifest.GetProperty("path").GetString(), manifest.GetProperty("kind").GetString()))
        |> Seq.toArray

    let buildFiles =
        snapshot.GetProperty("build_files").EnumerateArray()
        |> Seq.map (fun buildFile ->
            OrphanBuildFile(buildFile.GetProperty("path").GetString(), buildFile.GetProperty("state").GetString()))
        |> Seq.toArray

    let exemptions =
        snapshot.GetProperty("exemptions").EnumerateArray()
        |> Seq.map (fun exemption ->
            OrphanExemption(
                exemption.GetProperty("line").GetInt32(),
                exemption.GetProperty("kind").GetString(),
                exemption.GetProperty("path").GetString(),
                exemption.GetProperty("reason").GetString()
            ))
        |> Seq.toArray

    let actual =
        OrphanCrateSnapshot(
            directories :> IReadOnlyList<string>,
            manifests :> IReadOnlyList<OrphanManifest>,
            buildFiles :> IReadOnlyList<OrphanBuildFile>,
            exemptions :> IReadOnlyList<OrphanExemption>
        )
        |> validateOrphanCrateSnapshot

    let actualDiagnostics = JsonSerializer.SerializeToElement(actual.Diagnostics)
    let expected = fixture.RootElement.GetProperty("expected")
    let expectedDiagnostics = expected.GetProperty("diagnostics")

    let expectedPendingCount =
        expected.GetProperty("result").GetProperty("pending_exemption_count").GetInt32()

    Assert.True(
        JsonElement.DeepEquals(expectedDiagnostics, actualDiagnostics),
        $"Expected {expectedDiagnostics.GetRawText()}, but received {actualDiagnostics.GetRawText()}."
    )

    Assert.Equal(expectedPendingCount, actual.PendingExemptionCount)

[<Theory>]
[<InlineData("validation-tracked-artifacts-clean.json")>]
[<InlineData("validation-tracked-artifacts-forbidden.json")>]
[<InlineData("validation-tracked-artifacts-aliases.json")>]
[<InlineData("validation-tracked-artifacts-invalid.json")>]
[<InlineData("validation-tracked-artifacts-unicode-boundaries.json")>]
let ``tracked artifact validation matches shared conformance fixtures`` (fixtureName: string) =
    let fixturePath =
        Path.Combine(repositoryRoot, "code", "specs", "fixtures", "build-tool-v1", "cases", fixtureName)

    use fixture = JsonDocument.Parse(File.ReadAllText(fixturePath))

    let snapshot =
        fixture.RootElement.GetProperty("input").GetProperty("options").GetProperty("tracked_artifact_snapshot")

    let unicodeVersion = snapshot.GetProperty("unicode_version").GetString()

    let entries =
        snapshot.GetProperty("entries").EnumerateArray()
        |> Seq.map (fun entry ->
            TrackedArtifactEntry(
                entry.GetProperty("ordinal").GetInt32(),
                entry.GetProperty("path").GetString(),
                entry.GetProperty("entry_kind").GetString()
            ))
        |> Seq.toArray

    let actual =
        entries :> IReadOnlyList<TrackedArtifactEntry>
        |> validateTrackedArtifactSnapshot unicodeVersion
        |> JsonSerializer.SerializeToElement

    let expected =
        fixture.RootElement.GetProperty("expected").GetProperty("diagnostics")

    Assert.True(
        JsonElement.DeepEquals(expected, actual),
        $"Expected {expected.GetRawText()}, but received {actual.GetRawText()}."
    )
