namespace CodingAdventures.Wave.FSharp

open System

type Wave(amplitude: float, frequency: float, ?phase: float) =
    let phase = defaultArg phase 0.0

    do
        if amplitude < 0.0 then invalidArg "amplitude" "Amplitude must be non-negative"
        if frequency <= 0.0 then invalidArg "frequency" "Frequency must be positive"

    member _.Amplitude = amplitude
    member _.Frequency = frequency
    member _.Phase = phase

    member _.Period = 1.0 / frequency
    member _.AngularFrequency = 2.0 * Math.PI * frequency

    member this.Evaluate(time: float) =
        amplitude * sin (this.AngularFrequency * time + phase)
