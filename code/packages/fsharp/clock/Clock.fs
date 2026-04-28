namespace CodingAdventures.Clock.FSharp

open System
open System.Collections.Generic

[<CLIMutable>]
type ClockEdge =
    { Cycle: int64
      Value: int
      IsRising: bool
      IsFalling: bool }

[<AllowNullLiteral>]
type Clock(?frequencyHz: int64) =
    let frequencyHz = defaultArg frequencyHz 1_000_000L
    do
        if frequencyHz <= 0L then
            invalidArg "frequencyHz" "frequency_hz must be > 0."

    let listeners = ResizeArray<Action<ClockEdge>>()
    let mutable cycle = 0L
    let mutable value = 0
    let mutable totalTicks = 0L

    member _.FrequencyHz = frequencyHz
    member _.Cycle = cycle
    member _.Value = value
    member _.TotalTicks = totalTicks
    member _.PeriodNs = 1_000_000_000.0 / float frequencyHz

    member _.Tick() =
        let oldValue = value
        value <- 1 - value
        totalTicks <- totalTicks + 1L

        let isRising = oldValue = 0 && value = 1
        let isFalling = oldValue = 1 && value = 0

        if isRising then
            cycle <- cycle + 1L

        let edge =
            { Cycle = cycle
              Value = value
              IsRising = isRising
              IsFalling = isFalling }

        for listener in listeners |> Seq.toArray do
            listener.Invoke edge

        edge

    member this.FullCycle() =
        let rising = this.Tick()
        let falling = this.Tick()
        rising, falling

    member this.Run(cycles: int64) =
        if cycles < 0L then
            invalidArg "cycles" "cycles must be >= 0."

        let edges = ResizeArray<ClockEdge>()
        let mutable remaining = cycles

        while remaining > 0L do
            let rising, falling = this.FullCycle()
            edges.Add rising
            edges.Add falling
            remaining <- remaining - 1L

        edges |> Seq.toList

    member _.RegisterListener(callback: Action<ClockEdge>) =
        if isNull callback then
            nullArg "callback"

        listeners.Add callback

    member _.UnregisterListener(callback: Action<ClockEdge>) =
        if isNull callback then
            nullArg "callback"

        if not (listeners.Remove callback) then
            raise (ArgumentException("Listener was not registered.", "callback"))

    member _.Reset() =
        cycle <- 0L
        value <- 0
        totalTicks <- 0L

type ClockDivider(source: Clock, divisor: int64) =
    let source =
        if isNull source then
            nullArg "source"
        else
            source

    do
        if divisor < 2L then
            invalidArg "divisor" $"Divisor must be >= 2, got {divisor}."

    let output = Clock(source.FrequencyHz / divisor)
    let mutable counter = 0L

    let listener =
        Action<ClockEdge>(fun edge ->
            if edge.IsRising then
                counter <- counter + 1L

                if counter >= divisor then
                    counter <- 0L
                    output.Tick() |> ignore
                    output.Tick() |> ignore)

    do source.RegisterListener listener

    member _.Source = source
    member _.Divisor = divisor
    member _.Output = output

type MultiPhaseClock(source: Clock, ?phases: int) =
    let source =
        if isNull source then
            nullArg "source"
        else
            source

    let phases = defaultArg phases 4

    do
        if phases < 2 then
            invalidArg "phases" $"Phases must be >= 2, got {phases}."

    let phaseValues = Array.zeroCreate<int> phases
    let mutable activePhase = 0

    let listener =
        Action<ClockEdge>(fun edge ->
            if edge.IsRising then
                Array.Fill(phaseValues, 0)
                phaseValues[activePhase] <- 1
                activePhase <- (activePhase + 1) % phases)

    do source.RegisterListener listener

    member _.Source = source
    member _.Phases = phases
    member _.ActivePhase = activePhase
    member _.GetPhase(index: int) =
        if index < 0 || index >= phases then
            invalidArg "index" "index is outside the configured phase range."

        phaseValues[index]

    member _.PhaseValues = phaseValues |> Array.copy

[<RequireQualifiedAccess>]
module ClockPackage =
    [<Literal>]
    let Version = "0.1.0"
