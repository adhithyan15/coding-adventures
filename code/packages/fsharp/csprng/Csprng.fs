namespace CodingAdventures.Csprng.FSharp

open System
open System.Security.Cryptography

[<RequireQualifiedAccess>]
module Csprng =
    let fillRandom (buffer: byte array) =
        if isNull buffer then nullArg "buffer"
        if buffer.Length = 0 then invalidArg "buffer" "Random byte request must not be empty."
        RandomNumberGenerator.Fill(buffer)

    let randomBytes length =
        if length <= 0 then invalidArg "length" "Random byte request length must be positive."
        let buffer = Array.zeroCreate<byte> length
        RandomNumberGenerator.Fill(buffer)
        buffer

    let randomUInt32 () =
        let bytes = randomBytes 4
        BitConverter.ToUInt32(bytes, 0)

    let randomUInt64 () =
        let bytes = randomBytes 8
        BitConverter.ToUInt64(bytes, 0)
