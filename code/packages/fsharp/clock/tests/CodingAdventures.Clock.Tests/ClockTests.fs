namespace CodingAdventures.Clock.Tests

open System
open System.Collections.Generic
open Xunit
open CodingAdventures.Clock.FSharp

module ClockTests =
    [<Fact>]
    let ``clock starts in known state`` () =
        let clock = Clock()

        Assert.Equal(1_000_000L, clock.FrequencyHz)
        Assert.Equal(0L, clock.Cycle)
        Assert.Equal(0, clock.Value)
        Assert.Equal(0L, clock.TotalTicks)

    [<Fact>]
    let ``tick alternates edges and cycles`` () =
        let clock = Clock()

        let first = clock.Tick()
        let second = clock.Tick()
        let third = clock.Tick()

        Assert.Equal({ Cycle = 1L; Value = 1; IsRising = true; IsFalling = false }, first)
        Assert.Equal({ Cycle = 1L; Value = 0; IsRising = false; IsFalling = true }, second)
        Assert.Equal({ Cycle = 2L; Value = 1; IsRising = true; IsFalling = false }, third)
        Assert.Equal(3L, clock.TotalTicks)

    [<Fact>]
    let ``full cycle returns rising then falling`` () =
        let clock = Clock()

        let rising, falling = clock.FullCycle()

        Assert.True rising.IsRising
        Assert.True falling.IsFalling
        Assert.Equal(0, clock.Value)
        Assert.Equal(1L, clock.Cycle)
        Assert.Equal(2L, clock.TotalTicks)

    [<Fact>]
    let ``run produces two edges per cycle`` () =
        let clock = Clock()

        let edges = clock.Run 5L

        Assert.Equal(10, edges.Length)
        Assert.Equal(5L, clock.Cycle)
        Assert.Empty(clock.Run 0L)

    [<Fact>]
    let ``listeners receive edges and can be removed`` () =
        let clock = Clock()
        let received = List<ClockEdge>()
        let listener = Action<ClockEdge>(fun edge -> received.Add edge)

        clock.RegisterListener listener
        clock.Run 2L |> ignore
        clock.UnregisterListener listener
        clock.Tick() |> ignore

        Assert.Equal(4, received.Count)
        Assert.Throws<ArgumentException>(fun () -> clock.UnregisterListener listener) |> ignore

    [<Fact>]
    let ``reset preserves listeners and frequency`` () =
        let clock = Clock(5_000_000L)
        let received = List<ClockEdge>()
        clock.RegisterListener(Action<ClockEdge>(fun edge -> received.Add edge))

        clock.Run 3L |> ignore
        clock.Reset()
        clock.Tick() |> ignore

        Assert.Equal(5_000_000L, clock.FrequencyHz)
        Assert.Equal(1L, received[received.Count - 1].Cycle)
        Assert.Equal(1L, clock.Cycle)
        Assert.Equal(1L, clock.TotalTicks)

    [<Theory>]
    [<InlineData(1_000_000L, 1000.0)>]
    [<InlineData(1_000_000_000L, 1.0)>]
    let ``period uses frequency`` (frequency: int64) (expectedPeriodNs: double) =
        let clock = Clock frequency

        Assert.Equal(expectedPeriodNs, clock.PeriodNs, 10)

    [<Fact>]
    let ``clock rejects invalid frequency`` () =
        Assert.Throws<ArgumentException>(fun () -> Clock(0L) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Clock(-1L) |> ignore) |> ignore

    [<Fact>]
    let ``divider produces output cycles from rising edges`` () =
        let master = Clock 1_000_000_000L
        let divider = ClockDivider(master, 4L)

        master.Run 8L |> ignore

        Assert.Equal(250_000_000L, divider.Output.FrequencyHz)
        Assert.Equal(2L, divider.Output.Cycle)
        Assert.Equal(0, divider.Output.Value)

    [<Theory>]
    [<InlineData(0L)>]
    [<InlineData(1L)>]
    [<InlineData(-1L)>]
    let ``divider rejects invalid divisors`` (divisor: int64) =
        let master = Clock()

        Assert.Throws<ArgumentException>(fun () -> ClockDivider(master, divisor) |> ignore) |> ignore

    [<Fact>]
    let ``multi phase clock rotates one active phase`` () =
        let master = Clock()
        let phases = MultiPhaseClock(master, 4)

        for expected in 0 .. 3 do
            master.Tick() |> ignore

            for phase in 0 .. 3 do
                Assert.Equal((if phase = expected then 1 else 0), phases.GetPhase phase)

            master.Tick() |> ignore

        master.Tick() |> ignore
        Assert.Equal(1, phases.GetPhase 0)

    [<Fact>]
    let ``multi phase clock rejects invalid phase counts`` () =
        let master = Clock()

        Assert.Throws<ArgumentException>(fun () -> MultiPhaseClock(master, 1) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> MultiPhaseClock(master, 0) |> ignore) |> ignore
