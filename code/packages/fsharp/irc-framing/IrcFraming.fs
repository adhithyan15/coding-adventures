namespace CodingAdventures.IrcFraming.FSharp

open System

[<RequireQualifiedAccess>]
module IrcFraming =
    [<Literal>]
    let VERSION = "0.1.0"

    [<Literal>]
    let MAX_CONTENT_BYTES = 510

type Framer() =
    let buffer = ResizeArray<byte>()

    member _.BufferSize = buffer.Count

    member _.Feed(data: byte array) =
        if isNull data then nullArg "data"
        buffer.AddRange(data)

    member _.Frames() =
        let frames = ResizeArray<byte array>()
        let mutable keepGoing = true

        while keepGoing do
            let mutable lfPos = -1
            let mutable index = 0

            while lfPos = -1 && index < buffer.Count do
                if buffer[index] = byte '\n' then
                    lfPos <- index
                else
                    index <- index + 1

            if lfPos < 0 then
                keepGoing <- false
            else
                let contentEnd =
                    if lfPos > 0 && buffer[lfPos - 1] = byte '\r' then
                        lfPos - 1
                    else
                        lfPos

                let line = buffer.GetRange(0, contentEnd).ToArray()
                buffer.RemoveRange(0, lfPos + 1)

                if line.Length <= IrcFraming.MAX_CONTENT_BYTES then
                    frames.Add(line)

        frames |> Seq.toList

    member _.Reset() = buffer.Clear()
