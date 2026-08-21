namespace CodingAdventures.ImageCodecPng.FSharp

open System
open System.Buffers.Binary
open System.Collections.Generic
open System.Collections.ObjectModel
open System.IO
open System.Text
open CodingAdventures.PixelContainer
open CodingAdventures.Zip.FSharp

/// A stable, payload-blind IC18 PNG failure.
type PngError(code: string) =
    inherit Exception(code)
    member _.Code = code

/// Pure in-memory IC18 PNG framing, zlib wrapping, filtering, encoding, and decoding.
[<RequireQualifiedAccess>]
module Png =
    [<Literal>]
    let maxDimension = 16384

    [<Literal>]
    let defaultMaxPixels = 32 * 1024 * 1024

    let private adlerMod = 65521u
    let private signature = [| 0x89uy; 0x50uy; 0x4euy; 0x47uy; 0x0duy; 0x0auy; 0x1auy; 0x0auy |]

    let private errorCodeValues =
        [| "invalid-max-pixels"
           "invalid-image-dimensions"
           "invalid-pixel-data-length"
           "file-too-short"
           "invalid-signature"
           "truncated-chunk"
           "invalid-chunk-type"
           "chunk-crc-mismatch"
           "chunk-before-ihdr"
           "duplicate-ihdr"
           "invalid-ihdr-length"
           "invalid-dimensions"
           "dimension-limit"
           "pixel-limit"
           "unsupported-feature"
           "invalid-plte"
           "invalid-trns"
           "nonconsecutive-idat"
           "invalid-iend"
           "trailing-data"
           "unknown-critical-chunk"
           "missing-required-chunk"
           "invalid-zlib-header"
           "preset-dictionary"
           "inflate-failed"
           "inflated-length-mismatch"
           "idat-cavity"
           "adler-mismatch"
           "invalid-filter" |]

    /// The closed IC18 error taxonomy in normative order.
    let errorCodes () = Array.copy errorCodeValues

    let private fail code = raise (PngError(code))

    let internal validateMaxPixels value =
        match value with
        | None -> defaultMaxPixels
        | Some maximum when
            not (Double.IsFinite(maximum)) || Math.Truncate(maximum) <> maximum ||
            maximum <= 0.0 || maximum > float defaultMaxPixels -> fail "invalid-max-pixels"
        | Some maximum -> int maximum

    /// Compute the RFC 1950 Adler-32 checksum.
    let adler32 (data: byte[]) =
        if isNull data then nullArg "data"
        let mutable a = 1u
        let mutable b = 0u
        let mutable start = 0
        while start < data.Length do
            let finish = min (start + 5552) data.Length
            for index in start .. finish - 1 do
                a <- a + uint32 data[index]
                b <- b + a
            a <- a % adlerMod
            b <- b % adlerMod
            start <- finish
        (b <<< 16) ||| a

    let private paeth (a: byte) (b: byte) (c: byte) =
        let p = int a + int b - int c
        let pa = abs (p - int a)
        let pb = abs (p - int b)
        let pc = abs (p - int c)
        if pa <= pb && pa <= pc then a
        elif pb <= pc then b
        else c

    let private applyFilter filter (raw: byte[]) (prior: byte[]) bytesPerPixel (output: byte[]) =
        for index in 0 .. raw.Length - 1 do
            let left = if index >= bytesPerPixel then raw[index - bytesPerPixel] else 0uy
            let aboveLeft = if index >= bytesPerPixel then prior[index - bytesPerPixel] else 0uy
            let predicted =
                match filter with
                | 1uy -> left
                | 2uy -> prior[index]
                | 3uy -> byte ((uint16 left + uint16 prior[index]) / 2us)
                | 4uy -> paeth left prior[index] aboveLeft
                | _ -> 0uy
            output[index] <- byte (int raw[index] - int predicted &&& 0xff)

    let private chooseFilter (raw: byte[]) (prior: byte[]) bytesPerPixel (scratch: byte[]) (best: byte[]) =
        let mutable bestFilter = 0uy
        let mutable bestScore = Int32.MaxValue
        for filter in 0uy .. 4uy do
            applyFilter filter raw prior bytesPerPixel scratch
            let mutable score = 0
            for value in scratch do
                score <- score + if value < 128uy then int value else 256 - int value
            if score < bestScore then
                bestScore <- score
                bestFilter <- filter
                Array.Copy(scratch, best, scratch.Length)
        bestFilter

    let private undoFilter filter (row: byte[]) (prior: byte[]) bytesPerPixel =
        match filter with
        | 0uy -> ()
        | 1uy ->
            for index in bytesPerPixel .. row.Length - 1 do
                row[index] <- byte (int row[index] + int row[index - bytesPerPixel] &&& 0xff)
        | 2uy ->
            for index in 0 .. row.Length - 1 do
                row[index] <- byte (int row[index] + int prior[index] &&& 0xff)
        | 3uy ->
            for index in 0 .. row.Length - 1 do
                let left = if index >= bytesPerPixel then row[index - bytesPerPixel] else 0uy
                let prediction = (uint16 left + uint16 prior[index]) / 2us
                row[index] <- byte (int row[index] + int prediction &&& 0xff)
        | 4uy ->
            for index in 0 .. row.Length - 1 do
                let left = if index >= bytesPerPixel then row[index - bytesPerPixel] else 0uy
                let aboveLeft = if index >= bytesPerPixel then prior[index - bytesPerPixel] else 0uy
                row[index] <- byte (int row[index] + int (paeth left prior[index] aboveLeft) &&& 0xff)
        | _ -> fail "invalid-filter"

    let private validChunkType (chunkType: byte[]) =
        chunkType.Length = 4 && (chunkType[2] &&& 0x20uy) = 0uy &&
        (chunkType |> Array.forall (fun value ->
            (value >= byte 'A' && value <= byte 'Z') || (value >= byte 'a' && value <= byte 'z')))

    let private writeChunk (output: Stream) (chunkType: string) (data: byte[]) =
        let length = Array.zeroCreate<byte> 4
        BinaryPrimitives.WriteUInt32BigEndian(length, uint32 data.Length)
        output.Write(length)
        let typeBytes = Encoding.ASCII.GetBytes(chunkType)
        output.Write(typeBytes)
        output.Write(data)
        let checksum = RawRfc1951.crc32 data (RawRfc1951.crc32 typeBytes 0u)
        let checksumBytes = Array.zeroCreate<byte> 4
        BinaryPrimitives.WriteUInt32BigEndian(checksumBytes, checksum)
        output.Write(checksumBytes)

    /// Encode RGBA8 pixels as a bounded, non-interlaced PNG.
    let encodePng (pixels: PixelContainer) =
        if obj.ReferenceEquals(pixels, null) || pixels.Width <= 0 || pixels.Height <= 0 ||
           pixels.Width > maxDimension || pixels.Height > maxDimension then
            fail "invalid-image-dimensions"
        let pixelCount = int64 pixels.Width * int64 pixels.Height
        if pixelCount > int64 defaultMaxPixels then fail "invalid-image-dimensions"
        if isNull pixels.Data || int64 pixels.Data.LongLength <> pixelCount * 4L then
            fail "invalid-pixel-data-length"

        use output = new MemoryStream()
        output.Write(signature)
        let ihdr = Array.zeroCreate<byte> 13
        BinaryPrimitives.WriteUInt32BigEndian(ihdr, uint32 pixels.Width)
        BinaryPrimitives.WriteUInt32BigEndian(ihdr.AsSpan(4), uint32 pixels.Height)
        ihdr[8] <- 8uy
        ihdr[9] <- 6uy
        writeChunk output "IHDR" ihdr

        let stride = pixels.Width * 4
        let filtered = Array.zeroCreate<byte> (pixels.Height * (stride + 1))
        let prior = Array.zeroCreate<byte> stride
        let scratch = Array.zeroCreate<byte> stride
        let best = Array.zeroCreate<byte> stride
        for rowIndex in 0 .. pixels.Height - 1 do
            let raw = pixels.Data[rowIndex * stride .. (rowIndex + 1) * stride - 1]
            let destination = rowIndex * (stride + 1)
            filtered[destination] <- chooseFilter raw prior 4 scratch best
            Array.Copy(best, 0, filtered, destination + 1, stride)
            Array.Copy(raw, prior, stride)

        let deflated = RawRfc1951.rawDeflate filtered
        let idat = Array.zeroCreate<byte> (deflated.Length + 6)
        idat[0] <- 0x78uy
        idat[1] <- 0x9cuy
        Array.Copy(deflated, 0, idat, 2, deflated.Length)
        BinaryPrimitives.WriteUInt32BigEndian(idat.AsSpan(idat.Length - 4), adler32 filtered)
        writeChunk output "IDAT" idat
        writeChunk output "IEND" [||]
        output.ToArray()

    /// Decode the bounded, non-interlaced 8-bit IC18 PNG profile.
    let decodePng (data: byte[]) (maxPixels: float option) =
        let activeLimit = validateMaxPixels maxPixels
        if isNull data then nullArg "data"
        if data.Length < signature.Length then fail "file-too-short"
        if data[.. signature.Length - 1] <> signature then fail "invalid-signature"

        let mutable width = 0
        let mutable height = 0
        let mutable bitDepth = 0uy
        let mutable colourType = 0uy
        let mutable sawIhdr = false
        let mutable sawIend = false
        let mutable sawPlte = false
        let mutable sawTrns = false
        let mutable inIdat = false
        let mutable idatEnded = false
        let mutable transparentGrey: byte option = None
        let mutable transparentRgb: (byte * byte * byte) option = None
        let idatParts = ResizeArray<byte[]>()
        let mutable position = signature.Length

        while position < data.Length do
            if data.Length - position < 8 then fail "truncated-chunk"
            let length = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(position, 4))
            let chunkEnd = int64 position + 12L + int64 length
            if chunkEnd > int64 data.LongLength then fail "truncated-chunk"
            let typeStart = position + 4
            let dataStart = position + 8
            let dataEnd = int (int64 dataStart + int64 length)
            let typeBytes = data[typeStart .. dataStart - 1]
            if not (validChunkType typeBytes) then fail "invalid-chunk-type"
            let declaredCrc = BinaryPrimitives.ReadUInt32BigEndian(data.AsSpan(dataEnd, 4))
            if RawRfc1951.crc32 data[typeStart .. dataEnd - 1] 0u <> declaredCrc then
                fail "chunk-crc-mismatch"
            let chunkType = Encoding.ASCII.GetString(typeBytes)
            let chunkData = if length = 0u then [||] else data[dataStart .. dataEnd - 1]
            if not sawIhdr && chunkType <> "IHDR" then fail "chunk-before-ihdr"

            match chunkType with
            | "IHDR" ->
                if sawIhdr then fail "duplicate-ihdr"
                if length <> 13u then fail "invalid-ihdr-length"
                let widthRaw = BinaryPrimitives.ReadUInt32BigEndian(chunkData)
                let heightRaw = BinaryPrimitives.ReadUInt32BigEndian(ReadOnlySpan<byte>(chunkData, 4, chunkData.Length - 4))
                if widthRaw = 0u || heightRaw = 0u then fail "invalid-dimensions"
                if widthRaw > uint32 maxDimension || heightRaw > uint32 maxDimension then fail "dimension-limit"
                width <- int widthRaw
                height <- int heightRaw
                bitDepth <- chunkData[8]
                colourType <- chunkData[9]
                if int64 width * int64 height > int64 activeLimit then fail "pixel-limit"
                if chunkData[10] <> 0uy || chunkData[11] <> 0uy || chunkData[12] <> 0uy ||
                   bitDepth <> 8uy || not (colourType = 0uy || colourType = 2uy || colourType = 4uy || colourType = 6uy) then
                    fail "unsupported-feature"
                sawIhdr <- true

            | "PLTE" ->
                if sawPlte || idatParts.Count > 0 || sawTrns ||
                   not (colourType = 2uy || colourType = 6uy) ||
                   length < 3u || length > 768u || length % 3u <> 0u then
                    fail "invalid-plte"
                sawPlte <- true

            | "tRNS" ->
                if sawTrns || idatParts.Count > 0 then fail "invalid-trns"
                match colourType with
                | 0uy ->
                    if length <> 2u then fail "invalid-trns"
                    let value = BinaryPrimitives.ReadUInt16BigEndian(chunkData)
                    if value > uint16 Byte.MaxValue then fail "invalid-trns"
                    transparentGrey <- Some(byte value)
                | 2uy ->
                    if length <> 6u then fail "invalid-trns"
                    let red = BinaryPrimitives.ReadUInt16BigEndian(chunkData)
                    let green = BinaryPrimitives.ReadUInt16BigEndian(ReadOnlySpan<byte>(chunkData, 2, chunkData.Length - 2))
                    let blue = BinaryPrimitives.ReadUInt16BigEndian(ReadOnlySpan<byte>(chunkData, 4, chunkData.Length - 4))
                    if red > uint16 Byte.MaxValue || green > uint16 Byte.MaxValue || blue > uint16 Byte.MaxValue then
                        fail "invalid-trns"
                    transparentRgb <- Some(byte red, byte green, byte blue)
                | _ -> fail "invalid-trns"
                sawTrns <- true

            | "IDAT" ->
                if idatEnded then fail "nonconsecutive-idat"
                idatParts.Add(chunkData)
                inIdat <- true

            | "IEND" ->
                if length <> 0u then fail "invalid-iend"
                if chunkEnd <> int64 data.LongLength then fail "trailing-data"
                sawIend <- true

            | "acTL" | "fcTL" | "fdAT" -> fail "unsupported-feature"

            | _ ->
                if (typeBytes[0] &&& 0x20uy) = 0uy then fail "unknown-critical-chunk"

            if chunkType <> "IDAT" && inIdat then
                inIdat <- false
                idatEnded <- true
            position <- int chunkEnd

        if not sawIhdr || not sawIend || idatParts.Count = 0 then fail "missing-required-chunk"
        let zlibLength = idatParts |> Seq.sumBy (fun part -> int64 part.LongLength)
        if zlibLength > int64 data.LongLength || zlibLength > int64 Int32.MaxValue then fail "truncated-chunk"
        let zlibData = idatParts |> Seq.toArray |> Array.concat
        if zlibData.Length < 6 then fail "invalid-zlib-header"
        let cmf = zlibData[0]
        let flg = zlibData[1]
        if (cmf &&& 0x0fuy) <> 8uy || (cmf >>> 4) > 7uy ||
           ((int cmf <<< 8) ||| int flg) % 31 <> 0 then fail "invalid-zlib-header"
        if (flg &&& 0x20uy) <> 0uy then fail "preset-dictionary"

        let channels =
            match colourType with
            | 0uy -> 1
            | 2uy -> 3
            | 4uy -> 2
            | _ -> 4
        let stride64 = int64 width * int64 channels
        let expected64 = int64 height * (stride64 + 1L)
        let expected = int expected64
        let deflateData = zlibData[2 .. zlibData.Length - 5]
        let inflated =
            try RawRfc1951.rawInflateCounted deflateData expected
            with :? RawInflateError as error ->
                fail (if error.Code = "output-limit-exceeded" then "inflated-length-mismatch" else "inflate-failed")
        if inflated.Output.Length <> expected then fail "inflated-length-mismatch"
        if inflated.BytesConsumed <> deflateData.Length then fail "idat-cavity"
        if adler32 inflated.Output <> BinaryPrimitives.ReadUInt32BigEndian(ReadOnlySpan<byte>(zlibData, zlibData.Length - 4, 4)) then
            fail "adler-mismatch"

        let stride = int stride64
        let rowSize = stride + 1
        for rowIndex in 0 .. height - 1 do
            if inflated.Output[rowIndex * rowSize] > 4uy then fail "invalid-filter"

        let container = PixelContainer(width, height)
        let prior = Array.zeroCreate<byte> stride
        for rowIndex in 0 .. height - 1 do
            let at = rowIndex * rowSize
            let filter = inflated.Output[at]
            let row = inflated.Output[at + 1 .. at + rowSize - 1]
            undoFilter filter row prior channels
            let destinationRow = rowIndex * width * 4
            for x in 0 .. width - 1 do
                let source = x * channels
                let destination = destinationRow + x * 4
                match channels with
                | 1 ->
                    let grey = row[source]
                    container.Data[destination] <- grey
                    container.Data[destination + 1] <- grey
                    container.Data[destination + 2] <- grey
                    container.Data[destination + 3] <- if transparentGrey = Some grey then 0uy else 255uy
                | 2 ->
                    let grey = row[source]
                    container.Data[destination] <- grey
                    container.Data[destination + 1] <- grey
                    container.Data[destination + 2] <- grey
                    container.Data[destination + 3] <- row[source + 1]
                | 3 ->
                    let red, green, blue = row[source], row[source + 1], row[source + 2]
                    container.Data[destination] <- red
                    container.Data[destination + 1] <- green
                    container.Data[destination + 2] <- blue
                    container.Data[destination + 3] <-
                        if transparentRgb = Some(red, green, blue) then 0uy else 255uy
                | _ -> Array.Copy(row, source, container.Data, destination, 4)
            Array.Copy(row, prior, stride)
        container

/// An IImageCodec adapter for the bounded PNG profile.
type PngCodec(?maxPixels: float) =
    let activeLimit =
        Png.validateMaxPixels maxPixels |> ignore
        maxPixels

    interface IImageCodec with
        member _.MimeType = "image/png"
        member _.Encode(pixels) = Png.encodePng pixels
        member _.Decode(bytes) = Png.decodePng bytes activeLimit
