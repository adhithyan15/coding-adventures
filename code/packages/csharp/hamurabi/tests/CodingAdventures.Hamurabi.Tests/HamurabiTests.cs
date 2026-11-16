namespace CodingAdventures.Hamurabi.Tests;

using Xunit;

public class HamurabiTests
{
    [Fact]
    public void TestInitialStateInvariant()
    {
        var state = Hamurabi.NewGame(1337);
        Assert.Equal(1, state.Year);
        Assert.Equal(100u, state.Population);
        Assert.Equal(2800u, state.Bushels);
        Assert.Equal(1000u, state.Acres);
        Assert.InRange(state.LandPrice, 17u, 26u);
        Assert.Equal(3u, state.HarvestPerAcre);
        Assert.Equal(200u, state.RatsEaten);
        Assert.Equal(0u, state.StarvedLastYear);
        Assert.Equal(0u, state.StarvedTotal);
        Assert.Equal(0u, state.PercentStarvedSum);
        Assert.Equal(5u, state.ImmigrantsLastYear);
        Assert.False(state.PlagueStruck);
        Assert.IsType<GameStatus.Playing>(state.Status);
        Assert.Equal(1u, (uint)(state.RngInc & 1UL));
    }

    [Fact]
    public void TestAllEightErrorArms()
    {
        var state = Hamurabi.NewGame(1337);
        var normalOrder = new TurnOrder(0, 2000, 100);

        // Arm 1: GameAlreadyEnded
        var wonState = state with { Status = new GameStatus.Won(Rating.Charlemagne) };
        Assert.Equal(HamurabiError.GameAlreadyEnded, Hamurabi.ValidateOrder(wonState, normalOrder).Error);
        Assert.Equal(HamurabiError.GameAlreadyEnded, Hamurabi.StepYear(wonState, normalOrder).Error);

        var extinctState = state with { Status = new GameStatus.GameOverExtinction() };
        Assert.Equal(HamurabiError.GameAlreadyEnded, Hamurabi.ValidateOrder(extinctState, normalOrder).Error);

        var impeachedState = state with { Status = new GameStatus.Impeached(50, 50) };
        Assert.Equal(HamurabiError.GameAlreadyEnded, Hamurabi.ValidateOrder(impeachedState, normalOrder).Error);

        // Arm 2: InvalidOrderBounds (corrupted land price or arithmetic overflow on trade)
        var badPriceState = state with { LandPrice = 10 };
        Assert.Equal(HamurabiError.InvalidOrderBounds, Hamurabi.ValidateOrder(badPriceState, normalOrder).Error);

        var badYear0State = state with { Year = 0 };
        Assert.Equal(HamurabiError.InvalidOrderBounds, Hamurabi.ValidateOrder(badYear0State, normalOrder).Error);

        var badYear11State = state with { Year = 11 };
        Assert.Equal(HamurabiError.InvalidOrderBounds, Hamurabi.ValidateOrder(badYear11State, normalOrder).Error);

        var badHarvest0State = state with { HarvestPerAcre = 0 };
        Assert.Equal(HamurabiError.InvalidOrderBounds, Hamurabi.ValidateOrder(badHarvest0State, normalOrder).Error);

        var badHarvest6State = state with { HarvestPerAcre = 6 };
        Assert.Equal(HamurabiError.InvalidOrderBounds, Hamurabi.ValidateOrder(badHarvest6State, normalOrder).Error);

        var badEvenIncState = state with { RngInc = 0 };
        Assert.Equal(HamurabiError.InvalidOrderBounds, Hamurabi.ValidateOrder(badEvenIncState, normalOrder).Error);

        var badEvenInc2State = state with { RngInc = 108 };
        Assert.Equal(HamurabiError.InvalidOrderBounds, Hamurabi.ValidateOrder(badEvenInc2State, normalOrder).Error);

        // Revenue overflow on land sale
        var richLandState = state with { Acres = 2_000_000_000, Bushels = 4_000_000_000 };
        var overflowSell = new TurnOrder(-1_000_000_000, 0, 0);
        Assert.Equal(HamurabiError.InvalidOrderBounds, Hamurabi.ValidateOrder(richLandState, overflowSell).Error);

        // Acres overflow on land purchase
        var richAcresState = state with { Acres = uint.MaxValue - 10, Bushels = uint.MaxValue };
        var overflowBuy = new TurnOrder(20, 0, 0);
        Assert.Equal(HamurabiError.InvalidOrderBounds, Hamurabi.ValidateOrder(richAcresState, overflowBuy).Error);

        // Arm 3: InsufficientBushelsForLand
        var buyTooMuch = new TurnOrder(200, 0, 0);
        Assert.Equal(HamurabiError.InsufficientBushelsForLand, Hamurabi.ValidateOrder(state, buyTooMuch).Error);

        // Arm 4: InsufficientAcresToSell
        var sellTooMuch = new TurnOrder(-1001, 0, 0);
        Assert.Equal(HamurabiError.InsufficientAcresToSell, Hamurabi.ValidateOrder(state, sellTooMuch).Error);

        // Arm 5: InsufficientBushelsToFeed
        var feedTooMuch = new TurnOrder(0, 2801, 0);
        Assert.Equal(HamurabiError.InsufficientBushelsToFeed, Hamurabi.ValidateOrder(state, feedTooMuch).Error);

        // Arm 6: InsufficientAcresToPlant
        var plantTooMuchAcres = new TurnOrder(0, 2000, 1001);
        Assert.Equal(HamurabiError.InsufficientAcresToPlant, Hamurabi.ValidateOrder(state, plantTooMuchAcres).Error);

        // Arm 7: InsufficientPopulationToPlant
        // Population is 100, so max plantable is 1000. If we buy 50 acres, owned acres is 1050, but plant 1001:
        var richState = state with { Bushels = 50000 };
        var overLabor = new TurnOrder(50, 2000, 1001);
        Assert.Equal(HamurabiError.InsufficientPopulationToPlant, Hamurabi.ValidateOrder(richState, overLabor).Error);

        // Arm 8: InsufficientGrainForSeed
        // Bushels after feed is 100, but planting 300 acres needs 150 bushels
        var seedShort = new TurnOrder(0, 2700, 300);
        Assert.Equal(HamurabiError.InsufficientGrainForSeed, Hamurabi.ValidateOrder(state, seedShort).Error);
    }

