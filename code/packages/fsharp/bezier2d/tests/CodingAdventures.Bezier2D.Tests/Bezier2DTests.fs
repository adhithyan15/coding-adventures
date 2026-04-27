namespace CodingAdventures.Bezier2D.Tests

open System
open CodingAdventures.Bezier2D.FSharp
open CodingAdventures.Point2D
open Xunit

module Bezier2DTests =
    let epsilon = 1e-9

    let close left right =
        abs (left - right) < epsilon

    let pointClose (left: Point) (right: Point) =
        close left.X right.X && close left.Y right.Y

    [<Fact>]
    let ``version exists`` () =
        Assert.Equal("0.1.0", Bezier2D.VERSION)

    [<Fact>]
    let ``quadratic evaluate derivative split and elevate work`` () =
        let curve = QuadraticBezier.New(Point.New(0.0, 0.0), Point.New(1.0, 2.0), Point.New(2.0, 0.0))

        Assert.True(pointClose (curve.Evaluate 0.0) curve.P0)
        Assert.True(pointClose (curve.Evaluate 1.0) curve.P2)
        Assert.True(pointClose (curve.Evaluate 0.5) (Point.New(1.0, 1.0)))
        Assert.True(pointClose (curve.Derivative 0.0) (Point.New(2.0, 4.0)))

        let left, right = curve.Split 0.5
        let splitPoint = curve.Evaluate 0.5
        Assert.True(pointClose left.P2 splitPoint)
        Assert.True(pointClose right.P0 splitPoint)

        let cubic = curve.Elevate()
        for t in [ 0.0; 0.25; 0.5; 0.75; 1.0 ] do
            Assert.True(pointClose (curve.Evaluate t) (cubic.Evaluate t))

    [<Fact>]
    let ``quadratic polyline and bounding box work`` () =
        let straight = QuadraticBezier.New(Point.New(0.0, 0.0), Point.New(1.0, 0.0), Point.New(2.0, 0.0))
        Assert.Equal(2, (straight.ToPolyline 0.1).Length)

        let curve = QuadraticBezier.New(Point.New(0.0, 0.0), Point.New(0.0, 10.0), Point.New(10.0, 0.0))
        let points = curve.ToPolyline 0.1
        Assert.True(points.Length > 2)
        Assert.True(pointClose points[0] curve.P0)
        Assert.True(pointClose points[points.Length - 1] curve.P2)

        let bounds = curve.BoundingBox()
        Assert.True(bounds.X <= 0.0)
        Assert.True(bounds.Y <= 0.0)
        Assert.True(bounds.X + bounds.Width >= 10.0)
        Assert.True(bounds.Height > 0.0)
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> curve.ToPolyline -0.1 |> ignore) |> ignore

    [<Fact>]
    let ``cubic evaluate derivative and split work`` () =
        let curve = CubicBezier.New(Point.New(0.0, 0.0), Point.New(1.0, 2.0), Point.New(3.0, 2.0), Point.New(4.0, 0.0))

        Assert.True(pointClose (curve.Evaluate 0.0) curve.P0)
        Assert.True(pointClose (curve.Evaluate 1.0) curve.P3)
        Assert.True(close (curve.Evaluate 0.5).X 2.0)

        let straight = CubicBezier.New(Point.New(0.0, 0.0), Point.New(1.0, 0.0), Point.New(2.0, 0.0), Point.New(3.0, 0.0))
        Assert.True(pointClose (straight.Derivative 0.0) (Point.New(3.0, 0.0)))

        let left, right = curve.Split 0.5
        let splitPoint = curve.Evaluate 0.5
        Assert.True(pointClose left.P3 splitPoint)
        Assert.True(pointClose right.P0 splitPoint)

    [<Fact>]
    let ``cubic polyline and bounding box work`` () =
        let straight = CubicBezier.New(Point.New(0.0, 0.0), Point.New(1.0, 0.0), Point.New(2.0, 0.0), Point.New(3.0, 0.0))
        Assert.Equal(2, (straight.ToPolyline 0.1).Length)

        let curve = CubicBezier.New(Point.New(0.0, 0.0), Point.New(0.0, 10.0), Point.New(10.0, 10.0), Point.New(10.0, 0.0))
        let points = curve.ToPolyline 0.1
        Assert.True(points.Length > 2)
        Assert.True(pointClose points[0] curve.P0)
        Assert.True(pointClose points[points.Length - 1] curve.P3)

        let bounds = curve.BoundingBox()
        for index in 0 .. 20 do
            let point = curve.Evaluate(float index / 20.0)
            Assert.True(point.X >= bounds.X - 1e-6 && point.X <= bounds.X + bounds.Width + 1e-6)
            Assert.True(point.Y >= bounds.Y - 1e-6 && point.Y <= bounds.Y + bounds.Height + 1e-6)

        Assert.Equal(Rect.New(0.0, 0.0, 3.0, 0.0), straight.BoundingBox())
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> curve.ToPolyline Double.NaN |> ignore) |> ignore
