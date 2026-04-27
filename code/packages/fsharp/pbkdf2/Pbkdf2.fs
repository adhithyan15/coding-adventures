namespace CodingAdventures.Pbkdf2.FSharp

open System
open System.Buffers.Binary
open CodingAdventures.Hmac.FSharp

type Pbkdf2Hash =
    | Sha1
    | Sha256
    | Sha512

[<RequireQualifiedAccess>]
module Pbkdf2 =
    let private maxKeyLength = 1 <<< 20

    let private hashLength hashAlgorithm =
        match hashAlgorithm with
        | Sha1 -> 20
        | Sha256 -> 32
        | Sha512 -> 64

    let private computeHmac hashAlgorithm key message =
        match hashAlgorithm with
        | Sha1 -> Hmac.computeAllowEmptyKey CodingAdventures.Sha1.FSharp.Sha1.hash 64 key message
        | Sha256 -> Hmac.computeAllowEmptyKey CodingAdventures.Sha256.FSharp.Sha256.hash 64 key message
        | Sha512 -> Hmac.computeAllowEmptyKey CodingAdventures.Sha512.FSharp.Sha512.hash 128 key message

    let private deriveBlock (password: byte array) (salt: byte array) iterations hashAlgorithm blockIndex =
        let firstInput = Array.zeroCreate<byte> (salt.Length + 4)
        Buffer.BlockCopy(salt, 0, firstInput, 0, salt.Length)
        BinaryPrimitives.WriteUInt32BigEndian(firstInput.AsSpan(salt.Length, 4), uint32 blockIndex)

        let mutable u = computeHmac hashAlgorithm password firstInput
        let block = Array.copy u

        for _iteration in 1 .. iterations - 1 do
            u <- computeHmac hashAlgorithm password u

            for index in 0 .. block.Length - 1 do
                block[index] <- block[index] ^^^ u[index]

        block

    let derive (password: byte array) (salt: byte array) iterations keyLength hashAlgorithm allowEmptyPassword =
        if isNull password then nullArg "password"
        if isNull salt then nullArg "salt"
        if not allowEmptyPassword && password.Length = 0 then
            invalidArg "password" "PBKDF2 password must not be empty."
        if iterations <= 0 then invalidArg "iterations" "Iterations must be positive."
        if keyLength <= 0 then invalidArg "keyLength" "Key length must be positive."
        if keyLength > maxKeyLength then invalidArg "keyLength" "Key length must not exceed 2^20 bytes."

        let digestLength = hashLength hashAlgorithm
        let blockCount = (keyLength + digestLength - 1) / digestLength
        let derived = Array.zeroCreate<byte> (blockCount * digestLength)

        for blockIndex in 1 .. blockCount do
            let block = deriveBlock password salt iterations hashAlgorithm blockIndex
            Buffer.BlockCopy(block, 0, derived, (blockIndex - 1) * digestLength, digestLength)

        derived[0 .. keyLength - 1]

    let pbkdf2HmacSha1 password salt iterations keyLength =
        derive password salt iterations keyLength Sha1 false

    let pbkdf2HmacSha256 password salt iterations keyLength =
        derive password salt iterations keyLength Sha256 false

    let pbkdf2HmacSha256AllowEmptyPassword password salt iterations keyLength =
        derive password salt iterations keyLength Sha256 true

    let pbkdf2HmacSha512 password salt iterations keyLength =
        derive password salt iterations keyLength Sha512 false

    let private toHex (data: byte array) =
        Convert.ToHexString(data).ToLowerInvariant()

    let pbkdf2HmacSha1Hex password salt iterations keyLength =
        pbkdf2HmacSha1 password salt iterations keyLength |> toHex

    let pbkdf2HmacSha256Hex password salt iterations keyLength =
        pbkdf2HmacSha256 password salt iterations keyLength |> toHex

    let pbkdf2HmacSha512Hex password salt iterations keyLength =
        pbkdf2HmacSha512 password salt iterations keyLength |> toHex
