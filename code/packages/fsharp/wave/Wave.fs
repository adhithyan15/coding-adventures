namespace CodingAdventures.Wave.FSharp

open System
open CodingAdventures.Trig

type Wave(amplitude: float, frequency: float, ?phase: float) =
    let phase = defaultArg phase 0.0

    do
        if not (Double.IsFinite amplitude) || amplitude < 0.0 then
            invalidArg "amplitude" "Amplitude must be finite and non-negative"
        if not (Double.IsFinite frequency) || frequency <= 0.0 then
            invalidArg "frequency" "Frequency must be finite and positive"
        if frequency > Double.MaxValue / (2.0 * Trig.PI) then
            invalidArg "frequency" "Angular frequency must remain finite"
        if not (Double.IsFinite phase) then
            invalidArg "phase" "Phase must be finite"

    member _.Amplitude = amplitude
    member _.Frequency = frequency
    member _.Phase = phase

    member _.Period = 1.0 / frequency
    member _.AngularFrequency = 2.0 * Trig.PI * frequency

    member this.Evaluate(time: float) =
        if not (Double.IsFinite time) then
            invalidArg "time" "Time must be finite"
        elif amplitude = 0.0 then
            0.0
        else
            let reducedTime = time % this.Period
            let reducedPhase = phase % (2.0 * Trig.PI)
            let angle = 2.0 * Trig.PI * (frequency * reducedTime) + reducedPhase
            let unitValue = Trig.sin angle
            if unitValue >= 1.0 then amplitude
            elif unitValue <= -1.0 then -amplitude
            else amplitude * unitValue
