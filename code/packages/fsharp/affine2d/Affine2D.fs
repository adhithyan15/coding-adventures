namespace CodingAdventures.Affine2D.FSharp

open System
open CodingAdventures.Point2D
open CodingAdventures.Trig

type Affine2D =
    {
        A: float
        B: float
        C: float
        D: float
        E: float
        F: float
    }

    static member New(a: float, b: float, c: float, d: float, e: float, f: float) =
        { A = a; B = b; C = c; D = d; E = e; F = f }

    static member Identity() =
        Affine2D.New(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)

    static member Translate(tx: float, ty: float) =
        Affine2D.New(1.0, 0.0, 0.0, 1.0, tx, ty)

    static member Rotate(angle: float) =
        let cosine = Trig.cos angle
        let sine = Trig.sin angle
        Affine2D.New(cosine, sine, -sine, cosine, 0.0, 0.0)

    static member RotateAround(center: Point, angle: float) =
        let movedToOrigin: Affine2D = Affine2D.Translate(-center.X, -center.Y)
        let rotated: Affine2D = movedToOrigin.Then(Affine2D.Rotate angle)
        rotated.Then(Affine2D.Translate(center.X, center.Y))

    static member Scale(sx: float, sy: float) =
        Affine2D.New(sx, 0.0, 0.0, sy, 0.0, 0.0)

    static member ScaleUniform(scale: float) =
        Affine2D.Scale(scale, scale)

    static member SkewX(angle: float) =
        Affine2D.New(1.0, 0.0, Trig.tan angle, 1.0, 0.0, 0.0)

    static member SkewY(angle: float) =
        Affine2D.New(1.0, Trig.tan angle, 0.0, 1.0, 0.0, 0.0)

    member this.Then(next: Affine2D) =
        next.Multiply this

    member this.Multiply(other: Affine2D) =
        Affine2D.New(
            this.A * other.A + this.C * other.B,
            this.B * other.A + this.D * other.B,
            this.A * other.C + this.C * other.D,
            this.B * other.C + this.D * other.D,
            this.A * other.E + this.C * other.F + this.E,
            this.B * other.E + this.D * other.F + this.F)

    member this.ApplyToPoint(point: Point) =
        Point.New(
            this.A * point.X + this.C * point.Y + this.E,
            this.B * point.X + this.D * point.Y + this.F)

    member this.ApplyToVector(vector: Point) =
        Point.New(
            this.A * vector.X + this.C * vector.Y,
            this.B * vector.X + this.D * vector.Y)

    member this.Determinant() =
        this.A * this.D - this.B * this.C

    member this.Invert() =
        let determinant = this.Determinant()
        if abs determinant < 1e-12 then
            None
        else
            Some(
                Affine2D.New(
                    this.D / determinant,
                    -this.B / determinant,
                    -this.C / determinant,
                    this.A / determinant,
                    (this.C * this.F - this.D * this.E) / determinant,
                    (this.B * this.E - this.A * this.F) / determinant))

    member this.IsIdentity() =
        let epsilon = 1e-10
        abs (this.A - 1.0) < epsilon
        && abs this.B < epsilon
        && abs this.C < epsilon
        && abs (this.D - 1.0) < epsilon
        && abs this.E < epsilon
        && abs this.F < epsilon

    member this.IsTranslationOnly() =
        let epsilon = 1e-10
        abs (this.A - 1.0) < epsilon
        && abs this.B < epsilon
        && abs this.C < epsilon
        && abs (this.D - 1.0) < epsilon

    member this.ToArray() =
        [| this.A; this.B; this.C; this.D; this.E; this.F |]

[<RequireQualifiedAccess>]
module Affine2DPackage =
    [<Literal>]
    let VERSION = "0.1.0"
