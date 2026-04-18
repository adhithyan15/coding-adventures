namespace CodingAdventures.PaintVmAscii

open System
open System.Collections.Generic
open CodingAdventures.PaintInstructions
open CodingAdventures.PaintVm

[<RequireQualifiedAccess>]
module PaintVmAsciiPackage =
    [<Literal>]
    let VERSION = "0.1.0"

type AsciiOptions =
    {
        ScaleX: float
        ScaleY: float
    }

type UnsupportedAsciiFeatureError(message: string) =
    inherit Exception(message)

[<Flags>]
type internal CellFlags =
    | None = 0
    | Up = 1
    | Right = 2
    | Down = 4
    | Left = 8
    | Fill = 16
    | Text = 32

type internal ClipBounds =
    {
        MinCol: int
        MinRow: int
        MaxCol: int
        MaxRow: int
    }

/// A CharBuffer remembers both visible glyphs and directional tags so line
/// crossings can merge into box-drawing junctions instead of clobbering cells.
type CharBuffer(rows: int, cols: int) =
    let rows = max rows 0
    let cols = max cols 0
    let characters = Array.init rows (fun _ -> Array.create cols " ")
    let tags = Array.init rows (fun _ -> Array.create cols CellFlags.None)

    let isInsideClip row col clip =
        row >= clip.MinRow
        && row < clip.MaxRow
        && col >= clip.MinCol
        && col < clip.MaxCol

    let isInsideBuffer row col =
        row >= 0 && row < rows && col >= 0 && col < cols

    let resolveCell (flags: CellFlags) =
        let directions =
            flags
            &&& (CellFlags.Up ||| CellFlags.Right ||| CellFlags.Down ||| CellFlags.Left)

        match directions with
        | value when value = (CellFlags.Left ||| CellFlags.Right) -> "─"
        | value when value = (CellFlags.Up ||| CellFlags.Down) -> "│"
        | value when value = (CellFlags.Down ||| CellFlags.Right) -> "┌"
        | value when value = (CellFlags.Down ||| CellFlags.Left) -> "┐"
        | value when value = (CellFlags.Up ||| CellFlags.Right) -> "└"
        | value when value = (CellFlags.Up ||| CellFlags.Left) -> "┘"
        | value when value = (CellFlags.Left ||| CellFlags.Right ||| CellFlags.Down) -> "┬"
        | value when value = (CellFlags.Left ||| CellFlags.Right ||| CellFlags.Up) -> "┴"
        | value when value = (CellFlags.Up ||| CellFlags.Down ||| CellFlags.Right) -> "├"
        | value when value = (CellFlags.Up ||| CellFlags.Down ||| CellFlags.Left) -> "┤"
        | value when value = (CellFlags.Up ||| CellFlags.Down ||| CellFlags.Left ||| CellFlags.Right) -> "┼"
        | CellFlags.Right -> "─"
        | CellFlags.Left -> "─"
        | CellFlags.Up -> "│"
        | CellFlags.Down -> "│"
        | _ when flags.HasFlag(CellFlags.Fill) -> "█"
        | _ -> "+"

    member _.Rows = rows

    member _.Cols = cols

    member internal _.WriteTag(row: int, col: int, flags: CellFlags, clip: ClipBounds) =
        if isInsideClip row col clip && isInsideBuffer row col then
            let existing = tags[row][col]

            if not (existing.HasFlag(CellFlags.Text)) then
                let merged = existing ||| flags
                tags[row][col] <- merged
                characters[row][col] <- resolveCell merged

    member internal _.WriteChar(row: int, col: int, value: string, clip: ClipBounds) =
        if isInsideClip row col clip && isInsideBuffer row col then
            characters[row][col] <- value
            tags[row][col] <- CellFlags.Text

    override _.ToString() =
        let lines =
            characters
            |> Array.map (fun row -> String.Concat(row).TrimEnd())

        let mutable lastContent = lines.Length - 1

        while lastContent >= 0 && lines[lastContent] = String.Empty do
            lastContent <- lastContent - 1

        if lastContent < 0 then
            String.Empty
        else
            String.Join("\n", lines |> Array.take (lastContent + 1))

type AsciiContext() =
    let clipStack = ResizeArray<ClipBounds>()

    do clipStack.Add({ MinCol = 0; MinRow = 0; MaxCol = 0; MaxRow = 0 })

    member val Buffer = CharBuffer(0, 0) with get, set

    member internal _.ClipStack = clipStack

