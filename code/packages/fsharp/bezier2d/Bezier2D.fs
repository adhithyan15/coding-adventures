namespace CodingAdventures.Bezier2D.FSharp

open System
open CodingAdventures.Point2D
open CodingAdventures.Trig

module private BezierHelpers =
    let validateTolerance tolerance =
        if not (Double.IsFinite tolerance) || tolerance < 0.0 then
            raise (ArgumentOutOfRangeException("tolerance", tolerance, "tolerance must be finite and non-negative"))

    let extremaOfCubicDerivative v0 v1 v2 v3 =
        let a = -3.0 * v0 + 9.0 * v1 - 9.0 * v2 + 3.0 * v3
        let b = 6.0 * v0 - 12.0 * v1 + 6.0 * v2
        let c = -3.0 * v0 + 3.0 * v1
        let roots = ResizeArray<float>()

        if abs a < 1e-12 then
            if abs b > 1e-12 then
                let t = -c / b
                if t > 0.0 && t < 1.0 then
                    roots.Add t
        else
            let discriminant = b * b - 4.0 * a * c
            if discriminant >= 0.0 then
                let squareRoot = Trig.sqrt discriminant
                let t1 = (-b + squareRoot) / (2.0 * a)
                let t2 = (-b - squareRoot) / (2.0 * a)
                if t1 > 0.0 && t1 < 1.0 then
                    roots.Add t1
                if t2 > 0.0 && t2 < 1.0 then
                    roots.Add t2

        List.ofSeq roots

type QuadraticBezier =
    {
        P0: Point
        P1: Point
        P2: Point
    }

    static member New(p0: Point, p1: Point, p2: Point) =
        { P0 = p0; P1 = p1; P2 = p2 }

    member this.Evaluate(t: float) =
        let q0 = this.P0.Lerp(this.P1, t)
        let q1 = this.P1.Lerp(this.P2, t)
        q0.Lerp(q1, t)

    member this.Derivative(t: float) =
        let d0 = this.P1.Subtract this.P0
        let d1 = this.P2.Subtract this.P1
        (d0.Lerp(d1, t)).Scale(2.0)

    member this.Split(t: float) =
        let q0 = this.P0.Lerp(this.P1, t)
        let q1 = this.P1.Lerp(this.P2, t)
        let midpoint = q0.Lerp(q1, t)
        QuadraticBezier.New(this.P0, q0, midpoint), QuadraticBezier.New(midpoint, q1, this.P2)

    member this.ToPolyline(tolerance: float) =
        BezierHelpers.validateTolerance tolerance
        let chordMid = this.P0.Lerp(this.P2, 0.5)
        let curveMid = this.Evaluate 0.5

        if chordMid.Distance curveMid <= tolerance then
            [ this.P0; this.P2 ]
        else
            let left, right = this.Split 0.5
            let leftPoints = left.ToPolyline tolerance
            let rightPoints = right.ToPolyline tolerance
            leftPoints @ (rightPoints |> List.tail)

    member this.BoundingBox() =
        let mutable minX = min this.P0.X this.P2.X
        let mutable maxX = max this.P0.X this.P2.X
        let mutable minY = min this.P0.Y this.P2.Y
        let mutable maxY = max this.P0.Y this.P2.Y

        let denomX = this.P0.X - 2.0 * this.P1.X + this.P2.X
        if abs denomX > 1e-12 then
            let tx = (this.P0.X - this.P1.X) / denomX
            if tx > 0.0 && tx < 1.0 then
                let px = this.Evaluate tx
                minX <- min minX px.X
                maxX <- max maxX px.X

        let denomY = this.P0.Y - 2.0 * this.P1.Y + this.P2.Y
        if abs denomY > 1e-12 then
            let ty = (this.P0.Y - this.P1.Y) / denomY
            if ty > 0.0 && ty < 1.0 then
                let py = this.Evaluate ty
                minY <- min minY py.Y
                maxY <- max maxY py.Y

        Rect.New(minX, minY, maxX - minX, maxY - minY)

    member this.Elevate() =
        let q1 = (this.P0.Scale(1.0 / 3.0)).Add(this.P1.Scale(2.0 / 3.0))
        let q2 = (this.P1.Scale(2.0 / 3.0)).Add(this.P2.Scale(1.0 / 3.0))
        CubicBezier.New(this.P0, q1, q2, this.P2)

and CubicBezier =
    {
        P0: Point
        P1: Point
        P2: Point
        P3: Point
    }

    static member New(p0: Point, p1: Point, p2: Point, p3: Point) =
        { P0 = p0; P1 = p1; P2 = p2; P3 = p3 }

    member this.Evaluate(t: float) =
        let p01 = this.P0.Lerp(this.P1, t)
        let p12 = this.P1.Lerp(this.P2, t)
        let p23 = this.P2.Lerp(this.P3, t)
        let p012 = p01.Lerp(p12, t)
        let p123 = p12.Lerp(p23, t)
        p012.Lerp(p123, t)

    member this.Derivative(t: float) =
        let d0 = this.P1.Subtract this.P0
        let d1 = this.P2.Subtract this.P1
        let d2 = this.P3.Subtract this.P2
        let oneMinusT = 1.0 - t

        d0.Scale(oneMinusT * oneMinusT)
            .Add(d1.Scale(2.0 * oneMinusT * t))
            .Add(d2.Scale(t * t))
            .Scale(3.0)

    member this.Split(t: float) =
        let p01 = this.P0.Lerp(this.P1, t)
        let p12 = this.P1.Lerp(this.P2, t)
        let p23 = this.P2.Lerp(this.P3, t)
        let p012 = p01.Lerp(p12, t)
        let p123 = p12.Lerp(p23, t)
        let p0123 = p012.Lerp(p123, t)

        CubicBezier.New(this.P0, p01, p012, p0123), CubicBezier.New(p0123, p123, p23, this.P3)

    member this.ToPolyline(tolerance: float) =
        BezierHelpers.validateTolerance tolerance
        let chordMid = this.P0.Lerp(this.P3, 0.5)
        let curveMid = this.Evaluate 0.5

        if chordMid.Distance curveMid <= tolerance then
            [ this.P0; this.P3 ]
        else
            let left, right = this.Split 0.5
            let leftPoints = left.ToPolyline tolerance
            let rightPoints = right.ToPolyline tolerance
            leftPoints @ (rightPoints |> List.tail)

    member this.BoundingBox() =
        let mutable minX = min this.P0.X this.P3.X
        let mutable maxX = max this.P0.X this.P3.X
        let mutable minY = min this.P0.Y this.P3.Y
        let mutable maxY = max this.P0.Y this.P3.Y

        for t in BezierHelpers.extremaOfCubicDerivative this.P0.X this.P1.X this.P2.X this.P3.X do
            let px = this.Evaluate t
            minX <- min minX px.X
            maxX <- max maxX px.X

        for t in BezierHelpers.extremaOfCubicDerivative this.P0.Y this.P1.Y this.P2.Y this.P3.Y do
            let py = this.Evaluate t
            minY <- min minY py.Y
            maxY <- max maxY py.Y

        Rect.New(minX, minY, maxX - minX, maxY - minY)

[<RequireQualifiedAccess>]
module Bezier2D =
    [<Literal>]
    let VERSION = "0.1.0"
