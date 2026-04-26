namespace CodingAdventures.Aes.FSharp

open System
open System.Security.Cryptography

[<RequireQualifiedAccess>]
module AesBlock =
    let private blockSizeBytes = 16

    let private validateBlock (block: byte array) =
        if isNull block then nullArg "block"
        if block.Length <> blockSizeBytes then
            invalidArg "block" "AES block must be exactly 16 bytes."

    let private validateKey (key: byte array) =
        if isNull key then nullArg "key"
        if key.Length <> 16 && key.Length <> 24 && key.Length <> 32 then
            invalidArg "key" "AES key must be 16, 24, or 32 bytes."

    let private createAes (key: byte array) =
        let aes = Aes.Create()
        aes.Mode <- CipherMode.ECB
        aes.Padding <- PaddingMode.None
        aes.Key <- Array.copy key
        aes

    let encryptBlock (block: byte array) (key: byte array) =
        validateBlock block
        validateKey key
        use aes = createAes key
        use encryptor = aes.CreateEncryptor()
        encryptor.TransformFinalBlock(block, 0, block.Length)

    let decryptBlock (block: byte array) (key: byte array) =
        validateBlock block
        validateKey key
        use aes = createAes key
        use decryptor = aes.CreateDecryptor()
        decryptor.TransformFinalBlock(block, 0, block.Length)
