namespace CodingAdventures.Uuid

open System
open System.Buffers.Binary
open System.Globalization
open System.Text
open System.Text.RegularExpressions
open CodingAdventures.Csprng.FSharp
open CodingAdventures.Md5
open CodingAdventures.Sha1.FSharp

type UuidException(message: string, innerException: exn) =
    inherit ArgumentException(message, innerException)

    new(message: string) = UuidException(message, null)

type Uuid =
    { Msb: uint64
      Lsb: uint64 }

    member this.Version = int ((this.Msb >>> 12) &&& 0xFUL)

    member this.Variant =
        let top = int ((this.Lsb >>> 62) &&& 0x3UL)

        if top <= 1 then
            "ncs"
        elif top = 2 then
            "rfc4122"
        elif ((this.Lsb >>> 61) &&& 0x7UL) = 7UL then
            "reserved"
        else
            "microsoft"

    member this.IsNil = this.Msb = 0UL && this.Lsb = 0UL

    member this.IsMax = this.Msb = UInt64.MaxValue && this.Lsb = UInt64.MaxValue

    member this.ToBytes() =
        let bytes = Array.zeroCreate<byte> 16
        BinaryPrimitives.WriteUInt64BigEndian(bytes.AsSpan(0, 8), this.Msb)
        BinaryPrimitives.WriteUInt64BigEndian(bytes.AsSpan(8, 8), this.Lsb)
        bytes

    override this.ToString() =
        let hex =
            this.Msb.ToString("x16", CultureInfo.InvariantCulture)
            + this.Lsb.ToString("x16", CultureInfo.InvariantCulture)

        $"{hex.Substring(0, 8)}-{hex.Substring(8, 4)}-{hex.Substring(12, 4)}-{hex.Substring(16, 4)}-{hex.Substring(20)}"

