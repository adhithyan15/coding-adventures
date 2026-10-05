module BuildToolFSharp.Tests.GraphDiffConformanceTests

open System
open System.IO
open System.Text.Json
open BuildToolFSharp.Program
open CodingAdventures.BuildTool.CSharp
open Xunit

let private cases =
    let rec find (directory: DirectoryInfo) =
        if isNull directory then failwith "build-tool-v1 cases not found"
        else
            let path = Path.Combine(directory.FullName, "code", "specs", "fixtures", "build-tool-v1", "cases")
            if Directory.Exists(path) then path else find directory.Parent
    find (DirectoryInfo(AppContext.BaseDirectory))

let private graphIds =
    [| "graph/canonical-edge-order"; "graph/chain"; "graph/cycle"; "graph/diamond"
       "graph/empty"; "graph/isolated"; "graph/multiple-components"; "graph/partial-cycle-no-output" |]

let private diffIds =
    [| "diff-selection/exact-build-fronts"; "diff-selection/forced-package"
       "diff-selection/known-unmatched-near-build"; "diff-selection/match-work-at-limit"
       "diff-selection/match-work-over-limit"; "diff-selection/package-prefix"
       "diff-selection/repository-boundary-reverse-index"; "diff-selection/shared-input-multiconsumer"
       "diff-selection/strict-glob-character-classes"; "diff-selection/transitive-package-change"
       "diff-selection/unknown-path-all"; "diff-selection/unknown-path-error" |]

let private strings (array: JsonElement) =
    array.EnumerateArray() |> Seq.map (fun value -> value.GetString()) |> Seq.toArray

let private edges (array: JsonElement) =
    array.EnumerateArray()
    |> Seq.map (fun value -> GraphEdge(value.[0].GetString(), value.[1].GetString()))
    |> Seq.toArray

let private property (name: string) (value: JsonElement) =
    let mutable found = Unchecked.defaultof<JsonElement>
    if value.TryGetProperty(name, &found) then Some found else None

let private readId path =
    use document = JsonDocument.Parse(File.ReadAllText(path))
    document.RootElement.GetProperty("id").GetString()

let private assertRoster (prefix: string) (expected: string array) =
    let paths = Directory.GetFiles(cases, prefix + "-*.json")
    let ids = paths |> Array.map readId |> Array.sortWith (fun left right -> StringComparer.Ordinal.Compare(left, right))
    Assert.Equal<string>(expected, ids)
    paths

let private expectedError (expected: JsonElement) =
    expected.GetProperty("diagnostics").EnumerateArray()
    |> Seq.tryHead
    |> Option.map (fun diagnostic -> diagnostic.GetProperty("code").GetString())

[<Fact>]
let ``F sharp facade evaluates every graph fixture`` () =
    for path in assertRoster "graph" graphIds do
        use document = JsonDocument.Parse(File.ReadAllText(path))
        let root = document.RootElement
        let options = root.GetProperty("input").GetProperty("options")
        let actual = evaluateGraph (GraphInput(strings (options.GetProperty("packages")), edges (options.GetProperty("edges"))))
        let expected = root.GetProperty("expected")
        Assert.Equal(expectedError expected |> Option.toObj, actual.ErrorCode)
        if not (isNull actual.ErrorCode) then
            Assert.Empty(actual.Edges)
            Assert.Empty(actual.Levels)
        else
            let result = expected.GetProperty("result")
            Assert.Equal<GraphEdge>(edges (result.GetProperty("edges")), actual.Edges)
            let expectedLevels = result.GetProperty("levels").EnumerateArray() |> Seq.map strings |> Seq.toArray
            Assert.Equal(expectedLevels.Length, actual.Levels.Count)
            for index in 0 .. expectedLevels.Length - 1 do
                Assert.Equal<string>(expectedLevels.[index], actual.Levels.[index])

[<Fact>]
let ``F sharp facade evaluates every diff fixture`` () =
    let boundaryPath = Path.Combine(Directory.GetParent(cases).FullName, "repository-source-input-boundary.json")
    let boundary = JsonSerializer.Deserialize<RepositoryBoundary>(File.ReadAllText(boundaryPath))
    for path in assertRoster "diff-selection" diffIds do
        use document = JsonDocument.Parse(File.ReadAllText(path))
        let root = document.RootElement
        let input = root.GetProperty("input")
        let options = input.GetProperty("options")
        let packages =
            options.GetProperty("packages").EnumerateArray()
            |> Seq.map (fun value ->
                let globs = property "source_globs" value |> Option.map strings |> Option.toObj
                DiffPackage(value.GetProperty("name").GetString(), value.GetProperty("rel_path").GetString(),
                            value.GetProperty("source_mode").GetString(), globs))
            |> Seq.toArray
        let digest = property "boundary_sha256" options |> Option.map _.GetString() |> Option.toObj
        let actual =
            evaluateDiffSelection
                (DiffSelectionInput(packages, edges (options.GetProperty("edges")),
                                    strings (options.GetProperty("forced_packages")),
                                    options.GetProperty("unknown_path_policy").GetString(),
                                    strings (input.GetProperty("changed_paths")), digest,
                                    if isNull digest then null else boundary))
        let expected = root.GetProperty("expected")
        Assert.Equal(expectedError expected |> Option.toObj, actual.ErrorCode)
        if not (isNull actual.ErrorCode) then
            Assert.Empty(actual.ChangedPackages)
            Assert.Empty(actual.AffectedPackages)
            Assert.Empty(actual.PrerequisitePackages)
        else
            let result = expected.GetProperty("result")
            Assert.Equal<string>(strings (result.GetProperty("changed_packages")), actual.ChangedPackages)
            Assert.Equal<string>(strings (result.GetProperty("affected_packages")), actual.AffectedPackages)
            Assert.Equal<string>(strings (result.GetProperty("prerequisite_packages")), actual.PrerequisitePackages)

[<Fact>]
let ``F sharp facade returns empty failure values for invalid graph and diff`` () =
    let graph =
        evaluateGraph (GraphInput([| "a/a"; "b/b" |],
                                  [| GraphEdge("a/a", "b/b"); GraphEdge("b/b", "a/a") |]))
    Assert.Equal("GRAPH_CYCLE", graph.ErrorCode)
    Assert.Empty(graph.Edges)
    Assert.Empty(graph.Levels)
    let diff =
        evaluateDiffSelection
            (DiffSelectionInput([| DiffPackage("a/a", "a", "strict_globs", [| "src/[z-a].fs" |]) |],
                                Array.empty, [| "a/a" |], "error", [| "outside" |], null, null))
    Assert.Equal("DIFF_INVALID_INPUT", diff.ErrorCode)
    Assert.Empty(diff.ChangedPackages)
