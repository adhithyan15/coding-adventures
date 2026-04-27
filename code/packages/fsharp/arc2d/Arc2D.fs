namespace CodingAdventures.Arc2D.FSharp

open System
open CodingAdventures.Bezier2D.FSharp
open CodingAdventures.Point2D
open CodingAdventures.Trig

module private ArcHelpers =
    let angleBetween ux uy vx vy =
        Trig.atan2 (ux * vy - uy * vx) (ux * vx + uy * vy)

type CenterArc =
    {
        Center: Point
        Rx: float
        Ry: float
        StartAngle: float
        SweepAngle: float
        XRotation: float
    }

    static member New(center: Point, rx: float, ry: float, startAngle: float, sweepAngle: float, xRotation: float) =
        {
            Center = center
            Rx = rx
            Ry = ry
            StartAngle = startAngle
            SweepAngle = sweepAngle
            XRotation = xRotation
        }

    member this.Evaluate(t: float) =
        let angle = this.StartAngle + t * this.SweepAngle
        let xp = this.Rx * Trig.cos angle
        let yp = this.Ry * Trig.sin angle
        let cosineRotation = Trig.cos this.XRotation
        let sineRotation = Trig.sin this.XRotation

        Point.New(
            cosineRotation * xp - sineRotation * yp + this.Center.X,
            sineRotation * xp + cosineRotation * yp + this.Center.Y)

    member this.Tangent(t: float) =
        let angle = this.StartAngle + t * this.SweepAngle
        let dxp = -this.Rx * Trig.sin angle * this.SweepAngle
        let dyp = this.Ry * Trig.cos angle * this.SweepAngle
        let cosineRotation = Trig.cos this.XRotation
        let sineRotation = Trig.sin this.XRotation

        Point.New(
            cosineRotation * dxp - sineRotation * dyp,
            sineRotation * dxp + cosineRotation * dyp)

    member this.BoundingBox() =
        let samples = 100
        let mutable minX = Double.PositiveInfinity
        let mutable minY = Double.PositiveInfinity
        let mutable maxX = Double.NegativeInfinity
        let mutable maxY = Double.NegativeInfinity

        for index in 0 .. samples do
            let point = this.Evaluate(float index / float samples)
            minX <- min minX point.X
            maxX <- max maxX point.X
            minY <- min minY point.Y
            maxY <- max maxY point.Y

        Rect.New(minX, minY, maxX - minX, maxY - minY)

    member this.ToCubicBeziers() =
        let maxSegment = Trig.PI / 2.0
        let segmentCount = max 1 (int (Math.Ceiling(abs this.SweepAngle / maxSegment)))
        let segmentSweep = this.SweepAngle / float segmentCount
        let cosineRotation = Trig.cos this.XRotation
        let sineRotation = Trig.sin this.XRotation
        let k = (4.0 / 3.0) * Trig.tan (segmentSweep / 4.0)

        let rotateTranslate localX localY =
            Point.New(
                cosineRotation * localX - sineRotation * localY + this.Center.X,
                sineRotation * localX + cosineRotation * localY + this.Center.Y)

        [ for index in 0 .. segmentCount - 1 do
            let alpha = this.StartAngle + float index * segmentSweep
            let beta = alpha + segmentSweep
            let cosAlpha = Trig.cos alpha
            let sinAlpha = Trig.sin alpha
            let cosBeta = Trig.cos beta
            let sinBeta = Trig.sin beta

            let p0Local = this.Rx * cosAlpha, this.Ry * sinAlpha
            let p3Local = this.Rx * cosBeta, this.Ry * sinBeta
            let p1Local = fst p0Local + k * (-this.Rx * sinAlpha), snd p0Local + k * (this.Ry * cosAlpha)
            let p2Local = fst p3Local - k * (-this.Rx * sinBeta), snd p3Local - k * (this.Ry * cosBeta)

            CubicBezier.New(
                rotateTranslate (fst p0Local) (snd p0Local),
                rotateTranslate (fst p1Local) (snd p1Local),
                rotateTranslate (fst p2Local) (snd p2Local),
                rotateTranslate (fst p3Local) (snd p3Local)) ]

