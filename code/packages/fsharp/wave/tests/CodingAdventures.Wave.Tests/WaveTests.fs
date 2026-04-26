namespace CodingAdventures.Wave.FSharp.Tests

open System
open CodingAdventures.Wave.FSharp
open Xunit

type WaveTests() =
    let epsilon = 1e-10

    [<Fact>]
    member _.``constructor stores amplitude frequency and default phase``() =
        let wave = Wave(1.0, 440.0)

        Assert.Equal(1.0, wave.Amplitude)
        Assert.Equal(440.0, wave.Frequency)
        Assert.Equal(0.0, wave.Phase)

    [<Fact>]
    member _.``constructor stores explicit phase``() =
        let wave = Wave(2.0, 100.0, Math.PI / 2.0)

        Assert.Equal(Math.PI / 2.0, wave.Phase, epsilon)

    [<Fact>]
    member _.``period is inverse frequency``() =
        Assert.Equal(0.25, Wave(1.0, 4.0).Period, epsilon)

    [<Fact>]
    member _.``angular frequency is two pi times frequency``() =
        Assert.Equal(2.0 * Math.PI, Wave(1.0, 1.0).AngularFrequency, epsilon)

    [<Fact>]
    member _.``evaluate handles zero crossing peak and trough``() =
        Assert.Equal(0.0, Wave(1.0, 1.0).Evaluate 0.0, epsilon)
        Assert.Equal(3.0, Wave(3.0, 1.0).Evaluate 0.25, 1e-9)
        Assert.Equal(-2.0, Wave(2.0, 1.0).Evaluate 0.75, 1e-9)

    [<Fact>]
    member _.``evaluate is periodic``() =
        let wave = Wave(2.0, 5.0)
        let time = 0.123

        Assert.Equal(wave.Evaluate time, wave.Evaluate(time + wave.Period), 1e-9)

    [<Fact>]
    member _.``phase shifts wave``() =
        Assert.Equal(1.0, Wave(1.0, 1.0, Math.PI / 2.0).Evaluate 0.0, 1e-9)

        let wave = Wave(1.0, 1.0, 0.0)
        let opposite = Wave(1.0, 1.0, Math.PI)
        Assert.Equal(0.0, wave.Evaluate 0.3 + opposite.Evaluate 0.3, 1e-9)

    [<Fact>]
    member _.``zero amplitude always evaluates to zero``() =
        Assert.Equal(0.0, Wave(0.0, 1.0).Evaluate 0.5, epsilon)

    [<Fact>]
    member _.``invalid parameters throw``() =
        Assert.Throws<ArgumentException>(fun () -> Wave(-1.0, 1.0) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Wave(1.0, 0.0) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Wave(1.0, -1.0) |> ignore) |> ignore

    [<Fact>]
    member _.``high frequency has expected period and quarter cycle peak``() =
        let wave = Wave(1.0, 1000.0)

        Assert.Equal(0.001, wave.Period, epsilon)
        Assert.Equal(1.0, wave.Evaluate 0.00025, 1e-8)
