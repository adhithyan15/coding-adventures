namespace CodingAdventures.Hmac.FSharp

open System
open System.Security.Cryptography

[<RequireQualifiedAccess>]
module Hmac =
    let private ipad = 0x36uy
    let private opad = 0x5cuy

    let private ensureNonEmptyKey (key: byte array) =
        if key.Length = 0 then invalidArg "key" "HMAC key must not be empty."

    let private concat (left: byte array) (right: byte array) =
        Array.concat [ left; right ]

    let private normalizeKey (hashFunction: byte array -> byte array) blockSize (key: byte array) =
        let normalized =
            if key.Length > blockSize then
                hashFunction key
            else
                Array.copy key

        if normalized.Length > blockSize then
            invalidArg "hashFunction" "Hash output must not exceed the HMAC block size."

        Array.append normalized (Array.zeroCreate (blockSize - normalized.Length))

    let compute (hashFunction: byte array -> byte array) blockSize (key: byte array) (message: byte array) =
        if isNull (box hashFunction) then nullArg "hashFunction"
        if isNull key then nullArg "key"
        if isNull message then nullArg "message"
        if blockSize <= 0 then invalidArg "blockSize" "Block size must be positive."
        ensureNonEmptyKey key

        let keyPrime = normalizeKey hashFunction blockSize key
        let innerKey = keyPrime |> Array.map (fun value -> value ^^^ ipad)
        let outerKey = keyPrime |> Array.map (fun value -> value ^^^ opad)
        let inner = hashFunction (concat innerKey message)
        hashFunction (concat outerKey inner)

    let hmacMd5 key message = compute MD5.HashData 64 key message

    let hmacSha1 key message = compute SHA1.HashData 64 key message

    let hmacSha256 key message = compute SHA256.HashData 64 key message

    let hmacSha512 key message = compute SHA512.HashData 128 key message

    let private toHex (data: byte array) =
        Convert.ToHexString(data).ToLowerInvariant()

    let hmacMd5Hex key message = hmacMd5 key message |> toHex

    let hmacSha1Hex key message = hmacSha1 key message |> toHex

    let hmacSha256Hex key message = hmacSha256 key message |> toHex

    let hmacSha512Hex key message = hmacSha512 key message |> toHex

    let verify (expected: byte array) (actual: byte array) =
        if isNull expected then nullArg "expected"
        if isNull actual then nullArg "actual"
        CryptographicOperations.FixedTimeEquals(expected, actual)