[<RequireQualifiedAccess>]
module Uuid =
    [<Literal>]
    let VERSION = "0.1.0"

    let private uuidPattern =
        Regex(
            "^\\s*(?:urn:uuid:)?\\{?"
            + "([0-9a-fA-F]{8})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{4})-?([0-9a-fA-F]{12})"
            + "\\}?\\s*$",
            RegexOptions.Compiled ||| RegexOptions.CultureInvariant ||| RegexOptions.IgnoreCase
        )

    let private gregorianOffset = 122_192_928_000_000_000UL

    let private createClockSequence () =
        let bytes = Csprng.randomBytes 2
        ((int bytes[0] <<< 8) ||| int bytes[1]) &&& 0x3FFF

    let private clockSequence = createClockSequence ()

    let fromString (text: string) =
        if isNull text then
            raise (UuidException "UUID string must not be null")

        let matchResult = uuidPattern.Match(text.Trim())

        if not matchResult.Success then
            raise (UuidException $"Invalid UUID string: '{text}'")

        let hex =
            matchResult.Groups[1].Value
            + matchResult.Groups[2].Value
            + matchResult.Groups[3].Value
            + matchResult.Groups[4].Value
            + matchResult.Groups[5].Value

        { Msb = UInt64.Parse(hex.Substring(0, 16), NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture)
          Lsb = UInt64.Parse(hex.Substring(16), NumberStyles.AllowHexSpecifier, CultureInfo.InvariantCulture) }

    let fromBytes (bytes: byte array) =
        if isNull bytes then
            raise (UuidException "UUID bytes must be exactly 16, got null")

        if bytes.Length <> 16 then
            raise (UuidException $"UUID bytes must be exactly 16, got {bytes.Length}")

        { Msb = BinaryPrimitives.ReadUInt64BigEndian(bytes.AsSpan(0, 8))
          Lsb = BinaryPrimitives.ReadUInt64BigEndian(bytes.AsSpan(8, 8)) }

    let isValid (text: string) =
        not (isNull text) && uuidPattern.IsMatch(text.Trim())

    let toBytes (uuid: Uuid) = uuid.ToBytes()

    let version (uuid: Uuid) = uuid.Version

    let variant (uuid: Uuid) = uuid.Variant

    let isNil (uuid: Uuid) = uuid.IsNil

    let isMax (uuid: Uuid) = uuid.IsMax

    let NIL = { Msb = 0UL; Lsb = 0UL }

    let MAX = { Msb = UInt64.MaxValue; Lsb = UInt64.MaxValue }

    let NAMESPACE_DNS = fromString "6ba7b810-9dad-11d1-80b4-00c04fd430c8"

    let NAMESPACE_URL = fromString "6ba7b811-9dad-11d1-80b4-00c04fd430c8"

    let NAMESPACE_OID = fromString "6ba7b812-9dad-11d1-80b4-00c04fd430c8"

    let NAMESPACE_X500 = fromString "6ba7b814-9dad-11d1-80b4-00c04fd430c8"

    let private stampVersionVariant (bytes: byte array) version =
        bytes[6] <- byte ((int bytes[6] &&& 0x0F) ||| (version <<< 4))
        bytes[8] <- byte ((int bytes[8] &&& 0x3F) ||| 0x80)
        fromBytes bytes

    let v4 () =
        let bytes = Csprng.randomBytes 16
        stampVersionVariant bytes 4

    let v7 () =
        let timestampMs = uint64 (DateTimeOffset.UtcNow.ToUnixTimeMilliseconds())
        let random = Csprng.randomBytes 10
        let raw = Array.zeroCreate<byte> 16
        raw[0] <- byte ((timestampMs >>> 40) &&& 0xFFUL)
        raw[1] <- byte ((timestampMs >>> 32) &&& 0xFFUL)
        raw[2] <- byte ((timestampMs >>> 24) &&& 0xFFUL)
        raw[3] <- byte ((timestampMs >>> 16) &&& 0xFFUL)
        raw[4] <- byte ((timestampMs >>> 8) &&& 0xFFUL)
        raw[5] <- byte (timestampMs &&& 0xFFUL)
        raw[6] <- byte (0x70 ||| (int random[0] &&& 0x0F))
        raw[7] <- random[1]
        raw[8] <- byte (0x80 ||| (int random[2] &&& 0x3F))
        Array.Copy(random, 3, raw, 9, 7)
        fromBytes raw

    let v1 () =
        let timestamp =
            uint64 (DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()) * 10_000UL
            + gregorianOffset

        let timeLow = timestamp &&& 0xFFFFFFFFUL
        let timeMid = (timestamp >>> 32) &&& 0xFFFFUL
        let timeHi = (timestamp >>> 48) &&& 0x0FFFUL
        let msb = (timeLow <<< 32) ||| (timeMid <<< 16) ||| (0x1000UL ||| timeHi)
        let clockSeqHi = 0x80UL ||| (uint64 clockSequence >>> 8)
        let clockSeqLow = uint64 (clockSequence &&& 0xFF)
        let nodeBytes = Csprng.randomBytes 6
        nodeBytes[0] <- byte (int nodeBytes[0] ||| 0x01)

        let mutable node = 0UL

        for value in nodeBytes do
            node <- (node <<< 8) ||| uint64 value

        { Msb = msb
          Lsb = (clockSeqHi <<< 56) ||| (clockSeqLow <<< 48) ||| node }

    let private concat (left: byte array) (right: byte array) =
        let result = Array.zeroCreate<byte> (left.Length + right.Length)
        Buffer.BlockCopy(left, 0, result, 0, left.Length)
        Buffer.BlockCopy(right, 0, result, left.Length, right.Length)
        result

    let v5 (namespaceId: Uuid) (name: string) =
        if isNull name then
            nullArg (nameof name)

        let digest =
            concat (namespaceId.ToBytes()) (Encoding.UTF8.GetBytes name)
            |> Sha1.hash

        let raw = digest[..15]
        stampVersionVariant raw 5

    let v3 (namespaceId: Uuid) (name: string) =
        if isNull name then
            nullArg (nameof name)

        concat (namespaceId.ToBytes()) (Encoding.UTF8.GetBytes name)
        |> Md5.sumMd5
        |> fun digest -> stampVersionVariant digest 3
