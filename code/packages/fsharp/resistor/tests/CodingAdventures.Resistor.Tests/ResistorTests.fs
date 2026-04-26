namespace CodingAdventures.Resistor.Tests

open System
open CodingAdventures.Resistor
open Xunit

type ResistorTests() =
    [<Fact>]
    member _.VersionExists() =
        Assert.Equal("0.1.0", ResistorPackage.Version)

    [<Fact>]
    member _.StoresNominalProperties() =
        let resistor =
            Resistor(1_000.0, tolerance = 0.01, tempcoPpmPerC = 100.0, powerRatingWatts = 0.25)

        Assert.Equal(1_000.0, resistor.ResistanceOhms)
        Assert.Equal(0.01, resistor.Tolerance)
        Assert.Equal(100.0, resistor.TempcoPpmPerC)
        Assert.Equal(Some 0.25, resistor.PowerRatingWatts)

    [<Fact>]
    member _.RejectsInvalidConstruction() =
        Assert.Throws<ArgumentException>(fun () -> Resistor(0.0) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Resistor(100.0, tolerance = -0.01) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Resistor(100.0, powerRatingWatts = 0.0) |> ignore) |> ignore

    [<Fact>]
    member _.OhmsLawHelpersUseResistance() =
        let resistor = Resistor(1_000.0)

        Assert.Equal(0.001, resistor.Conductance(), 12)
        Assert.Equal(0.005, resistor.CurrentForVoltage(5.0), 12)
        Assert.Equal(5.0, resistor.VoltageForCurrent(0.005), 12)
        Assert.Equal(-0.002, resistor.CurrentForVoltage(-2.0), 12)

    [<Fact>]
    member _.PowerAndEnergyHelpersAgree() =
        let resistor = Resistor(1_000.0)

        Assert.Equal(0.025, resistor.PowerForVoltage(5.0), 12)
        Assert.Equal(0.025, resistor.PowerForCurrent(0.005), 12)
        Assert.Equal(0.05, resistor.EnergyForVoltage(5.0, 2.0), 12)
        Assert.Equal(0.05, resistor.EnergyForCurrent(0.005, 2.0), 12)

    [<Fact>]
    member _.EnergyRejectsNegativeDurations() =
        let resistor = Resistor(100.0)

        Assert.Throws<ArgumentException>(fun () -> resistor.EnergyForVoltage(10.0, -1.0) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> resistor.EnergyForCurrent(0.1, -1.0) |> ignore) |> ignore

    [<Fact>]
    member _.ToleranceAndTemperatureHelpersWork() =
        let resistor = Resistor(1_000.0, tolerance = 0.01, tempcoPpmPerC = 100.0)

        Assert.Equal(990.0, resistor.MinResistance(), 12)
        Assert.Equal(1_010.0, resistor.MaxResistance(), 12)
        Assert.Equal(1_005.0, resistor.ResistanceAtTemperature(75.0), 12)
        Assert.Equal(995.0, resistor.ResistanceAtTemperature(-25.0), 12)

    [<Fact>]
    member _.PowerRatingChecksRespectOptionalRating() =
        let unrated = Resistor(100.0)
        let rated = Resistor(100.0, powerRatingWatts = 0.25)

        Assert.True(unrated.IsWithinPowerRatingForVoltage(1_000.0))
        Assert.True(unrated.IsWithinPowerRatingForCurrent(100.0))
        Assert.True(rated.IsWithinPowerRatingForVoltage(5.0))
        Assert.False(rated.IsWithinPowerRatingForVoltage(10.0))
        Assert.True(rated.IsWithinPowerRatingForCurrent(0.04))
        Assert.False(rated.IsWithinPowerRatingForCurrent(0.1))

    [<Fact>]
    member _.NetworkHelpersComputeEquivalentResistance() =
        Assert.Equal(600.0, ResistorNetwork.seriesEquivalent [ Resistor(100.0); Resistor(200.0); Resistor(300.0) ], 12)
        Assert.Equal(500.0, ResistorNetwork.parallelEquivalent [ Resistor(1_000.0); Resistor(1_000.0) ], 12)

    [<Fact>]
    member _.NetworkHelpersValidateResistorLists() =
        Assert.Throws<ArgumentNullException>(fun () -> ResistorNetwork.seriesEquivalent Unchecked.defaultof<Resistor list> |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> ResistorNetwork.seriesEquivalent [] |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> ResistorNetwork.seriesEquivalent [ Unchecked.defaultof<Resistor> ] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> ResistorNetwork.parallelEquivalent [] |> ignore) |> ignore

    [<Fact>]
    member _.VoltageDividerUsesBottomResistanceRatio() =
        let top = Resistor(1_000.0)
        let bottom = Resistor(1_000.0)

        Assert.Equal(2.5, ResistorNetwork.voltageDivider 5.0 top bottom, 12)
        Assert.Throws<ArgumentNullException>(fun () -> ResistorNetwork.voltageDivider 5.0 Unchecked.defaultof<Resistor> bottom |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> ResistorNetwork.voltageDivider 5.0 top Unchecked.defaultof<Resistor> |> ignore) |> ignore
