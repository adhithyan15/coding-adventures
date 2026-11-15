namespace CodingAdventures.Hamurabi.Tests

open System
open System.IO
open System.Text.Json
open Xunit
open CodingAdventures.Hamurabi

module ConformanceTests =

    let private getCasesJsonPath () =
        let mutable dir = DirectoryInfo(AppContext.BaseDirectory)
        while not (isNull dir) && not (Directory.Exists(Path.Combine(dir.FullName, "code", "specs"))) do
            dir <- dir.Parent
        Assert.NotNull(dir)
        let path = Path.Combine(dir.FullName, "code", "specs", "fixtures", "hamurabi-v1", "cases.json")
        Assert.True(File.Exists(path), $"cases.json not found at {path}")
        path

    let private parseRating (s: string) : Rating =
        match s with
        | "Charlemagne" -> Charlemagne
        | "Acre" -> Acre
        | "Nero" -> Nero
        | "Disaster" -> Disaster
        | _ -> failwithf "Unknown rating: %s" s

    let private parseStatus (s: string) : GameStatus =
        if s = "Playing" then
            Playing
        elif s = "GameOverExtinction" then
            GameOverExtinction
        elif s.StartsWith("Won(") && s.EndsWith(")") then
            let rStr = s.Substring(4, s.Length - 5)
            let rating = parseRating rStr
            Won rating
        elif s.StartsWith("Impeached(") && s.EndsWith(")") then
            let inner = s.Substring(10, s.Length - 11)
            let parts = inner.Split(',')
            let starved = uint32 (parts.[0].Split('=').[1].Trim())
            let percent = uint32 (parts.[1].Split('=').[1].Trim())
            Impeached (starved, percent)
        else
            failwithf "Unknown status: %s" s

    let private parseState (el: JsonElement) : HamurabiState =
        {
            Year = el.GetProperty("year").GetByte()
            Population = el.GetProperty("population").GetUInt32()
            Bushels = el.GetProperty("bushels").GetUInt32()
            Acres = el.GetProperty("acres").GetUInt32()
            LandPrice = el.GetProperty("land_price").GetUInt32()
            HarvestPerAcre = el.GetProperty("harvest_per_acre").GetUInt32()
            RatsEaten = el.GetProperty("rats_eaten").GetUInt32()
            StarvedLastYear = el.GetProperty("starved_last_year").GetUInt32()
            StarvedTotal = el.GetProperty("starved_total").GetUInt32()
            PercentStarvedSum = el.GetProperty("percent_starved_sum").GetUInt32()
            ImmigrantsLastYear = el.GetProperty("immigrants_last_year").GetUInt32()
            PlagueStruck = el.GetProperty("plague_struck").GetBoolean()
            Status = parseStatus (el.GetProperty("status").GetString())
            RngState = el.GetProperty("rng_state").GetUInt64()
            RngInc = el.GetProperty("rng_inc").GetUInt64()
        }

    let private parseOrder (el: JsonElement) : TurnOrder =
        {
            AcresDelta = el.GetProperty("acres_delta").GetInt32()
            BushelsToFeed = el.GetProperty("bushels_to_feed").GetUInt32()
            AcresToPlant = el.GetProperty("acres_to_plant").GetUInt32()
        }

    let private parseError (s: string) : HamurabiError =
        match s with
        | "GameAlreadyEnded" -> GameAlreadyEnded
        | "InvalidOrderBounds" -> InvalidOrderBounds
        | "InsufficientBushelsForLand" -> InsufficientBushelsForLand
        | "InsufficientAcresToSell" -> InsufficientAcresToSell
        | "InsufficientBushelsToFeed" -> InsufficientBushelsToFeed
        | "InsufficientAcresToPlant" -> InsufficientAcresToPlant
        | "InsufficientPopulationToPlant" -> InsufficientPopulationToPlant
        | "InsufficientGrainForSeed" -> InsufficientGrainForSeed
        | _ -> failwithf "Unknown error: %s" s

    let private assertStatesEqual (expected: HamurabiState) (actual: HamurabiState) (context: string) =
        Assert.True(expected.Year = actual.Year, $"{context}: Year expected {expected.Year}, got {actual.Year}")
        Assert.True(expected.Population = actual.Population, $"{context}: Population expected {expected.Population}, got {actual.Population}")
        Assert.True(expected.Bushels = actual.Bushels, $"{context}: Bushels expected {expected.Bushels}, got {actual.Bushels}")
        Assert.True(expected.Acres = actual.Acres, $"{context}: Acres expected {expected.Acres}, got {actual.Acres}")
        Assert.True(expected.LandPrice = actual.LandPrice, $"{context}: LandPrice expected {expected.LandPrice}, got {actual.LandPrice}")
        Assert.True(expected.HarvestPerAcre = actual.HarvestPerAcre, $"{context}: HarvestPerAcre expected {expected.HarvestPerAcre}, got {actual.HarvestPerAcre}")
        Assert.True(expected.RatsEaten = actual.RatsEaten, $"{context}: RatsEaten expected {expected.RatsEaten}, got {actual.RatsEaten}")
        Assert.True(expected.StarvedLastYear = actual.StarvedLastYear, $"{context}: StarvedLastYear expected {expected.StarvedLastYear}, got {actual.StarvedLastYear}")
        Assert.True(expected.StarvedTotal = actual.StarvedTotal, $"{context}: StarvedTotal expected {expected.StarvedTotal}, got {actual.StarvedTotal}")
        Assert.True(expected.PercentStarvedSum = actual.PercentStarvedSum, $"{context}: PercentStarvedSum expected {expected.PercentStarvedSum}, got {actual.PercentStarvedSum}")
        Assert.True(expected.ImmigrantsLastYear = actual.ImmigrantsLastYear, $"{context}: ImmigrantsLastYear expected {expected.ImmigrantsLastYear}, got {actual.ImmigrantsLastYear}")
        Assert.True(expected.PlagueStruck = actual.PlagueStruck, $"{context}: PlagueStruck expected {expected.PlagueStruck}, got {actual.PlagueStruck}")
        Assert.True(expected.Status.ToString() = actual.Status.ToString(), $"{context}: Status expected {expected.Status}, got {actual.Status}")
        Assert.True(expected.RngState = actual.RngState, $"{context}: RngState expected {expected.RngState}, got {actual.RngState}")
        Assert.True(expected.RngInc = actual.RngInc, $"{context}: RngInc expected {expected.RngInc}, got {actual.RngInc}")

    [<Fact>]
    let ``All 31 cases in conformance corpus pass`` () =
        let jsonPath = getCasesJsonPath ()
        let jsonText = File.ReadAllText jsonPath
        use doc = JsonDocument.Parse jsonText

        let cases = doc.RootElement.GetProperty("cases").EnumerateArray()
        let mutable caseCount = 0

        for c in cases do
            caseCount <- caseCount + 1
            let id = c.GetProperty("id").GetString()
            let initialState = parseState (c.GetProperty("initial_state"))

            let mutable turnsEl = Unchecked.defaultof<JsonElement>
            let hasTurns = c.TryGetProperty("turns", &turnsEl)

            let mutable seedEl = Unchecked.defaultof<JsonElement>
            if hasTurns &&
               c.TryGetProperty("seed", &seedEl) &&
               seedEl.ValueKind = JsonValueKind.Number &&
               initialState.Year = 1uy &&
               initialState.Population = 100u &&
               initialState.Bushels = 2800u &&
               initialState.Acres = 1000u &&
               initialState.Status = Playing then
                let seed = seedEl.GetUInt64()
                let newGameState = Hamurabi.newGame seed
                assertStatesEqual initialState newGameState $"newGame({seed}) vs initial_state in case '{id}'"

            if hasTurns then
                let mutable currentState = initialState
                for turnEl in turnsEl.EnumerateArray() do
                    let turnNum = turnEl.GetProperty("turn").GetInt32()
                    let order = parseOrder (turnEl.GetProperty("order"))
                    let expectedState = parseState (turnEl.GetProperty("expected_state"))
                    let expectedReport = turnEl.GetProperty("expected_report").GetString()

                    let valRes = Hamurabi.validateOrder currentState order
                    Assert.True((match valRes with Ok () -> true | _ -> false), $"Case '{id}' Turn {turnNum} validateOrder failed: %A{valRes}")

                    let stepRes = Hamurabi.stepYear currentState order
                    match stepRes with
                    | Ok nextState ->
                        assertStatesEqual expectedState nextState $"Case '{id}' Turn {turnNum}"
                        let report = Hamurabi.formatReport nextState
                        Assert.Equal(expectedReport, report)
                        currentState <- nextState
                    | Error err ->
                        Assert.Fail $"Case '{id}' Turn {turnNum} stepYear failed with error: %A{err}"
            else
                let mutable errorEl = Unchecked.defaultof<JsonElement>
                if c.TryGetProperty("expected_error", &errorEl) then
                    let order = parseOrder (c.GetProperty("order"))
                    let expectedErr = parseError (errorEl.GetString())

                    let valRes = Hamurabi.validateOrder initialState order
                    Assert.Equal(Error expectedErr, valRes)

                    let stepRes = Hamurabi.stepYear initialState order
                    Assert.Equal(Error expectedErr, stepRes)
                else
                    Assert.Fail $"Case '{id}' has neither turns nor expected_error"

        Assert.Equal(31, caseCount)
