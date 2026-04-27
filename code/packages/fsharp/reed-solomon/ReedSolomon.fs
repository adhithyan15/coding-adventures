namespace CodingAdventures.ReedSolomon.FSharp

open System
open System.Collections.Generic
open CodingAdventures.Gf256

// ReedSolomon.fs -- Reed-Solomon block codes over the local GF(256) package
// ==========================================================================
//
// The encoder appends enough redundancy bytes that the resulting codeword is a
// multiple of the RS generator polynomial. The decoder reconstructs the shortest
// error-locator polynomial from the syndromes and then solves for where and how
// large the corruptions were.

exception TooManyErrorsError
exception InvalidInputError of string

[<AbstractClass; Sealed>]
type ReedSolomon =
    static member VERSION = "0.1.0"

    static member private ValidatePositiveEven(nCheck: int) =
        if nCheck <= 0 || nCheck % 2 <> 0 then
            raise (InvalidInputError(sprintf "reed-solomon: invalid input — nCheck must be a positive even number, got %d" nCheck))

    static member private PolyEvalBigEndian(coefficients: byte array, x: byte) =
        let mutable accumulator = 0uy

        for coefficient in coefficients do
            accumulator <- Gf256.add (Gf256.multiply accumulator x) coefficient

        accumulator

    static member private PolyEvalLittleEndian(coefficients: byte array, x: byte) =
        let mutable accumulator = 0uy

        for index in coefficients.Length - 1 .. -1 .. 0 do
            accumulator <- Gf256.add (Gf256.multiply accumulator x) coefficients[index]

        accumulator

    static member private PolyMulLittleEndian(left: byte array, right: byte array) =
        if left.Length = 0 || right.Length = 0 then
            Array.empty
        else
            let result = Array.zeroCreate<byte> (left.Length + right.Length - 1)

            for leftIndex in 0 .. left.Length - 1 do
                for rightIndex in 0 .. right.Length - 1 do
                    result[leftIndex + rightIndex] <-
                        result[leftIndex + rightIndex] ^^^ Gf256.multiply left[leftIndex] right[rightIndex]

            result

    static member private PolyModBigEndian(dividend: byte array, divisor: byte array) =
        if divisor.Length = 0 then
            invalidOp "poly_mod_be requires a non-empty divisor"

        if divisor[0] <> 1uy then
            invalidOp "poly_mod_be requires a monic divisor"

        let remainder = Array.copy dividend

        if remainder.Length < divisor.Length then
            remainder
        else
            let steps = remainder.Length - divisor.Length + 1

            for step in 0 .. steps - 1 do
                let coefficient = remainder[step]

                if coefficient <> 0uy then
                    for index in 0 .. divisor.Length - 1 do
                        remainder[step + index] <-
                            remainder[step + index] ^^^ Gf256.multiply coefficient divisor[index]

            remainder[remainder.Length - (divisor.Length - 1) ..]

    static member private InvLocator(position: int, length: int) =
        let exponent = (position + 256 - length) % 255
        Gf256.power 2uy exponent

    static member private AllZero(values: byte array) = values |> Array.forall ((=) 0uy)

    static member private TrimTrailingZerosLittleEndian(polynomial: byte array) =
        let mutable last = polynomial.Length - 1

        while last > 0 && polynomial[last] = 0uy do
            last <- last - 1

        polynomial[.. last]

    static member private BerlekampMassey(syndromes: byte array) =
        let mutable current = [| 1uy |]
        let mutable previous = [| 1uy |]
        let mutable errorCount = 0
        let mutable shift = 1
        let mutable previousScale = 1uy

        for sequenceIndex in 0 .. syndromes.Length - 1 do
            let mutable discrepancy = syndromes[sequenceIndex]

            for index in 1 .. errorCount do
                if index < current.Length && sequenceIndex >= index then
                    discrepancy <- discrepancy ^^^ Gf256.multiply current[index] syndromes[sequenceIndex - index]

            if discrepancy = 0uy then
                shift <- shift + 1
            else
                let scale = Gf256.divide discrepancy previousScale
                let neededLength = shift + previous.Length

                if current.Length < neededLength then
                    Array.Resize(&current, neededLength)

                let saved = Array.copy current

                for index in 0 .. previous.Length - 1 do
                    current[shift + index] <- current[shift + index] ^^^ Gf256.multiply scale previous[index]

                if 2 * errorCount <= sequenceIndex then
                    errorCount <- sequenceIndex + 1 - errorCount
                    previous <- saved
                    previousScale <- discrepancy
                    shift <- 1
                else
                    shift <- shift + 1

        ReedSolomon.TrimTrailingZerosLittleEndian(current), errorCount

    static member private ChienSearch(lambda: byte array, length: int) =
        let positions = ResizeArray<int>()

        for position in 0 .. length - 1 do
            if ReedSolomon.PolyEvalLittleEndian(lambda, ReedSolomon.InvLocator(position, length)) = 0uy then
                positions.Add(position)

        List.ofSeq positions

    static member private Forney(lambda: byte array, syndromes: byte array, positions: int list, length: int) =
        let omegaFull = ReedSolomon.PolyMulLittleEndian(syndromes, lambda)
        let omega = omegaFull |> Array.truncate syndromes.Length
        let lambdaPrime = Array.zeroCreate<byte> (max 0 (lambda.Length - 1))

        for index in 1 .. 2 .. lambda.Length - 1 do
            lambdaPrime[index - 1] <- lambdaPrime[index - 1] ^^^ lambda[index]

        let magnitudes = ResizeArray<byte>()

        for position in positions do
            let xiInverse = ReedSolomon.InvLocator(position, length)
            let numerator = ReedSolomon.PolyEvalLittleEndian(omega, xiInverse)
            let denominator = ReedSolomon.PolyEvalLittleEndian(lambdaPrime, xiInverse)

            if denominator = 0uy then
                raise TooManyErrorsError

            magnitudes.Add(Gf256.divide numerator denominator)

        magnitudes.ToArray()

    static member BuildGenerator(nCheck: int) =
        ReedSolomon.ValidatePositiveEven(nCheck)

        let mutable generator = [| 1uy |]

        for powerIndex in 1 .. nCheck do
            let alphaPower = Gf256.power 2uy powerIndex
            let next = Array.zeroCreate<byte> (generator.Length + 1)

            for index in 0 .. generator.Length - 1 do
                next[index] <- next[index] ^^^ Gf256.multiply generator[index] alphaPower
                next[index + 1] <- next[index + 1] ^^^ generator[index]

            generator <- next

        generator

    static member Encode(message: byte array, nCheck: int) =
        if isNull message then
            nullArg "message"

        ReedSolomon.ValidatePositiveEven(nCheck)

        let totalLength = message.Length + nCheck

        if totalLength > 255 then
            raise (InvalidInputError(sprintf "reed-solomon: invalid input — total codeword length %d exceeds GF(256) block size limit of 255" totalLength))

        let generatorBigEndian = ReedSolomon.BuildGenerator(nCheck) |> Array.rev
        let shifted = Array.zeroCreate<byte> totalLength
        Array.Copy(message, shifted, message.Length)

        let remainder = ReedSolomon.PolyModBigEndian(shifted, generatorBigEndian)
        let codeword = Array.zeroCreate<byte> totalLength
        Array.Copy(message, codeword, message.Length)

        let pad = nCheck - remainder.Length
        Array.Copy(remainder, 0, codeword, message.Length + pad, remainder.Length)
        codeword

    static member Syndromes(received: byte array, nCheck: int) =
        if isNull received then
            nullArg "received"

        Array.init nCheck (fun index -> ReedSolomon.PolyEvalBigEndian(received, Gf256.power 2uy (index + 1)))

    static member Decode(received: byte array, nCheck: int) =
        if isNull received then
            nullArg "received"

        ReedSolomon.ValidatePositiveEven(nCheck)

        if received.Length < nCheck then
            raise (InvalidInputError(sprintf "reed-solomon: invalid input — received length %d < nCheck %d" received.Length nCheck))

        let correctionCapacity = nCheck / 2
        let messageLength = received.Length - nCheck
        let syndromes = ReedSolomon.Syndromes(received, nCheck)

        if ReedSolomon.AllZero(syndromes) then
            received[.. messageLength - 1]
        else
            let lambda, errorCount = ReedSolomon.BerlekampMassey(syndromes)

            if errorCount > correctionCapacity then
                raise TooManyErrorsError

            let positions = ReedSolomon.ChienSearch(lambda, received.Length)

            if positions.Length <> errorCount then
                raise TooManyErrorsError

            let magnitudes = ReedSolomon.Forney(lambda, syndromes, positions, received.Length)
            let corrected = Array.copy received
            let positionArray = List.toArray positions

            for index in 0 .. positionArray.Length - 1 do
                corrected[positionArray[index]] <- corrected[positionArray[index]] ^^^ magnitudes[index]

            corrected[.. messageLength - 1]

    static member ErrorLocator(syndromes: byte array) =
        if isNull syndromes then
            nullArg "syndromes"

        ReedSolomon.BerlekampMassey(syndromes) |> fst
