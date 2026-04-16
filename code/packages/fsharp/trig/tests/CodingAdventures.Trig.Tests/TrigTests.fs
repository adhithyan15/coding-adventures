namespace CodingAdventures.Trig.Tests

open Xunit
open CodingAdventures.Trig

type TrigTests() =
    [<Fact>]
    member _.``Sin and cos match canonical values``() =
        let approxEqual left right = abs (left - right) < 1e-10

        Assert.True(approxEqual (Trig.sin 0.0) 0.0)
        Assert.True(approxEqual (Trig.sin (Trig.PI / 2.0)) 1.0)
        Assert.True(approxEqual (Trig.sin Trig.PI) 0.0)
        Assert.True(approxEqual (Trig.cos 0.0) 1.0)
        Assert.True(approxEqual (Trig.cos (Trig.PI / 2.0)) 0.0)
        Assert.True(approxEqual (Trig.cos Trig.PI) -1.0)

    [<Fact>]
    member _.``Symmetry and Pythagorean identities hold``() =
        let approxEqual left right = abs (left - right) < 1e-10

        for value in [| 0.5; 1.0; 2.0; Trig.PI / 4.0; Trig.PI / 3.0 |] do
            Assert.True(approxEqual (Trig.sin (-value)) (-Trig.sin value))
            Assert.True(approxEqual (Trig.cos (-value)) (Trig.cos value))

        for value in [| 0.0; 0.5; 1.0; Trig.PI / 6.0; Trig.PI / 4.0; Trig.PI / 3.0; Trig.PI / 2.0; Trig.PI; 2.5; 5.0 |] do
            let sine = Trig.sin value
            let cosine = Trig.cos value
            Assert.True(approxEqual (sine * sine + cosine * cosine) 1.0)

    [<Fact>]
    member _.``Range reduction and angle conversions behave``() =
        let approxEqual left right = abs (left - right) < 1e-10

        Assert.True(approxEqual (Trig.sin (1000.0 * Trig.PI)) 0.0)
        Assert.True(approxEqual (Trig.cos (500.0 * 2.0 * Trig.PI)) 1.0)
        Assert.True(approxEqual (Trig.radians 180.0) Trig.PI)
        Assert.True(approxEqual (Trig.radians 90.0) (Trig.PI / 2.0))
        Assert.True(approxEqual (Trig.degrees Trig.PI) 180.0)
        Assert.True(approxEqual (Trig.degrees (Trig.PI / 2.0)) 90.0)

        for degrees in [| 0.0; 45.0; 90.0; 180.0; 270.0; 360.0 |] do
            Assert.True(approxEqual (Trig.degrees (Trig.radians degrees)) degrees)

    [<Fact>]
    member _.``Square root matches reference values``() =
        let approxEqual left right = abs (left - right) < 1e-10

        Assert.Equal(0.0, Trig.sqrt 0.0)
        Assert.True(approxEqual (Trig.sqrt 1.0) 1.0)
        Assert.True(approxEqual (Trig.sqrt 4.0) 2.0)
        Assert.True(approxEqual (Trig.sqrt 9.0) 3.0)
        Assert.True(approxEqual (Trig.sqrt 2.0) 1.41421356237)
        Assert.True(approxEqual (Trig.sqrt 0.25) 0.5)
        Assert.True(abs (Trig.sqrt 1e10 - 1e5) < 1e-4)
        Assert.Throws<System.ArgumentOutOfRangeException>(fun () -> Trig.sqrt -1.0 |> ignore) |> ignore

    [<Fact>]
    member _.``Tan atan and atan2 cover standard identities``() =
        let approxEqual left right = abs (left - right) < 1e-10

        Assert.True(approxEqual (Trig.tan 0.0) 0.0)
        Assert.True(approxEqual (Trig.tan (Trig.PI / 4.0)) 1.0)
        Assert.True(approxEqual (Trig.tan (Trig.PI / 6.0)) (1.0 / Trig.sqrt 3.0))
        Assert.True(approxEqual (Trig.tan (-Trig.PI / 4.0)) -1.0)

        Assert.Equal(0.0, Trig.atan 0.0)
        Assert.True(approxEqual (Trig.atan 1.0) (Trig.PI / 4.0))
        Assert.True(approxEqual (Trig.atan -1.0) (-Trig.PI / 4.0))
        Assert.True(approxEqual (Trig.atan (Trig.sqrt 3.0)) (Trig.PI / 3.0))
        Assert.True(approxEqual (Trig.atan (1.0 / Trig.sqrt 3.0)) (Trig.PI / 6.0))
        Assert.True(abs (Trig.atan 1e10 - Trig.PI / 2.0) < 1e-5)
        Assert.True(abs (Trig.atan -1e10 + Trig.PI / 2.0) < 1e-5)

        Assert.True(approxEqual (Trig.atan2 0.0 1.0) 0.0)
        Assert.True(approxEqual (Trig.atan2 1.0 0.0) (Trig.PI / 2.0))
        Assert.True(approxEqual (Trig.atan2 0.0 -1.0) Trig.PI)
        Assert.True(approxEqual (Trig.atan2 -1.0 0.0) (-Trig.PI / 2.0))
        Assert.True(approxEqual (Trig.atan2 1.0 1.0) (Trig.PI / 4.0))
        Assert.True(approxEqual (Trig.atan2 1.0 -1.0) (3.0 * Trig.PI / 4.0))
        Assert.True(approxEqual (Trig.atan2 -1.0 -1.0) (-3.0 * Trig.PI / 4.0))
        Assert.True(approxEqual (Trig.atan2 -1.0 1.0) (-Trig.PI / 4.0))
