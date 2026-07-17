namespace CodingAdventures.X25519.FSharp

open System
open System.Numerics

/// X25519 elliptic-curve Diffie-Hellman from RFC 7748.
/// This educational implementation uses BigInteger field arithmetic, which is
/// not guaranteed to execute in constant time.
[<RequireQualifiedAccess>]
module X25519 =
    [<Literal>]
    let KeyLength = 32

    let private prime = (BigInteger.One <<< 255) - 19I
    let private primeMinusTwo = prime - 2I
    let private a24 = 121665I

    let private validateInput parameterName (value: byte array) =
        if isNull value then
            nullArg parameterName

        if value.Length <> KeyLength then
            invalidArg parameterName "X25519 inputs must be exactly 32 bytes."

    let private decodeLittleEndian (bytes: byte array) =
        let mutable result = BigInteger.Zero

        for index = bytes.Length - 1 downto 0 do
            result <- (result <<< 8) ||| bigint bytes[index]

        result

    let private decodeScalar (scalar: byte array) =
        let clamped = Array.copy scalar
        clamped[0] <- clamped[0] &&& 248uy
        clamped[31] <- clamped[31] &&& 127uy
        clamped[31] <- clamped[31] ||| 64uy
        decodeLittleEndian clamped

    let private decodeUCoordinate (uCoordinate: byte array) =
        let masked = Array.copy uCoordinate
        masked[31] <- masked[31] &&& 127uy
        decodeLittleEndian masked

    let private modulo value =
        let reduced = value % prime
        if reduced.Sign < 0 then reduced + prime else reduced

    let private add left right = modulo (left + right)
    let private subtract left right = modulo (left - right)
    let private multiply left right = modulo (left * right)
    let private square value = modulo (value * value)

    let private encodeUCoordinate value =
        let result = Array.zeroCreate<byte> KeyLength
        let mutable remaining = modulo value

        for index = 0 to KeyLength - 1 do
            result[index] <- byte (remaining &&& 255I)
            remaining <- remaining >>> 8

        result

    let private conditionalSwap (swap: int) (left: bigint) (right: bigint) =
        let mask = -(bigint swap)
        let difference = mask &&& (left ^^^ right)
        left ^^^ difference, right ^^^ difference

    /// Return a new copy of the standard Curve25519 base point, u = 9.
    let basePoint () =
        let result = Array.zeroCreate<byte> KeyLength
        result[0] <- 9uy
        result

    /// Multiply a 32-byte scalar by a 32-byte Montgomery u-coordinate.
    let x25519 (scalar: byte array) (uCoordinate: byte array) =
        validateInput "scalar" scalar
        validateInput "uCoordinate" uCoordinate

        let k = decodeScalar scalar
        let u = decodeUCoordinate uCoordinate
        let x1 = u
        let mutable x2 = BigInteger.One
        let mutable z2 = BigInteger.Zero
        let mutable x3 = u
        let mutable z3 = BigInteger.One
        let mutable swap = 0

        for bit = 254 downto 0 do
            let scalarBit = int ((k >>> bit) &&& BigInteger.One)
            swap <- swap ^^^ scalarBit

            let nextX2, nextX3 = conditionalSwap swap x2 x3
            let nextZ2, nextZ3 = conditionalSwap swap z2 z3
            x2 <- nextX2
            x3 <- nextX3
            z2 <- nextZ2
            z3 <- nextZ3
            swap <- scalarBit

            let a = add x2 z2
            let aa = square a
            let b = subtract x2 z2
            let bb = square b
            let e = subtract aa bb
            let c = add x3 z3
            let d = subtract x3 z3
            let da = multiply d a
            let cb = multiply c b

            x3 <- square (add da cb)
            z3 <- multiply x1 (square (subtract da cb))
            x2 <- multiply aa bb
            z2 <- multiply e (add aa (multiply a24 e))

        let finalX2, _ = conditionalSwap swap x2 x3
        let finalZ2, _ = conditionalSwap swap z2 z3
        let affine = multiply finalX2 (BigInteger.ModPow(finalZ2, primeMinusTwo, prime))
        let encoded = encodeUCoordinate affine

        if encoded |> Array.forall ((=) 0uy) then
            raise (InvalidOperationException("X25519 produced the all-zero output (low-order point)."))

        encoded

    /// Derive a public key by multiplying by the standard base point.
    let x25519Base scalar = x25519 scalar (basePoint ())

    /// Derive the public key for a 32-byte private key.
    let generateKeypair privateKey = x25519Base privateKey
