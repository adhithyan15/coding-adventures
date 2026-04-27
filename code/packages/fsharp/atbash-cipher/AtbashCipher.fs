namespace CodingAdventures.AtbashCipher

open System

/// Fixed reverse-alphabet substitution cipher.
[<RequireQualifiedAccess>]
module AtbashCipher =
    let private transform (ch: char) =
        if ch >= 'A' && ch <= 'Z' then
            let position = int ch - int 'A'
            char (int 'A' + (25 - position))
        elif ch >= 'a' && ch <= 'z' then
            let position = int ch - int 'a'
            char (int 'a' + (25 - position))
        else
            ch

    let encrypt (text: string) =
        if isNull text then
            nullArg (nameof text)

        text.ToCharArray()
        |> Array.map transform
        |> fun chars -> new string(chars)

    let decrypt (text: string) =
        if isNull text then
            nullArg (nameof text)

        encrypt text
