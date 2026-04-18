namespace CodingAdventures.PixelContainer.Tests

open System
open CodingAdventures.PixelContainer
open Xunit

type PixelContainerTests() =
    [<Fact>]
    member _.``Version is semver``() =
        Assert.Equal("0.1.0", PixelContainers.VERSION)

    [<Fact>]
    member _.``Create allocates width times height times four bytes``() =
        let pixels = PixelContainers.create 4 3
        Assert.Equal(4, pixels.Width)
        Assert.Equal(3, pixels.Height)
        Assert.Equal(48, pixels.Data.Length)

    [<Fact>]
    member _.``Constructor with data requires exact length``() =
        let error =
            Assert.Throws<ArgumentException>(fun () -> PixelContainer(2, 2, Array.zeroCreate<byte> 15) |> ignore)

        Assert.Contains("width * height * 4", error.Message)

    [<Fact>]
    member _.``Fresh containers start transparent black``() =
        let pixels = PixelContainers.create 2 2
        Assert.All(pixels.Data, fun value -> Assert.Equal(0uy, value))
        Assert.Equal({ R = 0uy; G = 0uy; B = 0uy; A = 0uy }, pixels.GetPixel(1, 1))

    [<Fact>]
    member _.``GetPixel uses row major offsets``() =
        let pixels = PixelContainers.create 3 2
        pixels.Data[20] <- 11uy
        pixels.Data[21] <- 22uy
        pixels.Data[22] <- 33uy
        pixels.Data[23] <- 44uy

        Assert.Equal({ R = 11uy; G = 22uy; B = 33uy; A = 44uy }, pixels.GetPixel(2, 1))

    [<Fact>]
    member _.``GetPixel returns transparent black for out of bounds``() =
        let pixels = PixelContainers.create 3 3
        let zero = { R = 0uy; G = 0uy; B = 0uy; A = 0uy }
        Assert.Equal(zero, pixels.GetPixel(-1, 0))
        Assert.Equal(zero, pixels.GetPixel(3, 0))
        Assert.Equal(zero, pixels.GetPixel(0, 3))

    [<Fact>]
    member _.``SetPixel writes RGBA values``() =
        let pixels = PixelContainers.create 2 2
        pixels.SetPixel(1, 0, 200uy, 100uy, 50uy, 255uy)
        Assert.Equal({ R = 200uy; G = 100uy; B = 50uy; A = 255uy }, pixels.GetPixel(1, 0))

    [<Fact>]
    member _.``SetPixel is no op when out of bounds``() =
        let pixels = PixelContainers.create 2 2
        pixels.SetPixel(50, 50, 1uy, 2uy, 3uy, 4uy)
        Assert.All(pixels.Data, fun value -> Assert.Equal(0uy, value))

    [<Fact>]
    member _.``Fill overwrites the whole buffer``() =
        let pixels = PixelContainers.create 3 2
        pixels.SetPixel(0, 0, 1uy, 2uy, 3uy, 4uy)
        pixels.Fill(100uy, 150uy, 200uy, 255uy)

        for y in 0 .. pixels.Height - 1 do
            for x in 0 .. pixels.Width - 1 do
                Assert.Equal({ R = 100uy; G = 150uy; B = 200uy; A = 255uy }, pixels.GetPixel(x, y))

    [<Fact>]
    member _.``ImageCodec can be implemented by a plain object``() =
        let codec =
            { new IImageCodec with
                member _.MimeType = "image/test"
                member _.Encode(pixels: PixelContainer) = [| byte pixels.Width; byte pixels.Height |]
                member _.Decode(bytes: byte array) = PixelContainers.create (int bytes[0]) (int bytes[1]) }

        let pixels = PixelContainers.create 3 2
        let encoded = codec.Encode(pixels)
        let decoded = codec.Decode(encoded)

        Assert.Equal("image/test", codec.MimeType)
        Assert.Equal<byte>([| 3uy; 2uy |], encoded)
        Assert.Equal(3, decoded.Width)
        Assert.Equal(2, decoded.Height)
