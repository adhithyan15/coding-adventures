namespace CodingAdventures.Hamurabi

open System
open System.Numerics
open System.Text

/// <summary>
/// Rating classification awarded to ruler upon completing a 10-year term.
/// </summary>
type Rating =
    | Charlemagne
    | Acre
    | Nero
    | Disaster

/// <summary>
/// Simulation lifecycle status.
/// </summary>
type GameStatus =
    | Playing
    | Won of rating: Rating
    | Impeached of starvedThisTurn: uint32 * percent: uint32
    | GameOverExtinction
    override this.ToString() =
        match this with
        | Playing -> "Playing"
        | Won r -> $"Won({r})"
        | Impeached (st, p) -> $"Impeached(starved={st}, percent={p})"
        | GameOverExtinction -> "GameOverExtinction"

/// <summary>
/// Closed enumeration of order rejection errors.
/// </summary>
type HamurabiError =
    | GameAlreadyEnded
    | InvalidOrderBounds
    | InsufficientBushelsForLand
    | InsufficientAcresToSell
    | InsufficientBushelsToFeed
    | InsufficientAcresToPlant
    | InsufficientPopulationToPlant
    | InsufficientGrainForSeed

/// <summary>
/// Player orders for a single year.
/// </summary>
type TurnOrder = {
    AcresDelta: int
    BushelsToFeed: uint32
    AcresToPlant: uint32
}

/// <summary>
/// Immutable value record capturing the complete simulation state.
/// </summary>
type HamurabiState = {
    Year: byte
    Population: uint32
    Bushels: uint32
    Acres: uint32
    LandPrice: uint32
    HarvestPerAcre: uint32
    RatsEaten: uint32
    StarvedLastYear: uint32
    StarvedTotal: uint32
    PercentStarvedSum: uint32
    ImmigrantsLastYear: uint32
    PlagueStruck: bool
    Status: GameStatus
    RngState: uint64
    RngInc: uint64
}

