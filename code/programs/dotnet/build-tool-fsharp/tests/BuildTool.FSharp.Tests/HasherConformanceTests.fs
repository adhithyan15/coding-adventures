module BuildToolFSharp.Tests.HasherConformanceTests

open System
open System.Collections.Generic
open System.IO
open System.Text
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

let private fixtureDirectory =
    Path.Combine(repositoryRoot, "code", "specs", "fixtures", "build-tool-v1")

let private tryProperty (name: string) (element: JsonElement) =
    let mutable value = Unchecked.defaultof<JsonElement>

    if element.TryGetProperty(name, &value) then
        Some value
    else
        None

let private stringsFrom (element: JsonElement) =
    element.EnumerateArray() |> Seq.map _.GetString() |> Seq.toArray

let private candidateFrom (element: JsonElement) =
    let tracked =
        tryProperty "tracked" element
        |> Option.map _.GetBoolean()
        |> Option.defaultValue false

    let content =
        tryProperty "content_hex" element
        |> Option.map (fun value -> Convert.FromHexString(value.GetString()))
        |> Option.defaultValue Array.empty

    SourceCollectionCandidate(
        element.GetProperty("path").GetString(),
        element.GetProperty("kind").GetString(),
        tracked,
        content
    )

[<Fact>]
let ``F sharp facade consumes every neutral source collection case`` () =
    let fixturePaths =
        Directory.GetFiles(Path.Combine(fixtureDirectory, "cases"), "source-collection-*.json")
        |> Array.sortWith (fun left right -> StringComparer.Ordinal.Compare(left, right))

    Assert.Equal(13, fixturePaths.Length)

    for fixturePath in fixturePaths do
        use fixture = JsonDocument.Parse(File.ReadAllText(fixturePath))
        let options = fixture.RootElement.GetProperty("input").GetProperty("options")
        let mode = options.GetProperty("mode").GetString()

        let digestProperty =
            if mode = "repository_boundary" then
                "boundary_sha256"
            else
                "registry_sha256"

        let declaredSources =
            tryProperty "declared_srcs" options
            |> Option.map stringsFrom
            |> Option.defaultValue Array.empty

        let candidates =
            options.GetProperty("candidates").EnumerateArray()
            |> Seq.map candidateFrom
            |> Seq.toArray

        let request =
            SourceCollectionRequest(
                options.GetProperty("language").GetString(),
                options.GetProperty("package_root").GetString(),
                mode,
                options.GetProperty(digestProperty).GetString(),
                declaredSources :> IReadOnlyList<string>,
                candidates :> IReadOnlyList<SourceCollectionCandidate>
            )

        let actual =
            selectSourceCandidates request
            |> Seq.map (fun file -> $"{file.Path}\u0000{file.Digest}")
            |> Seq.toArray

        let expected =
            fixture.RootElement.GetProperty("expected").GetProperty("result").GetProperty("files").EnumerateArray()
            |> Seq.map (fun file ->
                let path = file.GetProperty("path").GetString()
                let digest = file.GetProperty("digest").GetString()
                $"{path}\u0000{digest}")
            |> Seq.toArray

        Assert.Equal<string array>(expected, actual)

[<Fact>]
let ``F sharp facade proves exact production registry projections`` () =
    let checkedLanguageJson =
        File.ReadAllText(Path.Combine(fixtureDirectory, "language-source-input-registry.json"))

    let checkedBoundaryJson =
        File.ReadAllText(Path.Combine(fixtureDirectory, "repository-source-input-boundary.json"))

    use checkedLanguage = JsonDocument.Parse(checkedLanguageJson)
    use productionLanguage = JsonDocument.Parse(languageSourceInputRegistryJson ())
    use checkedBoundary = JsonDocument.Parse(checkedBoundaryJson)
    use productionBoundary = JsonDocument.Parse(repositorySourceInputBoundaryJson ())

    Assert.True(JsonElement.DeepEquals(checkedLanguage.RootElement, productionLanguage.RootElement))
    Assert.True(JsonElement.DeepEquals(checkedBoundary.RootElement, productionBoundary.RootElement))
    Assert.Equal(languageSourceInputRegistryDigest (), canonicalLanguageSourceInputRegistryDigest checkedLanguageJson)

    Assert.Equal(
        repositorySourceInputBoundaryDigest (),
        canonicalRepositorySourceInputBoundaryDigest checkedBoundaryJson
    )

[<Fact>]
let ``F sharp facade consumes every hashing v1 package digest`` () =
    let fixturePaths =
        Directory.GetFiles(Path.Combine(fixtureDirectory, "cases"), "hashing-cache-*.json")
        |> Array.sortWith (fun left right -> StringComparer.Ordinal.Compare(left, right))

    Assert.Equal(4, fixturePaths.Length)

    for fixturePath in fixturePaths do
        use fixture = JsonDocument.Parse(File.ReadAllText(fixturePath))

        let includePaths =
            fixture.RootElement
                .GetProperty("input")
                .GetProperty("options")
                .GetProperty("include_paths")
                .EnumerateArray()
            |> Seq.map _.GetString()
            |> Set.ofSeq

        let inputs =
            fixture.RootElement.GetProperty("workspace").GetProperty("files").EnumerateArray()
            |> Seq.filter (fun file -> includePaths.Contains(file.GetProperty("path").GetString()))
            |> Seq.map (fun file ->
                let content =
                    tryProperty "content_hex" file
                    |> Option.map (fun value -> Convert.FromHexString(value.GetString()))
                    |> Option.defaultWith (fun () ->
                        Encoding.UTF8.GetBytes(file.GetProperty("content_utf8").GetString()))

                PackageHashInput(file.GetProperty("path").GetString(), content))
            |> Seq.toArray

        let expected =
            fixture.RootElement.GetProperty("expected").GetProperty("result").GetProperty("package_digest").GetString()

        Assert.Equal(expected, hashPackageInputs (inputs :> IReadOnlyList<PackageHashInput>))

[<Fact>]
let ``F sharp facade preserves path frames and exact raw bytes`` () =
    let originalInputs: IReadOnlyList<PackageHashInput> =
        [| PackageHashInput("code/packages/fsharp/demo/src/data.bin", [| 0uy; 255uy; 13uy; 10uy |]) |]

    let renamedInputs: IReadOnlyList<PackageHashInput> =
        [| PackageHashInput("code/packages/fsharp/demo/src/renamed.bin", [| 0uy; 255uy; 13uy; 10uy |]) |]

    let normalizedInputs: IReadOnlyList<PackageHashInput> =
        [| PackageHashInput("code/packages/fsharp/demo/src/data.bin", [| 0uy; 255uy; 10uy |]) |]

    let original = hashPackageInputs originalInputs
    Assert.NotEqual<string>(original, hashPackageInputs renamedInputs)
    Assert.NotEqual<string>(original, hashPackageInputs normalizedInputs)
