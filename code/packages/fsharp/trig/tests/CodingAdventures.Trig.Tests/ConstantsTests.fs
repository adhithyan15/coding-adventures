namespace CodingAdventures.Trig.Tests

// The constants of TRIG-REF-1 (PHY00a 3.2) are pinned by bit pattern.  The
// manifest.json of the shared fixture lists every one; this suite compares the
// library's constants with that list, bit for bit, and checks that the decimal
// literals the source still uses (PI, the Part II thresholds) parse to the
// pinned bits (PHY00a 5: "a decimal constant is accepted only if its bits are
// asserted in the port's tests").

open System
open System.IO
open System.Text.Json
open Xunit
open CodingAdventures.Trig

type ConstantsTests() =
    let bitsOf (v: float) = BitConverter.DoubleToInt64Bits v

    [<Fact>]
    member _.``Every manifest constant is in the library bit for bit and nothing else is``() =
        let text = File.ReadAllText(Path.Combine(Fixture.directory, "manifest.json"))
        use doc = JsonDocument.Parse text

        let manifest =
            [ for p in doc.RootElement.GetProperty("constants").EnumerateObject() ->
                  p.Name, Convert.ToInt64(p.Value.GetString(), 16) ]
            |> List.sort

        let ours = TrigConstants.table |> List.map (fun (n, v) -> n, bitsOf v) |> List.sort
        Assert.Equal<(string * int64) list>(manifest, ours)

    [<Fact>]
    member _.``The exported circle constants are the manifest's``() =
        Assert.Equal(0x400921FB54442D18L, bitsOf Trig.PI)
        Assert.Equal(0x400921FB54442D18L, bitsOf TrigConstants.pi)
        Assert.Equal(0x401921FB54442D18L, bitsOf TrigConstants.twoPi)
        Assert.Equal(0x3FF921FB54442D18L, bitsOf TrigConstants.halfPi)

    [<Fact>]
    member _.``The degree constants equal the divisions of PI``() =
        Assert.Equal(TrigConstants.pi / 180.0, TrigConstants.radPerDeg)
        Assert.Equal(180.0 / TrigConstants.pi, TrigConstants.degPerRad)

    [<Fact>]
    member _.``Decimal literals agree with the pinned legacy bits``() =
        Assert.Equal(bitsOf 1.0e-15, bitsOf TrigConstants.tanPole)
        Assert.Equal(bitsOf 1.0e308, bitsOf TrigConstants.tanBig)
        Assert.Equal(bitsOf 1.0e-17, bitsOf TrigConstants.atanTerm)
        Assert.Equal(bitsOf 1.0e-300, bitsOf TrigConstants.sqrtTiny)
        Assert.Equal(bitsOf 7.450580596923828e-9, bitsOf TrigConstants.tiny)