/// <summary>
/// Pure deterministic simulation engine for Hamurabi (GAME01).
/// </summary>
module Hamurabi =

    /// <summary>LCG multiplier constant for 64-bit PCG32 (Knuth / Numerical Recipes).</summary>
    [<Literal>]
    let PcgMultiplier = 6364136223846793005UL

    /// <summary>Stream ID for PCG32 PRNG stream in Hamurabi simulation.</summary>
    [<Literal>]
    let HamurabiStreamId = 54UL

    /// <summary>Default odd increment for PCG32.</summary>
    [<Literal>]
    let DefaultRngInc = (HamurabiStreamId <<< 1) ||| 1UL

    /// <summary>Advances the PCG32 state by one step and returns next state and a 32-bit random output.</summary>
    let pcg32Step (state: uint64) (inc: uint64) : uint64 * uint32 =
        let nextState = (state * PcgMultiplier) + inc
        let xorshifted = uint32 (((state >>> 18) ^^^ state) >>> 27)
        let rot = int (state >>> 59)
        let result = BitOperations.RotateRight(xorshifted, rot)
        (nextState, result)

    /// <summary>Draws a uniform 32-bit integer in the inclusive range [min, max] via rejection sampling.</summary>
    let pcg32NextIntInRange (state: uint64) (inc: uint64) (min: uint32) (max: uint32) : uint64 * uint32 =
        let span = uint64 (max - min) + 1UL
        let threshold = ((1UL <<< 32) - span) % span
        let rec loop s =
            let sNext, rU32 = pcg32Step s inc
            let r = uint64 rU32
            if r >= threshold then
                (sNext, min + uint32 (r % span))
            else
                loop sNext
        loop state

    /// <summary>
    /// Initializes a new, deterministic 10-year reign for ruler Hamurabi.
    /// Seeds PCG32 with (seed, stream_id = 54) and draws initial year 1 land price (range 17..=26).
    /// </summary>
    let newGame (seed: uint64) : HamurabiState =
        let inc = DefaultRngInc
        let mutable s = 0UL
        s <- (s * PcgMultiplier) + inc
        s <- s + seed
        s <- (s * PcgMultiplier) + inc

        let sFinal, landPrice = pcg32NextIntInRange s inc 17u 26u

        {
            Year = 1uy
            Population = 100u
            Bushels = 2800u
            Acres = 1000u
            LandPrice = landPrice
            HarvestPerAcre = 3u
            RatsEaten = 200u
            StarvedLastYear = 0u
            StarvedTotal = 0u
            PercentStarvedSum = 0u
            ImmigrantsLastYear = 5u
            PlagueStruck = false
            Status = Playing
            RngState = sFinal
            RngInc = inc
        }

    let new_game (seed: uint64) = newGame seed
    let NewGame (seed: uint64) = newGame seed

    /// <summary>
    /// Validates player orders against current state constraints without advancing state.
    /// Evaluates checks in the canonical 7-step order specified by GAME01.
    /// </summary>
    let validateOrder (state: HamurabiState) (order: TurnOrder) : Result<unit, HamurabiError> =
        // 1. Status Check
        match state.Status with
        | Playing ->
            // State invariant checks (VER04m state_inv & CWE-400 / CWE-835)
            if (state.RngInc &&& 1UL) <> 1UL then
                Error InvalidOrderBounds
            elif state.Year < 1uy || state.Year > 10uy then
                Error InvalidOrderBounds
            elif state.LandPrice < 17u || state.LandPrice > 26u then
                Error InvalidOrderBounds
            elif state.HarvestPerAcre < 1u || state.HarvestPerAcre > 5u then
                Error InvalidOrderBounds
            else
                // 2. Bounds & Overflow Guard on Land Trade
                if order.AcresDelta > 0 then
                    let buyCount = uint64 order.AcresDelta
                    if state.LandPrice <> 0u && buyCount > (UInt64.MaxValue / uint64 state.LandPrice) then
                        Error InvalidOrderBounds
                    else
                        let cost = buyCount * uint64 state.LandPrice
                        if cost > uint64 state.Bushels then
                            Error InsufficientBushelsForLand
                        else
                            // Check acres trade balances
                            let newAcres = uint64 state.Acres + buyCount
                            if newAcres > uint64 UInt32.MaxValue then
                                Error InvalidOrderBounds
                            else
                                let acresAfterTrade = uint32 newAcres
                                let bushelsAfterTrade = uint32 (uint64 state.Bushels - cost)

                                // 4. Food Allocation Check
                                if order.BushelsToFeed > bushelsAfterTrade then
                                    Error InsufficientBushelsToFeed
                                else
                                    let bushelsAfterFeed = bushelsAfterTrade - order.BushelsToFeed

                                    // 5. Planting Land & Labor Checks
                                    if order.AcresToPlant > acresAfterTrade then
                                        Error InsufficientAcresToPlant
                                    else
                                        let maxLaborAcres =
                                            if uint64 state.Population > (UInt64.MaxValue / 10UL) then
                                                UInt64.MaxValue
                                            else
                                                uint64 state.Population * 10UL
                                        if uint64 order.AcresToPlant > maxLaborAcres then
                                            Error InsufficientPopulationToPlant
                                        else
                                            // 6. Planting Seed Check
                                            let seedNeeded = (uint64 order.AcresToPlant + 1UL) / 2UL
                                            if seedNeeded > uint64 bushelsAfterFeed then
                                                Error InsufficientGrainForSeed
                                            else
                                                let bushelsAfterSeed = bushelsAfterFeed - uint32 seedNeeded

                                                // 7. Maximum Harvest Overflow Guard (CWE-190)
                                                if uint64 order.AcresToPlant > (UInt64.MaxValue / 5UL) then
                                                    Error InvalidOrderBounds
                                                else
                                                    let maxHarvest = uint64 order.AcresToPlant * 5UL
                                                    if UInt64.MaxValue - uint64 bushelsAfterSeed < maxHarvest then
                                                        Error InvalidOrderBounds
                                                    else
                                                        let maxStored = uint64 bushelsAfterSeed + maxHarvest
                                                        if maxStored > uint64 UInt32.MaxValue then
                                                            Error InvalidOrderBounds
                                                        else
                                                            Ok ()
                elif order.AcresDelta < 0 then
                    let deltaLong = int64 order.AcresDelta
                    let sellCountLong = -deltaLong
                    if sellCountLong < 0L then
                        Error InvalidOrderBounds
                    else
                        let sellCount = uint64 sellCountLong
                        if sellCount > uint64 state.Acres then
                            Error InsufficientAcresToSell
                        elif state.LandPrice <> 0u && sellCount > (UInt64.MaxValue / uint64 state.LandPrice) then
                            Error InvalidOrderBounds
                        else
                            let revenue = sellCount * uint64 state.LandPrice
                            if UInt64.MaxValue - uint64 state.Bushels < revenue then
                                Error InvalidOrderBounds
                            else
                                let newBushels = uint64 state.Bushels + revenue
                                if newBushels > uint64 UInt32.MaxValue then
                                    Error InvalidOrderBounds
                                else
                                    let acresAfterTrade = uint32 (uint64 state.Acres - sellCount)
                                    let bushelsAfterTrade = uint32 newBushels

                                    // 4. Food Allocation Check
                                    if order.BushelsToFeed > bushelsAfterTrade then
                                        Error InsufficientBushelsToFeed
                                    else
                                        let bushelsAfterFeed = bushelsAfterTrade - order.BushelsToFeed

                                        // 5. Planting Land & Labor Checks
                                        if order.AcresToPlant > acresAfterTrade then
                                            Error InsufficientAcresToPlant
                                        else
                                            let maxLaborAcres =
                                                if uint64 state.Population > (UInt64.MaxValue / 10UL) then
                                                    UInt64.MaxValue
                                                else
                                                    uint64 state.Population * 10UL
                                            if uint64 order.AcresToPlant > maxLaborAcres then
                                                Error InsufficientPopulationToPlant
                                            else
                                                // 6. Planting Seed Check
                                                let seedNeeded = (uint64 order.AcresToPlant + 1UL) / 2UL
                                                if seedNeeded > uint64 bushelsAfterFeed then
                                                    Error InsufficientGrainForSeed
                                                else
                                                    let bushelsAfterSeed = bushelsAfterFeed - uint32 seedNeeded

                                                    // 7. Maximum Harvest Overflow Guard (CWE-190)
                                                    if uint64 order.AcresToPlant > (UInt64.MaxValue / 5UL) then
                                                        Error InvalidOrderBounds
                                                    else
                                                        let maxHarvest = uint64 order.AcresToPlant * 5UL
                                                        if UInt64.MaxValue - uint64 bushelsAfterSeed < maxHarvest then
                                                            Error InvalidOrderBounds
                                                        else
                                                            let maxStored = uint64 bushelsAfterSeed + maxHarvest
                                                            if maxStored > uint64 UInt32.MaxValue then
                                                                Error InvalidOrderBounds
                                                            else
                                                                Ok ()
                else
                    // order.AcresDelta = 0
                    let acresAfterTrade = state.Acres
                    let bushelsAfterTrade = state.Bushels

                    // 4. Food Allocation Check
                    if order.BushelsToFeed > bushelsAfterTrade then
                        Error InsufficientBushelsToFeed
                    else
                        let bushelsAfterFeed = bushelsAfterTrade - order.BushelsToFeed

                        // 5. Planting Land & Labor Checks
                        if order.AcresToPlant > acresAfterTrade then
                            Error InsufficientAcresToPlant
                        else
                            let maxLaborAcres =
                                if uint64 state.Population > (UInt64.MaxValue / 10UL) then
                                    UInt64.MaxValue
                                else
                                    uint64 state.Population * 10UL
                            if uint64 order.AcresToPlant > maxLaborAcres then
                                Error InsufficientPopulationToPlant
                            else
                                // 6. Planting Seed Check
                                let seedNeeded = (uint64 order.AcresToPlant + 1UL) / 2UL
                                if seedNeeded > uint64 bushelsAfterFeed then
                                    Error InsufficientGrainForSeed
                                else
                                    let bushelsAfterSeed = bushelsAfterFeed - uint32 seedNeeded

                                    // 7. Maximum Harvest Overflow Guard (CWE-190)
                                    if uint64 order.AcresToPlant > (UInt64.MaxValue / 5UL) then
                                        Error InvalidOrderBounds
                                    else
                                        let maxHarvest = uint64 order.AcresToPlant * 5UL
                                        if UInt64.MaxValue - uint64 bushelsAfterSeed < maxHarvest then
                                            Error InvalidOrderBounds
                                        else
                                            let maxStored = uint64 bushelsAfterSeed + maxHarvest
                                            if maxStored > uint64 UInt32.MaxValue then
                                                Error InvalidOrderBounds
                                            else
                                                Ok ()
        | _ -> Error GameAlreadyEnded

    let validate_order (state: HamurabiState) (order: TurnOrder) = validateOrder state order
    let ValidateOrder (state: HamurabiState) (order: TurnOrder) = validateOrder state order

    /// <summary>
    /// Rates ruler performance over 10-year term.
    /// </summary>
    let rateReign (avgStarved: uint32) (acresPerPerson: uint32) : Rating =
        if avgStarved <= 3u && acresPerPerson >= 10u then
            Charlemagne
        elif avgStarved <= 10u && acresPerPerson >= 9u then
            Acre
        elif avgStarved <= 33u && acresPerPerson >= 7u then
            Nero
        else
            Disaster

    let rate_reign (avgStarved: uint32) (acresPerPerson: uint32) = rateReign avgStarved acresPerPerson
    let RateReign (avgStarved: uint32) (acresPerPerson: uint32) = rateReign avgStarved acresPerPerson

    /// <summary>
    /// Advances the simulation by exactly one year according to player orders.
    /// Executes the canonical two-phase state transition: validation, followed by the strictly locked PRNG draw sequence.
    /// </summary>
    let stepYear (state: HamurabiState) (order: TurnOrder) : Result<HamurabiState, HamurabiError> =
        match validateOrder state order with
        | Error err -> Error err
        | Ok () ->
            let mutable pcgState = state.RngState
            let pcgInc = state.RngInc

            let acresAfterTrade =
                if order.AcresDelta >= 0 then
                    state.Acres + uint32 order.AcresDelta
                else
                    let sellCount = uint32 (-int64 order.AcresDelta)
                    state.Acres - sellCount

            let bushelsAfterTrade =
                if order.AcresDelta > 0 then
                    let cost = uint64 order.AcresDelta * uint64 state.LandPrice
                    uint32 (uint64 state.Bushels - cost)
                elif order.AcresDelta < 0 then
                    let sellCount = uint64 (-int64 order.AcresDelta)
                    let revenue = sellCount * uint64 state.LandPrice
                    uint32 (uint64 state.Bushels + revenue)
                else
                    state.Bushels

            let bushelsAfterFeed = bushelsAfterTrade - order.BushelsToFeed
            let seedNeeded = (uint64 order.AcresToPlant + 1UL) / 2UL
            let bushelsAfterSeed = bushelsAfterFeed - uint32 seedNeeded

            // Step 1: Harvest Yield Draw
            let s1, harvestPerAcre = pcg32NextIntInRange pcgState pcgInc 1u 5u
            pcgState <- s1
            let totalHarvest = uint64 order.AcresToPlant * uint64 harvestPerAcre
            let bushelsStored = uint64 bushelsAfterSeed + totalHarvest

            // Step 2: Rat Infestation Draws
            let s2, ratRoll = pcg32NextIntInRange pcgState pcgInc 1u 100u
            pcgState <- s2
            let ratsEaten =
                if ratRoll <= 40u then
                    let s3, ratFactor = pcg32NextIntInRange pcgState pcgInc 1u 5u
                    pcgState <- s3
                    let eaten = bushelsStored / uint64 ratFactor
                    uint32 (Math.Min(eaten, uint64 UInt32.MaxValue))
                else
                    0u
            let remaining = if bushelsStored >= uint64 ratsEaten then bushelsStored - uint64 ratsEaten else 0UL
            let bushelsFinal = uint32 (Math.Min(remaining, uint64 UInt32.MaxValue))

            // Step 3: Plague Draw
            let s4, plagueRoll = pcg32NextIntInRange pcgState pcgInc 1u 100u
            pcgState <- s4
            let plagueStruck = plagueRoll <= 15u

            // Step 4: Starvation & Impeachment Evaluation
            let peopleFed = order.BushelsToFeed / 20u
            let starved, initialSurvivors =
                if peopleFed >= state.Population then
                    0u, state.Population
                else
                    state.Population - peopleFed, peopleFed

            let percentStarved =
                if state.Population > 0u then
                    uint32 (Math.Min((uint64 starved * 100UL) / uint64 state.Population, 100UL))
                else
                    0u

            let newStarvedTotal = uint32 (Math.Min(uint64 state.StarvedTotal + uint64 starved, uint64 UInt32.MaxValue))
            let newPercentSum = uint32 (Math.Min(uint64 state.PercentStarvedSum + uint64 percentStarved, uint64 UInt32.MaxValue))

            let newStatus =
                if uint64 starved * 100UL > uint64 state.Population * 45UL then
                    Impeached (starved, percentStarved)
                elif initialSurvivors = 0u then
                    GameOverExtinction
                else
                    Playing

            // Step 5: Plague Resolution
            let survivors =
                if plagueStruck && newStatus = Playing then
                    initialSurvivors / 2u
                else
                    initialSurvivors

            // Step 6: Immigration Draw
            let s5, immFactor = pcg32NextIntInRange pcgState pcgInc 1u 5u
            pcgState <- s5
            let immigrants =
                if starved = 0u && newStatus = Playing then
                    let acresTerm = uint64 acresAfterTrade * 20UL
                    let inner = acresTerm + uint64 bushelsFinal
                    let numerator = uint64 immFactor * inner
                    let denominator = 100UL * uint64 state.Population
                    if denominator > 0UL then
                        uint32 (Math.Min((numerator / denominator) + 1UL, uint64 UInt32.MaxValue))
                    else
                        0u
                else
                    0u

            let finalPopulation = uint32 (Math.Min(uint64 survivors + uint64 immigrants, uint64 UInt32.MaxValue))

            // Step 7: Next Year Land Price Draw
            let s6, nextLandPrice = pcg32NextIntInRange pcgState pcgInc 17u 26u
            pcgState <- s6

            // Step 8: Turn Advance & Rating
            let nextYear = byte (Math.Min(int state.Year + 1, 255))
            let finalStatus =
                if nextYear = 11uy && newStatus = Playing then
                    let avgStarved = newPercentSum / 10u
                    let acresPerPerson = if finalPopulation > 0u then acresAfterTrade / finalPopulation else 0u
                    let rating = rateReign avgStarved acresPerPerson
                    Won rating
                else
                    newStatus

            Ok {
                Year = nextYear
                Population = finalPopulation
                Bushels = bushelsFinal
                Acres = acresAfterTrade
                LandPrice = nextLandPrice
                HarvestPerAcre = harvestPerAcre
                RatsEaten = ratsEaten
                StarvedLastYear = starved
                StarvedTotal = newStarvedTotal
                PercentStarvedSum = newPercentSum
                ImmigrantsLastYear = immigrants
                PlagueStruck = plagueStruck
                Status = finalStatus
                RngState = pcgState
                RngInc = pcgInc
            }

    let step_year (state: HamurabiState) (order: TurnOrder) = stepYear state order
    let StepYear (state: HamurabiState) (order: TurnOrder) = stepYear state order

    /// <summary>
    /// Formats the annual teletype report or final evaluation in authentic 1970s teletype styling.
    /// </summary>
    let formatReport (state: HamurabiState) : string =
        match state.Status with
        | Impeached (starved, _) ->
            $"YOU STARVED {starved} PEOPLE IN ONE YEAR!!!\n" +
            "DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n"
        | GameOverExtinction ->
            "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
        | Won rating ->
            let avgStarved = state.PercentStarvedSum / 10u
            let acresPerPerson = if state.Population > 0u then state.Acres / state.Population else 0u
            let ratingMsg =
                match rating with
                | Charlemagne -> "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!"
                | Acre -> "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE."
                | Nero -> "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU."
                | Disaster -> "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!"
            $"IN YOUR 10-YEAR TERM OF OFFICE, {avgStarved} PERCENT OF THE\n" +
            "POPULATION STARVED PER YEAR ON AVERAGE.\n" +
            $"A TOTAL OF {state.StarvedTotal} PEOPLE DIED!!\n" +
            "YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n" +
            $"{acresPerPerson} ACRES PER PERSON.\n\n" +
            $"{ratingMsg}\n"
        | Playing ->
            let sb = StringBuilder()
            sb.Append("HAMURABI: I BEG TO REPORT TO YOU,\n") |> ignore
            sb.Append($"IN YEAR {state.Year}, {state.StarvedLastYear} PEOPLE STARVED, {state.ImmigrantsLastYear} CAME TO THE CITY.\n") |> ignore
            if state.PlagueStruck then
                sb.Append("A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n") |> ignore
            sb.Append($"POPULATION IS NOW {state.Population}.\n") |> ignore
            sb.Append($"THE CITY NOW OWNS {state.Acres} ACRES.\n") |> ignore
            sb.Append($"YOU HARVESTED {state.HarvestPerAcre} BUSHELS PER ACRE.\n") |> ignore
            sb.Append($"RATS ATE {state.RatsEaten} BUSHELS.\n") |> ignore
            sb.Append($"YOU NOW HAVE {state.Bushels} BUSHELS IN STORE.\n") |> ignore
            sb.Append($"LAND IS TRADING AT {state.LandPrice} BUSHELS PER ACRE.\n") |> ignore
            sb.ToString()

    let format_report (state: HamurabiState) = formatReport state
    let FormatReport (state: HamurabiState) = formatReport state
    let reportYear (state: HamurabiState) = formatReport state
    let report_year (state: HamurabiState) = formatReport state
    let ReportYear (state: HamurabiState) = formatReport state
