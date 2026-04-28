namespace CodingAdventures.Resistor

open System

module ResistorPackage =
    [<Literal>]
    let Version = "0.1.0"

type Resistor(
    resistanceOhms: float,
    ?tolerance: float,
    ?tempcoPpmPerC: float,
    ?powerRatingWatts: float) =

    let tolerance = defaultArg tolerance 0.0
    let tempcoPpmPerC = defaultArg tempcoPpmPerC 0.0

    do
        if resistanceOhms <= 0.0 then
            invalidArg (nameof resistanceOhms) "Resistance must be > 0 ohms."

        if tolerance < 0.0 then
            invalidArg (nameof tolerance) "Tolerance must be >= 0."

        match powerRatingWatts with
        | Some rating when rating <= 0.0 -> invalidArg (nameof powerRatingWatts) "Power rating must be > 0 watts when provided."
        | _ -> ()

    member _.ResistanceOhms = resistanceOhms
    member _.Tolerance = tolerance
    member _.TempcoPpmPerC = tempcoPpmPerC
    member _.PowerRatingWatts = powerRatingWatts

    member _.Conductance() =
        1.0 / resistanceOhms

    member _.CurrentForVoltage(voltage: float) =
        voltage / resistanceOhms

    member _.VoltageForCurrent(current: float) =
        current * resistanceOhms

    member _.PowerForVoltage(voltage: float) =
        voltage * voltage / resistanceOhms

    member _.PowerForCurrent(current: float) =
        current * current * resistanceOhms

    member this.EnergyForVoltage(voltage: float, durationSeconds: float) =
        Resistor.validateDuration durationSeconds
        this.PowerForVoltage(voltage) * durationSeconds

    member this.EnergyForCurrent(current: float, durationSeconds: float) =
        Resistor.validateDuration durationSeconds
        this.PowerForCurrent(current) * durationSeconds

    member _.MinResistance() =
        resistanceOhms * (1.0 - tolerance)

    member _.MaxResistance() =
        resistanceOhms * (1.0 + tolerance)

    member _.ResistanceAtTemperature(celsius: float, ?referenceCelsius: float) =
        let referenceCelsius = defaultArg referenceCelsius 25.0
        let alpha = tempcoPpmPerC * 1e-6
        let deltaT = celsius - referenceCelsius
        resistanceOhms * (1.0 + alpha * deltaT)

    member this.IsWithinPowerRatingForVoltage(voltage: float) =
        match powerRatingWatts with
        | None -> true
        | Some rating -> this.PowerForVoltage(voltage) <= rating

    member this.IsWithinPowerRatingForCurrent(current: float) =
        match powerRatingWatts with
        | None -> true
        | Some rating -> this.PowerForCurrent(current) <= rating

    static member private validateDuration(durationSeconds: float) =
        if durationSeconds < 0.0 then
            invalidArg (nameof durationSeconds) "Duration must be >= 0 seconds."

module ResistorNetwork =
    let private materialize (resistors: seq<Resistor>) : Resistor list =
        if isNull (box resistors) then
            nullArg (nameof resistors)

        let items = resistors |> Seq.toList

        for resistor in items do
            if isNull (box resistor) then
                nullArg (nameof resistors)

        if List.isEmpty items then
            invalidArg (nameof resistors) "At least one resistor is required."

        items

    let seriesEquivalent (resistors: seq<Resistor>) =
        materialize resistors
        |> List.sumBy (fun resistor -> resistor.ResistanceOhms)

    let parallelEquivalent (resistors: seq<Resistor>) =
        let reciprocalSum =
            materialize resistors
            |> List.sumBy (fun resistor -> 1.0 / resistor.ResistanceOhms)

        1.0 / reciprocalSum

    let voltageDivider vin (rTop: Resistor) (rBottom: Resistor) =
        if isNull (box rTop) then
            nullArg (nameof rTop)

        if isNull (box rBottom) then
            nullArg (nameof rBottom)

        let total = rTop.ResistanceOhms + rBottom.ResistanceOhms
        vin * (rBottom.ResistanceOhms / total)
