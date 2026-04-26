namespace CodingAdventures.Sha512.FSharp

open System
open System.Collections.Generic
open System.Security.Cryptography

[<RequireQualifiedAccess>]
module Sha512 =
    [<Literal>]
    let DigestLength = 64

    let hash (data: byte array) =
        if isNull data then nullArg "data"
        SHA512.HashData(data)

    let hashHex (data: byte array) =
        hash data |> Convert.ToHexString |> fun value -> value.ToLowerInvariant()

type Sha512Hasher() =
    let data = List<byte>()

    member this.Update(bytes: byte array) =
        if isNull bytes then nullArg "bytes"
        data.AddRange(bytes)
        this

    member _.Digest() =
        data.ToArray() |> SHA512.HashData

    member this.HexDigest() =
        this.Digest() |> Convert.ToHexString |> fun value -> value.ToLowerInvariant()

    member _.Copy() =
        let copy = Sha512Hasher()
        copy.Update(data.ToArray()) |> ignore
        copy
