namespace CodingAdventures.Gf256

open System

// Gf256.fs -- Byte arithmetic inside a finite field instead of the integers
// ==========================================================================
//
// GF(2^8) uses the byte values 0 through 255 as field elements.
// Addition is XOR, and multiplication is reduced modulo the primitive
// polynomial 0x11D so that every non-zero element has an inverse.

[<RequireQualifiedAccess>]
module Gf256 =
    [<Literal>]
    let VERSION = "0.1.0"

    [<Literal>]
    let ZERO = 0uy

    [<Literal>]
    let ONE = 1uy

    [<Literal>]
    let PRIMITIVE_POLYNOMIAL = 0x11D

    let private logTable = Array.zeroCreate<int> 256
    let private alogTable = Array.zeroCreate<byte> 256

    do
        let mutable value = 1

        for index in 0 .. 254 do
            alogTable[index] <- byte value
            logTable[value] <- index
            value <- value <<< 1

            if value >= 256 then
                value <- value ^^^ PRIMITIVE_POLYNOMIAL

        alogTable[255] <- 1uy

    /// Antilog table where ALOG[i] = 2^i in GF(256).
    let ALOG = Array.copy alogTable

    /// Log table where LOG[x] = i such that 2^i = x in GF(256).
    let LOG = Array.copy logTable

    /// Add two field elements. In characteristic 2 this is XOR.
    let add (a: byte) (b: byte) = a ^^^ b

    /// Subtract two field elements. In characteristic 2 this is also XOR.
    let subtract (a: byte) (b: byte) = a ^^^ b

    /// Multiply two field elements using log and antilog tables.
    let multiply (a: byte) (b: byte) =
        if a = 0uy || b = 0uy then
            0uy
        else
            let exponent = (logTable[int a] + logTable[int b]) % 255
            alogTable[exponent]

    /// Divide a by b in GF(256).
    let divide (a: byte) (b: byte) =
        if b = 0uy then
            raise (InvalidOperationException("GF256: division by zero"))

        if a = 0uy then
            0uy
        else
            let exponent = ((logTable[int a] - logTable[int b] + 255) % 255 + 255) % 255
            alogTable[exponent]

    /// Raise a field element to a non-negative integer power.
    let power (baseValue: byte) exponent =
        if exponent < 0 then
            raise (ArgumentOutOfRangeException("exponent", "Exponent must be non-negative."))

        if baseValue = 0uy then
            if exponent = 0 then ONE else ZERO
        elif exponent = 0 then
            ONE
        else
            let tableIndex = ((logTable[int baseValue] * exponent) % 255 + 255) % 255
            alogTable[tableIndex]

    /// Compute the multiplicative inverse of a non-zero field element.
    let inverse (a: byte) =
        if a = 0uy then
            raise (InvalidOperationException("GF256: zero has no multiplicative inverse"))

        alogTable[255 - logTable[int a]]

    /// Return the additive identity.
    let zero () = ZERO

    /// Return the multiplicative identity.
    let one () = ONE

    /// GF(2^8) field parameterized by an arbitrary primitive polynomial.
    type Gf256Field(polynomial: int) =
        do
            if polynomial <= 0 || polynomial > 0x1FF then
                raise (ArgumentOutOfRangeException("polynomial", "Polynomial must fit in 9 bits."))

        let reduce = byte (polynomial &&& 0xFF)

        member _.Polynomial = polynomial

        member _.Add(a: byte, b: byte) = a ^^^ b

        member _.Subtract(a: byte, b: byte) = a ^^^ b

        member _.Multiply(a: byte, b: byte) =
            let mutable result = 0
            let mutable left = int a
            let mutable right = int b

            for _ in 0 .. 7 do
                if (right &&& 1) <> 0 then
                    result <- result ^^^ left

                let highBit = left &&& 0x80
                left <- (left <<< 1) &&& 0xFF

                if highBit <> 0 then
                    left <- left ^^^ int reduce

                right <- right >>> 1

            byte result

        member this.Divide(a: byte, b: byte) =
            if b = 0uy then
                raise (InvalidOperationException("GF256Field: division by zero"))

            this.Multiply(a, this.Power(b, 254))

        member this.Power(baseValue: byte, exponent: int) =
            if exponent < 0 then
                raise (ArgumentOutOfRangeException("exponent", "Exponent must be non-negative."))

            if baseValue = 0uy then
                if exponent = 0 then ONE else ZERO
            elif exponent = 0 then
                ONE
            else
                let mutable result = ONE
                let mutable factor = baseValue
                let mutable remaining = exponent

                while remaining > 0 do
                    if (remaining &&& 1) <> 0 then
                        result <- this.Multiply(result, factor)

                    factor <- this.Multiply(factor, factor)
                    remaining <- remaining >>> 1

                result

        member this.Inverse(a: byte) =
            if a = 0uy then
                raise (InvalidOperationException("GF256Field: zero has no multiplicative inverse"))

            this.Power(a, 254)

    /// Create a GF(2^8) field for a different primitive polynomial.
    let createField polynomial = Gf256Field(polynomial)
