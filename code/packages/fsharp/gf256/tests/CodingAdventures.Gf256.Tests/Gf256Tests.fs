namespace CodingAdventures.Gf256.Tests

open System
open Xunit
open CodingAdventures.Gf256

type Gf256Tests() =
    [<Fact>]
    member _.``exposes expected constants``() =
        Assert.Equal("0.1.0", Gf256.VERSION)
        Assert.Equal(0uy, Gf256.ZERO)
        Assert.Equal(1uy, Gf256.ONE)
        Assert.Equal(0x11d, Gf256.PRIMITIVE_POLYNOMIAL)

    [<Fact>]
    member _.``log and alog tables match known values``() =
        Assert.Equal(256, Gf256.ALOG.Length)
        Assert.Equal(256, Gf256.LOG.Length)
        Assert.Equal(1uy, Gf256.ALOG[0])
        Assert.Equal(2uy, Gf256.ALOG[1])
        Assert.Equal(29uy, Gf256.ALOG[8])
        Assert.Equal(0, Gf256.LOG[1])
        Assert.Equal(1, Gf256.LOG[2])

        for value in 1 .. 255 do
            Assert.Equal(byte value, Gf256.ALOG[Gf256.LOG[value]])

    [<Fact>]
    member _.``add and subtract are xor``() =
        for value in 0 .. 255 do
            Assert.Equal(byte value, Gf256.add 0uy (byte value))
            Assert.Equal(0uy, Gf256.add (byte value) (byte value))
            Assert.Equal(Gf256.add (byte value) 0x42uy, Gf256.subtract (byte value) 0x42uy)

        Assert.Equal(0x99uy, Gf256.add 0x53uy 0xcauy)

    [<Fact>]
    member _.``multiply obeys identity zero and known spot checks``() =
        for value in 0 .. 255 do
            Assert.Equal(0uy, Gf256.multiply (byte value) 0uy)
            Assert.Equal(byte value, Gf256.multiply (byte value) 1uy)

        Assert.Equal(1uy, Gf256.multiply 0x53uy 0x8cuy)
        Assert.Equal(Gf256.multiply 0x34uy 0x56uy, Gf256.multiply 0x56uy 0x34uy)

    [<Fact>]
    member _.``divide and inverse work for non-zero inputs``() =
        for value in 1 .. 255 do
            Assert.Equal(1uy, Gf256.divide (byte value) (byte value))
            Assert.Equal(byte value, Gf256.divide (byte value) 1uy)
            Assert.Equal(1uy, Gf256.multiply (byte value) (Gf256.inverse (byte value)))

        Assert.Equal(0uy, Gf256.divide 0uy 1uy)
        Assert.Throws<InvalidOperationException>(fun () -> Gf256.divide 1uy 0uy |> ignore)
        |> ignore
        Assert.Throws<InvalidOperationException>(fun () -> Gf256.inverse 0uy |> ignore)
        |> ignore

    [<Fact>]
    member _.``power handles zero and positive exponents``() =
        Assert.Equal(1uy, Gf256.power 0uy 0)
        Assert.Equal(0uy, Gf256.power 0uy 5)
        Assert.Equal(1uy, Gf256.power 0x53uy 0)
        Assert.Equal(0x53uy, Gf256.power 0x53uy 1)
        Assert.Equal(Gf256.multiply 0x53uy 0x53uy, Gf256.power 0x53uy 2)
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> Gf256.power 1uy -1 |> ignore)
        |> ignore

    [<Fact>]
    member _.``zero and one helpers return field identities``() =
        Assert.Equal(Gf256.ZERO, Gf256.zero ())
        Assert.Equal(Gf256.ONE, Gf256.one ())

    [<Fact>]
    member _.``alternate field supports aes polynomial``() =
        let aes = Gf256.createField 0x11b

        Assert.Equal(0x11b, aes.Polynomial)
        Assert.Equal(0x01uy, aes.Multiply(0x53uy, 0xcauy))
        Assert.Equal(0xc1uy, aes.Multiply(0x57uy, 0x83uy))
        Assert.Equal(0x53uy, aes.Divide(0x53uy, 1uy))
        Assert.Equal(0x01uy, aes.Multiply(0x57uy, aes.Inverse(0x57uy)))
        Assert.Throws<InvalidOperationException>(fun () -> aes.Divide(1uy, 0uy) |> ignore)
        |> ignore
        Assert.Throws<InvalidOperationException>(fun () -> aes.Inverse(0uy) |> ignore)
        |> ignore