    [Fact]
    public void TestInt32MinValueAcresDeltaDoesNotCrash()
    {
        var state = Hamurabi.NewGame(1337);
        var order = new TurnOrder(int.MinValue, 0, 0);

        // Negating int.MinValue without 64-bit widening would throw OverflowException.
        // It should cleanly return InsufficientAcresToSell because 2,147,483,648 > 1000 acres.
        var res = Hamurabi.ValidateOrder(state, order);
        Assert.False(res.IsOk);
        Assert.Equal(HamurabiError.InsufficientAcresToSell, res.Error);
    }

    [Fact]
    public void TestHarvestOverflowRefusal()
    {
        var state = Hamurabi.NewGame(1) with
        {
            Population = 500_000_000,
            Acres = 1_000_000_000,
            Bushels = 2_000_000_000
        };
        // 900M acres * 5 bushels/acre = 4.5B > uint.MaxValue
        var order = new TurnOrder(0, 0, 900_000_000);
        Assert.Equal(HamurabiError.InvalidOrderBounds, Hamurabi.ValidateOrder(state, order).Error);
        Assert.Equal(HamurabiError.InvalidOrderBounds, Hamurabi.StepYear(state, order).Error);
    }

    [Fact]
    public void TestStarvedTotalSaturatingAddition()
    {
        var state = Hamurabi.NewGame(1) with
        {
            StarvedTotal = uint.MaxValue,
            PercentStarvedSum = uint.MaxValue - 10
        };
        var order = new TurnOrder(0, 1000, 0); // Feeds 50 people, starves 50
        var res = Hamurabi.StepYear(state, order);
        Assert.True(res.IsOk);
        Assert.Equal(uint.MaxValue, res.Value.StarvedTotal);
        Assert.Equal(uint.MaxValue, res.Value.PercentStarvedSum);
    }

    [Fact]
    public void TestRateReignAllBranches()
    {
        Assert.Equal(Rating.Charlemagne, Hamurabi.RateReign(3, 10));
        Assert.Equal(Rating.Charlemagne, Hamurabi.RateReign(0, 15));
        Assert.Equal(Rating.Acre, Hamurabi.RateReign(3, 9));
        Assert.Equal(Rating.Acre, Hamurabi.RateReign(10, 9));
        Assert.Equal(Rating.Nero, Hamurabi.RateReign(11, 8));
        Assert.Equal(Rating.Nero, Hamurabi.RateReign(33, 7));
        Assert.Equal(Rating.Disaster, Hamurabi.RateReign(34, 10));
        Assert.Equal(Rating.Disaster, Hamurabi.RateReign(10, 6));
    }

