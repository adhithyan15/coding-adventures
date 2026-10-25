namespace CodingAdventures.Bezier2D.FSharp

open System
open System.Collections.Generic
open CodingAdventures.Point2D
open CodingAdventures.Trig

module private BezierHelpers =
    [<Literal>]
    let MaxDepth = 32

    [<Literal>]
    let MaxSubdivisions = 65_535

    let validateInputs tolerance (controlPoints: Point list) =
        if not (Double.IsFinite tolerance) || tolerance <= 0.0 then
            raise (ArgumentOutOfRangeException("tolerance", tolerance, "tolerance must be finite and positive"))

        for point in controlPoints do
            if not (Double.IsFinite point.X) || not (Double.IsFinite point.Y) then
                raise (ArgumentOutOfRangeException("controlPoints", point, "control points must be finite"))

    let finite value =
        if Double.IsFinite value then
            value
        else
            raise (InvalidOperationException("Bezier subdivision produced nonfinite geometry"))

    let distance (left: Point) (right: Point) =
        let dx = finite (left.X - right.X)
        let dy = finite (left.Y - right.Y)
        finite (Math.Sqrt(finite (dx * dx + dy * dy)))

    let distanceToSegment (point: Point) (startPoint: Point) (endPoint: Point) =
        let dx = finite (endPoint.X - startPoint.X)
        let dy = finite (endPoint.Y - startPoint.Y)
        let lengthSquared = finite (dx * dx + dy * dy)

        if lengthSquared = 0.0 then
            distance point startPoint
        else
            let offsetX = finite (point.X - startPoint.X)
            let offsetY = finite (point.Y - startPoint.Y)
            let projection =
                finite ((offsetX * dx + offsetY * dy) / lengthSquared)
                |> fun value -> Math.Clamp(value, 0.0, 1.0)
            let nearest = Point.New(finite (startPoint.X + projection * dx), finite (startPoint.Y + projection * dy))
            distance point nearest

    let budgetExceeded () =
        InvalidOperationException("Bezier flattening exceeded the depth or subdivision budget")

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
        BezierHelpers.validateInputs tolerance [ this.P0; this.P1; this.P2 ]
        let points = ResizeArray<Point>()
        let pending = Stack<QuadraticBezier * int>()
        points.Add this.P0
        pending.Push((this, 0))
        let mutable subdivisions = 0

        while pending.Count > 0 do
            let curve, depth = pending.Pop()
            let error = BezierHelpers.distanceToSegment curve.P1 curve.P0 curve.P2

            if error <= tolerance then
                points.Add curve.P2
            else
                if depth >= BezierHelpers.MaxDepth || subdivisions >= BezierHelpers.MaxSubdivisions then
                    raise (BezierHelpers.budgetExceeded())

                subdivisions <- subdivisions + 1
                let left, right = curve.Split 0.5
                pending.Push((right, depth + 1))
                pending.Push((left, depth + 1))

        List.ofSeq points

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
        BezierHelpers.validateInputs tolerance [ this.P0; this.P1; this.P2; this.P3 ]
        let points = ResizeArray<Point>()
        let pending = Stack<CubicBezier * int>()
        points.Add this.P0
        pending.Push((this, 0))
        let mutable subdivisions = 0

        while pending.Count > 0 do
            let curve, depth = pending.Pop()
            let error =
                max
                    (BezierHelpers.distanceToSegment curve.P1 curve.P0 curve.P3)
                    (BezierHelpers.distanceToSegment curve.P2 curve.P0 curve.P3)

            if error <= tolerance then
                points.Add curve.P3
            else
                if depth >= BezierHelpers.MaxDepth || subdivisions >= BezierHelpers.MaxSubdivisions then
                    raise (BezierHelpers.budgetExceeded())

                subdivisions <- subdivisions + 1
                let left, right = curve.Split 0.5
                pending.Push((right, depth + 1))
                pending.Push((left, depth + 1))

        List.ofSeq points

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
