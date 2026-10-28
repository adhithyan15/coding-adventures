namespace CodingAdventures.Bezier2D.Tests

open System
open System.IO
open System.Text.Json
open CodingAdventures.Bezier2D.FSharp
open CodingAdventures.Point2D
open Xunit

module FlatteningConformanceTests =
    let fixtureTolerance = 1e-12

    let rec fixturePath (directory: DirectoryInfo) =
        let candidate =
            Path.Combine(directory.FullName, "code", "specs", "fixtures", "bezier2d-flattening-v1", "cases.json")

        if File.Exists candidate then
            candidate
        elif isNull directory.Parent then
            raise (FileNotFoundException("could not locate the checked-in G2D02 flattening corpus"))
        else
            fixturePath directory.Parent

    let readPoint (element: JsonElement) =
        let values = element.EnumerateArray() |> Seq.map _.GetDouble() |> Seq.toArray
        Point.New(values[0], values[1])

    let readPoints (element: JsonElement) =
        element.EnumerateArray() |> Seq.map readPoint |> Seq.toArray

    let flatten degree (controls: Point array) tolerance =
        match degree with
        | 2 -> QuadraticBezier.New(controls[0], controls[1], controls[2]).ToPolyline tolerance
        | 3 -> CubicBezier.New(controls[0], controls[1], controls[2], controls[3]).ToPolyline tolerance
        | _ -> invalidOp $"unsupported fixture degree {degree}"

    let evaluate degree (controls: Point array) t =
        if degree = 2 then
            QuadraticBezier.New(controls[0], controls[1], controls[2]).Evaluate t
        else
            CubicBezier.New(controls[0], controls[1], controls[2], controls[3]).Evaluate t

    let distanceToSegment (point: Point) (startPoint: Point) (endPoint: Point) =
        let dx = endPoint.X - startPoint.X
        let dy = endPoint.Y - startPoint.Y
        let lengthSquared = dx * dx + dy * dy

        if lengthSquared = 0.0 then
            sqrt ((point.X - startPoint.X) ** 2.0 + (point.Y - startPoint.Y) ** 2.0)
        else
            let projection =
                Math.Clamp(
                    ((point.X - startPoint.X) * dx + (point.Y - startPoint.Y) * dy) / lengthSquared,
                    0.0,
                    1.0)

            let nearest = Point.New(startPoint.X + projection * dx, startPoint.Y + projection * dy)
            sqrt ((point.X - nearest.X) ** 2.0 + (point.Y - nearest.Y) ** 2.0)

    [<Fact>]
    let ``neutral flattening cases except budget conform`` () =
        use document =
            JsonDocument.Parse(File.ReadAllText(fixturePath (DirectoryInfo(Directory.GetCurrentDirectory()))))

        let root = document.RootElement
        Assert.Equal(1, root.GetProperty("version").GetInt32())
        Assert.Equal(fixtureTolerance, root.GetProperty("absolute_tolerance").GetDouble())
        Assert.Equal(32, root.GetProperty("max_depth").GetInt32())
        Assert.Equal(65_535, root.GetProperty("max_subdivisions").GetInt32())

        for testCase in root.GetProperty("cases").EnumerateArray() do
            let mutable termination = Unchecked.defaultof<JsonElement>

            if not (testCase.TryGetProperty("expected_termination", &termination)) then
                let id = testCase.GetProperty("id").GetString()
                let degree = testCase.GetProperty("degree").GetInt32()
                let tolerance = testCase.GetProperty("tolerance").GetDouble()
                let disposition = testCase.GetProperty("expected_disposition").GetString()
                let controls = readPoints (testCase.GetProperty("control_points"))

                if disposition = "invalid-tolerance" then
                    Assert.Throws<ArgumentOutOfRangeException>(fun () -> flatten degree controls tolerance |> ignore)
                    |> ignore
                else
                    let points = flatten degree controls tolerance
                    Assert.Equal(controls[0], points[0])
                    Assert.Equal(controls[controls.Length - 1], points[points.Length - 1])
                    Assert.All(points, fun point -> Assert.True(Double.IsFinite point.X && Double.IsFinite point.Y, id))

                    if disposition = "chord" then
                        Assert.Equal(2, points.Length)
                    else
                        Assert.True(points.Length > 2, id)

                    let witnessT = testCase.GetProperty("witness_t").GetDouble()
                    let expectedWitness = readPoint (testCase.GetProperty("expected_witness"))
                    let expectedDistance = testCase.GetProperty("expected_witness_distance").GetDouble()
                    let actualWitness = evaluate degree controls witnessT
                    Assert.True(abs (expectedWitness.X - actualWitness.X) <= fixtureTolerance, id)
                    Assert.True(abs (expectedWitness.Y - actualWitness.Y) <= fixtureTolerance, id)
                    Assert.True(
                        abs (distanceToSegment actualWitness controls[0] controls[controls.Length - 1] - expectedDistance)
                        <= fixtureTolerance,
                        id)

    [<Fact>]
    let ``budget exhaustion fails deterministically`` () =
        use document =
            JsonDocument.Parse(File.ReadAllText(fixturePath (DirectoryInfo(Directory.GetCurrentDirectory()))))

        let testCase =
            document.RootElement.GetProperty("cases").EnumerateArray()
            |> Seq.find (fun element ->
                let mutable value = Unchecked.defaultof<JsonElement>
                element.TryGetProperty("expected_termination", &value))

        let controls = readPoints (testCase.GetProperty("control_points"))
        let degree = testCase.GetProperty("degree").GetInt32()
        let tolerance = testCase.GetProperty("tolerance").GetDouble()
        Assert.Throws<InvalidOperationException>(fun () -> flatten degree controls tolerance |> ignore) |> ignore

    [<Fact>]
    let ``nonfinite controls fail before subdivision`` () =
        let origin = Point.New(0.0, 0.0)

        Assert.Throws<ArgumentOutOfRangeException>(fun () ->
            QuadraticBezier.New(origin, Point.New(Double.NaN, 1.0), Point.New(1.0, 0.0)).ToPolyline 0.1
            |> ignore)
        |> ignore

        Assert.Throws<ArgumentOutOfRangeException>(fun () ->
            CubicBezier.New(origin, Point.New(0.0, 1.0), Point.New(Double.PositiveInfinity, -1.0), origin)
                .ToPolyline 0.1
            |> ignore)
        |> ignore

    [<Fact>]
    let ``finite inputs that overflow derived geometry fail closed`` () =
        let negative = Point.New(-Double.MaxValue / 2.0, 0.0)
        let positive = Point.New(Double.MaxValue / 2.0, 0.0)

        Assert.Throws<InvalidOperationException>(fun () ->
            QuadraticBezier.New(negative, Point.New(0.0, 1.0), positive).ToPolyline 0.1 |> ignore)
        |> ignore

        Assert.Throws<InvalidOperationException>(fun () ->
            CubicBezier.New(negative, Point.New(0.0, 1.0), Point.New(0.0, -1.0), positive).ToPolyline 0.1
            |> ignore)
        |> ignore

    [<Fact>]
    let ``depth budget fails before returning a partial polyline`` () =
        let quadratic = QuadraticBezier.New(Point.New(0.0, 0.0), Point.New(0.5, 1.0), Point.New(1.0, 0.0))
        let cubic =
            CubicBezier.New(Point.New(0.0, 0.0), Point.New(0.0, 1.0), Point.New(1.0, 1.0), Point.New(1.0, 0.0))

        Assert.Throws<InvalidOperationException>(fun () -> quadratic.ToPolyline Double.Epsilon |> ignore) |> ignore
        Assert.Throws<InvalidOperationException>(fun () -> cubic.ToPolyline Double.Epsilon |> ignore) |> ignore

    [<Fact>]
    let ``subnormal geometry cannot underflow into a chord`` () =
        let curve =
            QuadraticBezier.New(Point.New(0.0, 0.0), Point.New(0.0, 1e-200), Point.New(1e-200, 0.0))

        Assert.Throws<InvalidOperationException>(fun () -> curve.ToPolyline Double.Epsilon |> ignore) |> ignore

    [<Fact>]
    let ``bounding boxes include interior x extrema`` () =
        let quadratic = QuadraticBezier.New(Point.New(0.0, 0.0), Point.New(10.0, 0.0), Point.New(0.0, 1.0))
        let quadraticBounds = quadratic.BoundingBox()
        Assert.True(quadraticBounds.X + quadraticBounds.Width > 4.9)

        let cubic =
            CubicBezier.New(Point.New(0.0, 0.0), Point.New(10.0, 0.0), Point.New(-10.0, 0.0), Point.New(0.0, 1.0))

        let cubicBounds = cubic.BoundingBox()
        Assert.True(cubicBounds.X < -2.8)
        Assert.True(cubicBounds.X + cubicBounds.Width > 2.8)
