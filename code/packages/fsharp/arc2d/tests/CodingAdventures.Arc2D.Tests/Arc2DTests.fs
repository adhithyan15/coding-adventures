namespace CodingAdventures.Arc2D.Tests

open System
open System.IO
open System.Text.Json
open CodingAdventures.Arc2D.FSharp
open CodingAdventures.Point2D
open CodingAdventures.Trig
open Xunit

module Arc2DTests =
    let epsilon = 1e-5

    let close left right =
        abs (left - right) < epsilon

    let pointClose (left: Point) (right: Point) =
        close left.X right.X && close left.Y right.Y

    [<Fact>]
    let ``version exists`` () =
        Assert.Equal("0.1.0", Arc2D.VERSION)

    [<Fact>]
    let ``center arc evaluates tangents and bounds`` () =
        let arc = CenterArc.New(Point.Origin(), 1.0, 1.0, 0.0, Trig.PI / 2.0, 0.0)

        Assert.True(pointClose (arc.Evaluate 0.0) (Point.New(1.0, 0.0)))
        Assert.True(pointClose (arc.Evaluate 1.0) (Point.New(0.0, 1.0)))
        Assert.True(close (arc.Tangent 0.0).X 0.0)
        Assert.True((arc.Tangent 0.0).Y > 0.0)

        let fullCircle = CenterArc.New(Point.Origin(), 1.0, 1.0, 0.0, 2.0 * Trig.PI, 0.0)
        let bounds = fullCircle.BoundingBox()
        Assert.True(abs (bounds.X + 1.0) < 0.05)
        Assert.True(abs (bounds.Width - 2.0) < 0.05)

    [<Fact>]
    let ``center arc builds cubic beziers`` () =
        let quarter = CenterArc.New(Point.Origin(), 1.0, 1.0, 0.0, Trig.PI / 2.0, 0.0)
        let quarterBeziers = quarter.ToCubicBeziers()
        Assert.Single(quarterBeziers) |> ignore
        Assert.True((quarter.Evaluate 0.5).Distance(quarterBeziers[0].Evaluate 0.5) < 0.001)

        let fullCircle = CenterArc.New(Point.Origin(), 1.0, 1.0, 0.0, 2.0 * Trig.PI, 0.0)
        let beziers = fullCircle.ToCubicBeziers()
        Assert.Equal(4, beziers.Length)
        for index in 0 .. beziers.Length - 2 do
            Assert.True(beziers[index].P3.Distance(beziers[index + 1].P0) < 1e-6)

    [<Fact>]
    let ``svg arc handles degenerate inputs`` () =
        Assert.True((SvgArc.New(Point.Origin(), Point.Origin(), 1.0, 1.0, 0.0, false, true).ToCenterArc()).IsNone)
        Assert.True((SvgArc.New(Point.New(0.0, 0.0), Point.New(1.0, 0.0), 0.0, 1.0, 0.0, false, true).ToCenterArc()).IsNone)
        Assert.Empty(SvgArc.New(Point.Origin(), Point.Origin(), 1.0, 1.0, 0.0, false, true).ToCubicBeziers())
        Assert.True((SvgArc.New(Point.Origin(), Point.Origin(), 1.0, 1.0, 0.0, false, true).Evaluate 0.0).IsSome)
        Assert.True((SvgArc.New(Point.Origin(), Point.Origin(), 1.0, 1.0, 0.0, false, true).BoundingBox()).IsSome)

    [<Fact>]
    let ``svg arc consumes neutral degenerate cases`` () =
        use fixture = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "geometry2d-v1-cases.json")))
        let root = fixture.RootElement
        let tolerance = root.GetProperty("absolute_tolerance").GetDouble()
        let readPoint (entry: JsonElement) (key: string) =
            let coordinates: JsonElement = entry.GetProperty(key)
            Point.New(coordinates[0].GetDouble(), coordinates[1].GetDouble())
        let mutable count = 0
        for entry in root.GetProperty("cases").EnumerateArray() do
            if entry.GetProperty("operation").GetString() = "svg-arc-degenerate" then
                count <- count + 1
                let arc = SvgArc.New(readPoint entry "from", readPoint entry "to",
                                     entry.GetProperty("rx").GetDouble(), entry.GetProperty("ry").GetDouble(),
                                     0.0, false, true)
                let id = entry.GetProperty("id").GetString()
                Assert.True(arc.ToCenterArc().IsNone, id)
                Assert.Empty(arc.ToCubicBeziers())
                let point = arc.Evaluate(entry.GetProperty("sample_t").GetDouble())
                Assert.True(point.IsSome, id)
                let expectedPoint = readPoint entry "expected_point"
                Assert.True(abs (point.Value.X - expectedPoint.X) <= tolerance, id)
                Assert.True(abs (point.Value.Y - expectedPoint.Y) <= tolerance, id)
                let bounds = arc.BoundingBox()
                Assert.True(bounds.IsSome, id)
                let expectedBounds = entry.GetProperty("expected_bounds")
                Assert.True(abs (bounds.Value.X - expectedBounds[0].GetDouble()) <= tolerance, id)
                Assert.True(abs (bounds.Value.Y - expectedBounds[1].GetDouble()) <= tolerance, id)
                Assert.True(abs (bounds.Value.Width - expectedBounds[2].GetDouble()) <= tolerance, id)
                Assert.True(abs (bounds.Value.Height - expectedBounds[3].GetDouble()) <= tolerance, id)
        Assert.Equal(4, count)

    [<Fact>]
    let ``svg arc keeps strict degenerate boundaries and signed radius`` () =
        let origin = Point.Origin()
        for arc in [
            SvgArc.New(origin, Point.New(1.0, 0.0), 1e-10, 1.0, 0.0, false, true)
            SvgArc.New(origin, Point.New(1e-10, 0.0), 1.0, 1.0, 0.0, false, true)
            SvgArc.New(origin, Point.New(8e-11, 8e-11), 1.0, 1.0, 0.0, false, true)
            SvgArc.New(origin, Point.New(1.0, 0.0), -1e-10, 1.0, 0.0, false, true)
        ] do
            let center = arc.ToCenterArc()
            Assert.True(center.IsSome)
            for value in [ center.Value.Center.X; center.Value.Center.Y; center.Value.Rx;
                           center.Value.Ry; center.Value.StartAngle; center.Value.SweepAngle;
                           center.Value.XRotation ] do
                Assert.True(System.Double.IsFinite(value))
        let positive = SvgArc.New(Point.New(1.0, 0.0), Point.New(0.0, 1.0), 1.0, 1.0, 0.0, false, true)
        let negative = { positive with Rx = -1.0 }
        let p = positive.Evaluate(0.25)
        let n = negative.Evaluate(0.25)
        Assert.True(p.IsSome && n.IsSome)
        Assert.True(abs (p.Value.X - n.Value.X) < 1e-12)
        Assert.True(abs (p.Value.Y - n.Value.Y) < 1e-12)

    [<Fact>]
    let ``svg arc converts endpoint form`` () =
        let arc = SvgArc.New(Point.New(1.0, 0.0), Point.New(0.0, 1.0), 1.0, 1.0, 0.0, false, true)
        let centerArc = arc.ToCenterArc()
        Assert.True(centerArc.IsSome)
        Assert.True(close centerArc.Value.Center.X 0.0)
        Assert.True(close centerArc.Value.Center.Y 0.0)
        Assert.True(centerArc.Value.SweepAngle > 0.0)

        let start = arc.Evaluate 0.0
        Assert.True(start.IsSome)
        Assert.True(pointClose start.Value (Point.New(1.0, 0.0)))
        Assert.NotEmpty(arc.ToCubicBeziers())
        Assert.True((arc.BoundingBox()).IsSome)

    [<Fact>]
    let ``svg arc flags select different sweeps`` () =
        let ccw = SvgArc.New(Point.New(1.0, 0.0), Point.New(0.0, 1.0), 1.0, 1.0, 0.0, false, true).ToCenterArc()
        let cw = SvgArc.New(Point.New(1.0, 0.0), Point.New(0.0, 1.0), 1.0, 1.0, 0.0, false, false).ToCenterArc()
        Assert.True(ccw.IsSome)
        Assert.True(cw.IsSome)
        Assert.True(ccw.Value.SweepAngle > 0.0)
        Assert.True(cw.Value.SweepAngle < 0.0)

        let large = SvgArc.New(Point.New(1.0, 0.0), Point.New(-1.0, 0.0), 1.0, 1.0, 0.0, true, true).ToCenterArc()
        Assert.True(large.IsSome)
        Assert.True(abs large.Value.SweepAngle > Trig.PI - 1e-6)
