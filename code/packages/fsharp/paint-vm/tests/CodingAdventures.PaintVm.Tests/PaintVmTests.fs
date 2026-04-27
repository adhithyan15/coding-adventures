namespace CodingAdventures.PaintVm.Tests

open System
open System.Collections.Generic
open CodingAdventures.PaintInstructions
open CodingAdventures.PaintVm
open CodingAdventures.PixelContainer
open Xunit

type PaintVmTests() =
    let createVm () =
        let vm = PaintVM<List<string>>(fun context background _ _ -> context.Add($"clear:{background}"))

        vm.Register(
            "rect",
            fun instruction context _ ->
                match instruction with
                | Rect rect -> context.Add($"rect:{rect.X},{rect.Y}")
                | _ -> ())

        vm.Register(
            "ellipse",
            fun instruction context _ ->
                match instruction with
                | Ellipse ellipse -> context.Add($"ellipse:{ellipse.Cx},{ellipse.Cy}")
                | _ -> ())

        vm.Register(
            "group",
            fun instruction context runtime ->
                match instruction with
                | Group group ->
                    context.Add("group:start")
                    group.Children |> List.iter (fun child -> runtime.Dispatch(child, context))
                    context.Add("group:end")
                | _ -> ())

        vm

    [<Fact>]
    member _.``Version is semver``() =
        Assert.Equal("0.1.0", PaintVmPackage.VERSION)

    [<Fact>]
    member _.``Register rejects duplicate kinds``() =
        let vm = PaintVM<List<string>>(fun _ _ _ _ -> ())
        vm.Register("rect", fun _ _ _ -> ())
        let error = Assert.Throws<DuplicateHandlerError>(fun () -> vm.Register("rect", fun _ _ _ -> ()))
        Assert.Equal("rect", error.Kind)

    [<Fact>]
    member _.``Dispatch uses specific handler before wildcard``() =
        let log = List<string>()
        let vm = PaintVM<List<string>>(fun _ _ _ _ -> ())
        vm.Register("*", fun instruction context _ -> context.Add($"wildcard:{instruction.Kind}"))
        vm.Register("rect", fun _ context _ -> context.Add("specific:rect"))
        vm.Dispatch(PaintInstructions.paintRect 0 0 10 10, log)
        Assert.Equal<string>([| "specific:rect" |], log |> Seq.toArray)

    [<Fact>]
    member _.``Dispatch throws for unknown instruction kind``() =
        let vm = PaintVM<List<string>>(fun _ _ _ _ -> ())
        let error = Assert.Throws<UnknownInstructionError>(fun () -> vm.Dispatch(PaintInstructions.paintRect 0 0 10 10, List<string>()))
        Assert.Equal("rect", error.Kind)

    [<Fact>]
    member _.``Execute clears then dispatches in order``() =
        let log = List<string>()
        let vm = createVm ()
        let scene =
            PaintInstructions.paintScene
                100
                50
                "#f0f0f0"
                [ PaintInstructions.paintRect 0 0 10 10; PaintInstructions.paintEllipse 20 20 5 5 ]

        vm.Execute(scene, log)

        Assert.Equal<string>([| "clear:#f0f0f0"; "rect:0,0"; "ellipse:20,20" |], log |> Seq.toArray)

    [<Fact>]
    member _.``Execute throws when context is null``() =
        let vm = createVm ()
        let scene = PaintInstructions.paintScene 1 1 "#fff" []
        Assert.Throws<NullContextError>(fun () -> vm.Execute(scene, null)) |> ignore

    [<Fact>]
    member _.``Patch without callbacks falls back to execute``() =
        let log = List<string>()
        let vm = createVm ()
        let oldScene = PaintInstructions.paintScene 100 50 "#fff" [ PaintInstructions.paintRect 0 0 10 10 ]
        let newScene = PaintInstructions.paintScene 100 50 "#fff" [ PaintInstructions.paintRect 0 0 20 20 ]
        vm.Patch(oldScene, newScene, log)
        Assert.Equal<string>([| "clear:#fff"; "rect:0,0" |], log |> Seq.toArray)

    [<Fact>]
    member _.``Patch reports delete and update operations``() =
        let vm = createVm ()
        let deletions = List<string>()
        let updates = List<string>()

        let keepOptions = { PaintInstructions.defaultPaintRectOptions with Id = Some "keep" }
        let deleteOptions = { PaintInstructions.defaultPaintRectOptions with Id = Some "delete-me" }

        let oldScene =
            PaintInstructions.paintScene
                100
                50
                "#fff"
                [ PaintInstructions.paintRectWith keepOptions 0 0 10 10
                  PaintInstructions.paintRectWith deleteOptions 20 0 10 10 ]

        let newScene =
            PaintInstructions.paintScene
                100
                50
                "#fff"
                [ PaintInstructions.paintRectWith keepOptions 0 0 12 12
                  PaintInstructions.paintEllipse 50 50 5 5 ]

        vm.Patch(
            oldScene,
            newScene,
            List<string>(),
            { OnDelete = Some(fun instruction -> deletions.Add(instruction.Id.Value))
              OnInsert = None
              OnUpdate = Some(fun oldInstruction newInstruction -> updates.Add($"{oldInstruction.Kind}->{newInstruction.Kind}")) })

        Assert.Equal<string>([| "delete-me" |], deletions |> Seq.toArray)
        Assert.Equal<string>([| "rect->rect"; "rect->ellipse" |], updates |> Seq.toArray)

    [<Fact>]
    member _.``Export throws when backend does not provide readback``() =
        let vm = createVm ()
        Assert.Throws<ExportNotSupportedError>(Action(fun () -> vm.Export(PaintInstructions.paintScene 10 10 "#fff" []) |> ignore))
        |> ignore

    [<Fact>]
    member _.``Export uses provided offscreen renderer``() =
        let vm =
            PaintVM<List<string>>(
                (fun _ _ _ _ -> ()),
                (fun scene _ options ->
                    let pixels = PixelContainer(int (scene.Width * options.Scale), int (scene.Height * options.Scale))
                    pixels.Fill(255uy, 0uy, 0uy, 255uy)
                    pixels))

        let pixels = vm.Export(PaintInstructions.paintScene 10 5 "#fff" [], { Scale = 2.0; Channels = 4; BitDepth = 8; ColorSpace = "display-p3" })

        Assert.Equal(20, pixels.Width)
        Assert.Equal(10, pixels.Height)
        Assert.Equal({ R = 255uy; G = 0uy; B = 0uy; A = 255uy }, pixels.GetPixel(0, 0))

    [<Fact>]
    member _.``RegisteredKinds returns sorted handler keys``() =
        let vm = PaintVM<List<string>>(fun _ _ _ _ -> ())
        vm.Register("group", fun _ _ _ -> ())
        vm.Register("*", fun _ _ _ -> ())
        vm.Register("rect", fun _ _ _ -> ())

        Assert.Equal<string>([| "*"; "group"; "rect" |], vm.RegisteredKinds())

    [<Fact>]
    member _.``Patch reports insert operations for appended instructions``() =
        let vm = createVm ()
        let insertions = List<string>()
        let oldScene = PaintInstructions.paintScene 100 50 "#fff" [ PaintInstructions.paintRect 0 0 10 10 ]
        let newScene =
            PaintInstructions.paintScene
                100
                50
                "#fff"
                [ PaintInstructions.paintRect 0 0 10 10
                  PaintInstructions.paintEllipse 50 50 5 5
                  PaintInstructions.paintRect 75 0 10 10 ]

        vm.Patch(
            oldScene,
            newScene,
            List<string>(),
            { OnDelete = None
              OnInsert = Some(fun instruction index -> insertions.Add($"{index}:{instruction.Kind}"))
              OnUpdate = None })

        Assert.Equal<string>([| "1:ellipse"; "2:rect" |], insertions |> Seq.toArray)

    [<Fact>]
    member _.``DeepEqual compares identical unions``() =
        let left = PaintInstructions.paintGroup [ PaintInstructions.paintRect 0 0 10 10 ]
        let right = PaintInstructions.paintGroup [ PaintInstructions.paintRect 0 0 10 10 ]
        let changed = PaintInstructions.paintGroup [ PaintInstructions.paintRect 0 0 20 10 ]

        Assert.True(PaintVM<obj>.DeepEqual(box left, box right))
        Assert.False(PaintVM<obj>.DeepEqual(box left, box changed))
