namespace CodingAdventures.VigenereCipher

open System
open System.Text

type BreakResult = {
    Key: string
    Plaintext: string
}

/// Vigenere cipher helpers and basic ciphertext-only analysis.
[<RequireQualifiedAccess>]
module VigenereCipher =
    let englishFrequencies =
        [|
            0.08167; 0.01492; 0.02782; 0.04253; 0.12702; 0.02228; 0.02015
            0.06094; 0.06966; 0.00153; 0.00772; 0.04025; 0.02406; 0.06749
            0.07507; 0.01929; 0.00095; 0.05987; 0.06327; 0.09056; 0.02758
            0.00978; 0.02360; 0.00150; 0.01974; 0.00074
        |]

    let private isAsciiLetter (ch: char) =
        (ch >= 'A' && ch <= 'Z') || (ch >= 'a' && ch <= 'z')

    let private validateKey (key: string) =
        if isNull key then
            nullArg (nameof key)

        if key.Length = 0 then
            invalidArg (nameof key) "Key must not be empty."

        let normalized = key.ToUpperInvariant()

        for ch in normalized do
            if ch < 'A' || ch > 'Z' then
                invalidArg (nameof key) "Key must contain only ASCII letters."

        normalized

    let private transform (text: string) (key: string) direction =
        let normalizedKey = validateKey key
        let builder = StringBuilder(text.Length)
        let mutable keyIndex = 0

        for ch in text do
            if ch >= 'A' && ch <= 'Z' then
                let shift = direction * (int normalizedKey[keyIndex % normalizedKey.Length] - int 'A')
                let shifted = int 'A' + (int ch - int 'A' + shift + 26) % 26
                builder.Append(char shifted) |> ignore
                keyIndex <- keyIndex + 1
            elif ch >= 'a' && ch <= 'z' then
                let shift = direction * (int normalizedKey[keyIndex % normalizedKey.Length] - int 'A')
                let shifted = int 'a' + (int ch - int 'a' + shift + 26) % 26
                builder.Append(char shifted) |> ignore
                keyIndex <- keyIndex + 1
            else
                builder.Append(ch) |> ignore

        builder.ToString()

    let encrypt (plaintext: string) (key: string) =
        if isNull plaintext then
            nullArg (nameof plaintext)

        transform plaintext key 1

    let decrypt (ciphertext: string) (key: string) =
        if isNull ciphertext then
            nullArg (nameof ciphertext)

        transform ciphertext key -1

    let private extractAlphaUpper (text: string) =
        let builder = StringBuilder(text.Length)

        for ch in text do
            if isAsciiLetter ch then
                builder.Append(Char.ToUpperInvariant(ch)) |> ignore

        builder.ToString()

    let private indexOfCoincidence (counts: int array) total =
        if total < 2 then
            0.0
        else
            let numerator =
                counts
                |> Array.sumBy (fun count -> int64 count * int64 (count - 1))

            float numerator / float (int64 total * int64 (total - 1))

    let findKeyLength (ciphertext: string) (maxLength: int) =
        if isNull ciphertext then
            nullArg (nameof ciphertext)

        let letters = extractAlphaUpper ciphertext

        if letters.Length < 2 then
            1
        else
            let limit = min maxLength (letters.Length / 2)

            if limit < 2 then
                1
            else
                let averageIcs = Array.zeroCreate<float> (limit + 1)
                let mutable bestAverageIc = 0.0

                for length in 2 .. limit do
                    let mutable totalIc = 0.0
                    let mutable validGroups = 0

                    for group in 0 .. length - 1 do
                        let counts = Array.zeroCreate<int> 26
                        let mutable groupLength = 0
                        let mutable position = group

                        while position < letters.Length do
                            let index = int letters[position] - int 'A'
                            counts[index] <- counts[index] + 1
                            groupLength <- groupLength + 1
                            position <- position + length

                        if groupLength > 1 then
                            totalIc <- totalIc + indexOfCoincidence counts groupLength
                            validGroups <- validGroups + 1

                    if validGroups > 0 then
                        averageIcs[length] <- totalIc / float validGroups
                        bestAverageIc <- max bestAverageIc averageIcs[length]

                if bestAverageIc <= 0.0 then
                    1
                else
                    let threshold = bestAverageIc * 0.90

                    let candidates =
                        [ for length in 2 .. limit do
                            if averageIcs[length] >= threshold then
                                length ]
                        |> ResizeArray

                    for smaller in List.ofSeq candidates do
                        candidates.RemoveAll(fun candidate -> candidate <> smaller && candidate % smaller = 0)
                        |> ignore

                    if candidates.Count = 0 then 1 else candidates[0]

    let findKeyLengthDefault (ciphertext: string) =
        findKeyLength ciphertext 20

    let private chiSquared (counts: int array) total =
        if total = 0 then
            Double.PositiveInfinity
        else
            let mutable sum = 0.0

            for i in 0 .. 25 do
                let expected = float total * englishFrequencies[i]
                let difference = float counts[i] - expected
                sum <- sum + difference * difference / expected

            sum

    let private minimalPeriod (key: string) =
        let mutable result = key
        let mutable found = false
        let mutable period = 1

        while not found && period <= key.Length / 2 do
            if key.Length % period = 0 then
                let mutable repeated = true
                let mutable index = period

                while repeated && index < key.Length do
                    if key[index] <> key[index % period] then
                        repeated <- false

                    index <- index + 1

                if repeated then
                    result <- key.Substring(0, period)
                    found <- true

            period <- period + 1

        result

    let findKey (ciphertext: string) (keyLength: int) =
        if isNull ciphertext then
            nullArg (nameof ciphertext)

        if keyLength <= 0 then
            invalidArg (nameof keyLength) "Key length must be positive."

        let letters = extractAlphaUpper ciphertext
        let key = Array.zeroCreate<char> keyLength

        for group in 0 .. keyLength - 1 do
            let groupLetters = ResizeArray<char>()
            let mutable position = group

            while position < letters.Length do
                groupLetters.Add(letters[position])
                position <- position + keyLength

            if groupLetters.Count = 0 then
                key[group] <- 'A'
            else
                let mutable bestShift = 0
                let mutable bestScore = Double.PositiveInfinity

                for shift in 0 .. 25 do
                    let counts = Array.zeroCreate<int> 26

                    for ch in groupLetters do
                        let decrypted = (int ch - int 'A' + 26 - shift) % 26
                        counts[decrypted] <- counts[decrypted] + 1

                    let score = chiSquared counts groupLetters.Count

                    if score < bestScore then
                        bestScore <- score
                        bestShift <- shift

                key[group] <- char (int 'A' + bestShift)

        new string(key) |> minimalPeriod

    let breakCipher (ciphertext: string) =
        if isNull ciphertext then
            nullArg (nameof ciphertext)

        let keyLength = findKeyLengthDefault ciphertext
        let key = findKey ciphertext keyLength

        {
            Key = key
            Plaintext = decrypt ciphertext key
        }
