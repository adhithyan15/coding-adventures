namespace CodingAdventures.PixelContainer

open System

[<Struct>]
type Rgba =
    {
        R: byte
        G: byte
        B: byte
        A: byte
    }

/// A fixed RGBA8 pixel buffer with row-major layout and a top-left origin.
type PixelContainer(width: int, height: int, ?data: byte array) =
    do
        if width < 0 then
            invalidArg "width" "width must be non-negative"

        if height < 0 then
            invalidArg "height" "height must be non-negative"

    let expectedLength =
        let length64 = int64 width * int64 height * 4L

        if length64 > int64 Int32.MaxValue then
            invalidArg "width" "width * height * 4 exceeds the supported array size"

        int length64

    let backing =
        match data with
        | Some provided when isNull provided -> nullArg "data"
        | Some provided when provided.Length <> expectedLength ->
            invalidArg "data" $"data length must be width * height * 4 ({expectedLength})"
        | Some provided -> provided
        | None -> Array.zeroCreate<byte> expectedLength

    member _.Width = width
    member _.Height = height
    member _.Data = backing

    member _.GetPixel(x: int, y: int) =
        if x < 0 || x >= width || y < 0 || y >= height then
            { R = 0uy; G = 0uy; B = 0uy; A = 0uy }
        else
            let index = ((y * width) + x) * 4

            {
                R = backing[index]
                G = backing[index + 1]
                B = backing[index + 2]
                A = backing[index + 3]
            }

    member _.SetPixel(x: int, y: int, r: byte, g: byte, b: byte, a: byte) =
        if x >= 0 && x < width && y >= 0 && y < height then
            let index = ((y * width) + x) * 4
            backing[index] <- r
            backing[index + 1] <- g
            backing[index + 2] <- b
            backing[index + 3] <- a

    member this.SetPixel(x: int, y: int, rgba: Rgba) =
        this.SetPixel(x, y, rgba.R, rgba.G, rgba.B, rgba.A)

    member _.Fill(r: byte, g: byte, b: byte, a: byte) =
        for index in 0 .. 4 .. (backing.Length - 4) do
            backing[index] <- r
            backing[index + 1] <- g
            backing[index + 2] <- b
            backing[index + 3] <- a

    member this.Fill(rgba: Rgba) =
        this.Fill(rgba.R, rgba.G, rgba.B, rgba.A)

type IImageCodec =
    abstract MimeType: string
    abstract Encode: PixelContainer -> byte array
    abstract Decode: byte array -> PixelContainer

[<RequireQualifiedAccess>]
module PixelContainers =
    [<Literal>]
    let VERSION = "0.1.0"

    let create width height = PixelContainer(width, height)

    let fromData width height data = PixelContainer(width, height, data)
