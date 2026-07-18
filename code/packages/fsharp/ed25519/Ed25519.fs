namespace CodingAdventures.Ed25519.FSharp

open System
open System.Numerics
open CodingAdventures.Sha512.FSharp

/// Ed25519 deterministic digital signatures from RFC 8032.
/// This educational implementation uses BigInteger field arithmetic, which is
/// not guaranteed to execute in constant time.
[<RequireQualifiedAccess>]
module Ed25519 =
    [<Literal>]
    let KeyLength = 32

    [<Literal>]
    let ExtendedLength = 64

    type private Point =
        { X: bigint
          Y: bigint
          Z: bigint
          T: bigint }

    let private prime = (BigInteger.One <<< 255) - 19I

    let private groupOrder =
        (BigInteger.One <<< 252)
        + BigInteger.Parse("27742317777372353535851937790883648493")

    let private modulo value =
        let reduced = value % prime
        if reduced.Sign < 0 then reduced + prime else reduced

    let private moduloGroup value =
        let reduced = value % groupOrder
        if reduced.Sign < 0 then reduced + groupOrder else reduced

    let private add left right = modulo (left + right)
    let private subtract left right = modulo (left - right)
    let private multiply left right = modulo (left * right)
    let private square value = modulo (value * value)
    let private invert value = BigInteger.ModPow(modulo value, prime - 2I, prime)

    let private d = multiply -121665I (invert 121666I)
    let private sqrtMinusOne = BigInteger.ModPow(2I, (prime - 1I) / 4I, prime)

    let private identity =
        { X = BigInteger.Zero
          Y = BigInteger.One
          Z = BigInteger.One
          T = BigInteger.Zero }

    let private decodeLittleEndian (bytes: ReadOnlySpan<byte>) =
        BigInteger(bytes, isUnsigned = true, isBigEndian = false)

    let private encodeLittleEndian (value: bigint) =
        let encoded = value.ToByteArray(isUnsigned = true, isBigEndian = false)
        let result = Array.zeroCreate<byte> KeyLength
        Array.Copy(encoded, result, encoded.Length)
        result

    let private fromAffine x y =
        { X = modulo x
          Y = modulo y
          Z = BigInteger.One
          T = multiply x y }

    let private tryRecoverX y sign =
        let ySquared = square y

        let xSquared =
            multiply (subtract ySquared 1I) (invert (add (multiply d ySquared) 1I))

        let mutable candidate = BigInteger.ModPow(xSquared, (prime + 3I) / 8I, prime)

        if square candidate <> xSquared then
            candidate <- multiply candidate sqrtMinusOne

        if square candidate <> xSquared || (candidate.IsZero && sign = 1) then
            None
        else
            let x =
                if int (candidate &&& BigInteger.One) = sign then
                    candidate
                else
                    modulo -candidate

            Some x

    let private basePoint =
        let y = multiply 4I (invert 5I)

        match tryRecoverX y 0 with
        | Some x -> fromAffine x y
        | None -> invalidOp "Unable to construct the Ed25519 base point."

    let private pointAdd left right =
        let a = multiply (subtract left.Y left.X) (subtract right.Y right.X)
        let b = multiply (add left.Y left.X) (add right.Y right.X)
        let c = multiply (2I * d) (multiply left.T right.T)
        let dValue = multiply 2I (multiply left.Z right.Z)
        let e = subtract b a
        let f = subtract dValue c
        let g = add dValue c
        let h = add b a

        { X = multiply e f
          Y = multiply g h
          Z = multiply f g
          T = multiply e h }

    let private pointDouble point =
        let a = square point.X
        let b = square point.Y
        let c = multiply 2I (square point.Z)
        let dValue = modulo -a
        let e = subtract (subtract (square (add point.X point.Y)) a) b
        let g = add dValue b
        let f = subtract g c
        let h = subtract dValue b

        { X = multiply e f
          Y = multiply g h
          Z = multiply f g
          T = multiply e h }

    let private scalarMultiply (scalar: bigint) point =
        let mutable result = identity
        let mutable addend = point
        let mutable remaining = scalar

        while remaining > BigInteger.Zero do
            if not remaining.IsEven then
                result <- pointAdd result addend

            addend <- pointDouble addend
            remaining <- remaining >>> 1

        result

    let private encodePoint point =
        let inverseZ = invert point.Z
        let x = multiply point.X inverseZ
        let y = multiply point.Y inverseZ
        let encoded = encodeLittleEndian y

        if not x.IsEven then
            encoded[31] <- encoded[31] ||| 0x80uy

        encoded

    let private tryDecodePoint (encoded: byte array) =
        if isNull encoded || encoded.Length <> KeyLength then
            None
        else
            let sign = int ((encoded[31] >>> 7) &&& 1uy)
            let yBytes = Array.copy encoded
            yBytes[31] <- yBytes[31] &&& 0x7fuy
            let y = decodeLittleEndian (ReadOnlySpan<byte>(yBytes))

            if y >= prime then
                None
            else
                match tryRecoverX y sign with
                | Some x -> Some(fromAffine x y)
                | None -> None

    let private pointsEqual left right =
        multiply left.X right.Z = multiply right.X left.Z
        && multiply left.Y right.Z = multiply right.Y left.Z

    let private clampScalar (digestPrefix: byte array) =
        let clamped = Array.copy digestPrefix
        clamped[0] <- clamped[0] &&& 248uy
        clamped[31] <- clamped[31] &&& 127uy
        clamped[31] <- clamped[31] ||| 64uy
        decodeLittleEndian (ReadOnlySpan<byte>(clamped))

    let private reduceScalar (bytes: byte array) =
        decodeLittleEndian (ReadOnlySpan<byte>(bytes)) % groupOrder

    let private hashParts (parts: byte array list) =
        parts |> Array.concat |> Sha512.hash

    let private validateLength parameterName description expectedLength (value: byte array) =
        if isNull value then
            nullArg parameterName

        if value.Length <> expectedLength then
            invalidArg
                parameterName
                $"Ed25519 {description} must be exactly {expectedLength} bytes."

    /// Generate a public key and 64-byte seed || publicKey secret key.
    let generateKeypair (seed: byte array) =
        validateLength "seed" "seed" KeyLength seed
        let digest = Sha512.hash seed
        let scalar = clampScalar digest[0 .. KeyLength - 1]
        let publicKey = scalarMultiply scalar basePoint |> encodePoint
        let secretKey = Array.append (Array.copy seed) publicKey
        publicKey, secretKey

    /// Sign a message with a 64-byte seed || publicKey key.
    let sign (message: byte array) (secretKey: byte array) =
        if isNull message then
            nullArg "message"

        validateLength "secretKey" "secret key" ExtendedLength secretKey
        let seed = secretKey[0 .. KeyLength - 1]
        let suppliedPublicKey = secretKey[KeyLength .. ExtendedLength - 1]
        let _, reconstructedSecretKey = generateKeypair seed

        if secretKey <> reconstructedSecretKey then
            invalidArg "secretKey" "Ed25519 secret key must be seed || publicKey."

        let digest = Sha512.hash seed
        let scalar = clampScalar digest[0 .. KeyLength - 1]
        let prefix = digest[KeyLength .. ExtendedLength - 1]
        let nonce = hashParts [ prefix; message ] |> reduceScalar
        let encodedR = scalarMultiply nonce basePoint |> encodePoint

        let challenge =
            hashParts [ encodedR; suppliedPublicKey; message ] |> reduceScalar

        let scalarS = moduloGroup (nonce + challenge * scalar)
        Array.append encodedR (encodeLittleEndian scalarS)

    /// Verify a signature. Malformed encodings return false rather than throw.
    let verify (message: byte array) (signature: byte array) (publicKey: byte array) =
        if isNull message then
            nullArg "message"

        if isNull signature
           || signature.Length <> ExtendedLength
           || isNull publicKey
           || publicKey.Length <> KeyLength then
            false
        else
            let encodedR = signature[0 .. KeyLength - 1]

            let scalarS =
                decodeLittleEndian (ReadOnlySpan<byte>(signature, KeyLength, KeyLength))

            if scalarS >= groupOrder then
                false
            else
                match tryDecodePoint encodedR, tryDecodePoint publicKey with
                | Some pointR, Some pointA ->
                    let challenge =
                        hashParts [ encodedR; publicKey; message ] |> reduceScalar

                    let left = scalarMultiply scalarS basePoint
                    let right = pointAdd pointR (scalarMultiply challenge pointA)
                    pointsEqual left right
                | _ -> false
