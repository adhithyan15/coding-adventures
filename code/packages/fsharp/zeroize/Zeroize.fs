namespace CodingAdventures.Zeroize.FSharp

open System

[<RequireQualifiedAccess>]
module Zeroize =
    let zeroizeBytes (buffer: byte array) =
        if isNull buffer then nullArg "buffer"

        for index in 0 .. buffer.Length - 1 do
            buffer[index] <- 0uy

    let zeroizeChars (buffer: char array) =
        if isNull buffer then nullArg "buffer"
        Array.Clear(buffer)

    let zeroizeArray (buffer: 'T array) =
        if isNull buffer then nullArg "buffer"
        Array.Clear(buffer)

type ZeroizingBuffer(buffer: byte array) =
    let mutable disposed = false

    do
        if isNull buffer then nullArg "buffer"

    member _.Buffer = buffer

    interface IDisposable with
        member _.Dispose() =
            if not disposed then
                Zeroize.zeroizeBytes buffer
                disposed <- true
