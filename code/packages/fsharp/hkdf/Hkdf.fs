namespace CodingAdventures.Hkdf.FSharp

open System
open CodingAdventures.Hmac.FSharp

type HkdfHash =
    | Sha256
    | Sha512

[<RequireQualifiedAccess>]
module Hkdf =
    let private hashLength hash =
        match hash with
        | Sha256 -> 32
        | Sha512 -> 64

    let private computeHmac hash key message =
        match hash with
        | Sha256 -> Hmac.computeAllowEmptyKey CodingAdventures.Sha256.FSharp.Sha256.hash 64 key message
        | Sha512 -> Hmac.computeAllowEmptyKey CodingAdventures.Sha512.FSharp.Sha512.hash 128 key message

    let extract (salt: byte array) (ikm: byte array) hash =
        if isNull salt then nullArg "salt"
        if isNull ikm then nullArg "ikm"

        let actualSalt =
            if salt.Length = 0 then
                Array.zeroCreate (hashLength hash)
            else
                salt

        computeHmac hash actualSalt ikm

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
            let input = Array.concat [ previous; info; [| byte counter |] ]
            previous <- computeHmac hash prk input
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
