namespace CodingAdventures.Sha256.FSharp

open System
open System.Collections.Generic
open System.Security.Cryptography

[<RequireQualifiedAccess>]
module Sha256 =
    [<Literal>]
    let DigestLength = 32

    let hash (data: byte array) =
        if isNull data then nullArg "data"
        SHA256.HashData(data)

    let hashHex (data: byte array) =
        hash data |> Convert.ToHexString |> fun value -> value.ToLowerInvariant()

type Sha256Hasher() =
    let data = List<byte>()

    member this.Update(bytes: byte array) =
        if isNull bytes then nullArg "bytes"
        data.AddRange(bytes)
        this

    member _.Digest() =
        data.ToArray() |> SHA256.HashData

    member this.HexDigest() =
        this.Digest() |> Convert.ToHexString |> fun value -> value.ToLowerInvariant()

    member _.Copy() =
        let copy = Sha256Hasher()
        copy.Update(data.ToArray()) |> ignore
        copy