[<RequireQualifiedAccess>]
module PaintVmAscii =
    let defaultAsciiOptions : AsciiOptions = { ScaleX = 8.0; ScaleY = 16.0 }

    let private fullClip cols rows =
        { MinCol = 0; MinRow = 0; MaxCol = cols; MaxRow = rows }

    let private toCol (x: float) (scaleX: float) =
        int (Math.Round(x / scaleX, MidpointRounding.AwayFromZero))

    let private toRow (y: float) (scaleY: float) =
        int (Math.Round(y / scaleY, MidpointRounding.AwayFromZero))

    let private topClip (context: AsciiContext) =
        context.ClipStack[context.ClipStack.Count - 1]

    let private isSafeTerminalCodePoint codePoint =
        if codePoint < 0x20 then
            false
        elif codePoint >= 0x7f && codePoint <= 0x9f then
            false
        elif codePoint = 0x200e || codePoint = 0x200f || codePoint = 0x061c then
            false
        elif codePoint >= 0x202a && codePoint <= 0x202e then
            false
        else
            codePoint < 0x2066 || codePoint > 0x2069

    let private toSafeTerminalGlyph codePoint =
        try
            if isSafeTerminalCodePoint codePoint then
                Char.ConvertFromUtf32(codePoint)
            else
                "?"
        with :? ArgumentOutOfRangeException ->
            "?"

    let private isIdentityTransform transform =
        match transform with
        | None -> true
        | Some transform ->
            transform.A = 1.0
            && transform.B = 0.0
            && transform.C = 0.0
            && transform.D = 1.0
            && transform.E = 0.0
            && transform.F = 0.0

    let private assertPlainGroup (group: PaintGroup) =
        if not (isIdentityTransform group.Transform) then
            raise (UnsupportedAsciiFeatureError("paint-vm-ascii does not support transformed groups"))

        match group.Opacity with
        | Some opacity when opacity <> 1.0 ->
            raise (UnsupportedAsciiFeatureError("paint-vm-ascii does not support group opacity"))
        | _ -> ()

    let private assertPlainLayer (layer: PaintLayer) =
        if not (isIdentityTransform layer.Transform) then
            raise (UnsupportedAsciiFeatureError("paint-vm-ascii does not support transformed layers"))

        match layer.Opacity with
        | Some opacity when opacity <> 1.0 ->
            raise (UnsupportedAsciiFeatureError("paint-vm-ascii does not support layer opacity"))
        | _ -> ()

        match layer.Filters with
        | Some filters when filters.Length > 0 ->
            raise (UnsupportedAsciiFeatureError("paint-vm-ascii does not support layer filters"))
        | _ -> ()

        match layer.BlendMode with
        | Some blendMode when blendMode <> BlendMode.Normal ->
            raise (UnsupportedAsciiFeatureError("paint-vm-ascii does not support layer blend modes"))
        | _ -> ()

    let private handleRect (instruction: PaintRect) (context: AsciiContext) scaleX scaleY =
        let clip = topClip context
        let c1 = toCol instruction.X scaleX
        let r1 = toRow instruction.Y scaleY
        let c2 = toCol (instruction.X + instruction.Width) scaleX
        let r2 = toRow (instruction.Y + instruction.Height) scaleY

        let hasFill =
            match instruction.Fill with
            | Some fill when fill <> String.Empty && fill <> "transparent" && fill <> "none" -> true
            | _ -> false

        let hasStroke =
            match instruction.Stroke with
            | Some stroke when stroke <> String.Empty -> true
            | _ -> false

        if hasFill then
            for row in r1 .. r2 do
                for col in c1 .. c2 do
                    context.Buffer.WriteTag(row, col, CellFlags.Fill, clip)

        if hasStroke then
            context.Buffer.WriteTag(r1, c1, CellFlags.Down ||| CellFlags.Right, clip)
            context.Buffer.WriteTag(r1, c2, CellFlags.Down ||| CellFlags.Left, clip)
            context.Buffer.WriteTag(r2, c1, CellFlags.Up ||| CellFlags.Right, clip)
            context.Buffer.WriteTag(r2, c2, CellFlags.Up ||| CellFlags.Left, clip)

            for col in (c1 + 1) .. (c2 - 1) do
                context.Buffer.WriteTag(r1, col, CellFlags.Left ||| CellFlags.Right, clip)
                context.Buffer.WriteTag(r2, col, CellFlags.Left ||| CellFlags.Right, clip)

            for row in (r1 + 1) .. (r2 - 1) do
                context.Buffer.WriteTag(row, c1, CellFlags.Up ||| CellFlags.Down, clip)
                context.Buffer.WriteTag(row, c2, CellFlags.Up ||| CellFlags.Down, clip)

    let private handleLine (instruction: PaintLine) (context: AsciiContext) scaleX scaleY =
        let clip = topClip context
        let c1 = toCol instruction.X1 scaleX
        let r1 = toRow instruction.Y1 scaleY
        let c2 = toCol instruction.X2 scaleX
        let r2 = toRow instruction.Y2 scaleY

        if r1 = r2 then
            let minCol = min c1 c2
            let maxCol = max c1 c2

            for col in minCol .. maxCol do
                let mutable flags = CellFlags.None

                if col > minCol then
                    flags <- flags ||| CellFlags.Left

                if col < maxCol then
                    flags <- flags ||| CellFlags.Right

                if col = minCol && col = maxCol then
                    flags <- CellFlags.Left ||| CellFlags.Right

                context.Buffer.WriteTag(r1, col, flags, clip)
        elif c1 = c2 then
            let minRow = min r1 r2
            let maxRow = max r1 r2

            for row in minRow .. maxRow do
                let mutable flags = CellFlags.None

                if row > minRow then
                    flags <- flags ||| CellFlags.Up

                if row < maxRow then
                    flags <- flags ||| CellFlags.Down

                if row = minRow && row = maxRow then
                    flags <- CellFlags.Up ||| CellFlags.Down

                context.Buffer.WriteTag(row, c1, flags, clip)
        else
            let deltaRow = abs (r2 - r1)
            let deltaCol = abs (c2 - c1)
            let stepRow = if r1 < r2 then 1 else -1
            let stepCol = if c1 < c2 then 1 else -1
            let mutable error = deltaCol - deltaRow
            let mutable rowCursor = r1
            let mutable colCursor = c1

            let mutable keepGoing = true

            while keepGoing do
                let flags =
                    if deltaCol > deltaRow then
                        CellFlags.Left ||| CellFlags.Right
                    else
                        CellFlags.Up ||| CellFlags.Down

                context.Buffer.WriteTag(rowCursor, colCursor, flags, clip)

                if rowCursor = r2 && colCursor = c2 then
                    keepGoing <- false
                else
                    let doubled = 2 * error

                    if doubled > -deltaRow then
                        error <- error - deltaRow
                        colCursor <- colCursor + stepCol

                    if doubled < deltaCol then
                        error <- error + deltaCol
                        rowCursor <- rowCursor + stepRow

    let private handleGlyphRun (instruction: PaintGlyphRun) (context: AsciiContext) scaleX scaleY =
        let clip = topClip context

        for glyph in instruction.Glyphs do
            context.Buffer.WriteChar(toRow glyph.Y scaleY, toCol glyph.X scaleX, toSafeTerminalGlyph glyph.GlyphId, clip)

    let private handleClip (instruction: PaintClip) (context: AsciiContext) (vm: PaintVM<AsciiContext>) scaleX scaleY =
        let parent = topClip context

        let next =
            { MinCol = max parent.MinCol (toCol instruction.X scaleX)
              MinRow = max parent.MinRow (toRow instruction.Y scaleY)
              MaxCol = min parent.MaxCol (toCol (instruction.X + instruction.Width) scaleX)
              MaxRow = min parent.MaxRow (toRow (instruction.Y + instruction.Height) scaleY) }

        context.ClipStack.Add(next)

        try
            instruction.Children |> List.iter (fun child -> vm.Dispatch(child, context))
        finally
            context.ClipStack.RemoveAt(context.ClipStack.Count - 1)

    let createAsciiContext () = AsciiContext()

    let createAsciiVMWith (options: AsciiOptions) =
        let scaleX = options.ScaleX
        let scaleY = options.ScaleY

        let vm =
            PaintVM<AsciiContext>(
                (fun context _ width height ->
                    let cols = max 0 (int (Math.Ceiling(width / scaleX)))
                    let rows = max 0 (int (Math.Ceiling(height / scaleY)))
                    context.Buffer <- CharBuffer(rows, cols)
                    context.ClipStack.Clear()
                    context.ClipStack.Add(fullClip cols rows)),
                (fun _ _ _ -> raise (ExportNotSupportedError("paint-vm-ascii"))))

        vm.Register(
            "rect",
            fun instruction context _ ->
                match instruction with
                | Rect rect -> handleRect rect context scaleX scaleY
                | _ -> ())

        vm.Register(
            "line",
            fun instruction context _ ->
                match instruction with
                | Line line -> handleLine line context scaleX scaleY
                | _ -> ())

        vm.Register(
            "glyph_run",
            fun instruction context _ ->
                match instruction with
                | GlyphRun glyphRun -> handleGlyphRun glyphRun context scaleX scaleY
                | _ -> ())

        vm.Register(
            "group",
            fun instruction context innerVm ->
                match instruction with
                | Group group ->
                    assertPlainGroup group
                    group.Children |> List.iter (fun child -> innerVm.Dispatch(child, context))
                | _ -> ())

        vm.Register(
            "clip",
            fun instruction context innerVm ->
                match instruction with
                | Clip clip -> handleClip clip context innerVm scaleX scaleY
                | _ -> ())

        vm.Register(
            "layer",
            fun instruction context innerVm ->
                match instruction with
                | Layer layer ->
                    assertPlainLayer layer
                    layer.Children |> List.iter (fun child -> innerVm.Dispatch(child, context))
                | _ -> ())

        vm

    let createAsciiVM () = createAsciiVMWith defaultAsciiOptions

    let renderToAsciiWith (options: AsciiOptions) scene =
        let context = createAsciiContext ()
        let vm = createAsciiVMWith options
        vm.Execute(scene, context)
        context.Buffer.ToString()

    let renderToAscii scene = renderToAsciiWith defaultAsciiOptions scene
