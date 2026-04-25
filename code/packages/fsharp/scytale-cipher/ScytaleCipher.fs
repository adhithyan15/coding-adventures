namespace CodingAdventures.ScytaleCipher

open System

type BruteForceResult = {
    Key: int
    Text: string
}

[<RequireQualifiedAccess>]
module ScytaleCipher =
    let private validateKey textLength key =
        if key < 2 then
            invalidArg (nameof key) "Key must be >= 2."

        if key > textLength then
            invalidArg (nameof key) "Key must be <= text length."

    let encrypt (text: string) (key: int) =
        if isNull text then
            nullArg (nameof text)

        if text.Length = 0 then
            String.Empty
        else
            validateKey text.Length key

            let rowCount = (text.Length + key - 1) / key
            let paddedLength = rowCount * key
            let padded = text.PadRight(paddedLength).ToCharArray()

            [|
                for column in 0 .. key - 1 do
                    for row in 0 .. rowCount - 1 do
                        padded[(row * key) + column]
            |]
            |> fun chars -> String(chars)

    let decrypt (text: string) (key: int) =
        if isNull text then
            nullArg (nameof text)

        if text.Length = 0 then
            String.Empty
        else
            validateKey text.Length key

            let rowCount = (text.Length + key - 1) / key
            let fullColumns = if text.Length % key = 0 then key else text.Length % key
            let columnStarts = Array.zeroCreate<int> key
            let columnLengths = Array.zeroCreate<int> key
            let mutable offset = 0

            for column in 0 .. key - 1 do
                columnStarts[column] <- offset
                let columnLength =
                    if text.Length % key = 0 || column < fullColumns then rowCount else rowCount - 1

                columnLengths[column] <- columnLength
                offset <- offset + columnLength

            let chars = text.ToCharArray()

            [|
                for row in 0 .. rowCount - 1 do
                    for column in 0 .. key - 1 do
                        if row < columnLengths[column] then
                            chars[columnStarts[column] + row]
            |]
            |> fun chars -> String(chars).TrimEnd(' ')

    let bruteForce (text: string) =
        if isNull text then
            nullArg (nameof text)

        if text.Length < 4 then
            []
        else
            [ for key in 2 .. text.Length / 2 -> { Key = key; Text = decrypt text key } ]
