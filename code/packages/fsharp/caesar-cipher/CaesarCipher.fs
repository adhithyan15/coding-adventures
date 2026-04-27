namespace CodingAdventures.CaesarCipher

open System

type BruteForceResult = {
    Shift: int
    Plaintext: string
}

/// Caesar cipher helpers and the simplest statistical attack against them.
[<RequireQualifiedAccess>]
module CaesarCipher =
    let englishFrequencies =
        [|
            0.08167; 0.01492; 0.02782; 0.04253; 0.12702; 0.02228; 0.02015
            0.06094; 0.06966; 0.00153; 0.00772; 0.04025; 0.02406; 0.06749
            0.07507; 0.01929; 0.00095; 0.05987; 0.06327; 0.09056; 0.02758
            0.00978; 0.02360; 0.00150; 0.01974; 0.00074
        |]

    let private normalizeShift shift = ((shift % 26) + 26) % 26

    let private shiftChar normalizedShift (ch: char) =
        if ch >= 'A' && ch <= 'Z' then
            let position = int ch - int 'A'
            char (int 'A' + (position + normalizedShift) % 26)
        elif ch >= 'a' && ch <= 'z' then
            let position = int ch - int 'a'
            char (int 'a' + (position + normalizedShift) % 26)
        else
            ch

    let encrypt (text: string) (shift: int) =
        if isNull text then
            nullArg (nameof text)

        let normalizedShift = normalizeShift shift

        text.ToCharArray()
        |> Array.map (shiftChar normalizedShift)
        |> fun chars -> new string(chars)

    let decrypt (text: string) (shift: int) =
        if isNull text then
            nullArg (nameof text)

        encrypt text (-shift)

    let rot13 (text: string) =
        if isNull text then
            nullArg (nameof text)

        encrypt text 13

    let bruteForce (ciphertext: string) =
        if isNull ciphertext then
            nullArg (nameof ciphertext)

        [ for shift in 1 .. 25 ->
            { Shift = shift; Plaintext = decrypt ciphertext shift } ]

    let private letterCounts (text: string) =
        let counts = Array.zeroCreate<int> 26

        for ch in text do
            if Char.IsAsciiLetter(ch) then
                let index = int (Char.ToUpperInvariant(ch)) - int 'A'
                counts[index] <- counts[index] + 1

        counts

    let private chiSquared (text: string) =
        let counts = letterCounts text
        let total = Array.sum counts

        if total = 0 then
            Double.MaxValue
        else
            let totalAsFloat = float total
            let mutable sum = 0.0

            for i in 0 .. 25 do
                let expected = totalAsFloat * englishFrequencies[i]
                let difference = float counts[i] - expected
                sum <- sum + difference * difference / expected

            sum

    let frequencyAnalysis (ciphertext: string) =
        if isNull ciphertext then
            nullArg (nameof ciphertext)

        let mutable bestShift = 1
        let mutable bestPlaintext = decrypt ciphertext 1
        let mutable bestScore = chiSquared bestPlaintext

        for shift in 2 .. 25 do
            let candidate = decrypt ciphertext shift
            let score = chiSquared candidate

            if score < bestScore then
                bestShift <- shift
                bestPlaintext <- candidate
                bestScore <- score

        bestShift, bestPlaintext