    [Fact]
    public void TestFormatReportAllVariants()
    {
        var state = Hamurabi.NewGame(1);

        // Playing without plague
        var rep1 = Hamurabi.FormatReport(state);
        Assert.Contains("HAMURABI: I BEG TO REPORT TO YOU", rep1);
        Assert.DoesNotContain("PLAGUE STRUCK", rep1);

        // Playing with plague
        var plagueState = state with { PlagueStruck = true };
        var rep2 = Hamurabi.FormatReport(plagueState);
        Assert.Contains("A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.", rep2);

        // Impeached
        var impState = state with { Status = new GameStatus.Impeached(50, 50) };
        var rep3 = Hamurabi.FormatReport(impState);
        Assert.Contains("YOU STARVED 50 PEOPLE IN ONE YEAR!!!", rep3);
        Assert.Contains("IMPEACHED", rep3);

        // Extinction
        var extState = state with { Status = new GameStatus.GameOverExtinction() };
        var rep4 = Hamurabi.FormatReport(extState);
        Assert.Equal("EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n", rep4);

        // Won Charlemagne
        var wonCharState = state with
        {
            Year = 11,
            Population = 100,
            Acres = 1000,
            PercentStarvedSum = 20,
            StarvedTotal = 5,
            Status = new GameStatus.Won(Rating.Charlemagne)
        };
        var rep5 = Hamurabi.FormatReport(wonCharState);
        Assert.Contains("CHARLEMAGNE, DISRAELI, AND JEFFERSON", rep5);

        // Won Acre
        var wonAcreState = wonCharState with { Status = new GameStatus.Won(Rating.Acre) };
        var rep6 = Hamurabi.FormatReport(wonAcreState);
        Assert.Contains("REASONABLE ACREAGE", rep6);

        // Won Nero
        var wonNeroState = wonCharState with { Status = new GameStatus.Won(Rating.Nero) };
        var rep7 = Hamurabi.FormatReport(wonNeroState);
        Assert.Contains("NERO WOULD HAVE BEEN PROUD", rep7);

        // Won Disaster
        var wonDisasterState = wonCharState with { Status = new GameStatus.Won(Rating.Disaster) };
        var rep8 = Hamurabi.FormatReport(wonDisasterState);
        Assert.Contains("OFFICIAL NATIONAL DISASTER", rep8);
    }

    [Fact]
    public void TestExtinctionConditionInStepYear()
    {
        // When everyone starves: order gives 0 bushels to feed, population was 100, so starved = 100.
        // Wait, starved 100% > 45% so that's impeached.
        // What about when population is starved to 0 without impeachment?
        // Wait, starved * 100 > population * 45 triggers impeachment whenever > 45%.
        // What if survivors == 0 and not impeached? (e.g. population was 0, or starved * 100 <= population * 45 with survivors == 0)
        // If population is 0, survivors is 0 and 0 * 100 > 0 * 45 is false, leading to GameOverExtinction!
        var state = Hamurabi.NewGame(1) with { Population = 0, Bushels = 1000 };
        var order = new TurnOrder(0, 0, 0);
        var res = Hamurabi.StepYear(state, order);
        Assert.True(res.IsOk);
        Assert.IsType<GameStatus.GameOverExtinction>(res.Value.Status);
    }

    [Fact]
    public void TestAliases()
    {
        var s = Hamurabi.new_game(42);
        var o = new TurnOrder(0, 2000, 1000);
        var v = Hamurabi.validate_order(s, o);
        Assert.True(v.IsOk);
        var step = Hamurabi.step_year(s, o);
        Assert.True(step.IsOk);
        Assert.Equal(Rating.Charlemagne, Hamurabi.rate_reign(0, 12));
        var rep = Hamurabi.format_report(step.Value);
        Assert.Equal(rep, Hamurabi.report_year(step.Value));
        Assert.Equal(rep, Hamurabi.ReportYear(step.Value));

        // HamurabiGame aliases
        var s2 = HamurabiGame.NewGame(42);
        Assert.Equal(s, s2);
        Assert.True(HamurabiGame.ValidateOrder(s2, o).IsOk);
        Assert.True(HamurabiGame.StepYear(s2, o).IsOk);
        Assert.Equal(Rating.Acre, HamurabiGame.RateReign(5, 9));
        Assert.Equal(rep, HamurabiGame.FormatReport(step.Value));
    }
}
