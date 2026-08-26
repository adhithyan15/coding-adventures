namespace CodingAdventures.ScytaleCipher

open System
open System.Text

type BruteForceResult = {
    Key: int
    Text: string
}

[<RequireQualifiedAccess>]
module ScytaleCipher =
    [<Literal>]
    let maxBruteForceTextLength = 4096

    let private scalars (text: string) = text.EnumerateRunes() |> Seq.toArray

    let private scalarString (values: Rune array) =
        let builder = StringBuilder(values.Length)
        values |> Array.iter (fun value -> builder.Append(value) |> ignore)
        builder.ToString()

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
            let values = scalars text
            validateKey values.Length key

            let rowCount = (values.Length + key - 1) / key
            let paddedLength = rowCount * key
            let padded = Array.create paddedLength (Rune(' '))
            Array.Copy(values, padded, values.Length)

            [|
                for column in 0 .. key - 1 do
                    for row in 0 .. rowCount - 1 do
                        padded[(row * key) + column]
            |]
            |> scalarString

    let decrypt (text: string) (key: int) =
        if isNull text then
            nullArg (nameof text)

        if text.Length = 0 then
            String.Empty
        else
            let values = scalars text
            validateKey values.Length key

            let rowCount = (values.Length + key - 1) / key
            let fullColumns = if values.Length % key = 0 then key else values.Length % key
            let columnStarts = Array.zeroCreate<int> key
            let columnLengths = Array.zeroCreate<int> key
            let mutable offset = 0

            for column in 0 .. key - 1 do
                columnStarts[column] <- offset
                let columnLength =
                    if values.Length % key = 0 || column < fullColumns then rowCount else rowCount - 1

                columnLengths[column] <- columnLength
                offset <- offset + columnLength

            let plaintext = [|
                for row in 0 .. rowCount - 1 do
                    for column in 0 .. key - 1 do
                        if row < columnLengths[column] then
                            values[columnStarts[column] + row]
            |]
            let mutable endIndex = plaintext.Length
            while endIndex > 0 && plaintext[endIndex - 1].Value = 0x20 do
                endIndex <- endIndex - 1
            if endIndex = 0 then
                String.Empty
            else
                plaintext[.. endIndex - 1] |> scalarString

    let bruteForce (text: string) =
        if isNull text then
            nullArg (nameof text)

        let scalarLength = scalars text |> Array.length

        if scalarLength > maxBruteForceTextLength then
            invalidArg (nameof text) "scytale-brute-force-limit"

        if scalarLength < 4 then
            []
        else
            [ for key in 2 .. scalarLength / 2 -> { Key = key; Text = decrypt text key } ]
