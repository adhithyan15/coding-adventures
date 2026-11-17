namespace CodingAdventures.Hamurabi;

using System;
using System.Numerics;
using System.Text;

/// <summary>
/// Rating classification awarded to ruler upon completing a 10-year term.
/// </summary>
public enum Rating
{
    /// <summary>Average starvation &lt;= 3% AND acres per person &gt;= 10.</summary>
    Charlemagne,

    /// <summary>Average starvation &lt;= 10% AND acres per person &gt;= 9.</summary>
    Acre,

    /// <summary>Average starvation &lt;= 33% AND acres per person &gt;= 7.</summary>
    Nero,

    /// <summary>Average starvation &gt; 33% OR acres per person &lt; 7.</summary>
    Disaster
}

/// <summary>
/// Simulation lifecycle status.
/// </summary>
public abstract record GameStatus
{
    private GameStatus() { }

    /// <summary>Active, playable year (years 1..=10).</summary>
    public sealed record Playing : GameStatus;

    /// <summary>Completed 10-year term successfully (transitioned to terminal year 11).</summary>
    public sealed record Won(Rating Rating) : GameStatus;

    /// <summary>Impeached and expelled from office due to extreme starvation (&gt; 45%).</summary>
    public sealed record Impeached(uint StarvedThisTurn, uint Percent) : GameStatus;

    /// <summary>Entire population died of starvation or disaster.</summary>
    public sealed record GameOverExtinction : GameStatus;

    /// <inheritdoc/>
    public override string ToString() => this switch
    {
        Playing => "Playing",
        Won w => $"Won({w.Rating})",
        Impeached imp => $"Impeached(starved={imp.StarvedThisTurn}, percent={imp.Percent})",
        GameOverExtinction => "GameOverExtinction",
        _ => base.ToString()!
    };
}

/// <summary>
/// Closed enumeration of order rejection errors.
/// </summary>
public enum HamurabiError
{
    /// <summary>Attempted to execute an order when status is not Playing.</summary>
    GameAlreadyEnded,

    /// <summary>Arithmetic overflow or invalid bounds on input parameters.</summary>
    InvalidOrderBounds,

    /// <summary>Player ordered to buy land but lacks sufficient bushels.</summary>
    InsufficientBushelsForLand,

    /// <summary>Player ordered to sell more land than the city owns.</summary>
    InsufficientAcresToSell,

    /// <summary>Player ordered to feed more bushels than available after land trade.</summary>
    InsufficientBushelsToFeed,

    /// <summary>Player ordered to plant more acres than owned after land trade.</summary>
    InsufficientAcresToPlant,

    /// <summary>Player ordered to plant more acres than population can tend (1 person tends 10 acres).</summary>
    InsufficientPopulationToPlant,

    /// <summary>Player lacks grain to seed fields (1 bushel seeds 2 acres).</summary>
    InsufficientGrainForSeed
}

/// <summary>
/// Unit record for void results.
/// </summary>
public readonly record struct HamurabiUnit;

/// <summary>
/// Pure functional result type without exception overhead.
/// </summary>
/// <typeparam name="T">Success value type.</typeparam>
/// <typeparam name="E">Error value type.</typeparam>
public readonly record struct Result<T, E>
{
    public bool IsOk { get; }
    public T Value { get; }
    public E Error { get; }

    private Result(bool isOk, T value, E error)
    {
        IsOk = isOk;
        Value = value;
        Error = error;
    }

    public static Result<T, E> Ok(T value) => new(true, value, default!);
    public static Result<T, E> Err(E error) => new(false, default!, error);
}

/// <summary>
/// Player orders for a single year.
/// </summary>
/// <param name="AcresDelta">Acres of land to buy (positive) or sell (negative). 0 indicates no trade.</param>
/// <param name="BushelsToFeed">Bushels of grain allocated to feed the population (20 bushels per person).</param>
/// <param name="AcresToPlant">Acres of owned land to sow with grain (1 bushel seeds 2 acres).</param>
public readonly record struct TurnOrder(int AcresDelta, uint BushelsToFeed, uint AcresToPlant);

