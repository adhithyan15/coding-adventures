module BuildToolFSharp.Tests.BuildToolTests

open System
open System.IO
open System.Text.Json
open CodingAdventures.BuildTool.CSharp
open Xunit

let private tempRoot () =
    let path = Path.Combine(Path.GetTempPath(), $"build-tool-fsharp-{Guid.NewGuid():N}")
    Directory.CreateDirectory(path) |> ignore
    path

let private writeFile (root: string) (relativePath: string) (content: string) =
    let fullPath = Path.Combine(root, relativePath.Replace('/', Path.DirectorySeparatorChar))
    Directory.CreateDirectory(Path.GetDirectoryName(fullPath)) |> ignore
    File.WriteAllText(fullPath, content)

[<Fact>]
let ``help exits successfully`` () =
    let exitCode =
        BuildToolApp.RunAsync([| "--help" |])
        |> Async.AwaitTask
        |> Async.RunSynchronously

    Assert.Equal(0, exitCode)

[<Fact>]
let ``force emit-plan writes a schema versioned plan`` () =
    let root = tempRoot()

    try
        writeFile root "code/packages/fsharp/md5/BUILD" "dotnet --version\n"
        writeFile root "code/packages/fsharp/md5/CodingAdventures.Md5.fsproj" "<Project />\n"

        let exitCode =
            BuildToolApp.RunAsync(
                [|
                    "--root"
                    root
                    "--force"
                    "--emit-plan"
                    "--plan-file"
                    "build-plan.json"
                |])
            |> Async.AwaitTask
            |> Async.RunSynchronously

        Assert.Equal(0, exitCode)

        let planPath = Path.Combine(root, "build-plan.json")
        Assert.True(File.Exists(planPath))

        use document = JsonDocument.Parse(File.ReadAllText(planPath))
        Assert.Equal(PlanFile.CurrentSchemaVersion, document.RootElement.GetProperty("schema_version").GetInt32())
    finally
        if Directory.Exists(root) then
            Directory.Delete(root, true)
