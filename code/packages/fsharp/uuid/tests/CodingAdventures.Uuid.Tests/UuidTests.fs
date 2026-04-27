namespace CodingAdventures.Uuid.Tests

open System
open CodingAdventures.Uuid
open Xunit

type UuidTests() =
    [<Fact>]
    member _.FromStringAcceptsSupportedForms() =
        let expected = "550e8400-e29b-41d4-a716-446655440000"

        Assert.Equal(expected, Uuid.fromString(expected).ToString())
        Assert.Equal(expected, Uuid.fromString("550E8400-E29B-41D4-A716-446655440000").ToString())
        Assert.Equal(expected, Uuid.fromString("550e8400e29b41d4a716446655440000").ToString())
        Assert.Equal(expected, Uuid.fromString("{550e8400-e29b-41d4-a716-446655440000}").ToString())
        Assert.Equal(expected, Uuid.fromString("urn:uuid:550e8400-e29b-41d4-a716-446655440000").ToString())
        Assert.Equal(expected, Uuid.fromString("  550e8400-e29b-41d4-a716-446655440000  ").ToString())

    [<Fact>]
    member _.FromStringRejectsInvalidText() =
        Assert.Throws<UuidException>(fun () -> Uuid.fromString "not-a-uuid" |> ignore) |> ignore
        Assert.Throws<UuidException>(fun () -> Uuid.fromString "550e8400-e29b-41d4-a716-44665544000z" |> ignore) |> ignore
        Assert.Throws<UuidException>(fun () -> Uuid.fromString "" |> ignore) |> ignore
        Assert.Throws<UuidException>(fun () -> Uuid.fromString null |> ignore) |> ignore

    [<Fact>]
    member _.FromBytesRoundTripsAndRejectsWrongLength() =
        let uuid = Uuid.fromString "550e8400-e29b-41d4-a716-446655440000"
        let bytes = Uuid.toBytes uuid

        Assert.Equal(16, bytes.Length)
        Assert.Equal(uuid, Uuid.fromBytes bytes)
        Assert.Throws<UuidException>(fun () -> Uuid.fromBytes (Array.zeroCreate 15) |> ignore) |> ignore
        Assert.Throws<UuidException>(fun () -> Uuid.fromBytes (Array.zeroCreate 17) |> ignore) |> ignore
        Assert.Throws<UuidException>(fun () -> Uuid.fromBytes null |> ignore) |> ignore

    [<Fact>]
    member _.PropertiesExposeVersionVariantAndExtremes() =
        Assert.Equal(4, (Uuid.fromString "550e8400-e29b-41d4-a716-446655440000").Version)
        Assert.Equal(1, (Uuid.fromString "6ba7b810-9dad-11d1-80b4-00c04fd430c8").Version)
        Assert.Equal(5, (Uuid.fromString "886313e1-3b8a-5372-9b90-0c9aee199e5d").Version)
        Assert.True(Uuid.NIL.IsNil)
        Assert.False((Uuid.v4 ()).IsNil)
        Assert.True(Uuid.MAX.IsMax)
        Assert.False((Uuid.v4 ()).IsMax)
        Assert.Equal("00000000-0000-0000-0000-000000000000", Uuid.NIL.ToString())
        Assert.Equal("ffffffff-ffff-ffff-ffff-ffffffffffff", Uuid.MAX.ToString())

    [<Fact>]
    member _.VariantClassifiesAllFamilies() =
        for nibble in [ "8"; "9"; "a"; "b" ] do
            let uuid = Uuid.fromString $"550e8400-e29b-41d4-{nibble}716-446655440000"
            Assert.Equal("rfc4122", uuid.Variant)

        Assert.Equal("ncs", { Msb = 0UL; Lsb = 0x0000_0000_0000_0000UL }.Variant)
        Assert.Equal("ncs", { Msb = 0UL; Lsb = 0x4000_0000_0000_0000UL }.Variant)
        Assert.Equal("microsoft", { Msb = 0UL; Lsb = 0xC000_0000_0000_0000UL }.Variant)
        Assert.Equal("reserved", { Msb = 0UL; Lsb = 0xE000_0000_0000_0000UL }.Variant)

    [<Fact>]
    member _.EqualityHashAndOrderingUseUnsignedBytes() =
        let first = Uuid.fromString "00000000-0000-0000-0000-000000000001"
        let second = Uuid.fromString "00000000-0000-0000-0000-000000000002"
        let high = Uuid.fromString "ffffffff-ffff-ffff-ffff-ffffffffffff"

        Assert.Equal(first, Uuid.fromString(first.ToString()))
        Assert.Equal(first.GetHashCode(), (Uuid.fromString(first.ToString())).GetHashCode())
        Assert.True(compare first second < 0)
        Assert.True(compare second first > 0)
        Assert.True(compare first high < 0)
        Assert.Equal(0, compare first first)
        Assert.Single(Set.ofList [ first; Uuid.fromString(first.ToString()) ]) |> ignore

    [<Fact>]
    member _.IsValidMatchesParser() =
        Assert.True(Uuid.isValid "550e8400-e29b-41d4-a716-446655440000")
        Assert.True(Uuid.isValid "550e8400e29b41d4a716446655440000")
        Assert.True(Uuid.isValid "{550e8400-e29b-41d4-a716-446655440000}")
        Assert.True(Uuid.isValid "urn:uuid:550e8400-e29b-41d4-a716-446655440000")
        Assert.False(Uuid.isValid "not-a-uuid")
        Assert.False(Uuid.isValid "")
        Assert.False(Uuid.isValid null)
        Assert.False(Uuid.isValid "550e8400-e29b-41d4-a716-44665544000z")

    [<Fact>]
    member _.V4HasVersionVariantFormatAndUniqueness() =
        let value = Uuid.v4 ()
        let text = value.ToString()

        Assert.Equal(4, value.Version)
        Assert.Equal("rfc4122", value.Variant)
        Assert.Equal('4', text[14])
        Assert.Contains(text[19], [| '8'; '9'; 'a'; 'b' |])

        let seen = [ 1..1000 ] |> List.map (fun _ -> Uuid.v4 ()) |> Set.ofList
        Assert.Equal(1000, seen.Count)

    [<Fact>]
    member _.V7HasVersionVariantTimestampAndUniqueness() =
        let first = Uuid.v7 ()
        let second = Uuid.v7 ()
        let firstTimestamp = first.Msb >>> 16
        let secondTimestamp = second.Msb >>> 16

        Assert.Equal(7, first.Version)
        Assert.Equal("rfc4122", first.Variant)
        Assert.True(firstTimestamp <= secondTimestamp)

        let seen = [ 1..100 ] |> List.map (fun _ -> Uuid.v7 ()) |> Set.ofList
        Assert.Equal(100, seen.Count)

    [<Fact>]
    member _.V1HasVersionVariantAndUniqueness() =
        let value = Uuid.v1 ()

        Assert.Equal(1, value.Version)
        Assert.Equal("rfc4122", value.Variant)

        let seen = [ 1..100 ] |> List.map (fun _ -> Uuid.v1 ()) |> Set.ofList
        Assert.Equal(100, seen.Count)

    [<Fact>]
    member _.V5MatchesRfcVectorAndIsDeterministic() =
        Assert.Equal("886313e1-3b8a-5372-9b90-0c9aee199e5d", (Uuid.v5 Uuid.NAMESPACE_DNS "python.org").ToString())
        Assert.Equal(Uuid.v5 Uuid.NAMESPACE_DNS "example.com", Uuid.v5 Uuid.NAMESPACE_DNS "example.com")
        Assert.NotEqual(Uuid.v5 Uuid.NAMESPACE_DNS "example.com", Uuid.v5 Uuid.NAMESPACE_DNS "example.org")
        Assert.NotEqual(Uuid.v5 Uuid.NAMESPACE_DNS "example.com", Uuid.v5 Uuid.NAMESPACE_URL "example.com")
        Assert.Equal(5, (Uuid.v5 Uuid.NAMESPACE_DNS "test").Version)
        Assert.Equal("rfc4122", (Uuid.v5 Uuid.NAMESPACE_DNS "test").Variant)
        Assert.Throws<ArgumentNullException>(fun () -> Uuid.v5 Uuid.NAMESPACE_DNS null |> ignore) |> ignore

    [<Fact>]
    member _.V3MatchesRfcVectorAndIsDeterministic() =
        Assert.Equal("6fa459ea-ee8a-3ca4-894e-db77e160355e", (Uuid.v3 Uuid.NAMESPACE_DNS "python.org").ToString())
        Assert.Equal(Uuid.v3 Uuid.NAMESPACE_DNS "example.com", Uuid.v3 Uuid.NAMESPACE_DNS "example.com")
        Assert.NotEqual(Uuid.v3 Uuid.NAMESPACE_DNS "python.org", Uuid.v5 Uuid.NAMESPACE_DNS "python.org")
        Assert.Equal(3, (Uuid.v3 Uuid.NAMESPACE_DNS "test").Version)
        Assert.Throws<ArgumentNullException>(fun () -> Uuid.v3 Uuid.NAMESPACE_DNS null |> ignore) |> ignore

    [<Fact>]
    member _.NamespaceConstantsMatchRfcValues() =
        Assert.Equal("6ba7b810-9dad-11d1-80b4-00c04fd430c8", Uuid.NAMESPACE_DNS.ToString())
        Assert.Equal("6ba7b811-9dad-11d1-80b4-00c04fd430c8", Uuid.NAMESPACE_URL.ToString())
        Assert.Equal("6ba7b812-9dad-11d1-80b4-00c04fd430c8", Uuid.NAMESPACE_OID.ToString())
        Assert.Equal("6ba7b814-9dad-11d1-80b4-00c04fd430c8", Uuid.NAMESPACE_X500.ToString())

    [<Fact>]
    member _.ExceptionCanWrapCause() =
        let inner = InvalidOperationException "boom"
        let exceptionValue = UuidException("wrapped", inner)

        Assert.Equal("wrapped", exceptionValue.Message)
        Assert.Same(inner, exceptionValue.InnerException)