/// <summary>
/// Immutable value record capturing the complete simulation state.
/// </summary>
public sealed record HamurabiState(
    byte Year,
    uint Population,
    uint Bushels,
    uint Acres,
    uint LandPrice,
    uint HarvestPerAcre,
    uint RatsEaten,
    uint StarvedLastYear,
    uint StarvedTotal,
    uint PercentStarvedSum,
    uint ImmigrantsLastYear,
    bool PlagueStruck,
    GameStatus Status,
    ulong RngState,
    ulong RngInc);

/// <summary>
/// Pure deterministic simulation engine for Hamurabi (GAME01).
/// </summary>
public static class Hamurabi
{
    /// <summary>LCG multiplier constant for 64-bit PCG32 (Knuth / Numerical Recipes).</summary>
    public const ulong PcgMultiplier = 6364136223846793005UL;

    /// <summary>Stream ID for PCG32 PRNG stream in Hamurabi simulation.</summary>
    public const ulong HamurabiStreamId = 54UL;

    /// <summary>Default odd increment for PCG32.</summary>
    public const ulong DefaultRngInc = (HamurabiStreamId << 1) | 1UL;

    /// <summary>Advances the PCG32 state by one step and returns a 32-bit random output.</summary>
    public static uint Pcg32Step(ref ulong state, ulong inc)
    {
        ulong oldState = state;
        unchecked
        {
            state = (oldState * PcgMultiplier) + inc;
        }
        uint xorshifted = (uint)(((oldState >> 18) ^ oldState) >> 27);
        int rot = (int)(oldState >> 59);
        return BitOperations.RotateRight(xorshifted, rot);
    }

    /// <summary>Draws a uniform 32-bit integer in the inclusive range [min, max] via rejection sampling.</summary>
    public static uint Pcg32NextIntInRange(ref ulong state, ulong inc, uint min, uint max)
    {
        ulong span = (ulong)(max - min) + 1UL;
        ulong threshold = ((1UL << 32) - span) % span;
        while (true)
        {
            ulong r = (ulong)Pcg32Step(ref state, inc);
            if (r >= threshold)
            {
                return min + (uint)(r % span);
            }
        }
    }

    /// <summary>
    /// Initializes a new, deterministic 10-year reign for ruler Hamurabi.
    /// Seeds PCG32 with (seed, stream_id = 54) and draws initial year 1 land price (range 17..=26).
    /// </summary>
    public static HamurabiState NewGame(ulong seed)
    {
        ulong inc = DefaultRngInc;
        ulong rngState = 0UL;
        unchecked
        {
            rngState = (rngState * PcgMultiplier) + inc;
            rngState += seed;
            rngState = (rngState * PcgMultiplier) + inc;
        }

        uint landPrice = Pcg32NextIntInRange(ref rngState, inc, 17, 26);

        return new HamurabiState(
            Year: 1,
            Population: 100,
            Bushels: 2800,
            Acres: 1000,
            LandPrice: landPrice,
            HarvestPerAcre: 3,
            RatsEaten: 200,
            StarvedLastYear: 0,
            StarvedTotal: 0,
            PercentStarvedSum: 0,
            ImmigrantsLastYear: 5,
            PlagueStruck: false,
            Status: new GameStatus.Playing(),
            RngState: rngState,
            RngInc: inc
        );
    }

    /// <summary>Alias for <see cref="NewGame"/>.</summary>
    public static HamurabiState new_game(ulong seed) => NewGame(seed);

    /// <summary>
    /// Validates player orders against current state constraints without advancing state.
    /// Evaluates checks in the canonical 7-step order specified by GAME01.
    /// </summary>
    public static Result<HamurabiUnit, HamurabiError> ValidateOrder(HamurabiState state, TurnOrder order)
    {
        // 1. Status Check
        if (state.Status is not GameStatus.Playing)
        {
            return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.GameAlreadyEnded);
        }