type SvgArc =
    {
        From: Point
        To: Point
        Rx: float
        Ry: float
        XRotation: float
        LargeArc: bool
        Sweep: bool
    }

    static member New(fromPoint: Point, toPoint: Point, rx: float, ry: float, xRotation: float, largeArc: bool, sweep: bool) =
        {
            From = fromPoint
            To = toPoint
            Rx = rx
            Ry = ry
            XRotation = xRotation
            LargeArc = largeArc
            Sweep = sweep
        }

    member this.ToCenterArc() =
        if abs (this.From.X - this.To.X) < 1e-12 && abs (this.From.Y - this.To.Y) < 1e-12 then
            None
        elif abs this.Rx < 1e-12 || abs this.Ry < 1e-12 then
            None
        else
            let cosineRotation = Trig.cos this.XRotation
            let sineRotation = Trig.sin this.XRotation
            let dx = (this.From.X - this.To.X) / 2.0
            let dy = (this.From.Y - this.To.Y) / 2.0
            let x1Prime = cosineRotation * dx + sineRotation * dy
            let y1Prime = -sineRotation * dx + cosineRotation * dy
            let mutable rx = abs this.Rx
            let mutable ry = abs this.Ry

            let lambda = (x1Prime / rx) * (x1Prime / rx) + (y1Prime / ry) * (y1Prime / ry)
            if lambda > 1.0 then
                let squareRootLambda = Trig.sqrt lambda
                rx <- rx * squareRootLambda
                ry <- ry * squareRootLambda

            let rxSquared = rx * rx
            let rySquared = ry * ry
            let x1PrimeSquared = x1Prime * x1Prime
            let y1PrimeSquared = y1Prime * y1Prime
            let numerator = rxSquared * rySquared - rxSquared * y1PrimeSquared - rySquared * x1PrimeSquared
            let denominator = rxSquared * y1PrimeSquared + rySquared * x1PrimeSquared

            let squareRoot =
                if abs denominator < 1e-12 then
                    0.0
                else
                    Trig.sqrt (max 0.0 (numerator / denominator))

            let sign = if this.LargeArc = this.Sweep then -1.0 else 1.0
            let cxPrime = sign * squareRoot * (rx * y1Prime / ry)
            let cyPrime = sign * squareRoot * -(ry * x1Prime / rx)
            let midX = (this.From.X + this.To.X) / 2.0
            let midY = (this.From.Y + this.To.Y) / 2.0
            let cx = cosineRotation * cxPrime - sineRotation * cyPrime + midX
            let cy = sineRotation * cxPrime + cosineRotation * cyPrime + midY

            let ux = (x1Prime - cxPrime) / rx
            let uy = (y1Prime - cyPrime) / ry
            let vx = (-x1Prime - cxPrime) / rx
            let vy = (-y1Prime - cyPrime) / ry
            let startAngle = ArcHelpers.angleBetween 1.0 0.0 ux uy
            let mutable sweepAngle = ArcHelpers.angleBetween ux uy vx vy

            if not this.Sweep && sweepAngle > 0.0 then
                sweepAngle <- sweepAngle - 2.0 * Trig.PI

            if this.Sweep && sweepAngle < 0.0 then
                sweepAngle <- sweepAngle + 2.0 * Trig.PI

            Some(CenterArc.New(Point.New(cx, cy), rx, ry, startAngle, sweepAngle, this.XRotation))

    member this.ToCubicBeziers() =
        match this.ToCenterArc() with
        | Some arc -> arc.ToCubicBeziers()
        | None -> []

    member this.Evaluate(t: float) =
        this.ToCenterArc() |> Option.map (fun arc -> arc.Evaluate t)

    member this.BoundingBox() =
        this.ToCenterArc() |> Option.map _.BoundingBox()

[<RequireQualifiedAccess>]
module Arc2D =
    [<Literal>]
    let VERSION = "0.1.0"
