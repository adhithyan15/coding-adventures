namespace CodingAdventures.Hkdf.FSharp

open System
open System.Security.Cryptography

type HkdfHash =
    | Sha256
    | Sha512

[<RequireQualifiedAccess>]
module Hkdf =
    let private hashLength hash =
        match hash with
        | Sha256 -> 32
        | Sha512 -> 64

    let private createHmac hash key =
        match hash with
        | Sha256 -> new HMACSHA256(key) :> HMAC
        | Sha512 -> new HMACSHA512(key) :> HMAC

    let extract (salt: byte array) (ikm: byte array) hash =
        if isNull salt then nullArg "salt"
        if isNull ikm then nullArg "ikm"

        let actualSalt =
            if salt.Length = 0 then
                Array.zeroCreate (hashLength hash)
            else
                salt

        use hmac = createHmac hash actualSalt
        hmac.ComputeHash(ikm)

    let expand (prk: byte array) (info: byte array) length hash =
        if isNull prk then nullArg "prk"
        if isNull info then nullArg "info"
        let digestLength = hashLength hash
        if length <= 0 then invalidArg "length" "HKDF output length must be positive."
        let maxLength = 255 * digestLength
        if length > maxLength then invalidArg "length" $"HKDF output length must not exceed {maxLength} bytes."

        let okm = Array.zeroCreate<byte> length
        let mutable previous = [||]
        let mutable offset = 0
        let mutable counter = 1

        while offset < length do
            use hmac = createHmac hash prk
            let input = Array.concat [ previous; info; [| byte counter |] ]
            previous <- hmac.ComputeHash(input)
            let toCopy = min previous.Length (length - offset)
            Buffer.BlockCopy(previous, 0, okm, offset, toCopy)
            offset <- offset + toCopy
            counter <- counter + 1

        okm

    let derive salt ikm info length hash =
        expand (extract salt ikm hash) info length hash

    let extractSha256 salt ikm = extract salt ikm Sha256

    let expandSha256 prk info length = expand prk info length Sha256

    let deriveSha256 salt ikm info length = derive salt ikm info length Sha256

    let extractSha512 salt ikm = extract salt ikm Sha512

    let expandSha512 prk info length = expand prk info length Sha512

    let deriveSha512 salt ikm info length = derive salt ikm info length Sha512