        // State invariant checks (VER04m state_inv & CWE-400 / CWE-835)
        if ((state.RngInc & 1UL) != 1UL)
        {
            return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
        }
        if (state.Year < 1 || state.Year > 10)
        {
            return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
        }
        if (state.LandPrice < 17 || state.LandPrice > 26)
        {
            return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
        }
        if (state.HarvestPerAcre < 1 || state.HarvestPerAcre > 5)
        {
            return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
        }

        // 2. Bounds & Overflow Guard on Land Trade
        if (order.AcresDelta > 0)
        {
            ulong buyCount = (ulong)order.AcresDelta;
            ulong cost;
            try
            {
                cost = checked(buyCount * (ulong)state.LandPrice);
            }
            catch (OverflowException)
            {
                return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
            }
            if (cost > (ulong)state.Bushels)
            {
                return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InsufficientBushelsForLand);
            }
        }
        else if (order.AcresDelta < 0)
        {
            long sellCountLong = -((long)order.AcresDelta);
            if (sellCountLong < 0)
            {
                return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
            }
            ulong sellCount = (ulong)sellCountLong;
            if (sellCount > (ulong)state.Acres)
            {
                return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InsufficientAcresToSell);
            }
        }

        // 3. Land Trade Balances
        uint acresAfterTrade;
        if (order.AcresDelta >= 0)
        {
            ulong newAcres = (ulong)state.Acres + (ulong)order.AcresDelta;
            if (newAcres > (ulong)uint.MaxValue)
            {
                return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
            }
            acresAfterTrade = (uint)newAcres;
        }
        else
        {
            ulong sellCount = (ulong)(-(long)order.AcresDelta);
            acresAfterTrade = (uint)(state.Acres - sellCount);
        }

        uint bushelsAfterTrade;
        if (order.AcresDelta > 0)
        {
            ulong cost = (ulong)order.AcresDelta * (ulong)state.LandPrice;
            bushelsAfterTrade = (uint)((ulong)state.Bushels - cost);
        }
        else if (order.AcresDelta < 0)
        {
            ulong sellCount = (ulong)(-(long)order.AcresDelta);
            ulong revenue;
            try
            {
                revenue = checked(sellCount * (ulong)state.LandPrice);
            }
            catch (OverflowException)
            {
                return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
            }
            ulong newBushels;
            try
            {
                newBushels = checked((ulong)state.Bushels + revenue);
            }
            catch (OverflowException)
            {
                return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
            }
            if (newBushels > (ulong)uint.MaxValue)
            {
                return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
            }
            bushelsAfterTrade = (uint)newBushels;
        }
        else
        {
            bushelsAfterTrade = state.Bushels;
        }

        // 4. Food Allocation Check
        if (order.BushelsToFeed > bushelsAfterTrade)
        {
            return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InsufficientBushelsToFeed);
        }
        uint bushelsAfterFeed = bushelsAfterTrade - order.BushelsToFeed;

        // 5. Planting Land & Labor Checks
        if (order.AcresToPlant > acresAfterTrade)
        {
            return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InsufficientAcresToPlant);
        }
        ulong maxLaborAcres;
        try
        {
            maxLaborAcres = checked((ulong)state.Population * 10UL);
        }
        catch (OverflowException)
        {
            return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
        }
        if ((ulong)order.AcresToPlant > maxLaborAcres)
        {
            return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InsufficientPopulationToPlant);
        }

        // 6. Planting Seed Check: each bushel seeds 2 acres (ceil(acres / 2))
        ulong seedNeeded = ((ulong)order.AcresToPlant + 1UL) / 2UL;
        if (seedNeeded > (ulong)bushelsAfterFeed)
        {
            return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InsufficientGrainForSeed);
        }
        uint bushelsAfterSeed = bushelsAfterFeed - (uint)seedNeeded;

        // 7. Maximum Harvest Overflow Guard (CWE-190)
        ulong maxHarvest;
        try
        {
            maxHarvest = checked((ulong)order.AcresToPlant * 5UL);
        }
        catch (OverflowException)
        {
            return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
        }
        ulong maxStored;
        try
        {
            maxStored = checked((ulong)bushelsAfterSeed + maxHarvest);
        }
        catch (OverflowException)
        {
            return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
        }
        if (maxStored > (ulong)uint.MaxValue)
        {
            return Result<HamurabiUnit, HamurabiError>.Err(HamurabiError.InvalidOrderBounds);
        }

        return Result<HamurabiUnit, HamurabiError>.Ok(default);
    }

    /// <summary>Alias for <see cref="ValidateOrder"/>.</summary>
    public static Result<HamurabiUnit, HamurabiError> validate_order(HamurabiState state, TurnOrder order) =>
        ValidateOrder(state, order);

    /// <summary>
    /// Advances the simulation by exactly one year according to player orders.
    /// Executes the canonical two-phase state transition: validation, followed by the strictly locked PRNG draw sequence.
    /// </summary>
    public static Result<HamurabiState, HamurabiError> StepYear(HamurabiState state, TurnOrder order)
    {
        var validation = ValidateOrder(state, order);
        if (!validation.IsOk)
        {
            return Result<HamurabiState, HamurabiError>.Err(validation.Error);
        }

        ulong pcgState = state.RngState;
        ulong pcgInc = state.RngInc;

        uint acresAfterTrade;
        if (order.AcresDelta >= 0)
        {
            acresAfterTrade = state.Acres + (uint)order.AcresDelta;
        }
        else
        {
            uint sellCount = (uint)(-(long)order.AcresDelta);
            acresAfterTrade = state.Acres - sellCount;
        }

        uint bushelsAfterTrade;
        if (order.AcresDelta > 0)
        {
            ulong cost = (ulong)order.AcresDelta * (ulong)state.LandPrice;
            bushelsAfterTrade = (uint)((ulong)state.Bushels - cost);
        }
        else if (order.AcresDelta < 0)
        {
            ulong sellCount = (ulong)(-(long)order.AcresDelta);
            ulong revenue = sellCount * (ulong)state.LandPrice;
            bushelsAfterTrade = (uint)((ulong)state.Bushels + revenue);
        }
        else
        {
            bushelsAfterTrade = state.Bushels;
        }

        uint bushelsAfterFeed = bushelsAfterTrade - order.BushelsToFeed;
        ulong seedNeeded = ((ulong)order.AcresToPlant + 1UL) / 2UL;
        uint bushelsAfterSeed = bushelsAfterFeed - (uint)seedNeeded;

        // Step 1: Harvest Yield Draw
        uint harvestPerAcre = Pcg32NextIntInRange(ref pcgState, pcgInc, 1, 5);
        ulong totalHarvest = (ulong)order.AcresToPlant * (ulong)harvestPerAcre;
        ulong bushelsStored = (ulong)bushelsAfterSeed + totalHarvest;

        // Step 2: Rat Infestation Draws
        uint ratRoll = Pcg32NextIntInRange(ref pcgState, pcgInc, 1, 100);
        uint ratsEaten = 0;
        if (ratRoll <= 40)
        {
            uint ratFactor = Pcg32NextIntInRange(ref pcgState, pcgInc, 1, 5);
            ulong eaten = bushelsStored / (ulong)ratFactor;
            ratsEaten = (uint)Math.Min(eaten, (ulong)uint.MaxValue);
        }
        ulong remaining = bushelsStored >= ratsEaten ? bushelsStored - ratsEaten : 0UL;
        uint bushelsFinal = (uint)Math.Min(remaining, (ulong)uint.MaxValue);

        // Step 3: Plague Draw
        uint plagueRoll = Pcg32NextIntInRange(ref pcgState, pcgInc, 1, 100);
        bool plagueStruck = plagueRoll <= 15;

        // Step 4: Starvation & Impeachment Evaluation
        uint peopleFed = order.BushelsToFeed / 20;
        uint starved;
        uint survivors;
        if (peopleFed >= state.Population)
        {
            starved = 0;
            survivors = state.Population;
        }
        else
        {
            starved = state.Population - peopleFed;
            survivors = peopleFed;
        }

        uint percentStarved = state.Population > 0
            ? (uint)Math.Min(((ulong)starved * 100UL) / (ulong)state.Population, 100UL)
            : 0;

        uint newStarvedTotal = (uint)Math.Min((ulong)state.StarvedTotal + (ulong)starved, (ulong)uint.MaxValue);
        uint newPercentSum = (uint)Math.Min((ulong)state.PercentStarvedSum + (ulong)percentStarved, (ulong)uint.MaxValue);

        GameStatus newStatus;
        if ((ulong)starved * 100UL > (ulong)state.Population * 45UL)
        {
            newStatus = new GameStatus.Impeached(starved, percentStarved);
        }
        else if (survivors == 0)
        {
            newStatus = new GameStatus.GameOverExtinction();
        }
        else
        {
            newStatus = new GameStatus.Playing();
        }

        // Step 5: Plague Resolution
        if (plagueStruck && newStatus is GameStatus.Playing)
        {
            survivors /= 2;
        }

        // Step 6: Immigration Draw
        uint immFactor = Pcg32NextIntInRange(ref pcgState, pcgInc, 1, 5);
        uint immigrants = 0;
        if (starved == 0 && newStatus is GameStatus.Playing)
        {
            ulong acresTerm = (ulong)acresAfterTrade * 20UL;
            ulong inner = acresTerm + (ulong)bushelsFinal;
            ulong numerator = (ulong)immFactor * inner;
            ulong denominator = 100UL * (ulong)state.Population;
            if (denominator > 0)
            {
                immigrants = (uint)Math.Min((numerator / denominator) + 1UL, (ulong)uint.MaxValue);
            }
        }
        uint finalPopulation = (uint)Math.Min((ulong)survivors + (ulong)immigrants, (ulong)uint.MaxValue);

        // Step 7: Next Year Land Price Draw
        uint nextLandPrice = Pcg32NextIntInRange(ref pcgState, pcgInc, 17, 26);

        // Step 8: Turn Advance & Rating
        byte nextYear = (byte)Math.Min((int)state.Year + 1, 255);
        if (nextYear == 11 && newStatus is GameStatus.Playing)
        {
            uint avgStarved = newPercentSum / 10;
            uint acresPerPerson = finalPopulation > 0 ? acresAfterTrade / finalPopulation : 0;
            Rating rating = RateReign(avgStarved, acresPerPerson);
            newStatus = new GameStatus.Won(rating);
        }

        return Result<HamurabiState, HamurabiError>.Ok(new HamurabiState(
            Year: nextYear,
            Population: finalPopulation,
            Bushels: bushelsFinal,
            Acres: acresAfterTrade,
            LandPrice: nextLandPrice,
            HarvestPerAcre: harvestPerAcre,
            RatsEaten: ratsEaten,
            StarvedLastYear: starved,
            StarvedTotal: newStarvedTotal,
            PercentStarvedSum: newPercentSum,
            ImmigrantsLastYear: immigrants,
            PlagueStruck: plagueStruck,
            Status: newStatus,
            RngState: pcgState,
            RngInc: pcgInc
        ));
    }

    /// <summary>Alias for <see cref="StepYear"/>.</summary>
    public static Result<HamurabiState, HamurabiError> step_year(HamurabiState state, TurnOrder order) =>
        StepYear(state, order);

    /// <summary>
    /// Rates ruler performance over 10-year term.
    /// </summary>
    public static Rating RateReign(uint avgStarved, uint acresPerPerson)
    {
        if (avgStarved <= 3 && acresPerPerson >= 10)
        {
            return Rating.Charlemagne;
        }
        if (avgStarved <= 10 && acresPerPerson >= 9)
        {
            return Rating.Acre;
        }
        if (avgStarved <= 33 && acresPerPerson >= 7)
        {
            return Rating.Nero;
        }
        return Rating.Disaster;
    }

    /// <summary>Alias for <see cref="RateReign"/>.</summary>
    public static Rating rate_reign(uint avgStarved, uint acresPerPerson) =>
        RateReign(avgStarved, acresPerPerson);

    /// <summary>
    /// Formats the annual teletype report or final evaluation in authentic 1970s teletype styling.
    /// </summary>
    public static string FormatReport(HamurabiState state)
    {
        switch (state.Status)
        {
            case GameStatus.Impeached impeached:
                return $"YOU STARVED {impeached.StarvedThisTurn} PEOPLE IN ONE YEAR!!!\n" +
                       "DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n";

            case GameStatus.GameOverExtinction:
                return "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n";

            case GameStatus.Won won:
            {
                uint avgStarved = state.PercentStarvedSum / 10;
                uint acresPerPerson = state.Population > 0 ? state.Acres / state.Population : 0;
                string ratingMsg = won.Rating switch
                {
                    Rating.Charlemagne => "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!",
                    Rating.Acre => "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE.",
                    Rating.Nero => "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU.",
                    _ => "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!"
                };
                return $"IN YOUR 10-YEAR TERM OF OFFICE, {avgStarved} PERCENT OF THE\n" +
                       "POPULATION STARVED PER YEAR ON AVERAGE.\n" +
                       $"A TOTAL OF {state.StarvedTotal} PEOPLE DIED!!\n" +
                       "YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n" +
                       $"{acresPerPerson} ACRES PER PERSON.\n\n" +
                       $"{ratingMsg}\n";
            }

            case GameStatus.Playing:
            default:
            {
                var sb = new StringBuilder();
                sb.Append("HAMURABI: I BEG TO REPORT TO YOU,\n");
                sb.Append($"IN YEAR {state.Year}, {state.StarvedLastYear} PEOPLE STARVED, {state.ImmigrantsLastYear} CAME TO THE CITY.\n");
                if (state.PlagueStruck)
                {
                    sb.Append("A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n");
                }
                sb.Append($"POPULATION IS NOW {state.Population}.\n");
                sb.Append($"THE CITY NOW OWNS {state.Acres} ACRES.\n");
                sb.Append($"YOU HARVESTED {state.HarvestPerAcre} BUSHELS PER ACRE.\n");
                sb.Append($"RATS ATE {state.RatsEaten} BUSHELS.\n");
                sb.Append($"YOU NOW HAVE {state.Bushels} BUSHELS IN STORE.\n");
                sb.Append($"LAND IS TRADING AT {state.LandPrice} BUSHELS PER ACRE.\n");
                return sb.ToString();
            }
        }
    }

    /// <summary>Alias for <see cref="FormatReport"/>.</summary>
    public static string format_report(HamurabiState state) => FormatReport(state);

    /// <summary>Alias for <see cref="FormatReport"/>.</summary>
    public static string ReportYear(HamurabiState state) => FormatReport(state);

    /// <summary>Alias for <see cref="FormatReport"/>.</summary>
    public static string report_year(HamurabiState state) => FormatReport(state);
}

/// <summary>Alias class for callers referring to <see cref="HamurabiGame"/>.</summary>
public static class HamurabiGame
{
    public static HamurabiState NewGame(ulong seed) => Hamurabi.NewGame(seed);
    public static Result<HamurabiUnit, HamurabiError> ValidateOrder(HamurabiState state, TurnOrder order) => Hamurabi.ValidateOrder(state, order);
    public static Result<HamurabiState, HamurabiError> StepYear(HamurabiState state, TurnOrder order) => Hamurabi.StepYear(state, order);
    public static Rating RateReign(uint avgStarved, uint acresPerPerson) => Hamurabi.RateReign(avgStarved, acresPerPerson);
    public static string FormatReport(HamurabiState state) => Hamurabi.FormatReport(state);
}
