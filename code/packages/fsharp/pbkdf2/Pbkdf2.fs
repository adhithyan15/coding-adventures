namespace CodingAdventures.Pbkdf2.FSharp

open System
open System.Security.Cryptography

[<RequireQualifiedAccess>]
module Pbkdf2 =
    let private maxKeyLength = 1 <<< 20

    let derive (password: byte array) (salt: byte array) iterations keyLength hashAlgorithm allowEmptyPassword =
        if isNull password then nullArg "password"
        if isNull salt then nullArg "salt"
        if not allowEmptyPassword && password.Length = 0 then
            invalidArg "password" "PBKDF2 password must not be empty."
        if iterations <= 0 then invalidArg "iterations" "Iterations must be positive."
        if keyLength <= 0 then invalidArg "keyLength" "Key length must be positive."
        if keyLength > maxKeyLength then invalidArg "keyLength" "Key length must not exceed 2^20 bytes."

        Rfc2898DeriveBytes.Pbkdf2(password, salt, iterations, hashAlgorithm, keyLength)

    let pbkdf2HmacSha1 password salt iterations keyLength =
        derive password salt iterations keyLength HashAlgorithmName.SHA1 false

    let pbkdf2HmacSha256 password salt iterations keyLength =
        derive password salt iterations keyLength HashAlgorithmName.SHA256 false

    let pbkdf2HmacSha256AllowEmptyPassword password salt iterations keyLength =
        derive password salt iterations keyLength HashAlgorithmName.SHA256 true

    let pbkdf2HmacSha512 password salt iterations keyLength =
        derive password salt iterations keyLength HashAlgorithmName.SHA512 false

    let private toHex (data: byte array) =
        Convert.ToHexString(data).ToLowerInvariant()

    let pbkdf2HmacSha1Hex password salt iterations keyLength =
        pbkdf2HmacSha1 password salt iterations keyLength |> toHex

    let pbkdf2HmacSha256Hex password salt iterations keyLength =
        pbkdf2HmacSha256 password salt iterations keyLength |> toHex

    let pbkdf2HmacSha512Hex password salt iterations keyLength =
        pbkdf2HmacSha512 password salt iterations keyLength |> toHex
