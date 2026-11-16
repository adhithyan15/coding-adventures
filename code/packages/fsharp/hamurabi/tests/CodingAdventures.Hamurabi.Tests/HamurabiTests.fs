namespace CodingAdventures.Hamurabi.Tests

open System
open Xunit
open CodingAdventures.Hamurabi

module HamurabiTests =

    [<Fact>]
    let ``Initial state matches invariant`` () =
        let state = Hamurabi.newGame 1337UL
        Assert.Equal(1uy, state.Year)
        Assert.Equal(100u, state.Population)
        Assert.Equal(2800u, state.Bushels)
        Assert.Equal(1000u, state.Acres)
        Assert.InRange(state.LandPrice, 17u, 26u)
        Assert.Equal(3u, state.HarvestPerAcre)
        Assert.Equal(200u, state.RatsEaten)
        Assert.Equal(0u, state.StarvedLastYear)
        Assert.Equal(0u, state.StarvedTotal)
        Assert.Equal(0u, state.PercentStarvedSum)
        Assert.Equal(5u, state.ImmigrantsLastYear)
        Assert.False(state.PlagueStruck)
        Assert.Equal(Playing, state.Status)
        Assert.Equal(1u, uint32 (state.RngInc &&& 1UL))

    [<Fact>]
    let ``All eight error arms verified`` () =
        let state = Hamurabi.newGame 1337UL
        let normalOrder = { AcresDelta = 0; BushelsToFeed = 2000u; AcresToPlant = 100u }

        // Arm 1: GameAlreadyEnded
        let wonState = { state with Status = Won Charlemagne }
        Assert.Equal(Error GameAlreadyEnded, Hamurabi.validateOrder wonState normalOrder)
        Assert.Equal(Error GameAlreadyEnded, Hamurabi.stepYear wonState normalOrder)

        let extinctState = { state with Status = GameOverExtinction }
        Assert.Equal(Error GameAlreadyEnded, Hamurabi.validateOrder extinctState normalOrder)

        let impeachedState = { state with Status = Impeached (50u, 50u) }
        Assert.Equal(Error GameAlreadyEnded, Hamurabi.validateOrder impeachedState normalOrder)

        // Arm 2: InvalidOrderBounds (corrupted land price or arithmetic overflow on trade)
        let badPriceState = { state with LandPrice = 10u }
        Assert.Equal(Error InvalidOrderBounds, Hamurabi.validateOrder badPriceState normalOrder)

        let badYear0State = { state with Year = 0uy }
        Assert.Equal(Error InvalidOrderBounds, Hamurabi.validateOrder badYear0State normalOrder)

        let badYear11State = { state with Year = 11uy }
        Assert.Equal(Error InvalidOrderBounds, Hamurabi.validateOrder badYear11State normalOrder)

        let badHarvest0State = { state with HarvestPerAcre = 0u }
        Assert.Equal(Error InvalidOrderBounds, Hamurabi.validateOrder badHarvest0State normalOrder)

        let badHarvest6State = { state with HarvestPerAcre = 6u }
        Assert.Equal(Error InvalidOrderBounds, Hamurabi.validateOrder badHarvest6State normalOrder)

        let badEvenIncState = { state with RngInc = 0UL }
        Assert.Equal(Error InvalidOrderBounds, Hamurabi.validateOrder badEvenIncState normalOrder)

        let badEvenInc2State = { state with RngInc = 108UL }
        Assert.Equal(Error InvalidOrderBounds, Hamurabi.validateOrder badEvenInc2State normalOrder)

        // Revenue overflow on land sale
        let richLandState = { state with Acres = 2_000_000_000u; Bushels = 4_000_000_000u }
        let overflowSell = { AcresDelta = -1_000_000_000; BushelsToFeed = 0u; AcresToPlant = 0u }
        Assert.Equal(Error InvalidOrderBounds, Hamurabi.validateOrder richLandState overflowSell)

        // Acres overflow on land purchase
        let richAcresState = { state with Acres = UInt32.MaxValue - 10u; Bushels = UInt32.MaxValue }
        let overflowBuy = { AcresDelta = 20; BushelsToFeed = 0u; AcresToPlant = 0u }
        Assert.Equal(Error InvalidOrderBounds, Hamurabi.validateOrder richAcresState overflowBuy)

        // Arm 3: InsufficientBushelsForLand
        let buyTooMuch = { AcresDelta = 200; BushelsToFeed = 0u; AcresToPlant = 0u }
        Assert.Equal(Error InsufficientBushelsForLand, Hamurabi.validateOrder state buyTooMuch)

        // Arm 4: InsufficientAcresToSell
        let sellTooMuch = { AcresDelta = -1001; BushelsToFeed = 0u; AcresToPlant = 0u }
        Assert.Equal(Error InsufficientAcresToSell, Hamurabi.validateOrder state sellTooMuch)

        // Arm 5: InsufficientBushelsToFeed
        let feedTooMuch = { AcresDelta = 0; BushelsToFeed = 2801u; AcresToPlant = 0u }
        Assert.Equal(Error InsufficientBushelsToFeed, Hamurabi.validateOrder state feedTooMuch)

        // Arm 6: InsufficientAcresToPlant
        let plantTooMuchAcres = { AcresDelta = 0; BushelsToFeed = 2000u; AcresToPlant = 1001u }
        Assert.Equal(Error InsufficientAcresToPlant, Hamurabi.validateOrder state plantTooMuchAcres)

        // Arm 7: InsufficientPopulationToPlant
        let richState = { state with Bushels = 50000u }
        let overLabor = { AcresDelta = 50; BushelsToFeed = 2000u; AcresToPlant = 1001u }
        Assert.Equal(Error InsufficientPopulationToPlant, Hamurabi.validateOrder richState overLabor)

        // Arm 8: InsufficientGrainForSeed
        let seedShort = { AcresDelta = 0; BushelsToFeed = 2700u; AcresToPlant = 300u }
        Assert.Equal(Error InsufficientGrainForSeed, Hamurabi.validateOrder state seedShort)

    [<Fact>]
    let ``Int32 MinValue AcresDelta does not crash`` () =
        let state = Hamurabi.newGame 1337UL
        let order = { AcresDelta = Int32.MinValue; BushelsToFeed = 0u; AcresToPlant = 0u }
        let res = Hamurabi.validateOrder state order
        Assert.Equal(Error InsufficientAcresToSell, res)

    [<Fact>]
    let ``Harvest overflow refusal`` () =
        let state = {
            Hamurabi.newGame 1UL with
                Population = 500_000_000u
                Acres = 1_000_000_000u
                Bushels = 2_000_000_000u
        }
        let order = { AcresDelta = 0; BushelsToFeed = 0u; AcresToPlant = 900_000_000u }
        Assert.Equal(Error InvalidOrderBounds, Hamurabi.validateOrder state order)
        Assert.Equal(Error InvalidOrderBounds, Hamurabi.stepYear state order)

    [<Fact>]
    let ``Starved total saturating addition`` () =
        let state = {
            Hamurabi.newGame 1UL with
                StarvedTotal = UInt32.MaxValue
                PercentStarvedSum = UInt32.MaxValue - 10u
        }
        let order = { AcresDelta = 0; BushelsToFeed = 1000u; AcresToPlant = 0u }
        match Hamurabi.stepYear state order with
        | Ok next ->
            Assert.Equal(UInt32.MaxValue, next.StarvedTotal)
            Assert.Equal(UInt32.MaxValue, next.PercentStarvedSum)
        | Error err ->
            Assert.Fail $"Unexpected error: %A{err}"

    [<Fact>]
    let ``Rate reign all branches`` () =
        Assert.Equal(Charlemagne, Hamurabi.rateReign 3u 10u)
        Assert.Equal(Charlemagne, Hamurabi.rateReign 0u 15u)
        Assert.Equal(Acre, Hamurabi.rateReign 3u 9u)
        Assert.Equal(Acre, Hamurabi.rateReign 10u 9u)
        Assert.Equal(Nero, Hamurabi.rateReign 11u 8u)
        Assert.Equal(Nero, Hamurabi.rateReign 33u 7u)
        Assert.Equal(Disaster, Hamurabi.rateReign 34u 10u)
        Assert.Equal(Disaster, Hamurabi.rateReign 10u 6u)

    [<Fact>]
    let ``Format report all variants`` () =
        let state = Hamurabi.newGame 1UL

        // Playing without plague
        let rep1 = Hamurabi.formatReport state
        Assert.Contains("HAMURABI: I BEG TO REPORT TO YOU", rep1)
        Assert.DoesNotContain("PLAGUE STRUCK", rep1)

        // Playing with plague
        let plagueState = { state with PlagueStruck = true }
        let rep2 = Hamurabi.formatReport plagueState
        Assert.Contains("A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.", rep2)

        // Impeached
        let impState = { state with Status = Impeached (50u, 50u) }
        let rep3 = Hamurabi.formatReport impState
        Assert.Contains("YOU STARVED 50 PEOPLE IN ONE YEAR!!!", rep3)
        Assert.Contains("IMPEACHED", rep3)

        // Extinction
        let extState = { state with Status = GameOverExtinction }
        let rep4 = Hamurabi.formatReport extState
        Assert.Equal("EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n", rep4)

        // Won Charlemagne
        let wonCharState = {
            state with
                Year = 11uy
                Population = 100u
                Acres = 1000u
                PercentStarvedSum = 20u
                StarvedTotal = 5u
                Status = Won Charlemagne
        }
        let rep5 = Hamurabi.formatReport wonCharState
        Assert.Contains("CHARLEMAGNE, DISRAELI, AND JEFFERSON", rep5)

        // Won Acre
        let wonAcreState = { wonCharState with Status = Won Acre }
        let rep6 = Hamurabi.formatReport wonAcreState
        Assert.Contains("REASONABLE ACREAGE", rep6)

        // Won Nero
        let wonNeroState = { wonCharState with Status = Won Nero }
        let rep7 = Hamurabi.formatReport wonNeroState
        Assert.Contains("NERO WOULD HAVE BEEN PROUD", rep7)

        // Won Disaster
        let wonDisasterState = { wonCharState with Status = Won Disaster }
        let rep8 = Hamurabi.formatReport wonDisasterState
        Assert.Contains("OFFICIAL NATIONAL DISASTER", rep8)

    [<Fact>]
    let ``Extinction condition in step year`` () =
        let state = { Hamurabi.newGame 1UL with Population = 0u; Bushels = 1000u }
        let order = { AcresDelta = 0; BushelsToFeed = 0u; AcresToPlant = 0u }
        match Hamurabi.stepYear state order with
        | Ok next -> Assert.Equal(GameOverExtinction, next.Status)
        | Error err -> Assert.Fail $"Unexpected error: %A{err}"

    [<Fact>]
    let ``Land buy and sell step year execution`` () =
        let state = Hamurabi.newGame 1337UL
        let buyOrder = { AcresDelta = 10; BushelsToFeed = 2000u; AcresToPlant = 500u }
        let buyRes = Hamurabi.stepYear state buyOrder
        Assert.True(match buyRes with Ok s -> s.Acres = 1010u | Error _ -> false)

        let sellOrder = { AcresDelta = -50; BushelsToFeed = 2000u; AcresToPlant = 500u }
        let sellRes = Hamurabi.stepYear state sellOrder
        Assert.True(match sellRes with Ok s -> s.Acres = 950u | Error _ -> false)

    [<Fact>]
    let ``Aliases test`` () =
        let s = Hamurabi.new_game 42UL
        let o = { AcresDelta = 0; BushelsToFeed = 2000u; AcresToPlant = 1000u }
        let v = Hamurabi.validate_order s o
        Assert.Equal(Ok (), v)
        let step = Hamurabi.step_year s o
        match step with
        | Ok next ->
            Assert.Equal(Charlemagne, Hamurabi.rate_reign 0u 12u)
            let rep = Hamurabi.format_report next
            Assert.Equal(rep, Hamurabi.report_year next)
            Assert.Equal(rep, Hamurabi.ReportYear next)
            Assert.Equal(rep, Hamurabi.reportYear next)
        | Error err -> Assert.Fail $"Unexpected error: %A{err}"
