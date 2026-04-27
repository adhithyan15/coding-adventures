namespace CodingAdventures.Affine2D.Tests

open System
open CodingAdventures.Affine2D.FSharp
open CodingAdventures.Point2D
open CodingAdventures.Trig
open Xunit

module Affine2DTests =
    let epsilon = 1e-9

    let close left right =
        abs (left - right) < epsilon

    let pointClose (left: Point) (right: Point) =
        close left.X right.X && close left.Y right.Y

    let affineClose (left: Affine2D) (right: Affine2D) =
        Array.zip (left.ToArray()) (right.ToArray())
        |> Array.forall (fun (a, b) -> close a b)

    [<Fact>]
    let ``version exists`` () =
        Assert.Equal("0.1.0", Affine2DPackage.VERSION)

    [<Fact>]
    let ``factories apply expected transforms`` () =
        let identity = Affine2D.Identity()
        Assert.True(Array.zip (identity.ToArray()) [| 1.0; 0.0; 0.0; 1.0; 0.0; 0.0 |] |> Array.forall (fun (a, b) -> close a b))
        Assert.True(pointClose (identity.ApplyToPoint(Point.New(3.0, 4.0))) (Point.New(3.0, 4.0)))

        Assert.True(pointClose (Affine2D.Translate(5.0, -3.0).ApplyToPoint(Point.New(1.0, 2.0))) (Point.New(6.0, -1.0)))
        Assert.True(pointClose (Affine2D.Translate(5.0, -3.0).ApplyToVector(Point.New(1.0, 2.0))) (Point.New(1.0, 2.0)))
        Assert.True(pointClose (Affine2D.Scale(2.0, 3.0).ApplyToPoint(Point.New(1.0, 1.0))) (Point.New(2.0, 3.0)))
        Assert.True(pointClose (Affine2D.ScaleUniform(5.0).ApplyToPoint(Point.New(2.0, 3.0))) (Point.New(10.0, 15.0)))

    [<Fact>]
    let ``rotation and skew factories work`` () =
        Assert.True(pointClose (Affine2D.Rotate(Trig.PI / 2.0).ApplyToPoint(Point.New(1.0, 0.0))) (Point.New(0.0, 1.0)))
        Assert.True(Affine2D.Rotate(2.0 * Trig.PI).IsIdentity())

        let center = Point.New(1.0, 0.0)
        Assert.True(pointClose (Affine2D.RotateAround(center, Trig.PI / 2.0).ApplyToPoint(center)) center)
        Assert.True(pointClose (Affine2D.SkewX(Trig.PI / 4.0).ApplyToPoint(Point.New(0.0, 1.0))) (Point.New(1.0, 1.0)))
        Assert.True(pointClose (Affine2D.SkewY(Trig.PI / 4.0).ApplyToPoint(Point.New(1.0, 0.0))) (Point.New(1.0, 1.0)))

    [<Fact>]
    let ``composition determinant and invert work`` () =
        let scale = Affine2D.ScaleUniform 2.0
        let translate = Affine2D.Translate(10.0, 0.0)
        let composed = translate.Multiply scale
        Assert.True(pointClose (composed.ApplyToPoint(Point.New(1.0, 1.0))) (Point.New(12.0, 2.0)))

        Assert.True(affineClose (Affine2D.Identity().Multiply translate) translate)
        Assert.True(affineClose (scale.Then translate) composed)
        Assert.True(close (Affine2D.Scale(2.0, 3.0).Determinant()) 6.0)
        Assert.True(close (Affine2D.Rotate(Trig.PI / 3.0).Determinant()) 1.0)

        let inverse = translate.Invert()
        Assert.True(inverse.IsSome)
        Assert.True((translate.Multiply inverse.Value).IsIdentity())
        Assert.True((Affine2D.New(0.0, 0.0, 0.0, 0.0, 0.0, 0.0).Invert()).IsNone)

    [<Fact>]
    let ``predicates distinguish transform kinds`` () =
        Assert.True(Affine2D.Identity().IsIdentity())
        Assert.False(Affine2D.Translate(1.0, 0.0).IsIdentity())
        Assert.True(Affine2D.Translate(5.0, 3.0).IsTranslationOnly())
        Assert.False(Affine2D.Rotate(0.1).IsTranslationOnly())
        Assert.False(Affine2D.Scale(2.0, 1.0).IsTranslationOnly())
