module BuildToolFSharp.Tests.CIGateConformanceTests

open System
open System.IO
open System.Text.Json
open BuildToolFSharp.Program
open CodingAdventures.BuildTool.CSharp
open Xunit

let private caseDirectory =
    let rec find (directory: DirectoryInfo) =
        if isNull directory then failwith "build-tool-v1 cases not found"
        else
            let path = Path.Combine(directory.FullName, "code", "specs", "fixtures", "build-tool-v1", "cases")
            if Directory.Exists(path) then path else find directory.Parent
    find (DirectoryInfo(AppContext.BaseDirectory))

let private expectedIds =
    [| "ci-gate-selection/character-classes"; "ci-gate-selection/force"
       "ci-gate-selection/machinery"; "ci-gate-selection/match-work-at-limit"
       "ci-gate-selection/match-work-over-limit"; "ci-gate-selection/null-affected"
       "ci-gate-selection/null-changed-files"; "ci-gate-selection/package-and-path"
       "ci-gate-selection/recursive-glob"; "ci-gate-selection/shared-pattern-at-limit"
       "ci-gate-selection/unrelated-change" |]

let private nullableStrings (value: JsonElement) : string array =
    if value.ValueKind = JsonValueKind.Null then null
    else value.EnumerateArray() |> Seq.map (fun item -> item.GetString()) |> Seq.toArray

[<Fact>]
let ``F sharp facade evaluates every neutral CI gate fixture`` () =
    let paths = Directory.GetFiles(caseDirectory, "ci-gate-selection-*.json")
    let ids =
        paths |> Array.map (fun path ->
            use file = JsonDocument.Parse(File.ReadAllText(path))
            file.RootElement.GetProperty("id").GetString())
        |> Array.sortWith (fun left right -> StringComparer.Ordinal.Compare(left, right))
    Assert.Equal<string>(expectedIds, ids)

    for path in paths do
        use file = JsonDocument.Parse(File.ReadAllText(path))
        let root = file.RootElement
        let options = root.GetProperty("input").GetProperty("options")
        let registry = options.GetProperty("registry")
        let gates =
            registry.GetProperty("gates").EnumerateArray()
            |> Seq.map (fun gate ->
                CIGateDefinition(gate.GetProperty("id").GetString(),
                    gate.GetProperty("scope").GetString(),
                    gate.GetProperty("description").GetString(),
                    nullableStrings (gate.GetProperty("packages")),
                    nullableStrings (gate.GetProperty("paths"))))
            |> Seq.toArray
        let actual =
            evaluateCIGates (CIGateSelectionInput(
                CIGateRegistry(registry.GetProperty("schema_version").GetInt32(), gates),
                nullableStrings (options.GetProperty("affected_packages")),
                nullableStrings (options.GetProperty("changed_files")),
                options.GetProperty("force").GetBoolean()))
        let expected = root.GetProperty("expected")
        Assert.Equal(root.GetProperty("id").GetString(), expected.GetProperty("case_id").GetString())
        let diagnostics = expected.GetProperty("diagnostics").EnumerateArray() |> Seq.toArray
        if expected.GetProperty("outcome").GetString() = "error" then
            Assert.Single(diagnostics) |> ignore
            Assert.Equal(diagnostics.[0].GetProperty("code").GetString(), actual.ErrorCode)
            Assert.Equal("error", diagnostics.[0].GetProperty("severity").GetString())
            Assert.Empty(actual.Gates)
        else
            Assert.Empty(diagnostics)
            Assert.Null(actual.ErrorCode)
            let expectedGates =
                expected.GetProperty("result").GetProperty("gates").EnumerateArray()
                |> Seq.map (fun gate ->
                    CIGateVerdict(gate.GetProperty("id").GetString(),
                        gate.GetProperty("required").GetBoolean(),
                        gate.GetProperty("output_name").GetString()))
                |> Seq.toArray
            Assert.Equal<CIGateVerdict>(expectedGates, actual.Gates)

[<Fact>]
let ``F sharp facade preserves implicit job scope`` () =
    let registry = CIGateRegistry(1,
        [| CIGateDefinition("default-job", "", "implicit job scope", [| "rust/alpha" |], [||]) |])
    let actual = evaluateCIGates (CIGateSelectionInput(registry, [| "rust/alpha" |], [||], false))
    Assert.Null(actual.ErrorCode)
    Assert.Equal<CIGateVerdict>([| CIGateVerdict("default-job", true, "run_default_job") |], actual.Gates)
